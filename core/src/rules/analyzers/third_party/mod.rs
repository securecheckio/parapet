// Shared rate limiter for third-party API analyzers
pub mod rate_limiter;
pub mod redis_cache;

#[cfg(feature = "token-mint")]
pub mod token_mint;

#[cfg(feature = "helius")]
pub mod helius_identity;

#[cfg(feature = "helius")]
pub mod helius_transfer;

#[cfg(feature = "helius")]
pub mod helius_funding;

#[cfg(feature = "ottersec")]
pub mod ottersec_verified;

#[cfg(feature = "jupiter")]
pub mod jupiter_token;

#[cfg(feature = "rugcheck")]
pub mod rugcheck;

pub mod squads_v4;

#[cfg(feature = "token-mint")]
pub use token_mint::TokenMintAnalyzer;

#[cfg(feature = "helius")]
pub use helius_identity::HeliusIdentityAnalyzer;

#[cfg(feature = "helius")]
pub use helius_transfer::HeliusTransferAnalyzer;

#[cfg(feature = "helius")]
pub use helius_funding::HeliusFundingAnalyzer;

#[cfg(feature = "ottersec")]
pub use ottersec_verified::OtterSecVerifiedAnalyzer;

#[cfg(feature = "jupiter")]
pub use jupiter_token::JupiterTokenAnalyzer;

#[cfg(feature = "rugcheck")]
pub use rugcheck::RugcheckAnalyzer;

pub use squads_v4::SquadsV4Analyzer;
