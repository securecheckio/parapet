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
