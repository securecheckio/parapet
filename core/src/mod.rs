use solana_sdk::transaction::Transaction;
use anyhow::Result;
use std::time::Instant;

mod blocklist;
mod delegation;
mod authority;
mod patterns;
mod scorer;

pub use blocklist::BlocklistChecker;
pub use delegation::{DelegationDetector, DelegationRisk};
pub use authority::{AuthorityDetector, AuthorityRisk};
pub use patterns::PatternAnalyzer;
pub use scorer::RiskScorer;

#[derive(Debug, Clone, serde::Serialize)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, serde::Serialize)]
pub struct FastCheckResult {
    pub risk_score: u8,
    pub risk_level: RiskLevel,
    pub issues: Vec<String>,
    pub delegation_detected: bool,
    pub blocked_programs: Vec<String>,
    pub authority_changes: bool,
    pub check_duration_ms: u64,
}

pub struct FastChecker {
    blocklist: BlocklistChecker,
    delegation_detector: DelegationDetector,
    authority_detector: AuthorityDetector,
    pattern_analyzer: PatternAnalyzer,
    risk_scorer: RiskScorer,
}

impl FastChecker {
    pub async fn new(cache: crate::cache::Cache) -> Result<Self> {
        let blocklist = BlocklistChecker::new(cache).await?;
        
        Ok(Self {
            blocklist,
            delegation_detector: DelegationDetector::new(),
            authority_detector: AuthorityDetector::new(),
            pattern_analyzer: PatternAnalyzer::new(),
            risk_scorer: RiskScorer::new(),
        })
    }
    
    pub async fn check(&self, transaction: &Transaction) -> Result<FastCheckResult> {
        let start = Instant::now();
        
        log::debug!("🔍 Starting fast check...");
        
        // 1. Blocklist check (5ms target)
        let blocked = self.blocklist.check(transaction).await?;
        log::debug!("  ✓ Blocklist check: {} programs flagged", blocked.len());
        
        // 2. Delegation detection (10ms target)
        let delegation = self.delegation_detector.detect(transaction)?;
        log::debug!("  ✓ Delegation check: {:?}", delegation);
        
        // 3. Authority change detection (10ms target)
        let authority = self.authority_detector.detect(transaction)?;
        log::debug!("  ✓ Authority check: {:?}", authority);
        
        // 4. Pattern analysis (20ms target)
        let patterns = self.pattern_analyzer.analyze(transaction)?;
        log::debug!("  ✓ Pattern analysis: {} issues", patterns.len());
        
        // 5. Score calculation (5ms target)
        let result = self.risk_scorer.calculate(
            &blocked,
            &delegation,
            &authority,
            &patterns,
        );
        
        let duration = start.elapsed().as_millis() as u64;
        log::debug!("⚡ Fast check completed in {}ms", duration);
        
        Ok(FastCheckResult {
            check_duration_ms: duration,
            ..result
        })
    }
}
