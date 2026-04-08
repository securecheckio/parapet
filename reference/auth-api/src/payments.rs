use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use solana_sdk::signature::Signature;
use sqlx::PgPool;
use std::str::FromStr;

// Default credits per package (can be overridden via env)
pub const DEFAULT_CREDITS: &[(&str, i64)] = &[
    ("small", 100_000),      // 100k requests
    ("medium", 500_000),     // 500k requests  
    ("large", 1_000_000),    // 1M requests
    ("xlarge", 5_000_000),   // 5M requests
];

fn get_token_amount_from_env(package: &str) -> Option<u64> {
    let env_key = format!("CREDITS_PRICE_{}", package.to_uppercase());
    std::env::var(&env_key)
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
}

fn get_credits_from_env(package: &str) -> Option<i64> {
    let env_key = format!("CREDITS_AMOUNT_{}", package.to_uppercase());
    std::env::var(&env_key)
        .ok()
        .and_then(|v| v.parse::<i64>().ok())
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Payment {
    pub id: uuid::Uuid,
    pub user_id: uuid::Uuid,
    pub tier: String,
    pub amount: i64,
    pub token_type: String,
    pub credits_purchased: Option<i64>,
    pub signature: Option<String>,
    pub status: String, // pending, completed, failed
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Serialize)]
pub struct PaymentRequest {
    pub payment_id: String,
    pub recipient: String,
    pub amount: u64,
    pub spl_token: String,
    pub reference: String,
    pub label: String,
    pub message: String,
}

impl PaymentRequest {
    pub fn to_url(&self) -> String {
        format!(
            "solana:{}?amount={}&spl-token={}&reference={}&label={}&message={}",
            self.recipient,
            self.amount,
            self.spl_token,
            self.reference,
            urlencoding::encode(&self.label),
            urlencoding::encode(&self.message)
        )
    }
}

pub async fn create_payment_request(
    db: &PgPool,
    user_id: uuid::Uuid,
    package: &str,
    token_type: &str,
) -> Result<PaymentRequest> {
    // Get token config from env
    let (token_mint, token_name) = match token_type {
        "xlabs" | "payment" => (
            std::env::var("PAYMENT_TOKEN_MINT")
                .unwrap_or_else(|_| "7B2tQy8DwYt6aXHzt6UVDuqBB6WmykyZQodLSReQ9Wcz".to_string()),
            std::env::var("PAYMENT_TOKEN_NAME")
                .unwrap_or_else(|_| "xLABS".to_string()),
        ),
        "usdc" => {
            // USDC support coming soon for advanced features
            return Err(anyhow::anyhow!("USDC payments coming soon for advanced features"));
        }
        _ => return Err(anyhow::anyhow!("Unsupported token type")),
    };

    let treasury = std::env::var("TREASURY_WALLET")
        .expect("TREASURY_WALLET must be set");

    // Get pricing for package (env overrides defaults)
    let amount = get_token_amount_from_env(package)
        .ok_or_else(|| anyhow::anyhow!("Invalid package or missing price config: {}", package))?;
    
    let credits = get_credits_from_env(package)
        .or_else(|| DEFAULT_CREDITS.iter().find(|(p, _)| *p == package).map(|(_, c)| *c))
        .ok_or_else(|| anyhow::anyhow!("Invalid package"))?;

    // Create payment record
    let payment_id = sqlx::query_scalar::<_, uuid::Uuid>(
        "INSERT INTO payments (user_id, tier, amount, token_type, credits_purchased, status) 
         VALUES ($1, $2, $3, $4, $5, 'pending') 
         RETURNING id",
    )
    .bind(&user_id)
    .bind(package)
    .bind(amount as i64)
    .bind(token_type)
    .bind(credits)
    .fetch_one(db)
    .await?;

    // Create Solana Pay request
    Ok(PaymentRequest {
        payment_id: payment_id.to_string(),
        recipient: treasury,
        amount,
        spl_token: token_mint,
        reference: payment_id.to_string(),
        label: format!("SecureCheck RPC Credits ({})", token_name),
        message: format!("{} requests", format_number(credits)),
    })
}

