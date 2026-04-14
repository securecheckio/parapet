use anyhow::Result;
use axum::{
    extract::{Json, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post, put},
    Router,
};
use axum_extra::extract::cookie::{Cookie, CookieJar};
use clap::Parser;
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use tower_http::cors::CorsLayer;

mod auth;
mod config;
mod learning;
mod payments;
mod push;
mod session;
mod state;
mod wallet_scan;
mod websocket;

use state::PlatformState;

#[derive(Parser)]
#[command(name = "parapet-platform")]
#[command(about = "Parapet Platform - Full-featured API with multi-user dashboard")]
struct Cli {
    /// Path to base API config file (default: ./config.toml)
    #[arg(short, long, default_value = "config.toml")]
    config: String,

    /// Path to platform config file (default: ./platform-config.toml)
    #[arg(short, long, default_value = "platform-config.toml")]
    platform_config: String,
}

fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    env_logger::init();

    log::info!("🚀 Starting Parapet Platform");

    let cli = Cli::parse();

    // Load base API config
    let api_config = parapet_api_core::config::load_config_from_file(&cli.config)?;
    log::info!("✅ Loaded API config from {}", cli.config);

    // Load platform-specific config
    let platform_config = config::load_platform_config_from_file(&cli.platform_config)?;
    log::info!("✅ Loaded platform config from {}", cli.platform_config);

    let server_addr = format!("{}:{}", api_config.server_host, api_config.server_port);
    let frontend_url = platform_config.frontend_url.clone();

    // Build Tokio runtime with configured worker threads
    let mut builder = tokio::runtime::Builder::new_multi_thread();
    builder.enable_all();

    if let Some(threads) = api_config.worker_threads {
        log::info!("🧵 Configuring {} worker threads", threads);
        builder.worker_threads(threads);
    }

    let runtime = builder.build()?;

    runtime.block_on(async move {
        // Initialize extended state
        let state = PlatformState::new(api_config, platform_config).await?;

        // Build router: core API routes + platform routes
        let core_router = parapet_api_core::create_router(state.clone());
        let platform_router = Router::new()
            .route("/health", get(health_check))
            .route("/vapid-public-key", get(get_vapid_public_key))
            // Session-based auth endpoints (for dashboard)
            .route("/auth/login", post(login))
            .route("/auth/me", get(get_current_user))
            .route("/auth/logout", post(logout))
            .route("/auth/api-key", get(get_my_api_key))
            .route("/auth/api-key/regenerate", post(regenerate_my_api_key))
            // Session-protected dashboard endpoints
            .route("/dashboard/stats", get(get_my_stats))
            .route("/dashboard/events", get(get_my_events))
            .route("/dashboard/usage", get(get_my_usage))
            .route("/dashboard/rules", get(get_active_rules))
            .route("/dashboard/threshold", put(update_blocking_threshold))
            .route("/dashboard/notifications", put(toggle_notifications))
            .route("/dashboard/push/subscribe", post(push::subscribe_push))
            .route("/dashboard/ws", get(websocket::websocket_handler))
            .route("/system/network", get(get_network_info))
            // Learning system endpoints (public and session-protected)
            .route("/learn/courses", get(learning::list_courses))
            .route("/learn/courses/:course_id", get(learning::get_course_by_id))
            .route(
                "/learn/courses/slug/:slug",
                get(learning::get_course_by_slug),
            )
            .route("/learn/badges", get(learning::list_badges))
            .route(
                "/learn/badges/course/:course_id",
                get(learning::get_course_badges),
            )
            // Session-protected learning endpoints
            .route("/learn/progress/me", get(learning::get_my_progress))
            .route(
                "/learn/progress/course/:course_id",
                get(learning::get_my_course_progress),
            )
            .route(
                "/learn/progress/course/:course_id",
                put(learning::update_my_course_progress),
            )
            .route("/learn/badges/me", get(learning::get_my_badges))
            // Legacy API key endpoints (for programmatic access)
            .route("/signup", post(signup))
            .route("/usage", get(get_usage))
            .route("/api-keys", get(list_api_keys))
            .route("/api-keys", post(create_api_key))
            .route("/payment/create", post(create_payment))
            .route("/payment/verify", post(verify_payment_handler))
            .route("/payment/pricing", get(get_pricing))
            .route("/stats/user/:api_key", get(get_user_stats))
            .route("/stats/global", get(get_global_stats))
            .route("/stats/events/:api_key", get(get_security_events))
            .route("/events/update-signature", post(update_event_signature))
            .route("/internal/push/send", post(push::internal_send_push))
            // Wallet security scanner endpoint
            .route("/wallet/scan", post(wallet_scan::scan_wallet))
            .with_state(state.clone());

        // Merge core + platform routes
        let app = core_router.merge(platform_router).layer(
            CorsLayer::new()
                .allow_origin(
                    frontend_url
                        .parse::<axum::http::HeaderValue>()
                        .expect("Invalid FRONTEND_URL"),
                )
                .allow_methods([
                    axum::http::Method::GET,
                    axum::http::Method::POST,
                    axum::http::Method::PUT,
                    axum::http::Method::DELETE,
                    axum::http::Method::OPTIONS,
                ])
                .allow_headers([
                    axum::http::header::CONTENT_TYPE,
                    axum::http::header::AUTHORIZATION,
                ])
                .allow_credentials(true),
        );

        log::info!("🔐 CORS configured for frontend: {}", frontend_url);
        log::info!("📡 Platform listening on http://{}", server_addr);
        log::info!("✅ Core API routes: /api/v1/*, /mcp/*, /ws/escalations");
        log::info!("✅ Platform routes: /auth/*, /dashboard/*, /learn/*, /wallet/*");

        let listener = tokio::net::TcpListener::bind(&server_addr).await?;
        axum::serve(listener, app).await?;

        Ok::<(), anyhow::Error>(())
    })
}

async fn health_check() -> impl IntoResponse {
    Json(serde_json::json!({ "status": "ok" }))
}

