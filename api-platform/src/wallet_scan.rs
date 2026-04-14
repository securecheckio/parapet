use axum::{
    extract::{Json, State},
    http::{HeaderMap, StatusCode},
};
use parapet_scanner::{ScanConfig, ScanReport, WalletScanner};
use serde::{Deserialize, Serialize};

use crate::state::PlatformState;

#[derive(Debug, Deserialize)]
pub struct WalletScanRequest {
    pub wallet: String,
    #[serde(default)]
    pub max_transactions: Option<usize>,
    #[serde(default)]
    pub time_window_days: Option<u32>,
    #[serde(default)]
    pub rpc_delay_ms: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct WalletScanResponse {
    pub report: ScanReport,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

/// Scan a wallet for security threats
/// POST /wallet/scan
/// Requires: API key in X-API-Key header
pub async fn scan_wallet(
    State(state): State<PlatformState>,
    headers: HeaderMap,
    Json(req): Json<WalletScanRequest>,
) -> Result<Json<WalletScanResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Verify API key
    let api_key = headers
        .get("X-API-Key")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse {
                    error: "Missing or invalid API key".to_string(),
                }),
            )
        })?;

    // Verify API key against database.
    let api_key_hash = crate::auth::hash_api_key(api_key);
    let is_valid = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(
            SELECT 1 FROM users WHERE api_key_hash = $1 AND active = true
            UNION
            SELECT 1 FROM api_keys WHERE key_hash = $1
        )",
    )
    .bind(&api_key_hash)
    .fetch_one(&state.db)
    .await
    .map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Failed to validate API key".to_string(),
            }),
        )
    })?;
    if !is_valid {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "Invalid API key".to_string(),
            }),
        ));
    }

    log::info!("Wallet scan request authenticated");

    // Get RPC URL from environment
    let rpc_url = std::env::var("SOLANA_RPC_URL")
        .unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".to_string());

    // Create basic scanner (no analyzers for now - will add in next phase)
    let scanner = WalletScanner::new(rpc_url).map_err(|e| {
        log::error!("Failed to create scanner: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Failed to initialize scanner".to_string(),
            }),
        )
    })?;

    // Build scan config with RPC throttling
    let config = ScanConfig {
        max_transactions: req.max_transactions.or(Some(100)),
        time_window_days: req.time_window_days.or(Some(30)),
        rpc_delay_ms: req.rpc_delay_ms.unwrap_or(150), // Safe default (~6 req/sec)
        check_active_threats: true,
        check_historical: false, // Disabled for basic scanner without analyzers
        commitment: solana_sdk::commitment_config::CommitmentConfig::confirmed(),
    };

    log::info!(
        "Scanning wallet {} (max_tx: {:?}, window_days: {:?})",
        req.wallet,
        config.max_transactions,
        config.time_window_days
    );

    // Perform scan
    let report = scanner.scan(&req.wallet, config).await.map_err(|e| {
        log::error!("Wallet scan failed for {}: {}", req.wallet, e);
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: format!("Scan failed: {}", e),
            }),
        )
    })?;

    log::info!(
        "Scan complete for {}: security_score={}, threats={}",
        req.wallet,
        report.security_score,
        report.threats.len()
    );

    Ok(Json(WalletScanResponse { report }))
}
