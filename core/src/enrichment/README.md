# Enrichment Services

This module provides **low-level API clients** for fetching data from third-party security and reputation services.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     Transaction Analysis                     │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│          Analyzers (rules/analyzers/third_party/)           │
│  • RugcheckAnalyzer - extracts is_rugged, risk_score        │
│  • HeliusIdentityAnalyzer - extracts verified, twitter      │
│  • JupiterTokenAnalyzer - extracts price, liquidity         │
│  • OtterSecVerifiedAnalyzer - extracts program verified     │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│          Enrichment Services (enrichment/)                   │
│  • RugcheckClient - API client for rugcheck.xyz             │
│  • HeliusClient - API client for Helius APIs                │
│  • JupiterClient - API client for Jupiter price API         │
│  • OtterSecClient - API client for OtterSec verification    │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
                    External APIs (HTTP)
```

## Enrichment vs. Analyzers

### Enrichment Services (this module)

**Purpose:** Fetch raw data from third-party APIs

- Direct HTTP clients
- Return structured data types (RugcheckData, HeliusData, etc.)
- No policy evaluation or field extraction
- Can be used anywhere in the codebase
- Handle API keys, rate limiting, error handling

**Example:**

```rust
use parapet_core::enrichment::RugcheckClient;

let client = RugcheckClient::new();
let data = client.get_token_report("So11111...").await?;
println!("Is rugged: {}", data.is_rugged);
```

### Third-Party Analyzers (`rules/analyzers/third_party/`)

**Purpose:** Use enrichment data for transaction analysis

- Implement `TransactionAnalyzer` trait
- Use enrichment clients internally
- Extract fields for rule matching (e.g., `rugcheck:is_rugged`)
- Evaluate policies and generate warnings
- Part of the rules engine pipeline

**Example:**

```rust
use parapet_core::rules::analyzers::third_party::RugcheckAnalyzer;

let analyzer = RugcheckAnalyzer::new(client);
let result = analyzer.analyze(&transaction, &context)?;

// Fields extracted for rules:
// - rugcheck:is_rugged
// - rugcheck:risk_score
// - rugcheck:danger_count
```

## Available Services

### RugcheckClient

Token reputation and rug detection

- Endpoint: `https://api.rugcheck.xyz`
- Returns: Risk scores, holder analysis, LP data
- Rate limit: 10 requests/second

### HeliusClient

Solana program and identity verification

- Endpoint: `https://api.helius.xyz`
- Returns: Verified programs, identity data, funding analysis
- Requires: HELIUS_API_KEY

### JupiterClient

Token pricing and liquidity

- Endpoint: `https://price.jup.ag`
- Returns: USD price, 24h volume, liquidity
- Rate limit: 600 requests/minute

### OtterSecClient

Smart contract security verification

- Endpoint: `https://api.ottersec.xyz`
- Returns: Verified programs, audit status
- Rate limit: 100 requests/minute

## Usage

### EnrichmentService (Unified)

```rust
use parapet_core::enrichment::EnrichmentService;

let service = EnrichmentService::new()
    .with_rugcheck()
    .with_helius("api_key")
    .with_jupiter()
    .with_ottersec();

let data = service.enrich_token("So11111...").await?;

// Access all enrichment data
if let Some(rugcheck) = data.rugcheck {
    println!("Risk score: {}", rugcheck.risk_score);
}
if let Some(jupiter) = data.jupiter {
    println!("Price: ${}", jupiter.price);
}
```

### Individual Clients

```rust
use parapet_core::enrichment::{RugcheckClient, HeliusClient};

// Rugcheck
let rugcheck = RugcheckClient::new();
let report = rugcheck.get_token_report(mint).await?;

// Helius
let helius = HeliusClient::new("api_key");
let identity = helius.get_identity(address).await?;
```

## Rate Limiting

All clients include built-in rate limiting to respect API quotas:

- Concurrent request limiting
- Per-second/minute throttling
- Automatic retry with backoff
- Cache integration (when Redis available)

## Error Handling

All clients return `Result<T, anyhow::Error>`:

- Network errors
- API quota exceeded
- Invalid responses
- Timeout errors

Use `.ok()` or `unwrap_or_default()` for graceful degradation when enrichment is not critical.

## Testing

Enrichment services are optional and feature-gated:

```bash
# Build with enrichment support
cargo build --features reqwest,helius,jupiter,rugcheck,ottersec

# Test without external dependencies
cargo test --no-default-features
```

## Configuration

Set API keys via environment variables:

```bash
export HELIUS_API_KEY="your_key"
export OTTERSEC_API_KEY="your_key"
```

Rugcheck and Jupiter do not require API keys.