pub async fn verify_payment(
    db: &PgPool,
    payment_id: uuid::Uuid,
    signature: &str,
    rpc_url: &str,
) -> Result<bool> {
    // Parse signature
    let sig = Signature::from_str(signature)
        .map_err(|e| anyhow::anyhow!("Invalid signature: {}", e))?;

    // Verify transaction on-chain
    let verified = verify_transaction_on_chain(rpc_url, &sig, payment_id).await?;

    if verified {
        // Get payment details
        let payment: Payment = sqlx::query_as(
            "SELECT * FROM payments WHERE id = $1"
        )
        .bind(&payment_id)
        .fetch_one(db)
        .await?;

        let credits = payment.credits_purchased
            .ok_or_else(|| anyhow::anyhow!("Payment missing credits_purchased"))?;

        // Update payment status and add credits to user (transaction)
        let mut tx = db.begin().await?;

        sqlx::query(
            "UPDATE payments 
             SET status = 'completed', signature = $1, completed_at = $2 
             WHERE id = $3",
        )
        .bind(signature)
        .bind(Utc::now())
        .bind(&payment_id)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            "UPDATE users 
             SET credits_balance = credits_balance + $1, updated_at = NOW() 
             WHERE id = $2"
        )
        .bind(credits)
        .bind(&payment.user_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        log::info!(
            "✅ Payment verified and credits added: user={} credits={}",
            payment.user_id,
            credits
        );

        Ok(true)
    } else {
        // Mark as failed
        sqlx::query(
            "UPDATE payments SET status = 'failed' WHERE id = $1"
        )
        .bind(&payment_id)
        .execute(db)
        .await?;

        Ok(false)
    }
}

async fn verify_transaction_on_chain(
    rpc_url: &str,
    signature: &Signature,
    _reference: uuid::Uuid,
) -> Result<bool> {
    use solana_client::rpc_client::RpcClient;
    use solana_sdk::commitment_config::CommitmentConfig;
    use solana_transaction_status::UiTransactionEncoding;

    let client = RpcClient::new_with_commitment(rpc_url.to_string(), CommitmentConfig::confirmed());

    // Get transaction
    let transaction = client
        .get_transaction_with_config(
            signature,
            solana_client::rpc_config::RpcTransactionConfig {
                encoding: Some(UiTransactionEncoding::Json),
                commitment: Some(CommitmentConfig::confirmed()),
                max_supported_transaction_version: Some(0),
            },
        )
        .map_err(|e| anyhow::anyhow!("Transaction not found: {}", e))?;

    // Verify transaction exists and was successful
    if let Some(meta) = transaction.transaction.meta {
        if meta.err.is_none() {
            // Transaction succeeded
            // In production, parse transaction to verify:
            // 1. Transfer to treasury wallet
            // 2. Correct token (xLABS)
            // 3. Correct amount
            // 4. Reference in account keys or memo
            return Ok(true);
        }
    }

    Ok(false)
}

pub fn get_package_info(package: &str) -> Option<(u64, i64)> {
    let amount = get_token_amount_from_env(package)?;
    let credits = get_credits_from_env(package)
        .or_else(|| DEFAULT_CREDITS.iter().find(|(p, _)| *p == package).map(|(_, c)| *c))?;
    Some((amount, credits))
}

pub fn format_token_amount(amount: u64) -> String {
    // Get decimals from env (default 6)
    let decimals = std::env::var("PAYMENT_TOKEN_DECIMALS")
        .ok()
        .and_then(|d| d.parse::<u32>().ok())
        .unwrap_or(6);
    
    let divisor = 10_u64.pow(decimals);
    let major = amount / divisor;
    let minor = amount % divisor;
    
    format!("{}.{:0width$}", major, minor, width = decimals as usize)
}

pub fn format_xlabs_amount(amount: u64) -> String {
    // Backward compatibility - just use token formatter
    format_token_amount(amount)
}

fn format_number(n: i64) -> String {
    if n >= 1_000_000 {
        format!("{}M", n / 1_000_000)
    } else if n >= 1_000 {
        format!("{}k", n / 1_000)
    } else {
        n.to_string()
    }
}
