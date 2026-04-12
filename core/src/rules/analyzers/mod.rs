pub mod core;
pub mod simulation;
pub mod third_party;

#[cfg(any(
    feature = "helius",
    feature = "ottersec",
    feature = "jupiter",
    feature = "rugcheck"
))]
pub mod feed_updater;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod test_analyzer_fields;

// Re-export core analyzers
pub use core::{
    BasicAnalyzer, CoreSecurityAnalyzer, InnerInstructionAnalyzer, InstructionDataAnalyzer,
    ProgramComplexityAnalyzer, SystemProgramAnalyzer, TokenInstructionAnalyzer,
    TransactionLogAnalyzer,
};

// Re-export simulation analyzers
pub use simulation::{
    SimulationAnalyzer, SimulationAnalyzerRegistry, SimulationBalanceAnalyzer,
    SimulationComputeAnalyzer, SimulationCpiAnalyzer, SimulationFailureAnalyzer,
    SimulationLogAnalyzer, SimulationTokenAnalyzer,
};

// Re-export feed updater (requires reqwest)
#[cfg(any(
    feature = "helius",
    feature = "ottersec",
    feature = "jupiter",
    feature = "rugcheck"
))]
pub use feed_updater::FeedUpdater;

// Re-export third-party analyzers
#[cfg(feature = "token-mint")]
pub use third_party::TokenMintAnalyzer;

#[cfg(feature = "helius")]
pub use third_party::{HeliusFundingAnalyzer, HeliusIdentityAnalyzer, HeliusTransferAnalyzer};

#[cfg(feature = "ottersec")]
pub use third_party::OtterSecVerifiedAnalyzer;

#[cfg(feature = "jupiter")]
pub use third_party::JupiterTokenAnalyzer;

#[cfg(feature = "rugcheck")]
pub use third_party::RugcheckAnalyzer;

pub use third_party::SquadsV4Analyzer;
