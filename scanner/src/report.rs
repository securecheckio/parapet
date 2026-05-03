use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use solana_commitment_config::CommitmentConfig;

use crate::detector::{Severity, ThreatAssessment};

/// Suspicious program detected during wallet scan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuspiciousProgram {
    /// Program public key
    pub program_id: String,

    /// Risk score 0-100 (higher = more suspicious)
    pub risk_score: u8,

    /// Confidence in assessment 0.0-1.0
    pub confidence: f64,

    /// Classification: "unknown", "rule_violation", "high_complexity", "token_interaction"
    pub threat_type: String,

    /// When first seen in scan
    pub first_seen: DateTime<Utc>,

    /// Transactions where it appeared
    pub transaction_signatures: Vec<String>,

    /// Number of times seen in this scan
    pub occurrence_count: usize,

    /// Human-readable explanation
    pub analysis_summary: String,

    /// What user should do
    pub recommendation: String,

    /// Analyzer that flagged it
    pub detected_by: Vec<String>,
}

/// Configuration for wallet scan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanConfig {
    /// Maximum number of transactions to analyze
    pub max_transactions: Option<usize>,

    /// Time window in days to scan
    pub time_window_days: Option<u32>,

    /// Delay between RPC requests in milliseconds (for rate limiting)
    pub rpc_delay_ms: u64,

    /// Check for active threats (current state)
    pub check_active_threats: bool,

    /// Check historical transactions
    pub check_historical: bool,

    /// RPC commitment level
    #[serde(skip)]
    pub commitment: CommitmentConfig,
}

impl Default for ScanConfig {
    fn default() -> Self {
        Self {
            max_transactions: Some(100),
            time_window_days: Some(30),
            rpc_delay_ms: 0, // Auto-calculate from registered analyzers
            check_active_threats: true,
            check_historical: true,
            commitment: CommitmentConfig::confirmed(),
        }
    }
}

/// Comprehensive wallet security scan report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanReport {
    /// Wallet address scanned
    pub wallet: String,

    /// Timestamp of scan
    pub scanned_at: DateTime<Utc>,

    /// Overall security score (0-100, higher = safer)
    pub security_score: u8,

    /// Risk level summary
    pub risk_level: String,

    /// All identified threats
    pub threats: Vec<ThreatAssessment>,

    /// Suspicious programs detected
    pub suspicious_programs: Vec<SuspiciousProgram>,

    /// Scan statistics
    pub stats: ScanStats,
}

/// Statistics about the scan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanStats {
    /// Number of transactions analyzed
    pub transactions_analyzed: usize,

    /// Time range scanned in days
    pub time_range_days: u32,

    /// Total threats found
    pub threats_found: usize,

    /// Critical threats
    pub critical_count: usize,

    /// High risk threats
    pub high_count: usize,

    /// Medium risk threats
    pub medium_count: usize,

    /// Low risk threats
    pub low_count: usize,

    /// Scan duration in milliseconds
    pub scan_duration_ms: u64,
}

/// Generate final scan report with security scoring
pub async fn generate_report(
    wallet: &str,
    threats: Vec<ThreatAssessment>,
    suspicious_programs: Vec<SuspiciousProgram>,
    transactions_analyzed: usize,
    time_range_days: u32,
    scan_duration_ms: u64,
) -> Result<ScanReport> {
    // Count threats by severity
    let critical_count = threats
        .iter()
        .filter(|t| matches!(t.severity, Severity::Critical))
        .count();
    let high_count = threats
        .iter()
        .filter(|t| matches!(t.severity, Severity::High))
        .count();
    let medium_count = threats
        .iter()
        .filter(|t| matches!(t.severity, Severity::Medium))
        .count();
    let low_count = threats
        .iter()
        .filter(|t| matches!(t.severity, Severity::Low))
        .count();

    // Calculate security score (100 = perfectly safe, 0 = extremely dangerous)
    let security_score =
        calculate_security_score(critical_count, high_count, medium_count, low_count);

    // Determine overall risk level
    let risk_level = match security_score {
        0..=30 => "CRITICAL",
        31..=50 => "HIGH",
        51..=75 => "MEDIUM",
        76..=90 => "LOW",
        _ => "SAFE",
    };

    Ok(ScanReport {
        wallet: wallet.to_string(),
        scanned_at: Utc::now(),
        security_score,
        risk_level: risk_level.to_string(),
        threats,
        suspicious_programs,
        stats: ScanStats {
            transactions_analyzed,
            time_range_days,
            threats_found: critical_count + high_count + medium_count + low_count,
            critical_count,
            high_count,
            medium_count,
            low_count,
            scan_duration_ms,
        },
    })
}

/// Calculate security score based on threat severity distribution
fn calculate_security_score(critical: usize, high: usize, medium: usize, low: usize) -> u8 {
    // Start at 100 and deduct points based on threats
    let mut score = 100i32;

    // Deduct heavily for critical threats
    score -= critical as i32 * 50;

    // Deduct moderately for high threats
    score -= high as i32 * 20;

    // Deduct lightly for medium threats
    score -= medium as i32 * 5;

    // Deduct minimally for low threats
    score -= low as i32 * 1;

    // Clamp to 0-100 range
    score.max(0).min(100) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_score_perfect() {
        assert_eq!(calculate_security_score(0, 0, 0, 0), 100);
    }

    #[test]
    fn test_security_score_critical() {
        assert_eq!(calculate_security_score(1, 0, 0, 0), 50);
        assert_eq!(calculate_security_score(2, 0, 0, 0), 0);
        assert_eq!(calculate_security_score(3, 0, 0, 0), 0);
    }

    #[test]
    fn test_security_score_mixed() {
        assert_eq!(calculate_security_score(1, 1, 2, 5), 15);
    }
}
