use chrono::{DateTime, Utc};
use parapet_core::rules::{RuleAction, RuleDecision};
use std::collections::HashSet;

use crate::report::SuspiciousProgram;

/// Core Solana system programs (always safe)
fn get_core_programs() -> HashSet<String> {
    let mut core = HashSet::new();

    // Core Solana programs
    core.insert("11111111111111111111111111111111".to_string()); // System
    core.insert("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".to_string()); // SPL Token
    core.insert("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb".to_string()); // Token-2022
    core.insert("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL".to_string()); // Associated Token
    core.insert("MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr".to_string()); // Memo
    core.insert("ComputeBudget111111111111111111111111111111".to_string()); // Compute Budget
    core.insert("Stake11111111111111111111111111111111111111".to_string()); // Stake
    core.insert("Vote111111111111111111111111111111111111111".to_string()); // Vote

    // Common known programs
    core.insert("JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4".to_string()); // Jupiter
    core.insert("whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc".to_string()); // Orca Whirlpool
    core.insert("675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8".to_string()); // Raydium

    core
}

/// Check if a program is known and safe
pub fn is_known_program(program_id: &str) -> bool {
    get_core_programs().contains(program_id)
}

/// Calculate risk score for a program
pub fn calculate_program_risk_score(
    program_id: &str,
    rule_decision: Option<&RuleDecision>,
    occurrence_count: usize,
) -> u8 {
    let mut score = 0i32;

    // Rule-based detection (highest priority)
    if let Some(decision) = rule_decision {
        match decision.action {
            RuleAction::Block => score += 50,
            RuleAction::Alert => score += 30,
            RuleAction::Pass => {}
        }
    }

    // Unknown program penalty
    if !is_known_program(program_id) {
        score += 20;
    }

    // Frequency adjustment (seen many times = likely legitimate)
    if occurrence_count > 10 {
        score = score.saturating_sub(10);
    } else if occurrence_count == 1 {
        // Single occurrence is more suspicious
        score += 5;
    }

    // Clamp to 0-100 range
    score.max(0).min(100) as u8
}

/// Determine threat type classification
pub fn classify_threat_type(
    risk_score: u8,
    rule_decision: Option<&RuleDecision>,
    is_known: bool,
) -> String {
    if let Some(decision) = rule_decision {
        match decision.action {
            RuleAction::Block => return "rule_violation".to_string(),
            RuleAction::Alert => return "rule_violation".to_string(),
            _ => {}
        }
    }

    if !is_known {
        if risk_score > 60 {
            "high_risk_unknown".to_string()
        } else {
            "unknown".to_string()
        }
    } else {
        "monitored".to_string()
    }
}

/// Calculate confidence in the assessment
pub fn calculate_confidence(
    risk_score: u8,
    rule_decision: Option<&RuleDecision>,
    occurrence_count: usize,
) -> f64 {
    let mut confidence: f64 = 0.5;

    // High confidence if rules flagged it
    if let Some(decision) = rule_decision {
        match decision.action {
            RuleAction::Block => confidence = 0.95,
            RuleAction::Alert => confidence = 0.85,
            _ => {}
        }
    }

    // Confidence increases with occurrence count (more data = better assessment)
    if occurrence_count > 5 {
        confidence += 0.1;
    }

    // High risk scores increase confidence
    if risk_score > 70 {
        confidence += 0.1;
    }

    confidence.min(1.0)
}

/// Generate analysis summary
pub fn generate_analysis_summary(
    program_id: &str,
    risk_score: u8,
    rule_decision: Option<&RuleDecision>,
    is_known: bool,
    occurrence_count: usize,
) -> String {
    if let Some(decision) = rule_decision {
        if matches!(decision.action, RuleAction::Block | RuleAction::Alert) {
            let rules: Vec<_> = decision
                .matched_rules
                .iter()
                .map(|r| r.rule_name.as_str())
                .collect();

            if rules.is_empty() {
                format!(
                    "Program {} triggered security rules in {} transaction(s). Risk score: {}",
                    program_id, occurrence_count, risk_score
                )
            } else {
                format!(
                    "Program {} triggered rules: {} in {} transaction(s). Risk score: {}",
                    program_id,
                    rules.join(", "),
                    occurrence_count,
                    risk_score
                )
            }
        } else if !is_known {
            format!(
                "Unknown program {} encountered in {} transaction(s). Risk score: {}",
                program_id, occurrence_count, risk_score
            )
        } else {
            format!(
                "Known program {} used in {} transaction(s). Risk score: {}",
                program_id, occurrence_count, risk_score
            )
        }
    } else if !is_known {
        format!(
            "Unknown program {} encountered in {} transaction(s). Risk score: {}. No security rules triggered.",
            program_id, occurrence_count, risk_score
        )
    } else {
        format!(
            "Known program {} used in {} transaction(s).",
            program_id, occurrence_count
        )
    }
}