async fn get_vapid_public_key(
    State(state): State<PlatformState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    match &state.platform_config.push_notifications.public_key {
        Some(key) => Ok(Json(serde_json::json!({ "publicKey": key }))),
        None => {
            log::error!("VAPID public key not configured");
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[derive(Serialize)]
struct NetworkInfo {
    network: String,
}

async fn get_network_info(State(state): State<PlatformState>) -> impl IntoResponse {
    Json(NetworkInfo {
        network: state.config.solana_network.clone(),
    })
}

#[derive(Deserialize)]
struct SignupRequest {
    wallet_address: String,
    message: String,
    signature: String,
}

#[derive(Serialize)]
struct SignupResponse {
    user_id: String,
    api_key: String,
}

async fn signup(
    State(state): State<PlatformState>,
    Json(req): Json<SignupRequest>,
) -> Result<Json<SignupResponse>, AppError> {
    // Verify wallet signature
    auth::verify_wallet_signature(&req.wallet_address, &req.message, &req.signature)
        .map_err(|_| AppError::BadRequest("Invalid signature".into()))?;

    // Generate API key
    let api_key = auth::generate_api_key();
    let api_key_hash = auth::hash_api_key(&api_key);

    // Insert or update user
    let user_id = sqlx::query_scalar::<_, uuid::Uuid>(
        "INSERT INTO users (wallet_address, api_key_hash, tier) 
         VALUES ($1, $2, 'free') 
         ON CONFLICT (wallet_address) 
         DO UPDATE SET api_key_hash = $2, updated_at = NOW()
         RETURNING id",
    )
    .bind(&req.wallet_address)
    .bind(&api_key_hash)
    .fetch_one(&state.db)
    .await
    .map_err(|_| AppError::Internal)?;

    log::info!("✅ Wallet registered: {}", req.wallet_address);

    Ok(Json(SignupResponse {
        user_id: user_id.to_string(),
        api_key,
    }))
}

#[derive(Serialize)]
struct UsageResponse {
    total_requests: i64,
    blocked_requests: i64,
    current_month_requests: i64,
}

async fn get_usage(
    State(state): State<PlatformState>,
    Json(req): Json<serde_json::Value>,
) -> Result<Json<UsageResponse>, AppError> {
    let api_key = req["api_key"]
        .as_str()
        .ok_or(AppError::BadRequest("Missing api_key".into()))?;

    let api_key_hash = auth::hash_api_key(api_key);
    let user_id = sqlx::query_scalar::<_, uuid::Uuid>(
        "SELECT id FROM users WHERE api_key_hash = $1 AND active = true",
    )
    .bind(&api_key_hash)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::Unauthorized)?;

    // Consolidate usage counts into parallel queries for efficiency
    #[derive(sqlx::FromRow)]
    struct UsageCounts {
        total_requests: i64,
        current_month_requests: i64,
    }

    let usage_counts = sqlx::query_as::<_, UsageCounts>(
        "SELECT 
            COUNT(*) as total_requests,
            COUNT(*) FILTER (WHERE created_at >= DATE_TRUNC('month', NOW())) as current_month_requests
         FROM rpc_usage_logs 
         WHERE user_id = $1",
    )
    .bind(&user_id)
    .fetch_one(&state.db)
    .await?;

    let blocked_requests: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM security_events WHERE user_id = $1 AND event_type = 'blocked'",
    )
    .bind(&user_id)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(UsageResponse {
        total_requests: usage_counts.total_requests,
        blocked_requests,
        current_month_requests: usage_counts.current_month_requests,
    }))
}

#[derive(Serialize)]
struct ApiKeyInfo {
    id: String,
    name: Option<String>,
    last_used_at: Option<chrono::DateTime<chrono::Utc>>,
    created_at: chrono::DateTime<chrono::Utc>,
}

async fn list_api_keys(
    State(_state): State<PlatformState>,
    Json(_req): Json<serde_json::Value>,
) -> Result<Json<Vec<ApiKeyInfo>>, AppError> {
    // TODO: Implement API key listing
    Ok(Json(vec![]))
}

#[derive(Deserialize)]
struct CreateApiKeyRequest {
    api_key: String,
    name: Option<String>,
}

#[derive(Serialize)]
struct CreateApiKeyResponse {
    api_key: String,
}

async fn create_api_key(
    State(state): State<PlatformState>,
    Json(req): Json<CreateApiKeyRequest>,
) -> Result<Json<CreateApiKeyResponse>, AppError> {
    // Get user from existing API key
    let api_key_hash = auth::hash_api_key(&req.api_key);
    let user_id = sqlx::query_scalar::<_, uuid::Uuid>(
        "SELECT id FROM users WHERE api_key_hash = $1 AND active = true",
    )
    .bind(&api_key_hash)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::Unauthorized)?;

    // Generate new API key
    let new_api_key = auth::generate_api_key();
    let new_api_key_hash = auth::hash_api_key(&new_api_key);

    // Insert new key
    sqlx::query("INSERT INTO api_keys (user_id, key_hash, name) VALUES ($1, $2, $3)")
        .bind(&user_id)
        .bind(&new_api_key_hash)
        .bind(&req.name)
        .execute(&state.db)
        .await?;

    log::info!("✅ New API key created for user {}", user_id);

    Ok(Json(CreateApiKeyResponse {
        api_key: new_api_key,
    }))
}

pub enum AppError {
    BadRequest(String),
    Unauthorized,
    Conflict(String),
    NotFound(String),
    Internal,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, "Unauthorized".into()),
            AppError::Conflict(msg) => (StatusCode::CONFLICT, msg),
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            AppError::Internal => (StatusCode::INTERNAL_SERVER_ERROR, "Internal error".into()),
        };

        (status, Json(serde_json::json!({ "error": message }))).into_response()
    }
}

impl From<sqlx::Error> for AppError {
    fn from(_: sqlx::Error) -> Self {
        AppError::Internal
    }
}

// Payment handlers

#[derive(Deserialize)]
struct CreatePaymentRequest {
    api_key: String,
    package: String,
    #[serde(default = "default_token_type")]
    token_type: String,
}

fn default_token_type() -> String {
    "xlabs".to_string()
}

#[derive(Serialize)]
struct CreatePaymentResponse {
    payment_id: String,
    payment_url: String,
    qr_code_data: String,
    amount: String,
    recipient: String,
}

