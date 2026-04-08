use crate::output::event::{TransactionEvent, TransactionOutcome};
use crate::output::formatter::OutputFormatter;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;

/// xBRL-JSON format for SEC/MiCA financial reporting
pub struct XbrlJsonFormatter;

#[async_trait]
impl OutputFormatter for XbrlJsonFormatter {
    fn format_event(&self, event: &TransactionEvent) -> Result<Vec<u8>> {
        // Only format allowed transactions for financial statements
        if !matches!(event.outcome, TransactionOutcome::Allowed) {
            return Ok(Vec::new());
        }

        // Extract amount
        let amount = event
            .amount
            .as_ref()
            .and_then(|a| a.split_whitespace().next())
            .and_then(|a| a.parse::<f64>().ok())
            .unwrap_or(0.0);

        let currency = event.tokens.first().map(|s| s.as_str()).unwrap_or("SOL");

        // xBRL-JSON structure per SEC/ESMA guidelines
        let xbrl = json!({
            "documentInfo": {
                "documentType": "https://xbrl.sec.gov/dei/2023",
                "namespaces": {
                    "dei": "https://xbrl.sec.gov/dei/2023",
                    "crypto": "https://xbrl.sec.gov/crypto/2026",
                    "securecheck": "https://securecheck.io/xbrl/2026"
                },
                "taxonomy": "https://xbrl.sec.gov/crypto/2026",
            },
            "facts": {
                // Transaction identification
                "crypto:TransactionId": {
                    "value": event.event_id,
                    "decimals": null,
                    "dimensions": {}
                },
                "crypto:TransactionSignature": {
                    "value": event.signature,
                    "decimals": null,
                    "dimensions": {}
                },
                "crypto:TransactionTimestamp": {
                    "value": event.timestamp.to_rfc3339(),
                    "decimals": null,
                    "dimensions": {}
                },

                // Financial data
                "crypto:DigitalAssetTransferAmount": {
                    "value": amount,
                    "decimals": 9,
                    "dimensions": {
                        "crypto:DigitalAsset": currency,
                        "crypto:TransactionType": event.action_type.as_deref().unwrap_or("transfer")
                    },
                    "unit": currency
                },

                // Party identification
                "dei:EntityIdentifier": {
                    "value": event.wallet,
                    "decimals": null,
                    "dimensions": {}
                },
                "dei:EntityName": {
                    "value": event.identity,
                    "decimals": null,
                    "dimensions": {}
                },

                // Protocol/counterparty
                "crypto:CounterpartyProtocol": {
                    "value": event.protocol,
                    "decimals": null,
                    "dimensions": {}
                },
                "crypto:CounterpartyAddress": {
                    "value": event.destination,
                    "decimals": null,
                    "dimensions": {}
                },

                // Risk assessment (MiCA compliance)
                "securecheck:RiskScore": {
                    "value": event.risk_score,
                    "decimals": 0,
                    "dimensions": {},
                    "unit": "pure"
                },
                "securecheck:RiskLevel": {
                    "value": event.risk_level.as_str(),
                    "decimals": null,
                    "dimensions": {}
                },
                "securecheck:ComplianceStatus": {
                    "value": "APPROVED",
                    "decimals": null,
                    "dimensions": {}
                },

                // Blockchain metadata
                "crypto:BlockchainNetwork": {
                    "value": "Solana",
                    "decimals": null,
                    "dimensions": {}
                },
                "crypto:BlockSlot": {
                    "value": event.slot,
                    "decimals": 0,
                    "dimensions": {}
                },
                "crypto:ComputeUnits": {
                    "value": event.compute_units,
                    "decimals": 0,
                    "dimensions": {}
                },

                // Programs involved
                "crypto:SmartContractPrograms": {
                    "value": event.program_names.join(", "),
                    "decimals": null,
                    "dimensions": {}
                }
            }
        });

        Ok(serde_json::to_vec_pretty(&xbrl)?)
    }

    fn content_type(&self) -> &str {
        "application/json"
    }

    fn name(&self) -> &str {
        "xbrl-json"
    }
}
