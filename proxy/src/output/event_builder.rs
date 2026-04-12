use crate::auth::AuthContext;
use crate::output::event::{RiskLevel, RuleMatch, TransactionEvent, TransactionOutcome};
use parapet_core::rules::types::RuleDecision;
use std::collections::HashMap;

/// Build a TransactionEvent from RPC request analysis
pub struct EventBuilder {
    event: TransactionEvent,
}

impl EventBuilder {
    pub fn new(wallet: String, method: String) -> Self {
        Self {
            event: TransactionEvent::new(wallet, method),
        }
    }

    /// Add authentication context
    pub fn with_auth_context(mut self, auth_context: &AuthContext) -> Self {
        self.event.user_id = if auth_context.identity != "anonymous" {
            Some(auth_context.identity.clone())
        } else {
            None
        };
        self.event.identity = Some(auth_context.identity.clone());
        self.event.tier = auth_context.tier.clone();
        self.event.scopes = auth_context.scopes.clone();
        self
    }

    /// Add rule decision (block/alert/pass)
    pub fn with_rule_decision(mut self, decision: &RuleDecision) -> Self {
        // Use the actual risk score from the decision
        self.event.risk_score = decision.total_risk as u32;
        self.event.risk_level = match decision.total_risk {
            0..=25 => RiskLevel::Low,
            26..=50 => RiskLevel::Medium,
            51..=75 => RiskLevel::High,
            _ => RiskLevel::Critical,
        };

        if !decision.message.is_empty() {
            self.event.issues.push(decision.message.clone());
        }

        // Add all matched rules (not just the first one)
        for matched in &decision.matched_rules {
            self.event.rule_matches.push(RuleMatch {
                rule_id: matched.rule_id.clone(),
                rule_name: matched.rule_name.clone(),
                action: format!("{:?}", matched.action).to_lowercase(),
                reason: matched.message.clone(),
                matched_fields: HashMap::new(),
            });
        }

        // Set outcome based on action
        self.event.outcome = match decision.action {
            parapet_core::rules::types::RuleAction::Block => {
                self.event.block_reason = Some(decision.message.clone());
                TransactionOutcome::Blocked
            }
            parapet_core::rules::types::RuleAction::Alert => TransactionOutcome::Allowed,
            parapet_core::rules::types::RuleAction::Pass => TransactionOutcome::Allowed,
        };

        // Generate human-readable summary
        self.event.summary = match decision.action {
            parapet_core::rules::types::RuleAction::Block => {
                if decision.matched_rules.len() > 1 {
                    format!(
                        "Blocked: {} risk factors detected ({}/100 risk weight)",
                        decision.matched_rules.len(),
                        decision.total_risk
                    )
                } else {
                    format!("Blocked: {}", decision.message)
                }
            }
            parapet_core::rules::types::RuleAction::Alert => {
                if decision.matched_rules.len() > 1 {
                    format!(
                        "{} suspicious patterns ({}/100 risk weight)",
                        decision.matched_rules.len(),
                        decision.total_risk
                    )
                } else {
                    decision.message.clone()
                }
            }
            parapet_core::rules::types::RuleAction::Pass => {
                "All security checks passed".to_string()
            }
        };

        self
    }

    /// Add transaction signature (after successful send)
    pub fn with_signature(mut self, signature: String, slot: Option<u64>) -> Self {
        self.event.signature = Some(signature);
        self.event.slot = slot;
        self.event.outcome = TransactionOutcome::Allowed;

        if self.event.summary.is_empty() {
            self.event.summary = "Transaction sent successfully".to_string();
        }

        self
    }

    /// Mark as allowed (passed all checks)
    pub fn allowed(mut self) -> Self {
        self.event.outcome = TransactionOutcome::Allowed;
        if self.event.summary.is_empty() {
            self.event.summary = "Transaction allowed".to_string();
        }
        self
    }

    /// Build the final event
    pub fn build(self) -> TransactionEvent {
        self.event
    }
}

/// Helper function to emit event asynchronously (non-blocking)
pub async fn emit_event(
    output_manager: &Option<std::sync::Arc<crate::output::OutputManager>>,
    event: TransactionEvent,
) {
    if let Some(manager) = output_manager {
        log::debug!("📤 emit_event called for event_id: {}", event.event_id);
        let manager = manager.clone();
        tokio::spawn(async move {
            log::debug!("🔄 Writing event to output manager...");
            if let Err(e) = manager.write_event(&event).await {
                log::error!("❌ Failed to write output event: {}", e);
            } else {
                log::debug!("✅ Event written to output manager successfully");
            }
        });
    } else {
        log::warn!("⚠️  No output manager configured, event not emitted");
    }
}