async fn create_payment(
    State(state): State<PlatformState>,
    Json(req): Json<CreatePaymentRequest>,
) -> Result<Json<CreatePaymentResponse>, AppError> {
    // Check if payments are enabled
    if !state.platform_config.payments.enabled {
        return Err(AppError::BadRequest(
            "Payments are not enabled on this instance".to_string(),
        ));
    }

    // Get user from API key
    let api_key_hash = auth::hash_api_key(&req.api_key);
    let user_id = sqlx::query_scalar::<_, uuid::Uuid>(
        "SELECT id FROM users WHERE api_key_hash = $1 AND active = true",
    )
    .bind(&api_key_hash)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::Unauthorized)?;

    // Create payment request
    let payment_req =
        payments::create_payment_request(&state.db, user_id, &req.package, &req.token_type)
            .await
            .map_err(|e| {
                log::error!("Failed to create payment: {}", e);
                AppError::BadRequest(e.to_string())
            })?;

    let payment_url = payment_req.to_url();
    let amount_formatted = payments::format_xlabs_amount(payment_req.amount);

    log::info!(
        "Payment request created: {} for package {}",
        payment_req.payment_id,
        req.package
    );

    Ok(Json(CreatePaymentResponse {
        payment_id: payment_req.payment_id.clone(),
        payment_url: payment_url.clone(),
        qr_code_data: payment_url, // Can be used to generate QR code
        amount: amount_formatted,
        recipient: payment_req.recipient,
    }))
}

#[derive(Deserialize)]
struct VerifyPaymentRequest {
    payment_id: String,
    signature: String,
}

#[derive(Serialize)]
struct VerifyPaymentResponse {
    verified: bool,
    tier: Option<String>,
}

async fn verify_payment_handler(
    State(state): State<PlatformState>,
    Json(req): Json<VerifyPaymentRequest>,
) -> Result<Json<VerifyPaymentResponse>, AppError> {
    // Check if payments are enabled
    if !state.platform_config.payments.enabled {
        return Err(AppError::BadRequest(
            "Payments are not enabled on this instance".to_string(),
        ));
    }

    let payment_id = uuid::Uuid::parse_str(&req.payment_id)
        .map_err(|_| AppError::BadRequest("Invalid payment ID".into()))?;

    let rpc_url = state.config.solana_rpc_url.as_str();

    let verified = payments::verify_payment(&state.db, payment_id, &req.signature, &rpc_url)
        .await
        .map_err(|e| {
            log::error!("Payment verification failed: {}", e);
            AppError::BadRequest(e.to_string())
        })?;

    let credits_added = if verified {
        sqlx::query_scalar::<_, Option<i64>>("SELECT credits_purchased FROM payments WHERE id = $1")
            .bind(&payment_id)
            .fetch_optional(&state.db)
            .await?
            .flatten()
    } else {
        None
    };

    log::info!(
        "Payment verification: {} -> {} (credits: {:?})",
        req.payment_id,
        verified,
        credits_added
    );

    Ok(Json(VerifyPaymentResponse {
        verified,
        tier: credits_added.map(|c| format!("{} credits", c)),
    }))
}

#[derive(Serialize)]
struct PricingInfo {
    package: String,
    token_amount: u64,
    token_amount_formatted: String,
    credits: i64,
    credits_formatted: String,
}

#[derive(Serialize)]
struct PricingResponse {
    enabled: bool,
    packages: Vec<PricingInfo>,
    token_info: Option<TokenInfo>,
}

#[derive(Serialize)]
struct TokenInfo {
    name: String,
    symbol: String,
    mint: String,
    logo: String,
    decimals: u8,
}

async fn get_pricing(State(state): State<PlatformState>) -> Json<PricingResponse> {
    if !state.platform_config.payments.enabled {
        return Json(PricingResponse {
            enabled: false,
            packages: vec![],
            token_info: None,
        });
    }

    let packages = vec!["small", "medium", "large", "xlarge"];

    let pricing: Vec<PricingInfo> = packages
        .into_iter()
        .filter_map(|package| {
            payments::get_package_info(package).map(|(amount, credits)| PricingInfo {
                package: package.to_string(),
                token_amount: amount,
                token_amount_formatted: payments::format_token_amount(amount),
                credits,
                credits_formatted: format_credits(credits),
            })
        })
        .collect();

    let token_info = TokenInfo {
        name: state.platform_config.payments.token.name.clone(),
        symbol: state.platform_config.payments.token.symbol.clone(),
        mint: state.platform_config.payments.token.mint.clone(),
        logo: state.platform_config.payments.token.logo.clone(),
        decimals: state.platform_config.payments.token.decimals,
    };

    Json(PricingResponse {
        enabled: true,
        packages: pricing,
        token_info: Some(token_info),
    })
}

fn format_credits(n: i64) -> String {
    if n >= 1_000_000 {
        format!("{}M requests", n / 1_000_000)
    } else if n >= 1_000 {
        format!("{}k requests", n / 1_000)
    } else {
        format!("{} requests", n)
    }
}

#[derive(Serialize)]
struct UserStatsResponse {
    api_key: String,
    wallet_address: String,
    credits_balance: i64,
    credits_used_this_month: i64,
    total_requests: i64,
    total_blocked: i64,
    total_warnings: i64,
    blocking_threshold: i32,
    notifications_enabled: bool,
}

async fn get_user_stats(
    State(state): State<PlatformState>,
    axum::extract::Path(api_key): axum::extract::Path<String>,
) -> Result<Json<UserStatsResponse>, AppError> {
    let api_key_hash = auth::hash_api_key(&api_key);

    #[derive(sqlx::FromRow)]
    struct UserStats {
        id: String,
        wallet_address: String,
        credits_balance: i64,
        credits_used_this_month: i64,
        blocking_threshold: i32,
        notifications_enabled: bool,
    }

    let user = sqlx::query_as::<_, UserStats>(
        "SELECT id, wallet_address, credits_balance, credits_used_this_month, blocking_threshold, notifications_enabled FROM users WHERE api_key_hash = $1"
    )
    .bind(api_key_hash)
    .fetch_optional(&state.db)
    .await
    .map_err(|_| AppError::Internal)?
    .ok_or(AppError::Unauthorized)?;

    // Count actual transactions analyzed with a single query using conditional aggregates
    #[derive(sqlx::FromRow)]
    struct EventCounts {
        total_requests: i64,
        blocked: i64,
        warnings: i64,
    }

    let counts = sqlx::query_as::<_, EventCounts>(
        "SELECT 
            COUNT(*) as total_requests,
            COUNT(*) FILTER (WHERE event_type = 'blocked') as blocked,
            COUNT(*) FILTER (WHERE event_type = 'warned') as warnings
         FROM security_events 
         WHERE user_id = $1",
    )
    .bind(&user.id)
    .fetch_one(&state.db)
    .await
    .unwrap_or(EventCounts {
        total_requests: 0,
        blocked: 0,
        warnings: 0,
    });

    Ok(Json(UserStatsResponse {
        api_key,
        wallet_address: user.wallet_address,
        credits_balance: user.credits_balance,
        credits_used_this_month: user.credits_used_this_month,
        total_requests: counts.total_requests,
        total_blocked: counts.blocked,
        total_warnings: counts.warnings,
        blocking_threshold: user.blocking_threshold,
        notifications_enabled: user.notifications_enabled,
    }))
}

