use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::{get, post},
    Json, Router,
};
use demo_monitor::{
    monitor::{generate_monitor_alert, get_baseline_result},
    rpc_client::simulate_via_parapet,
    stages::{get_demo_stages, get_stage_by_id},
    types::{FireEventRequest, FireEventResponse, SimulateRequest, SimulateResponse},
};
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::{cors::CorsLayer, services::ServeDir};

#[derive(Clone)]
struct AppState {
    rpc_proxy_url: String,
    alerts: Arc<RwLock<Vec<demo_monitor::types::MonitorAlert>>>,
}

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let rpc_proxy_url =
        std::env::var("PARAPET_RPC_URL").unwrap_or_else(|_| "http://localhost:8899".to_string());

    eprintln!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    eprintln!("🛡️  Parapet Security - Drift Attack Demo Monitor");
    eprintln!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    log::info!(
        "Demo Monitor starting with Parapet RPC at: {}",
        rpc_proxy_url
    );

    let state = AppState {
        rpc_proxy_url,
        alerts: Arc::new(RwLock::new(Vec::new())),
    };

    let app = Router::new()
        .route("/", get(serve_index))
        .route("/api/stages", get(list_stages))
        .route("/api/rules", get(list_rules))
        .route("/api/stage/fire", post(fire_event))
        .route("/api/simulate", post(simulate_transaction))
        .route("/api/alerts", get(get_alerts))
        .nest_service("/static", ServeDir::new("frontend"))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = "0.0.0.0:3030";
    log::info!("Demo Monitor listening on http://{}", addr);
    eprintln!("✅ Server ready!");
    eprintln!("\n📍 Demo UI: http://localhost:3030");
    eprintln!("📍 API Docs: http://localhost:3030/api/stages\n");
    eprintln!("Press Ctrl+C to stop\n");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn serve_index() -> impl IntoResponse {
    let html = tokio::fs::read_to_string("frontend/index.html")
        .await
        .unwrap_or_else(|_| {
            r#"
<!DOCTYPE html>
<html>
<head><title>Demo Monitor</title></head>
<body>
    <h1>Parapet Demo Monitor</h1>
    <p>Frontend files not found. Please ensure frontend/index.html exists.</p>
</body>
</html>
            "#
            .to_string()
        });
    Html(html)
}

async fn list_stages() -> Json<Vec<demo_monitor::types::DemoStage>> {
    Json(get_demo_stages())
}

async fn list_rules() -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let rules_path = "../../rules/examples/drift-attack-detection.json";
    let content = tokio::fs::read_to_string(rules_path).await.map_err(|e| {
        log::error!("Failed to read rules file: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to read rules: {}", e),
        )
    })?;

    let rules: serde_json::Value = serde_json::from_str(&content).map_err(|e| {
        log::error!("Failed to parse rules JSON: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to parse rules: {}", e),
        )
    })?;

    Ok(Json(rules))
}

async fn fire_event(
    State(state): State<AppState>,
    Json(payload): Json<FireEventRequest>,
) -> Result<Json<FireEventResponse>, (StatusCode, String)> {
    log::info!("Firing event for stage {}", payload.stage_id);

    let stage = get_stage_by_id(payload.stage_id).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            format!("Stage {} not found", payload.stage_id),
        )
    })?;

    let baseline_result = get_baseline_result(payload.stage_id);

    // Simulate transaction through Parapet
    let tx_sig = stage.tx_signature.as_ref().ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            "Stage has no transaction signature".to_string(),
        )
    })?;

    let mut parapet_result = simulate_via_parapet(tx_sig, &state.rpc_proxy_url)
        .await
        .map_err(|e| {
            log::error!("Failed to simulate transaction: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("RPC error: {}", e),
            )
        })?;

    // Stages 3 & 4 are already executed on-chain - can only alert/monitor, not block
    // Convert "block" to "alert" for on-chain execution stages
    if (payload.stage_id == 3 || payload.stage_id == 4) && parapet_result.action == "block" {
        log::info!(
            "Stage {} is on-chain execution - converting block to alert (monitoring)",
            payload.stage_id
        );
        parapet_result.action = "alert".to_string();
    }

    // Generate and store alert
    let alert = generate_monitor_alert(payload.stage_id, &parapet_result);
    state.alerts.write().await.push(alert);

    Ok(Json(FireEventResponse {
        stage: stage.clone(),
        baseline_result,
        parapet_result,
    }))
}

async fn simulate_transaction(
    State(state): State<AppState>,
    Json(payload): Json<SimulateRequest>,
) -> Result<Json<SimulateResponse>, (StatusCode, String)> {
    let stage = get_stage_by_id(payload.stage_id).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            format!("Stage {} not found", payload.stage_id),
        )
    })?;

    let tx_sig = payload
        .transaction
        .or_else(|| stage.tx_signature.clone())
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                "No transaction provided".to_string(),
            )
        })?;

    let parapet_result = simulate_via_parapet(&tx_sig, &state.rpc_proxy_url)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("RPC error: {}", e),
            )
        })?;

    Ok(Json(SimulateResponse {
        stage,
        parapet_result,
    }))
}

async fn get_alerts(State(state): State<AppState>) -> Json<Vec<demo_monitor::types::MonitorAlert>> {
    let alerts = state.alerts.read().await;
    Json(alerts.clone())
}
