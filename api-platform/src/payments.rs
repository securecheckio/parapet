use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use solana_sdk::message::VersionedMessage;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signature;
use solana_transaction_status::option_serializer::OptionSerializer;
use solana_transaction_status::{
    EncodedTransaction, UiTransactionEncoding, UiTransactionStatusMeta, UiTransactionTokenBalance,
};
use sqlx::PgPool;
use std::collections::HashMap;
use std::str::FromStr;

// Default credits per package (can be overridden via env)
pub const DEFAULT_CREDITS: &[(&str, i64)] = &[
    ("small", 100_000),    // 100k requests
    ("medium", 500_000),   // 500k requests
    ("large", 1_000_000),  // 1M requests
    ("xlarge", 5_000_000), // 5M requests
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
            std::env::var("PAYMENT_TOKEN_NAME").unwrap_or_else(|_| "xLABS".to_string()),
        ),
        "usdc" => {
            // USDC support coming soon for advanced features
            return Err(anyhow::anyhow!(
                "USDC payments coming soon for advanced features"
            ));
        }
        _ => return Err(anyhow::anyhow!("Unsupported token type")),
    };

    let treasury = std::env::var("TREASURY_WALLET").expect("TREASURY_WALLET must be set");

    // Get pricing for package (env overrides defaults)
    let amount = get_token_amount_from_env(package)
        .ok_or_else(|| anyhow::anyhow!("Invalid package or missing price config: {}", package))?;

    let credits = get_credits_from_env(package)
        .or_else(|| {
            DEFAULT_CREDITS
                .iter()
                .find(|(p, _)| *p == package)
                .map(|(_, c)| *c)
        })
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
    let payment: Payment = sqlx::query_as("SELECT * FROM payments WHERE id = $1")
        .bind(&payment_id)
        .fetch_one(db)
        .await?;

    if payment.status != "pending" {
        return Err(anyhow::anyhow!(
            "Payment {} is not pending (status={})",
            payment_id,
            payment.status
        ));
    }

    // Parse signature
    let sig =
        Signature::from_str(signature).map_err(|e| anyhow::anyhow!("Invalid signature: {}", e))?;

    // Verify transaction on-chain (mint, treasury, amount, reference)
    let verified = verify_transaction_on_chain(rpc_url, &sig, &payment).await?;

    if verified {
        let credits = payment
            .credits_purchased
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
             WHERE id = $2",
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
        sqlx::query("UPDATE payments SET status = 'failed' WHERE id = $1")
            .bind(&payment_id)
            .execute(db)
            .await?;

        Ok(false)
    }
}

async fn verify_transaction_on_chain(
    rpc_url: &str,
    signature: &Signature,
    payment: &Payment,
) -> Result<bool> {
    use solana_client::rpc_client::RpcClient;
    use solana_sdk::commitment_config::CommitmentConfig;

    let treasury = std::env::var("TREASURY_WALLET")
        .map_err(|_| anyhow::anyhow!("TREASURY_WALLET must be set for payment verification"))?;
    let mint_str = match payment.token_type.as_str() {
        "xlabs" | "payment" => std::env::var("PAYMENT_TOKEN_MINT").unwrap_or_else(|_| {
            "7B2tQy8DwYt6aXHzt6UVDuqBB6WmykyZQodLSReQ9Wcz".to_string()
        }),
        other => {
            return Err(anyhow::anyhow!(
                "Payment verification not implemented for token_type={}",
                other
            ));
        }
    };

    let expected_amount = u64::try_from(payment.amount)
        .map_err(|_| anyhow::anyhow!("Invalid payment amount in database"))?;
    let payment_ref = payment.id.to_string();

    let client = RpcClient::new_with_commitment(rpc_url.to_string(), CommitmentConfig::confirmed());

    // Base64 encoding allows decoding the wire transaction for memo/reference checks.
    let enc = client
        .get_transaction_with_config(
            signature,
            solana_client::rpc_config::RpcTransactionConfig {
                encoding: Some(UiTransactionEncoding::Base64),
                commitment: Some(CommitmentConfig::confirmed()),
                max_supported_transaction_version: Some(0),
            },
        )
        .map_err(|e| anyhow::anyhow!("Transaction not found: {}", e))?;

    let meta = enc
        .transaction
        .meta
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Transaction meta missing"))?;

    if meta.err.is_some() {
        return Ok(false);
    }

    if !treasury_token_credit_matches(meta, &treasury, &mint_str, expected_amount) {
        log::warn!("Payment verification: token credit to treasury did not match");
        return Ok(false);
    }

    let enc_tx = &enc.transaction.transaction;
    if !payment_reference_verified(meta, enc_tx, &payment_ref) {
        log::warn!("Payment verification: payment reference not found in logs or memos");
        return Ok(false);
    }

    Ok(true)
}