#[derive(Serialize, Deserialize)]
struct GlobalStatsResponse {
    total_requests: i64,
    total_blocked: i64,
    total_warnings: i64,
    requests_per_second: f64,
}

async fn get_global_stats(
    State(state): State<PlatformState>,
) -> Result<Json<GlobalStatsResponse>, AppError> {
    // Check Redis cache first (cache for 10 seconds to reduce DB load)
    let cache_key = "global_stats:response";
    if let Ok(mut conn) = state.redis.get_multiplexed_async_connection().await {
        if let Ok(Some(cached_json)) = conn.get::<_, Option<String>>(cache_key).await {
            if let Ok(cached_response) = serde_json::from_str::<GlobalStatsResponse>(&cached_json) {
                log::debug!("✅ Cache hit for global stats");
                return Ok(Json(cached_response));
            }
        }
    }

    // Cache miss - query database
    log::debug!("❌ Cache miss for global stats, querying DB");

    // Calculate stats from security_events table (avoids hot row UPDATEs)
    // Use the global_stats table for fast counter access instead of full table scan
    #[derive(sqlx::FromRow)]
    struct GlobalStatsRow {
        total_requests: i64,
        total_blocked: i64,
        total_warnings: i64,
    }

    let stats = sqlx::query_as::<_, GlobalStatsRow>(
        "SELECT total_requests, total_blocked, total_warnings FROM global_stats WHERE id = 1",
    )
    .fetch_one(&state.db)
    .await
    .map_err(|_| AppError::Internal)?;

    // Calculate RPS based on last 5 minutes (rolling window for real-time metric)
    #[derive(sqlx::FromRow)]
    struct RecentRequests {
        count: i64,
    }

    let recent = sqlx::query_as::<_, RecentRequests>(
        "SELECT COUNT(*) as count FROM security_events WHERE created_at > NOW() - INTERVAL '5 minutes'"
    )
    .fetch_one(&state.db)
    .await
    .unwrap_or(RecentRequests { count: 0 });

    // Calculate RPS based on 5-minute window (300 seconds)
    let rps = recent.count as f64 / 300.0;

    let response = GlobalStatsResponse {
        total_requests: stats.total_requests,
        total_blocked: stats.total_blocked,
        total_warnings: stats.total_warnings,
        requests_per_second: rps,
    };

    // Cache the response for 10 seconds
    if let Ok(mut conn) = state.redis.get_multiplexed_async_connection().await {
        if let Ok(response_json) = serde_json::to_string(&response) {
            let _: Result<(), _> = conn.set_ex(cache_key, response_json, 10).await;
            log::debug!("✅ Cached global stats for 10 seconds");
        }
    }

    // Note: requests_per_second is calculated from the last 5 minutes for real-time throughput
    Ok(Json(response))
}

#[derive(Serialize)]
struct SecurityEvent {
    id: String,
    event_type: String,
    severity: String,
    threat_category: Option<String>,
    description: Option<String>,
    created_at: String,
    signature: Option<String>,
    wallet: Option<String>,
    method: Option<String>,
    summary: Option<String>,
    programs: Option<Vec<String>>,
    amount: Option<String>,
    risk_score: Option<i32>,
    rule_matches: Option<i32>,
    matched_rule_ids: Option<Vec<String>>, // Just IDs for performance
}

