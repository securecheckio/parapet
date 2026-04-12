# Parapet Platform

Full-featured SaaS platform with multi-user dashboard, authentication, payments, push notifications, and learning system. Extends `api-core` with platform-specific features.

## Features

All features from `api-core` plus:

- **User Management**: PostgreSQL-backed user accounts and sessions
- **Dashboard**: Session-based authentication for web dashboard
- **Payment System**: Solana token payments for credit purchases
- **Push Notifications**: Web Push for real-time alerts
- **Learning System**: Courses, progress tracking, and badges
- **Wallet Scanner**: Security analysis for Solana wallets
- **Analytics**: Global stats, user stats, security events

## Quick Start

### 1. Prerequisites

- PostgreSQL database
- Redis
- Solana RPC endpoint

### 2. Configuration

Copy example configs:

```bash
cd api-platform
cp ../api-core/config.example.toml config.toml
cp platform-config.example.toml platform-config.toml
```

`**config.toml**` (shared with api-core):

- Server, runtime, Redis, Solana, auth, rate limiting

`**platform-config.toml**` (platform-specific):

- Database URL
- Frontend URL for CORS
- Push notification VAPID keys
- Payment configuration (token, pricing, treasury wallet)
- Learning system settings
- Rules display path

Environment variables can override settings. See example files for details.

### 3. Database Setup

```bash
# Run migrations
sqlx migrate run
```

### 4. Run

```bash
cargo run --bin parapet-platform

# Or with custom config paths:
cargo run --bin parapet-platform -- \
  --config /path/to/config.toml \
  --platform-config /path/to/platform-config.toml
```

## API Endpoints

### Core API (from `api-core`)

All endpoints from `parapet-api-core` are available:

- `/health`, `/api/v1/auth/nonce`
- `/api/v1/rules/*` (wallet-authenticated)
- `/api/v1/escalations/*` (wallet-authenticated)
- `/mcp/*` (API key authenticated)
- `/ws/escalations` (WebSocket)

### Platform Endpoints

#### Public

- `GET /vapid-public-key` - VAPID public key for push subscriptions
- `GET /system/network` - Solana network info
- `GET /learn/courses` - List courses
- `GET /learn/courses/:id` - Get course
- `GET /learn/badges` - List badges

#### Session-based (Dashboard)

- `POST /auth/login` - Login with wallet signature
- `GET /auth/me` - Get current user
- `POST /auth/logout` - Logout
- `GET /auth/api-key` - Get my API key
- `POST /auth/api-key/regenerate` - Regenerate API key
- `GET /dashboard/stats` - My usage stats
- `GET /dashboard/events` - My security events
- `GET /dashboard/usage` - My usage details
- `GET /dashboard/rules` - Active rules with hit counts
- `PUT /dashboard/threshold` - Update blocking threshold
- `PUT /dashboard/notifications` - Toggle push notifications
- `POST /dashboard/push/subscribe` - Subscribe to push
- `GET /dashboard/ws` - WebSocket for dashboard events
- `GET /learn/progress/me` - My learning progress
- `POST /wallet/scan` - Scan wallet security

#### Legacy API Key Endpoints

- `POST /signup` - Register wallet + get API key
- `GET /usage` - Get usage stats
- `GET /api-keys` - List API keys
- `POST /api-keys` - Create API key
- `POST /payment/create` - Create payment intent
- `POST /payment/verify` - Verify payment transaction
- `GET /payment/pricing` - Get pricing info
- `GET /stats/user/:api_key` - User stats
- `GET /stats/global` - Global stats
- `GET /stats/events/:api_key` - Security events

## Architecture

### Extending Core API

`api-platform` imports `parapet_api_core` as a library and extends it:

```rust
// PlatformState implements ApiStateAccess
pub struct PlatformState {
    // Core fields (for ApiStateAccess trait)
    pub redis_conn_mgr: Arc<Option<ConnectionManager>>,
    pub config: Arc<ApiConfig>,
    pub mcp_rate_limiter: McpRateLimiter,
    
    // Platform-specific additions
    pub redis: redis::Client,  // For cache operations
    pub db: PgPool,
    pub sessions: SessionStore,
    pub platform_config: Arc<PlatformConfig>,
}
```

Routes are merged in `main.rs`:

```rust
let core_router = parapet_api_core::create_router(state.clone());
let platform_router = Router::new()
    .route("/auth/login", post(login))
    // ... platform routes ...
    .with_state(state);

let app = core_router.merge(platform_router);
```

### Configuration

Two-tier configuration:

1. `**config.toml**`: Loaded by `parapet_api_core`, used for runtime, Redis, Solana, MCP
2. `**platform-config.toml**`: Platform-specific settings (DB, payments, frontend)

Both support environment variable overrides.

## Development

### Building

```bash
# Check compilation
cargo check -p parapet-platform

# Build release
cargo build --release -p parapet-platform

# Run tests
cargo test -p parapet-platform
```

### Database Migrations

```bash
sqlx migrate add my_migration
# Edit the generated SQL file
sqlx migrate run
```

## Deployment

```bash
# Build release binary
cargo build --release --bin parapet-platform

# Copy binary and configs
cp target/release/parapet-platform /opt/parapet/
cp config.toml platform-config.toml /opt/parapet/

# Run (from directory containing config files)
cd /opt/parapet && ./parapet-platform
```

## Environment Variables

See `config.example.toml` and `platform-config.example.toml` for all supported overrides.

Key production overrides:

- `DATABASE_URL` - PostgreSQL connection
- `REDIS_URL` - Redis connection
- `VAPID_PUBLIC_KEY` / `VAPID_PRIVATE_KEY` - Push notification keys
- `PAYMENTS_ENABLED` - Enable/disable payments
- `FRONTEND_URL` - CORS configuration

## Related

- `api-core/`: Lean core API library and binary
- `mcp/`: Standalone STDIO MCP server
- `proxy/`: Solana transaction proxy