/// Generate recommendation
pub fn generate_recommendation(
    risk_score: u8,
    rule_decision: Option<&RuleDecision>,
    is_known: bool,
) -> String {
    if let Some(decision) = rule_decision {
        match decision.action {
            RuleAction::Block => {
                return format!(
                    "CRITICAL: This program triggered blocking rules. Review all transactions involving this program and revoke any active delegations immediately. {}",
                    decision.message
                );
            }
            RuleAction::Alert => {
                return format!(
                    "WARNING: This program triggered security alerts. Review transactions for suspicious activity. {}",
                    decision.message
                );
            }
            _ => {}
        }
    }

    if !is_known {
        if risk_score > 60 {
            "This unknown program has a high risk score. Investigate transactions involving this program and verify its legitimacy before further interaction.".to_string()
        } else {
            "This program is not in the known safe list. Verify its legitimacy through on-chain data and community sources.".to_string()
        }
    } else {
        "This program is known and generally considered safe. Monitor for any unusual behavior."
            .to_string()
    }
}

/// Create a SuspiciousProgram from encounter data
pub fn create_suspicious_program(
    program_id: String,
    transaction_signatures: Vec<String>,
    first_seen: DateTime<Utc>,
    rule_decision: Option<&RuleDecision>,
) -> SuspiciousProgram {
    let occurrence_count = transaction_signatures.len();
    let is_known = is_known_program(&program_id);
    let risk_score = calculate_program_risk_score(&program_id, rule_decision, occurrence_count);
    let confidence = calculate_confidence(risk_score, rule_decision, occurrence_count);
    let threat_type = classify_threat_type(risk_score, rule_decision, is_known);
    let analysis_summary = generate_analysis_summary(
        &program_id,
        risk_score,
        rule_decision,
        is_known,
        occurrence_count,
    );
    let recommendation = generate_recommendation(risk_score, rule_decision, is_known);

    // Extract analyzer names from rule decision
    let detected_by = if let Some(decision) = rule_decision {
        decision
            .matched_rules
            .iter()
            .map(|r| r.rule_name.clone())
            .collect()
    } else {
        vec!["HistoryScanner".to_string()]
    };

    SuspiciousProgram {
        program_id,
        risk_score,
        confidence,
        threat_type,
        first_seen,
        transaction_signatures,
        occurrence_count,
        analysis_summary,
        recommendation,
        detected_by,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_known_programs() {
        assert!(is_known_program("11111111111111111111111111111111"));
        assert!(is_known_program(
            "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
        ));
        assert!(!is_known_program("UnknownProgram1234567890"));
    }

    #[test]
    fn test_risk_score_unknown_program() {
        let score = calculate_program_risk_score("UnknownProgram", None, 1);
        assert_eq!(score, 25); // 20 (unknown) + 5 (single occurrence)
    }

    #[test]
    fn test_risk_score_frequent_program() {
        let score = calculate_program_risk_score("UnknownProgram", None, 15);
        assert_eq!(score, 10); // 20 (unknown) - 10 (frequent)
    }

    #[test]
    fn test_threat_type_classification() {
        let threat_type = classify_threat_type(80, None, false);
        assert_eq!(threat_type, "high_risk_unknown");

        let threat_type2 = classify_threat_type(40, None, false);
        assert_eq!(threat_type2, "unknown");

        let threat_type3 = classify_threat_type(40, None, true);
        assert_eq!(threat_type3, "monitored");
    }
}
