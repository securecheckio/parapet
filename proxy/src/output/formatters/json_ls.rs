use crate::output::event::{TransactionEvent, TransactionOutcome};
use crate::output::formatter::OutputFormatter;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;

/// JSON Lines format for Splunk/security monitoring
pub struct JsonLsFormatter;

#[async_trait]
impl OutputFormatter for JsonLsFormatter {
    fn format_event(&self, event: &TransactionEvent) -> Result<Vec<u8>> {
        // Splunk-optimized JSON format with flat structure
        let output = json!({
            "event_id": event.event_id,
            "timestamp": event.timestamp.to_rfc3339(),
            "event_type": match event.outcome {
                TransactionOutcome::Allowed => "transaction_allowed",
                TransactionOutcome::Blocked => "transaction_blocked",
                TransactionOutcome::Failed => "transaction_failed",
                TransactionOutcome::RequiresApproval => "transaction_requires_approval",
                TransactionOutcome::Simulation => "transaction_simulation",
            },

            // Attribution
            "user_id": event.user_id,
            "identity": event.identity,
            "wallet": event.wallet,
            "ip_address": event.ip_address,
            "tier": event.tier,

            // Security metrics
            "risk_score": event.risk_score,
            "risk_level": event.risk_level.as_str(),
            "issues": event.issues,
            "block_reason": event.block_reason,

            // Transaction details
            "method": event.method,
            "signature": event.signature,
            "slot": event.slot,
            "programs": event.programs,
            "program_names": event.program_names,

            // Human-readable
            "summary": event.summary,
            "action_type": event.action_type,
            "protocol": event.protocol,
            "amount": event.amount,
            "tokens": event.tokens,

            // Analyzers
            "analyzers_used": event.analyzers_used,
            "rule_matches": event.rule_matches,

            // Metadata
            "engine_version": event.engine_version,
            "compute_units": event.compute_units,

            // Splunk indexing hints
            "sourcetype": "securecheck:transaction",
            "source": "sol-shield-rpc-proxy",
        });

        let mut bytes = serde_json::to_vec(&output)?;
        bytes.push(b'\n'); // JSON Lines format
        Ok(bytes)
    }

    fn content_type(&self) -> &str {
        "application/x-ndjson"
    }

    fn name(&self) -> &str {
        "json-ls"
    }
}
