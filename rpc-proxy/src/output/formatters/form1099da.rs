use crate::output::event::{TransactionEvent, TransactionOutcome};
use crate::output::formatter::OutputFormatter;
use anyhow::Result;
use async_trait::async_trait;
use chrono::Datelike;
use serde_json::json;

/// Form 1099-DA format for IRS digital asset reporting (2026+)
pub struct Form1099DaFormatter;

#[async_trait]
impl OutputFormatter for Form1099DaFormatter {
    fn format_event(&self, event: &TransactionEvent) -> Result<Vec<u8>> {
        // Only format allowed transactions that involve value transfer
        if !matches!(event.outcome, TransactionOutcome::Allowed) {
            return Ok(Vec::new());
        }

        // Only report actual transfers/sales, not simulations
        if event.action_type.as_deref() != Some("transfer")
            && event.action_type.as_deref() != Some("swap")
        {
            return Ok(Vec::new());
        }

        // Extract amount
        let amount = event
            .amount
            .as_ref()
            .and_then(|a| a.split_whitespace().next())
            .and_then(|a| a.parse::<f64>().ok())
            .unwrap_or(0.0);

        let asset = event.tokens.first().map(|s| s.as_str()).unwrap_or("SOL");

        // IRS Form 1099-DA structure (2026 format)
        let form = json!({
            "formType": "1099-DA",
            "taxYear": event.timestamp.year(),
            "corrected": false,
            "void": false,

            // Payer (your organization)
            "payer": {
                "name": "SecureCheck RPC Proxy",
                "tin": "", // To be configured
                "address": {
                    "line1": "",
                    "city": "",
                    "state": "",
                    "zip": ""
                }
            },

            // Payee (user)
            "payee": {
                "name": event.identity,
                "tin": "", // Should come from KYC
                "walletAddress": event.wallet,
                "accountNumber": event.wallet.chars().take(10).collect::<String>()
            },

            // Transaction details
            "transaction": {
                "date": event.timestamp.format("%Y-%m-%d").to_string(),
                "time": event.timestamp.format("%H:%M:%S").to_string(),
                "type": match event.action_type.as_deref() {
                    Some("swap") => "SALE",
                    Some("transfer") => "TRANSFER",
                    _ => "OTHER"
                },

                // Box 1a: Date and time of transaction
                "dateTime": event.timestamp.to_rfc3339(),

                // Box 1b: Type of digital asset
                "assetType": asset,
                "assetDescription": format!("{} on Solana", asset),

                // Box 1c: Number of units
                "units": amount,

                // Box 1d: Fair market value (would need price oracle)
                "fmvUsd": null,

                // Box 2: Gross proceeds (for sales)
                "grossProceeds": if event.action_type.as_deref() == Some("swap") {
                    Some(amount)
                } else {
                    None
                },

                // Transaction ID for audit trail
                "transactionId": event.signature,
                "blockchainSignature": event.signature,
                "blockSlot": event.slot,

                // Protocol/exchange
                "exchange": event.protocol,

                // Risk assessment (for suspicious activity reporting)
                "riskScore": event.risk_score,
                "riskFlags": if event.risk_score >= 70 {
                    event.issues.clone()
                } else {
                    vec![]
                }
            },

            // Metadata
            "filingRequirement": if amount >= 10.0 { "REQUIRED" } else { "OPTIONAL" },
            "reportingEntity": "BROKER",
            "recordType": "TRANSACTION",

            // Compliance
            "complianceChecks": {
                "amlScreening": event.risk_score < 80,
                "kycCompliant": event.identity.is_some(),
                "sanctionsScreened": !event.issues.iter().any(|i| i.contains("blocklist"))
            }
        });

        Ok(serde_json::to_vec_pretty(&form)?)
    }

    fn content_type(&self) -> &str {
        "application/json"
    }

    fn name(&self) -> &str {
        "1099-da"
    }
}