async fn get_security_events(
    State(state): State<PlatformState>,
    axum::extract::Path(api_key): axum::extract::Path<String>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Json<Vec<SecurityEvent>>, AppError> {
    let api_key_hash = auth::hash_api_key(&api_key);

    // Get user ID from API key
    let user_id: uuid::Uuid = sqlx::query_scalar("SELECT id FROM users WHERE api_key_hash = $1")
        .bind(api_key_hash)
        .fetch_optional(&state.db)
        .await
        .map_err(|_| AppError::Internal)?
        .ok_or(AppError::Unauthorized)?;

    // Parse pagination parameters
    let limit: i64 = params
        .get("limit")
        .and_then(|l| l.parse().ok())
        .unwrap_or(100)
        .min(1000); // Cap at 1k events max for performance and payload size

    let offset: i64 = params
        .get("offset")
        .and_then(|o| o.parse().ok())
        .unwrap_or(0)
        .max(0); // Ensure non-negative

    #[derive(sqlx::FromRow)]
    struct EventRow {
        id: uuid::Uuid,
        event_type: String,
        severity: String,
        threat_category: Option<String>,
        description: Option<String>,
        created_at: chrono::DateTime<chrono::Utc>,
        signature: Option<String>,
        wallet: Option<String>,
        method: Option<String>,
        summary: Option<String>,
        programs: Option<sqlx::types::JsonValue>,
        amount: Option<String>,
        risk_score: Option<i32>,
        rule_matches_count: Option<i32>,
        matched_rules: Option<sqlx::types::JsonValue>,
    }

    let events: Vec<EventRow> = sqlx::query_as(
        "SELECT id, event_type, severity, threat_category, description, created_at,
                transaction_data->>'signature' as signature,
                transaction_data->>'wallet' as wallet,
                transaction_data->>'method' as method,
                transaction_data->>'summary' as summary,
                transaction_data->'program_names' as programs,
                transaction_data->>'amount' as amount,
                (transaction_data->>'risk_score')::int as risk_score,
                CASE 
                    WHEN jsonb_typeof(transaction_data->'rule_matches') = 'array' THEN jsonb_array_length(transaction_data->'rule_matches')
                    WHEN jsonb_typeof(transaction_data->'rule_matches') = 'number' THEN (transaction_data->>'rule_matches')::int
                    ELSE 0
                END as rule_matches_count,
                CASE 
                    WHEN jsonb_typeof(transaction_data->'rule_matches') = 'array' THEN transaction_data->'rule_matches'
                    ELSE NULL
                END as matched_rules
         FROM security_events 
         WHERE user_id = $1 
         ORDER BY created_at DESC 
         LIMIT $2 OFFSET $3"
    )
    .bind(user_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.db)
    .await
    .map_err(|_| AppError::Internal)?;

    let security_events: Vec<SecurityEvent> = events
        .into_iter()
        .map(|e| SecurityEvent {
            id: e.id.to_string(),
            event_type: e.event_type,
            severity: e.severity,
            threat_category: e.threat_category,
            description: e.description,
            created_at: e.created_at.to_rfc3339(),
            signature: e.signature,
            wallet: e.wallet,
            method: e.method,
            summary: e.summary,
            programs: e.programs.and_then(|p| {
                p.as_array().map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
            }),
            amount: e.amount,
            risk_score: e.risk_score,
            rule_matches: e.rule_matches_count,
            matched_rule_ids: e.matched_rules.and_then(|m| {
                m.as_array().map(|arr| {
                    arr.iter()
                        .filter_map(|v| {
                            v.get("rule_id")
                                .and_then(|id| id.as_str())
                                .map(|s| s.to_string())
                        })
                        .collect()
                })
            }),
        })
        .collect();

    Ok(Json(security_events))
}

// ============================================================================
// Session-Based Authentication Handlers (for Dashboard)
// ============================================================================

#[derive(Deserialize)]
struct LoginRequest {
    wallet_address: String,
    message: String,
    signature: String,
}

#[derive(Serialize)]
struct LoginResponse {
    success: bool,
    user_id: String,
    wallet_address: String,
}

async fn login(
    State(state): State<PlatformState>,
    jar: CookieJar,
    Json(req): Json<LoginRequest>,
) -> Result<(CookieJar, Json<LoginResponse>), AppError> {
    // Verify wallet signature
    auth::verify_wallet_signature(&req.wallet_address, &req.message, &req.signature)
        .map_err(|_| AppError::BadRequest("Invalid signature".into()))?;

    // Get or create user
    let api_key_hash = auth::hash_api_key(&format!("temp_{}", req.wallet_address));

    let user_id = sqlx::query_scalar::<_, uuid::Uuid>(
        "INSERT INTO users (wallet_address, api_key_hash, tier) 
         VALUES ($1, $2, 'free') 
         ON CONFLICT (wallet_address) 
         DO UPDATE SET updated_at = NOW()
         RETURNING id",
    )
    .bind(&req.wallet_address)
    .bind(&api_key_hash)
    .fetch_one(&state.db)
    .await
    .map_err(|_| AppError::Internal)?;

    // Create session
    let session_id = state
        .sessions
        .create_session(user_id, &req.wallet_address)
        .await
        .map_err(|_| AppError::Internal)?;

    // Create HTTP-only secure cookie
    let cookie = Cookie::build(("session_id", session_id))
        .http_only(true)
        .secure(false) // Set to true in production with HTTPS
        .same_site(axum_extra::extract::cookie::SameSite::Lax)
        .path("/")
        .max_age(time::Duration::days(1))
        .build();

    log::info!("✅ User logged in: {} ({})", req.wallet_address, user_id);

    Ok((
        jar.add(cookie),
        Json(LoginResponse {
            success: true,
            user_id: user_id.to_string(),
            wallet_address: req.wallet_address,
        }),
    ))
}

#[derive(Serialize)]
struct CurrentUserResponse {
    user_id: String,
    wallet_address: String,
    credits_balance: i64,
    tier: String,
}

async fn get_current_user(
    State(state): State<PlatformState>,
    jar: CookieJar,
) -> Result<Json<CurrentUserResponse>, AppError> {
    let session_id = jar
        .get("session_id")
        .map(|c| c.value().to_string())
        .ok_or(AppError::Unauthorized)?;

    let session = state
        .sessions
        .get_session(&session_id)
        .await
        .map_err(|_| AppError::Internal)?
        .ok_or(AppError::Unauthorized)?;

    // Refresh session on activity
    let _ = state.sessions.refresh_session(&session_id).await;

    // Get user details
    #[derive(sqlx::FromRow)]
    struct UserInfo {
        wallet_address: String,
        credits_balance: i64,
        tier: String,
    }

    let user: UserInfo =
        sqlx::query_as("SELECT wallet_address, credits_balance, tier FROM users WHERE id = $1")
            .bind(uuid::Uuid::parse_str(&session.user_id).map_err(|_| AppError::Internal)?)
            .fetch_one(&state.db)
            .await
            .map_err(|_| AppError::Unauthorized)?;

    Ok(Json(CurrentUserResponse {
        user_id: session.user_id,
        wallet_address: user.wallet_address,
        credits_balance: user.credits_balance,
        tier: user.tier,
    }))
}

async fn logout(
    State(state): State<PlatformState>,
    jar: CookieJar,
) -> Result<(CookieJar, Json<serde_json::Value>), AppError> {
    if let Some(cookie) = jar.get("session_id") {
        let session_id = cookie.value();
        let _ = state.sessions.delete_session(session_id).await;
    }

    // Remove cookie
    let cookie = Cookie::build(("session_id", ""))
        .http_only(true)
        .path("/")
        .max_age(time::Duration::seconds(0))
        .build();

    Ok((
        jar.add(cookie),
        Json(serde_json::json!({ "success": true })),
    ))
}

#[derive(Serialize)]
struct ApiKeyResponse {
    api_key: String,
    created_at: String,
}

async fn get_my_api_key(
    State(state): State<PlatformState>,
    jar: CookieJar,
) -> Result<Json<ApiKeyResponse>, AppError> {
    let session_id = jar
        .get("session_id")
        .map(|c| c.value().to_string())
        .ok_or(AppError::Unauthorized)?;

    let session = state
        .sessions
        .get_session(&session_id)
        .await
        .map_err(|_| AppError::Internal)?
        .ok_or(AppError::Unauthorized)?;

    // Get user's API key (we'll show the hash, user should regenerate to see full key)
    #[derive(sqlx::FromRow)]
    struct ApiKeyInfo {
        api_key_hash: String,
        created_at: chrono::DateTime<chrono::Utc>,
    }

    let key_info: ApiKeyInfo =
        sqlx::query_as("SELECT api_key_hash, created_at FROM users WHERE id = $1")
            .bind(uuid::Uuid::parse_str(&session.user_id).map_err(|_| AppError::Internal)?)
            .fetch_one(&state.db)
            .await
            .map_err(|_| AppError::Internal)?;

    Ok(Json(ApiKeyResponse {
        api_key: format!("{}...", &key_info.api_key_hash[..12]),
        created_at: key_info.created_at.to_rfc3339(),
    }))
}

#[derive(Serialize)]
struct RegenerateApiKeyResponse {
    api_key: String,
    warning: String,
}

async fn regenerate_my_api_key(
    State(state): State<PlatformState>,
    jar: CookieJar,
) -> Result<Json<RegenerateApiKeyResponse>, AppError> {
    let session_id = jar
        .get("session_id")
        .map(|c| c.value().to_string())
        .ok_or(AppError::Unauthorized)?;

    let session = state
        .sessions
        .get_session(&session_id)
        .await
        .map_err(|_| AppError::Internal)?
        .ok_or(AppError::Unauthorized)?;

    // Generate new API key
    let api_key = auth::generate_api_key();
    let api_key_hash = auth::hash_api_key(&api_key);

    sqlx::query("UPDATE users SET api_key_hash = $1, updated_at = NOW() WHERE id = $2")
        .bind(&api_key_hash)
        .bind(uuid::Uuid::parse_str(&session.user_id).map_err(|_| AppError::Internal)?)
        .execute(&state.db)
        .await
        .map_err(|_| AppError::Internal)?;

    log::info!("🔑 API key regenerated for user {}", session.user_id);

    Ok(Json(RegenerateApiKeyResponse {
        api_key,
        warning: "Save this key now. It won't be shown again.".to_string(),
    }))
}

async fn get_my_stats(
    State(state): State<PlatformState>,
    jar: CookieJar,
) -> Result<Json<UserStatsResponse>, AppError> {
    let session_id = jar
        .get("session_id")
        .map(|c| c.value().to_string())
        .ok_or(AppError::Unauthorized)?;

    let session = state
        .sessions
        .get_session(&session_id)
        .await
        .map_err(|_| AppError::Internal)?
        .ok_or(AppError::Unauthorized)?;

    let user_uuid = uuid::Uuid::parse_str(&session.user_id).map_err(|_| AppError::Internal)?;

    #[derive(sqlx::FromRow)]
    struct UserStats {
        wallet_address: String,
        credits_balance: i64,
        credits_used_this_month: i64,
        blocking_threshold: i32,
        notifications_enabled: bool,
    }

    let user = sqlx::query_as::<_, UserStats>(
        "SELECT wallet_address, credits_balance, credits_used_this_month, blocking_threshold, notifications_enabled FROM users WHERE id = $1"
    )
    .bind(&user_uuid)
    .fetch_optional(&state.db)
    .await
    .map_err(|_| AppError::Internal)?
    .ok_or(AppError::Unauthorized)?;

    // Count actual transactions analyzed with a single query using conditional aggregates
    #[derive(sqlx::FromRow)]
    struct EventCounts {
        total_requests: i64,
        blocked: i64,
        warnings: i64,
    }

    let counts = sqlx::query_as::<_, EventCounts>(
        "SELECT 
            COUNT(*) as total_requests,
            COUNT(*) FILTER (WHERE event_type = 'blocked') as blocked,
            COUNT(*) FILTER (WHERE event_type = 'warned') as warnings
         FROM security_events 
         WHERE user_id = $1",
    )
    .bind(&user_uuid)
    .fetch_one(&state.db)
    .await
    .unwrap_or(EventCounts {
        total_requests: 0,
        blocked: 0,
        warnings: 0,
    });

    Ok(Json(UserStatsResponse {
        api_key: "***".to_string(),
        wallet_address: user.wallet_address,
        credits_balance: user.credits_balance,
        credits_used_this_month: user.credits_used_this_month,
        total_requests: counts.total_requests,
        total_blocked: counts.blocked,
        total_warnings: counts.warnings,
        blocking_threshold: user.blocking_threshold,
        notifications_enabled: user.notifications_enabled,
    }))
}

async fn get_my_events(
    State(state): State<PlatformState>,
    jar: CookieJar,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Json<Vec<SecurityEvent>>, AppError> {
    let session_id = jar
        .get("session_id")
        .map(|c| c.value().to_string())
        .ok_or(AppError::Unauthorized)?;

    let session = state
        .sessions
        .get_session(&session_id)
        .await
        .map_err(|_| AppError::Internal)?
        .ok_or(AppError::Unauthorized)?;

    let user_uuid = uuid::Uuid::parse_str(&session.user_id).map_err(|_| AppError::Internal)?;

    // Parse pagination parameters
    let limit: i64 = params
        .get("limit")
        .and_then(|l| l.parse().ok())
        .unwrap_or(100)
        .min(1000); // Cap at 1k events max for performance and payload size

    let offset: i64 = params
        .get("offset")
        .and_then(|o| o.parse().ok())
        .unwrap_or(0)
        .max(0); // Ensure non-negative

    #[derive(sqlx::FromRow)]
    struct EventRow {
        id: uuid::Uuid,
        event_type: String,
        severity: String,
        threat_category: Option<String>,
        description: Option<String>,
        created_at: chrono::DateTime<chrono::Utc>,
        signature: Option<String>,
        wallet: Option<String>,
        method: Option<String>,
        summary: Option<String>,
        programs: Option<sqlx::types::JsonValue>,
        amount: Option<String>,
        risk_score: Option<i32>,
        rule_matches_count: Option<i32>,
        matched_rules: Option<sqlx::types::JsonValue>,
    }

    let events: Vec<EventRow> = sqlx::query_as(
        "SELECT id, event_type, severity, threat_category, description, created_at,
                transaction_data->>'signature' as signature,
                transaction_data->>'wallet' as wallet,
                transaction_data->>'method' as method,
                transaction_data->>'summary' as summary,
                transaction_data->'program_names' as programs,
                transaction_data->>'amount' as amount,
                (transaction_data->>'risk_score')::int as risk_score,
                CASE 
                    WHEN jsonb_typeof(transaction_data->'rule_matches') = 'array' THEN jsonb_array_length(transaction_data->'rule_matches')
                    WHEN jsonb_typeof(transaction_data->'rule_matches') = 'number' THEN (transaction_data->>'rule_matches')::int
                    ELSE 0
                END as rule_matches_count,
                CASE 
                    WHEN jsonb_typeof(transaction_data->'rule_matches') = 'array' THEN transaction_data->'rule_matches'
                    ELSE NULL
                END as matched_rules
         FROM security_events 
         WHERE user_id = $1 
         ORDER BY created_at DESC 
         LIMIT $2 OFFSET $3"
    )
    .bind(user_uuid)
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.db)
    .await
    .map_err(|_| AppError::Internal)?;

    let security_events: Vec<SecurityEvent> = events
        .into_iter()
        .map(|e| SecurityEvent {
            id: e.id.to_string(),
            event_type: e.event_type,
            severity: e.severity,
            threat_category: e.threat_category,
            description: e.description,
            created_at: e.created_at.to_rfc3339(),
            signature: e.signature,
            wallet: e.wallet,
            method: e.method,
            summary: e.summary,
            programs: e.programs.and_then(|p| {
                p.as_array().map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
            }),
            amount: e.amount,
            risk_score: e.risk_score,
            rule_matches: e.rule_matches_count,
            matched_rule_ids: e.matched_rules.and_then(|m| {
                m.as_array().map(|arr| {
                    arr.iter()
                        .filter_map(|v| {
                            v.get("rule_id")
                                .and_then(|id| id.as_str())
                                .map(|s| s.to_string())
                        })
                        .collect()
                })
            }),
        })
        .collect();

    Ok(Json(security_events))
}

#[derive(Deserialize)]
struct UpdateSignatureRequest {
    event_id: String,
    signature: String,
}

async fn update_event_signature(
    State(state): State<PlatformState>,
    headers: HeaderMap,
    Json(req): Json<UpdateSignatureRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    verify_internal_secret(&state, &headers)?;

    // Update the transaction_data JSONB field to add the signature
    let result = sqlx::query(
        "UPDATE security_events 
         SET transaction_data = jsonb_set(
             COALESCE(transaction_data, '{}'::jsonb),
             '{signature}',
             to_jsonb($2::text),
             true
         )
         WHERE transaction_data->>'event_id' = $1
         RETURNING id",
    )
    .bind(&req.event_id)
    .bind(&req.signature)
    .fetch_optional(&state.db)
    .await
    .map_err(|_| AppError::Internal)?;

    if result.is_some() {
        log::info!("✅ Updated event {} with signature", req.event_id);
        Ok(Json(serde_json::json!({ "success": true })))
    } else {
        log::warn!("⚠️ Event {} not found for signature update", req.event_id);
        Err(AppError::Internal)
    }
}

fn verify_internal_secret(state: &PlatformState, headers: &HeaderMap) -> Result<(), AppError> {
    let expected_secret = state
        .platform_config
        .internal_api_secret
        .as_deref()
        .ok_or_else(|| AppError::BadRequest("Internal API secret is not configured".to_string()))?;
    let provided_secret = headers
        .get("X-Internal-Secret")
        .and_then(|v| v.to_str().ok())
        .ok_or(AppError::Unauthorized)?;
    if provided_secret != expected_secret {
        return Err(AppError::Unauthorized);
    }
    Ok(())
}

#[derive(Serialize)]
struct RpcUsageLog {
    id: String,
    method: String,
    success: bool,
    created_at: String,
}

async fn get_my_usage(
    State(state): State<PlatformState>,
    jar: CookieJar,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Json<Vec<RpcUsageLog>>, AppError> {
    let session_id = jar
        .get("session_id")
        .map(|c| c.value().to_string())
        .ok_or(AppError::Unauthorized)?;

    let session = state
        .sessions
        .get_session(&session_id)
        .await
        .map_err(|_| AppError::Internal)?
        .ok_or(AppError::Unauthorized)?;

    let user_uuid = uuid::Uuid::parse_str(&session.user_id).map_err(|_| AppError::Internal)?;

    // Parse pagination parameters
    let limit: i64 = params
        .get("limit")
        .and_then(|l| l.parse().ok())
        .unwrap_or(100)
        .min(1000); // Cap at 1k usage logs

    let offset: i64 = params
        .get("offset")
        .and_then(|o| o.parse().ok())
        .unwrap_or(0)
        .max(0);

    #[derive(sqlx::FromRow)]
    struct UsageRow {
        id: uuid::Uuid,
        method: String,
        success: bool,
        created_at: chrono::DateTime<chrono::Utc>,
    }

    let logs: Vec<UsageRow> = sqlx::query_as(
        "SELECT id, method, success, created_at 
         FROM rpc_usage_logs 
         WHERE user_id = $1 
         ORDER BY created_at DESC 
         LIMIT $2 OFFSET $3",
    )
    .bind(user_uuid)
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.db)
    .await
    .map_err(|_| AppError::Internal)?;

    let usage_logs: Vec<RpcUsageLog> = logs
        .into_iter()
        .map(|l| RpcUsageLog {
            id: l.id.to_string(),
            method: l.method,
            success: l.success,
            created_at: l.created_at.to_rfc3339(),
        })
        .collect();

    Ok(Json(usage_logs))
}

#[derive(Serialize, Deserialize)]
struct ActiveRule {
    id: String,
    name: String,
    description: String,
    action: String,
    severity: String,
    enabled: bool,
    hit_count: i64,
}

#[derive(Serialize, Deserialize)]
struct ActiveRulesResponse {
    rules_source: String,
    total_rules: usize,
    active_rules: usize,
    total_hits: i64,
    rules: Vec<ActiveRule>,
}

async fn get_active_rules(
    State(state): State<PlatformState>,
    jar: CookieJar,
) -> Result<Json<ActiveRulesResponse>, AppError> {
    // Verify session
    let session_id = jar
        .get("session_id")
        .map(|c| c.value().to_string())
        .ok_or(AppError::Unauthorized)?;

    let _session = state
        .sessions
        .get_session(&session_id)
        .await
        .map_err(|_| AppError::Internal)?
        .ok_or(AppError::Unauthorized)?;

    // Check Redis cache first (cache entire response for 60 seconds)
    let cache_key = "active_rules:response";
    if let Ok(mut conn) = state.redis.get_multiplexed_async_connection().await {
        if let Ok(Some(cached_json)) = conn.get::<_, Option<String>>(cache_key).await {
            if let Ok(cached_response) = serde_json::from_str::<ActiveRulesResponse>(&cached_json) {
                log::debug!("✅ Cache hit for active rules");
                return Ok(Json(cached_response));
            }
        }
    }

    // Get rules path from environment (same as reverse-proxy)
    let rules_path = &state.platform_config.rules_display_path;

    // Read and parse rules file
    let rules_content = tokio::fs::read_to_string(&rules_path).await.map_err(|e| {
        log::error!("Failed to read rules file at {}: {}", rules_path, e);
        AppError::Internal
    })?;

    let rules_json: Vec<serde_json::Value> = serde_json::from_str(&rules_content).map_err(|e| {
        log::error!("Failed to parse rules JSON: {}", e);
        AppError::Internal
    })?;

    // Get hit counts from database for all rules
    #[derive(sqlx::FromRow)]
    struct RuleHitCount {
        rule_id: String,
        hit_count: i64,
    }

    let hit_counts: Vec<RuleHitCount> = sqlx::query_as(
        "SELECT rule_id, COUNT(*) as hit_count 
         FROM security_events 
         WHERE rule_id IS NOT NULL 
         GROUP BY rule_id",
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    // Convert to HashMap for easy lookup
    let hit_counts_map: std::collections::HashMap<String, i64> = hit_counts
        .into_iter()
        .map(|r| (r.rule_id, r.hit_count))
        .collect();

    let total_rules = rules_json.len();
    let mut active_rules_count = 0;
    let mut active_rules = Vec::new();
    let mut total_hits = 0i64;

    for rule in rules_json {
        let enabled = rule
            .get("enabled")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        if enabled {
            active_rules_count += 1;
        }

        let rule_id = rule
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        let hit_count = hit_counts_map.get(&rule_id).copied().unwrap_or(0);
        total_hits += hit_count;

        let rule_obj = ActiveRule {
            id: rule_id,
            name: rule
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("Unnamed Rule")
                .to_string(),
            description: rule
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            action: rule
                .get("rule")
                .and_then(|r| r.get("action"))
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string(),
            severity: rule
                .get("metadata")
                .and_then(|m| m.get("severity"))
                .and_then(|v| v.as_str())
                .unwrap_or("medium")
                .to_string(),
            enabled,
            hit_count,
        };

        active_rules.push(rule_obj);
    }

    // Extract just the filename from the path for display
    let rules_source = std::path::Path::new(&rules_path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(&rules_path)
        .to_string();

    let response = ActiveRulesResponse {
        rules_source,
        total_rules,
        active_rules: active_rules_count,
        total_hits,
        rules: active_rules,
    };

    // Cache the response for 60 seconds
    if let Ok(mut conn) = state.redis.get_multiplexed_async_connection().await {
        if let Ok(response_json) = serde_json::to_string(&response) {
            let _: Result<(), _> = conn.set_ex(cache_key, response_json, 60).await;
        }
    }

    Ok(Json(response))
}

#[derive(Deserialize)]
struct UpdateThresholdRequest {
    threshold: i32,
}

#[derive(Serialize)]
struct UpdateThresholdResponse {
    blocking_threshold: i32,
}

async fn update_blocking_threshold(
    State(state): State<PlatformState>,
    jar: CookieJar,
    Json(req): Json<UpdateThresholdRequest>,
) -> Result<Json<UpdateThresholdResponse>, AppError> {
    let session_id = jar
        .get("session_id")
        .map(|c| c.value().to_string())
        .ok_or(AppError::Unauthorized)?;

    let session = state
        .sessions
        .get_session(&session_id)
        .await
        .map_err(|_| AppError::Internal)?
        .ok_or(AppError::Unauthorized)?;

    let user_uuid = uuid::Uuid::parse_str(&session.user_id).map_err(|_| AppError::Internal)?;

    // Validate threshold is reasonable (0-100)
    if req.threshold < 0 || req.threshold > 100 {
        return Err(AppError::BadRequest(format!(
            "Threshold must be between 0 and 100, got {}",
            req.threshold
        )));
    }

    // Update user's threshold
    sqlx::query("UPDATE users SET blocking_threshold = $1 WHERE id = $2")
        .bind(req.threshold)
        .bind(&user_uuid)
        .execute(&state.db)
        .await
        .map_err(|_| AppError::Internal)?;

    // Invalidate user cache (keyed by user_id, so this is instant)
    use redis::AsyncCommands;
    let mut conn = state
        .redis
        .get_multiplexed_async_connection()
        .await
        .map_err(|_| AppError::Internal)?;

    let cache_key = format!("auth:user:{}", user_uuid);
    let deleted: u32 = conn.del(&cache_key).await.map_err(|_| AppError::Internal)?;

    log::info!(
        "✅ Updated blocking threshold for user {} to {} (cache invalidated: {})",
        user_uuid,
        req.threshold,
        deleted > 0
    );

    Ok(Json(UpdateThresholdResponse {
        blocking_threshold: req.threshold,
    }))
}

#[derive(Deserialize)]
struct ToggleNotificationsRequest {
    enabled: bool,
}

#[derive(Serialize)]
struct ToggleNotificationsResponse {
    notifications_enabled: bool,
}

async fn toggle_notifications(
    State(state): State<PlatformState>,
    jar: CookieJar,
    Json(req): Json<ToggleNotificationsRequest>,
) -> Result<Json<ToggleNotificationsResponse>, AppError> {
    let session_id = jar
        .get("session_id")
        .map(|c| c.value().to_string())
        .ok_or(AppError::Unauthorized)?;

    let session = state
        .sessions
        .get_session(&session_id)
        .await
        .map_err(|_| AppError::Internal)?
        .ok_or(AppError::Unauthorized)?;

    let user_uuid = uuid::Uuid::parse_str(&session.user_id).map_err(|_| AppError::Internal)?;

    sqlx::query("UPDATE users SET notifications_enabled = $1 WHERE id = $2")
        .bind(req.enabled)
        .bind(&user_uuid)
        .execute(&state.db)
        .await
        .map_err(|_| AppError::Internal)?;

    log::info!(
        "✅ Updated notifications for user {} to {}",
        user_uuid,
        req.enabled
    );

    Ok(Json(ToggleNotificationsResponse {
        notifications_enabled: req.enabled,
    }))
}
