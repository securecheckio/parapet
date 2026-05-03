//! Helpers for parsing comma-separated RPC URL lists (`SOLANA_RPC_URL`, `UPSTREAM_RPC_URLS`, etc.).

/// Split a comma-separated list of RPC URLs into trimmed non-empty strings.
pub fn parse_upstream_urls_list(s: &str) -> Vec<String> {
    s.split(',')
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty())
        .collect()
}
