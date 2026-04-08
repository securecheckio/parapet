# SecureCheck RPC Gateway

The secure RPC gateway that wraps the open-source `parapet-rpc-proxy` with SaaS features.

## Architecture

```
User Wallet
    │
    └─> https://rpc.securecheck.io (Authorization: Bearer sk_xxx)
           │
           ├─> SaasAuthProvider (this repo)
           │   ├─> PostgreSQL (users, wallets, tiers)
           │   └─> Redis (caching, rate limits)
           │
           └─> OSS RPC Proxy (inherited from parapet)
               ├─> Rules Engine (bot-essentials.json)
               └─> Upstream RPC
```

## Features

- **Database-Backed Auth**: User accounts with API keys
- **Rate Limiting**: Per-tier quotas (10k free, 100k starter, 1M pro)
- **Simple & Secure**: Transaction signatures prove wallet ownership (no claiming needed)
- **Inherits All OSS Features**: Rules engine, analyzers, caching

## Quick Start

### 1. Set up database

```bash
# Create PostgreSQL database
createdb securecheck

# Run migrations
psql securecheck < schema.sql
```

### 2. Configure environment

```bash
cp .env.example .env
# Edit .env with your database/redis credentials
```

### 3. Run the server

```bash
cargo run --release
```

The authenticated RPC will be available at `http://localhost:8899`

## Usage

### For Users

```bash
curl -X POST http://localhost:8899 \
  -H "Authorization: Bearer sk_abc123..." \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "sendTransaction",
    "params": ["..."]
  }'
```

### Rate Limit Headers

```
X-RateLimit-Remaining: 9500
X-RateLimit-Reset: 1234567890
```

## Tiers

| Tier | Requests/Month | Price |
|------|----------------|-------|
| Free | 10,000 | $0 |
| Starter | 100,000 | $29 |
| Pro | 1,000,000 | $99 |
| Enterprise | Unlimited | Contact |

## Development

### Test with existing OSS proxy

```bash
# In parapet
cd parapet-rpc-proxy
cargo build

# In securecheck-saas
cd reverse-proxy
cargo run
```

Changes to the OSS proxy are automatically inherited!

## Deployment

See `../deploy/` for production deployment scripts.
