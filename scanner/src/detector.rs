use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::history::ProgramEncounter;
use crate::report::SuspiciousProgram;
use crate::classifier;

/// Threat severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
}

/// Types of threats that can be detected
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ThreatType {
    /// Active unlimited delegation that can be exploited now
    ActiveUnlimitedDelegation {
        token_account: String,
        delegate: String,
        amount: u64,
        granted_at: Option<i64>,
    },
    
    /// Delegation was granted but is now missing (possibly exploited)
    PossibleExploitedDelegation {
        token_account: String,
        delegate: String,
        amount: u64,
        granted_at: i64,
    },
    
    /// Account authority has been changed
    CompromisedAuthority {
        account: String,
        expected_owner: String,
        actual_owner: String,
    },
    
    /// Suspicious transaction detected in history
    SuspiciousTransaction {
        signature: String,
        threat_description: String,
        risk_score: u8,
        timestamp: Option<i64>,
    },
    
    /// Pattern of unusual activity
    UnusualPattern {
        pattern_description: String,
        occurrences: u32,
        first_seen: Option<i64>,
        last_seen: Option<i64>,
    },
}

/// Assessment of a specific threat
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatAssessment {
    pub threat_type: ThreatType,
    pub severity: Severity,
    pub recommendation: String,
}

/// Classifies and correlates threats from different scan sources
pub struct ThreatDetector;

impl ThreatDetector {
    pub fn new() -> Self {
        Self
    }

    /// Correlate active state with historical findings to identify exploited threats
    /// Returns (threats, suspicious_programs)
    pub async fn correlate_threats(
        &self,
        active_threats: Vec<ThreatAssessment>,
        historical_threats: Vec<ThreatAssessment>,
        program_encounters: HashMap<String, ProgramEncounter>,
    ) -> Result<(Vec<ThreatAssessment>, Vec<SuspiciousProgram>)> {
        let mut all_threats = Vec::new();
        
        // Add all active threats (highest priority)
        all_threats.extend(active_threats.clone());
        
        // Check for exploited delegations (historical approval but no current delegation)
        let mut exploited = self.find_exploited_delegations(&active_threats, &historical_threats);
        all_threats.append(&mut exploited);
        
        // Add remaining historical threats
        all_threats.extend(historical_threats);
        
        // Process suspicious programs
        let suspicious_programs = self.process_suspicious_programs(program_encounters);
        
        Ok((all_threats, suspicious_programs))
    }
    
    /// Process program encounters into suspicious program list
    fn process_suspicious_programs(
        &self,
        program_encounters: HashMap<String, ProgramEncounter>,
    ) -> Vec<SuspiciousProgram> {
        let mut suspicious: Vec<SuspiciousProgram> = program_encounters
            .into_iter()
            .filter_map(|(program_id, encounter)| {
                // Filter: Only report programs that are either:
                // 1. Unknown (not in core programs list)
                // 2. Triggered security rules
                let is_known = classifier::is_known_program(&program_id);
                let has_rule_violation = encounter.rule_decision.is_some();
                
                if !is_known || has_rule_violation {
                    Some(classifier::create_suspicious_program(
                        program_id,
                        encounter.transaction_signatures,
                        encounter.first_seen,
                        encounter.rule_decision.as_ref(),
                    ))
                } else {
                    None
                }
            })
            .collect();
        
        // Sort by risk score (highest first)
        suspicious.sort_by(|a, b| b.risk_score.cmp(&a.risk_score));
        
        // Limit to top 20 for readability
        suspicious.truncate(20);
        
        suspicious
    }

    /// Identify delegations that were granted historically but are now missing
    fn find_exploited_delegations(
        &self,
        active: &[ThreatAssessment],
        historical: &[ThreatAssessment],
    ) -> Vec<ThreatAssessment> {
        let mut exploited = Vec::new();

        // Extract historical approvals
        for hist_threat in historical {
            if let ThreatType::SuspiciousTransaction { 
                threat_description, 
                timestamp,
                signature,
                .. 
            } = &hist_threat.threat_type {
                // Check if this was a delegation grant
                if threat_description.contains("delegation") || 
                   threat_description.contains("approve") {
                    
                    // Check if there's a corresponding active delegation
                    let still_active = active.iter().any(|a| {
                        matches!(&a.threat_type, ThreatType::ActiveUnlimitedDelegation { .. })
                    });

                    if !still_active {
                        // Delegation was granted but is now missing - possible exploitation
                        exploited.push(ThreatAssessment {
                            threat_type: ThreatType::PossibleExploitedDelegation {
                                token_account: "unknown".to_string(),
                                delegate: "unknown".to_string(),
                                amount: u64::MAX,
                                granted_at: timestamp.unwrap_or(0),
                            },
                            severity: Severity::Critical,
                            recommendation: format!(
                                "Delegation found in transaction {} but no longer active. \
                                Check transaction history for unauthorized transfers.",
                                signature
                            ),
                        });
                    }
                }
            }
        }

        exploited
    }
}

impl Default for ThreatDetector {
    fn default() -> Self {
        Self::new()
    }
}
