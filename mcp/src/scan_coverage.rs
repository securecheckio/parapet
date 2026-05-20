//! Build marketing-friendly scan coverage from engine state.

use parapet_core::rules::AnalyzerRegistry;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanCoverage {
    pub rules_loaded: Vec<String>,
    pub analyzers_enabled: Vec<String>,
    pub providers: Vec<ProviderCoverage>,
    pub core: CoreCoverage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderCoverage {
    pub id: String,
    pub label: String,
    pub configured: bool,
    pub enabled_for_scan: bool,
    pub has_results: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub highlights: Vec<CoverageHighlight>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageHighlight {
    pub label: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreCoverage {
    pub label: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub highlights: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RulesSnapshot {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_commit: Option<String>,
    pub feeds: Vec<FeedSnapshotEntry>,
    pub merged_rule_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshot_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedSnapshotEntry {
    pub url: Option<String>,
    pub name: String,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub published_at: Option<String>,
    pub rule_count: usize,
}

const PROVIDERS: &[(&str, &str, &[&str])] = &[
    ("helius", "Helius", &["helius_identity", "helius_transfer", "helius_funding"]),
    ("jupiter", "Jupiter", &["jupiter"]),
    ("rugcheck", "Rugcheck", &["rugcheck"]),
    ("ottersec", "OtterSec", &["ottersec"]),
];

fn provider_configured(id: &str) -> bool {
    match id {
        "helius" => std::env::var("HELIUS_API_KEY").is_ok(),
        "ottersec" => std::env::var("OTTERSEC_API_KEY").is_ok(),
        "jupiter" => true, // public API available
        "rugcheck" => true,
        _ => false,
    }
}

fn namespace_has_data(fields: &HashMap<String, Value>, prefixes: &[&str]) -> bool {
    fields.iter().any(|(k, v)| {
        prefixes.iter().any(|p| k == *p || k.starts_with(&format!("{p}:")))
            && !v.is_null()
            && v != &Value::Bool(false)
            && v != &Value::String(String::new())
    })
}

fn pick_highlights(fields: &HashMap<String, Value>, prefixes: &[&str]) -> Vec<CoverageHighlight> {
    let keys = [
        ("is_verified", "Verified"),
        ("risk_score", "Risk score"),
        ("is_sus", "Suspicious"),
        ("is_rugged", "Rugged"),
        ("danger_count", "Danger signals"),
        ("label", "Label"),
    ];
    let mut out = Vec::new();
    for (key, label) in keys {
        for (fk, fv) in fields {
            if prefixes.iter().any(|p| fk.starts_with(p)) && fk.contains(key) {
                if let Some(s) = value_display(fv) {
                    out.push(CoverageHighlight {
                        label: label.to_string(),
                        value: s,
                    });
                    break;
                }
            }
        }
        if out.len() >= 4 {
            break;
        }
    }
    out
}

fn value_display(v: &Value) -> Option<String> {
    match v {
        Value::Bool(b) => Some(b.to_string()),
        Value::Number(n) => Some(n.to_string()),
        Value::String(s) if !s.is_empty() => Some(s.clone()),
        _ => None,
    }
}

/// Wallet quick-scan coverage (no full rule engine on history).
pub fn wallet_scan_coverage(threat_count: usize, delegation_checked: bool) -> ScanCoverage {
    let summary = if threat_count > 0 {
        format!("{threat_count} active threat(s) detected")
    } else if delegation_checked {
        "Checked — no active delegations".to_string()
    } else {
        "Delegation check skipped".to_string()
    };

    ScanCoverage {
        rules_loaded: vec!["wallet-state-scan".to_string()],
        analyzers_enabled: vec!["state_scanner:delegations".to_string()],
        providers: vec![ProviderCoverage {
            id: "core".to_string(),
            label: "Parapet Core".to_string(),
            configured: true,
            enabled_for_scan: true,
            has_results: threat_count > 0,
            summary: Some(summary),
            highlights: vec![],
        }],
        core: CoreCoverage {
            label: "Parapet Core".to_string(),
            highlights: vec!["Active token delegation scan".to_string()],
        },
    }
}

pub fn build_scan_coverage(
    required_analyzers: &[String],
    analyzer_fields: &HashMap<String, Value>,
    registry: &AnalyzerRegistry,
    rules_loaded: Vec<String>,
) -> ScanCoverage {
    let required_set: HashSet<_> = required_analyzers.iter().cloned().collect();
    let mut providers = Vec::new();
    let mut core_highlights = Vec::new();

    for (id, label, namespaces) in PROVIDERS {
        let configured = provider_configured(id);
        let enabled = namespaces
            .iter()
            .any(|ns| required_set.contains(*ns) || required_set.iter().any(|r| r.starts_with(ns)));
        let has_results = namespace_has_data(analyzer_fields, namespaces);
        let highlights = if has_results {
            pick_highlights(analyzer_fields, namespaces)
        } else {
            vec![]
        };

        let summary = if !configured {
            Some("Not configured on this deployment".to_string())
        } else if !enabled {
            Some("Not required by loaded rules".to_string())
        } else if has_results {
            None
        } else {
            Some("Checked — no signals".to_string())
        };

        providers.push(ProviderCoverage {
            id: (*id).to_string(),
            label: (*label).to_string(),
            configured,
            enabled_for_scan: enabled,
            has_results,
            summary,
            highlights,
        });
    }

    if namespace_has_data(
        analyzer_fields,
        &["basic", "token_instructions", "system", "security"],
    ) {
        if let Some(n) = analyzer_fields.get("basic:instruction_count").and_then(|v| v.as_u64()) {
            core_highlights.push(format!("{n} instructions"));
        }
    }

    let core_enabled = required_analyzers
        .iter()
        .any(|a| !PROVIDERS.iter().any(|(_, _, ns)| ns.contains(&a.as_str())));

    providers.push(ProviderCoverage {
        id: "core".to_string(),
        label: "Parapet Core".to_string(),
        configured: true,
        enabled_for_scan: core_enabled || !required_analyzers.is_empty(),
        has_results: !core_highlights.is_empty() || namespace_has_data(analyzer_fields, &["basic"]),
        summary: if core_enabled && core_highlights.is_empty() {
            Some("Checked — on-chain patterns".to_string())
        } else {
            None
        },
        highlights: vec![],
    });

    let _ = registry; // reserved for future per-analyzer availability

    ScanCoverage {
        rules_loaded,
        analyzers_enabled: required_analyzers.to_vec(),
        providers,
        core: CoreCoverage {
            label: "Parapet Core".to_string(),
            highlights: core_highlights,
        },
    }
}

pub fn build_rules_snapshot(
    git_commit: Option<String>,
    feeds: Vec<FeedSnapshotEntry>,
    merged_rule_count: usize,
) -> RulesSnapshot {
    let snapshot_hash = {
        let mut ids: Vec<String> = feeds.iter().map(|f| format!("{}:{}", f.name, f.version)).collect();
        ids.sort();
        Some(format!("{:x}", md5_hash(&ids.join("|"))))
    };

    RulesSnapshot {
        git_commit: git_commit.or_else(|| std::env::var("RULES_GIT_COMMIT").ok()),
        feeds,
        merged_rule_count,
        snapshot_hash,
    }
}

fn md5_hash(s: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    s.hash(&mut h);
    h.finish()
}