fn token_balances_slice(
    b: &OptionSerializer<Vec<UiTransactionTokenBalance>>,
) -> &[UiTransactionTokenBalance] {
    match b {
        OptionSerializer::Some(v) => v.as_slice(),
        _ => &[],
    }
}

fn owner_matches_treasury(owner: &OptionSerializer<String>, treasury: &str) -> bool {
    match owner {
        OptionSerializer::Some(s) => s == treasury,
        _ => false,
    }
}

/// Returns net increase in token amount (smallest units) for the treasury owner's account for `mint`.
fn treasury_token_credit_matches(
    meta: &UiTransactionStatusMeta,
    treasury: &str,
    mint: &str,
    expected: u64,
) -> bool {
    let pre = token_balances_slice(&meta.pre_token_balances);
    let post = token_balances_slice(&meta.post_token_balances);

    let mut pre_map: HashMap<u8, u64> = HashMap::new();
    for b in pre {
        if b.mint == mint && owner_matches_treasury(&b.owner, treasury) {
            if let Ok(a) = b.ui_token_amount.amount.parse::<u64>() {
                pre_map.insert(b.account_index, a);
            }
        }
    }

    let mut best_delta = 0u64;
    for b in post {
        if b.mint == mint && owner_matches_treasury(&b.owner, treasury) {
            let post_amt = match b.ui_token_amount.amount.parse::<u64>() {
                Ok(a) => a,
                Err(_) => continue,
            };
            let prev = pre_map.get(&b.account_index).copied().unwrap_or(0);
            if post_amt > prev {
                best_delta = best_delta.max(post_amt - prev);
            }
        }
    }

    best_delta == expected
}

fn logs_joined(meta: &UiTransactionStatusMeta) -> String {
    match &meta.log_messages {
        OptionSerializer::Some(logs) => logs.join("\n"),
        _ => String::new(),
    }
}

fn payment_reference_verified(
    meta: &UiTransactionStatusMeta,
    enc_tx: &EncodedTransaction,
    payment_ref: &str,
) -> bool {
    if logs_joined(meta).contains(payment_ref) {
        return true;
    }
    memo_contains_payment_ref(enc_tx, payment_ref)
}

fn memo_contains_payment_ref(enc_tx: &EncodedTransaction, payment_ref: &str) -> bool {
    let Some(vtx) = enc_tx.decode() else {
        return false;
    };
    let memo_program: Pubkey = solana_sdk::pubkey!("Memo1UhkJRfHyvLMcVucJwxXeuD728EqVDDwQDxFMNo");
    let keys = vtx.message.static_account_keys();
    let instructions = match &vtx.message {
        VersionedMessage::Legacy(m) => m.instructions.as_slice(),
        VersionedMessage::V0(m) => m.instructions.as_slice(),
    };
    for ix in instructions {
        let Some(program_id) = keys.get(ix.program_id_index as usize) else {
            continue;
        };
        if program_id == &memo_program {
            if let Ok(s) = std::str::from_utf8(&ix.data) {
                if s.contains(payment_ref) {
                    return true;
                }
            }
        }
    }
    false
}

pub fn get_package_info(package: &str) -> Option<(u64, i64)> {
    let amount = get_token_amount_from_env(package)?;
    let credits = get_credits_from_env(package).or_else(|| {
        DEFAULT_CREDITS
            .iter()
            .find(|(p, _)| *p == package)
            .map(|(_, c)| *c)
    })?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_info_uses_env_overrides() {
        std::env::set_var("CREDITS_PRICE_SMALL", "1234");
        std::env::set_var("CREDITS_AMOUNT_SMALL", "5678");
        let info = get_package_info("small").expect("package should resolve");
        assert_eq!(info, (1234, 5678));
        std::env::remove_var("CREDITS_PRICE_SMALL");
        std::env::remove_var("CREDITS_AMOUNT_SMALL");
    }

    #[test]
    fn token_amount_format_respects_decimals_env() {
        std::env::set_var("PAYMENT_TOKEN_DECIMALS", "3");
        assert_eq!(format_token_amount(12_345), "12.345");
        std::env::remove_var("PAYMENT_TOKEN_DECIMALS");
    }

    #[test]
    fn solana_pay_url_contains_reference_and_token() {
        let req = PaymentRequest {
            payment_id: "pid".to_string(),
            recipient: "wallet123".to_string(),
            amount: 1000,
            spl_token: "mint123".to_string(),
            reference: "ref123".to_string(),
            label: "SecureCheck".to_string(),
            message: "100k requests".to_string(),
        };
        let url = req.to_url();
        assert!(url.contains("solana:wallet123?amount=1000"));
        assert!(url.contains("spl-token=mint123"));
        assert!(url.contains("reference=ref123"));
    }
}
