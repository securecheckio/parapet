// Enrichment services for external data sources
//
// This module provides access to third-party APIs for token reputation,
// program verification, and other off-chain data that enriches on-chain analysis.
//
// Unlike analyzers (which are used by the rules engine), enrichment services
// are called directly and return structured data without policy evaluation.

#[cfg(feature = "reqwest")]
mod rugcheck;
#[cfg(feature = "reqwest")]
mod helius;
#[cfg(feature = "reqwest")]
mod jupiter;
#[cfg(feature = "reqwest")]
mod ottersec;

#[cfg(feature = "reqwest")]
pub use rugcheck::{RugcheckClient, RugcheckData, InsiderAnalysis, VaultAnalysis, DomainRegistration};
#[cfg(feature = "reqwest")]
pub use helius::{HeliusClient, HeliusData};
#[cfg(feature = "reqwest")]
pub use jupiter::{JupiterClient, JupiterData};
#[cfg(feature = "reqwest")]
pub use ottersec::{OtterSecClient, OtterSecData};

#[cfg(feature = "reqwest")]
use anyhow::Result;

/// Unified enrichment service coordinating all third-party data sources
#[cfg(feature = "reqwest")]
pub struct EnrichmentService {
    rugcheck: Option<RugcheckClient>,
    helius: Option<HeliusClient>,
    jupiter: Option<JupiterClient>,
    ottersec: Option<OtterSecClient>,
}

/// Complete enrichment data for a token/program
#[cfg(feature = "reqwest")]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EnrichmentData {
    pub rugcheck: Option<RugcheckData>,
    pub insider_analysis: Option<InsiderAnalysis>,
    pub vault_analysis: Option<VaultAnalysis>,
    pub domain_registration: Option<DomainRegistration>,
    pub helius: Option<HeliusData>,
    pub jupiter: Option<JupiterData>,
    pub ottersec: Option<OtterSecData>,
}

#[cfg(feature = "reqwest")]
impl EnrichmentService {
    /// Create new enrichment service
    /// Services are enabled based on available API keys
    pub fn new() -> Self {
        log::info!("🔌 Initializing enrichment services...");

        let rugcheck = Some(RugcheckClient::new());
        log::info!("  ✅ Rugcheck (free, no key required)");

        let helius = if std::env::var("HELIUS_API_KEY").is_ok() {
            log::info!("  ✅ Helius (API key found)");
            Some(HeliusClient::new())
        } else {
            log::info!("  ⏭️  Helius (no API key, skipping)");
            None
        };

        let jupiter = if std::env::var("JUPITER_API_KEY").is_ok() {
            log::info!("  ✅ Jupiter (API key found)");
            Some(JupiterClient::new())
        } else {
            log::info!("  ⏭️  Jupiter (no API key, using public API)");
            Some(JupiterClient::new()) // Public API available
        };

        let ottersec = if std::env::var("OTTERSEC_API_KEY").is_ok() {
            log::info!("  ✅ OtterSec (API key found)");
            Some(OtterSecClient::new())
        } else {
            log::info!("  ⏭️  OtterSec (no API key, skipping)");
            None
        };

        Self {
            rugcheck,
            helius,
            jupiter,
            ottersec,
        }
    }

    /// Get enrichment data for a token address
    /// Calls all available services in parallel (including advanced Rugcheck features)
    pub async fn enrich_token(&self, token_address: &str) -> Result<EnrichmentData> {
        log::debug!("🔍 Enriching token: {}", token_address);

        // Call all services in parallel (respecting rate limits)
        let (rugcheck, insider_analysis, vault_analysis, domain_registration, jupiter) = tokio::join!(
            async {
                if let Some(client) = &self.rugcheck {
                    client.get_token_data(token_address).await.ok()
                } else {
                    None
                }
            },
            async {
                if let Some(client) = &self.rugcheck {
                    client.get_insider_analysis(token_address).await.ok()
                } else {
                    None
                }
            },
            async {
                if let Some(client) = &self.rugcheck {
                    client.get_vault_analysis(token_address).await.ok()
                } else {
                    None
                }
            },
            async {
                if let Some(client) = &self.rugcheck {
                    client.lookup_domain(token_address).await.ok().flatten()
                } else {
                    None
                }
            },
            async {
                if let Some(client) = &self.jupiter {
                    client.get_token_data(token_address).await.ok()
                } else {
                    None
                }
            }
        );

        Ok(EnrichmentData {
            rugcheck,
            insider_analysis,
            vault_analysis,
            domain_registration,
            helius: None, // Helius is for programs, not tokens
            jupiter,
            ottersec: None, // OtterSec is for programs, not tokens
        })
    }
    
    /// Enrich multiple tokens in bulk (much faster than individual calls)
    pub async fn enrich_tokens_bulk(&self, token_addresses: &[String]) -> Result<std::collections::HashMap<String, EnrichmentData>> {
        use std::collections::HashMap;
        
        log::debug!("🔍 Bulk enriching {} tokens", token_addresses.len());

        let mut results = HashMap::new();

        // Use bulk Rugcheck API
        let rugcheck_data = if let Some(client) = &self.rugcheck {
            client.get_bulk_summaries(token_addresses).await.unwrap_or_default()
        } else {
            HashMap::new()
        };

        // For now, just return Rugcheck data
        // TODO: Add bulk support for other enrichment sources
        for token in token_addresses {
            let rugcheck_result = rugcheck_data.get(token).cloned();
            
            results.insert(token.clone(), EnrichmentData {
                rugcheck: rugcheck_result,
                insider_analysis: None,
                vault_analysis: None,
                domain_registration: None,
                helius: None,
                jupiter: None,
                ottersec: None,
            });
        }

        Ok(results)
    }

    /// Get enrichment data for a program address
    pub async fn enrich_program(&self, program_address: &str) -> Result<EnrichmentData> {
        log::debug!("🔍 Enriching program: {}", program_address);

        // Call all services in parallel
        let (helius, ottersec) = tokio::join!(
            async {
                if let Some(client) = &self.helius {
                    client.get_program_data(program_address).await.ok()
                } else {
                    None
                }
            },
            async {
                if let Some(client) = &self.ottersec {
                    client.get_verification_data(program_address).await.ok()
                } else {
                    None
                }
            }
        );

        Ok(EnrichmentData {
            rugcheck: None,
            insider_analysis: None,
            vault_analysis: None,
            domain_registration: None,
            helius,
            jupiter: None,
            ottersec,
        })
    }

    /// Get available services (for diagnostics)
    pub fn available_services(&self) -> Vec<&str> {
        let mut services = Vec::new();
        if self.rugcheck.is_some() {
            services.push("rugcheck");
        }
        if self.helius.is_some() {
            services.push("helius");
        }
        if self.jupiter.is_some() {
            services.push("jupiter");
        }
        if self.ottersec.is_some() {
            services.push("ottersec");
        }
        services
    }
}

#[cfg(feature = "reqwest")]
impl Default for EnrichmentService {
    fn default() -> Self {
        Self::new()
    }
}
