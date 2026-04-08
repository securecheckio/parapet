use anyhow::Result;
use clap::Parser;
use parapet_api::{config::load_config_from_file, create_router, state};

#[derive(Parser)]
#[command(name = "parapet-api")]
#[command(about = "Parapet API - MCP server for AI agents")]
struct Cli {
    /// Path to config file (default: ./config.toml)
    #[arg(short, long, default_value = "config.toml")]
    config: String,
}

fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    env_logger::init();

    log::info!("🚀 Starting Parapet Core API Service");

    let cli = Cli::parse();

    // Load configuration from specified path
    let config = load_config_from_file(&cli.config)?;
    log::info!("✅ Loaded config from {}", cli.config);

    let server_addr = format!("{}:{}", config.server_host, config.server_port);

    // Build Tokio runtime with configured worker threads
    let mut builder = tokio::runtime::Builder::new_multi_thread();
    builder.enable_all();

    if let Some(threads) = config.worker_threads {
        log::info!("🧵 Configuring {} worker threads", threads);
        builder.worker_threads(threads);
    } else {
        log::info!("🧵 Using default worker threads (one per CPU core)");
    }

    let runtime = builder.build()?;

    runtime.block_on(async move {
        // Initialize state
        let app_state = state::AppState::new(config).await?;

        // Build router using library function
        let app = create_router(app_state);

        log::info!("📡 Core API listening on http://{}", server_addr);
        log::info!("📊 WebSocket endpoint: ws://{}/ws/escalations", server_addr);

        let listener = tokio::net::TcpListener::bind(&server_addr).await?;
        axum::serve(listener, app).await?;

        Ok::<(), anyhow::Error>(())
    })
}
