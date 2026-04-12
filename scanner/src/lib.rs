pub mod classifier;
mod detector;
mod history;
mod report;
mod state;

pub use detector::{Severity, ThreatAssessment, ThreatType};
pub use report::{ScanConfig, ScanReport, ScanStats, SuspiciousProgram};

use anyhow::Result;
use log::{info, warn};
use parapet_core::rules::{AnalyzerRegistry, RuleEngine};
use solana_client::rpc_client::RpcClient;
use std::sync::Arc;

#[cfg(feature = "reqwest")]
use parapet_core::enrichment::EnrichmentService;

use detector::ThreatDetector;
use state::StateScanner;

/// Main wallet scanner that orchestrates security analysis
pub struct WalletScanner {
    rpc_client: RpcClient,
    analyzer_registry: Option<Arc<AnalyzerRegistry>>,
    rule_engine: Option<Arc<RuleEngine>>,
    #[cfg(feature = "reqwest")]
    enrichment: Option<Arc<EnrichmentService>>,
}

impl WalletScanner {
    /// Create a basic scanner - only checks active delegations (no deep analysis)
    pub fn new(rpc_url: String) -> Result<Self> {
        Ok(Self {
            rpc_client: RpcClient::new(rpc_url),
            analyzer_registry: None,
            rule_engine: None,
            #[cfg(feature = "reqwest")]
            enrichment: None,
        })
    }

    /// Create a full scanner - with analyzers and rules for deep historical analysis
    pub fn with_analyzers(
        rpc_url: String,
        registry: Arc<AnalyzerRegistry>,
        engine: Arc<RuleEngine>,
    ) -> Result<Self> {
        Ok(Self {
            rpc_client: RpcClient::new(rpc_url),
            analyzer_registry: Some(registry),
            rule_engine: Some(engine),
            #[cfg(feature = "reqwest")]
            enrichment: None,
        })
    }

    /// Enable enrichment service for rules to access off-chain data
    #[cfg(feature = "reqwest")]
    pub fn with_enrichment(mut self, service: Arc<EnrichmentService>) -> Self {
        log::info!("✅ Enrichment service enabled - rules can now access off-chain data");
        self.enrichment = Some(service);
        self
    }

    /// Get reference to enrichment service (for passing to history scanner)
    #[cfg(feature = "reqwest")]
    pub fn enrichment(&self) -> Option<Arc<EnrichmentService>> {
        self.enrichment.clone()
    }

    /// Scan a wallet for security threats
    pub async fn scan(&self, wallet: &str, mut config: ScanConfig) -> Result<ScanReport> {
        // Auto-calculate delay from analyzers if not set
        if config.rpc_delay_ms == 0 {
            if let Some(registry) = &self.analyzer_registry {
                let recommended = registry.get_recommended_delay_ms();
                if recommended > 0 {
                    log::info!("📊 Auto-calculated delay from analyzers: {}ms", recommended);
                    config.rpc_delay_ms = recommended;
                }
            }
        }

        let start_time = std::time::Instant::now();
        info!("🔍 Starting wallet scan for: {}", wallet);

        // Always scan current state for active threats
        let active_threats = if config.check_active_threats {
            info!("Scanning current state for active threats...");
            StateScanner::scan_current_state(&self.rpc_client, wallet, config.commitment).await?
        } else {
            Vec::new()
        };

        // Scan history only if analyzers provided and requested
        let (historical_threats, program_encounters, transactions_analyzed) = if config
            .check_historical
        {
            if let (Some(registry), Some(engine)) = (&self.analyzer_registry, &self.rule_engine) {
                info!("Scanning transaction history...");
                let result = history::HistoryScanner::scan_history(
                    &self.rpc_client,
                    registry,
                    engine,
                    wallet,
                    &config,
                    #[cfg(feature = "reqwest")]
                    self.enrichment.as_ref(),
                )
                .await?;
                (
                    result.threats,
                    result.program_encounters,
                    result.transactions_analyzed,
                )
            } else {
                warn!("Historical scan requested but no analyzers provided - skipping");
                (Vec::new(), std::collections::HashMap::new(), 0)
            }
        } else {
            (Vec::new(), std::collections::HashMap::new(), 0)
        };

        // Classify and correlate threats
        let detector = ThreatDetector::new();
        let (all_threats, suspicious_programs) = detector
            .correlate_threats(active_threats, historical_threats, program_encounters)
            .await?;

        // Generate final report
        let scan_duration_ms = start_time.elapsed().as_millis() as u64;
        report::generate_report(
            wallet,
            all_threats,
            suspicious_programs,
            transactions_analyzed,
            config.time_window_days.unwrap_or(30),
            scan_duration_ms,
        )
        .await
    }
}
