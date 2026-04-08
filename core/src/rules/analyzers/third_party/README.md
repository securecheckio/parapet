# Third-Party Analyzers

Analyzers that integrate external security and reputation services via API calls.

## Overview

Third-party analyzers enrich transaction analysis with off-chain data from security vendors, DEXes, and reputation systems. Unlike core analyzers, these require network access and external API keys.

**Key Differences from Core:**

- Require external API calls (adds 50-500ms latency)
- Need API keys and authentication
- Feature-gated (opt-in via Cargo features)
- Include rate limiting and caching to respect API quotas

## Architecture

```
Transaction → Core Analyzers (fast, on-chain)
           ↓
           → Third-Party Analyzers (slower, off-chain enrichment)
           ↓
           → Rules Engine
```

## Configuration

Enable via Cargo features:

```toml
parapet-core = { features = ["helius", "rugcheck", "jupiter", "ottersec"] }
```

API keys via environment variables:

```bash
HELIUS_API_KEY=your_key
OTTERSEC_API_KEY=your_key
```

## Performance

Third-party analyzers use:

- **Redis caching** - Reduces redundant API calls
- **Rate limiting** - Respects vendor quotas
- **Concurrent limits** - Prevents API exhaustion
- **Timeouts** - Fails gracefully when APIs are slow

## Documentation

Each analyzer has its own `.md` file in this directory with detailed field descriptions, use cases, and example rules.


| Doc                                          | Analyzer area                         |
| -------------------------------------------- | ------------------------------------- |
| [helius_funding.md](helius_funding.md)       | Funding / sybil signals (Helius)      |
| [helius_identity.md](helius_identity.md)     | Wallet identity / reputation (Helius) |
| [helius_transfer.md](helius_transfer.md)     | Transfer velocity & patterns (Helius) |
| [jupiter_token.md](jupiter_token.md)         | Jupiter token metadata                |
| [ottersec_verified.md](ottersec_verified.md) | OtterSec verification                 |
| [rugcheck.md](rugcheck.md)                   | RugCheck token risk                   |
| [squads_v4.md](squads_v4.md)                 | Squads v4                             |
| [token_mint.md](token_mint.md)               | Token mint validation                 |


