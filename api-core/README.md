# Parapet Core API

Lean, headless API for AI agents and embedded use cases. Provides MCP server endpoints, rule management, and escalation workflows without user management or dashboard features.

## Features

- **MCP HTTP Server**: HTTP SSE + POST endpoints for remote AI agent access
- **Rule Management**: Create, update, delete dynamic security rules stored in Redis
- **Escalation Workflow**: Transaction review and approval system
- **WebSocket Events**: Real-time escalation notifications
- **Wallet Authentication**: Ed25519 signature verification for rule/escalation operations
- **API Key Authentication**: Bearer token auth for MCP endpoints with rate limiting
- **Graceful Degradation**: Starts without Redis, returns 503 for Redis-dependent routes

## Quick Start

### 1. Configuration

Copy the example config:

```bash
cp config.example.toml config.toml
```

Edit `config.toml` to match your environment. Key settings:

- `server.host` / `server.port`: Where to listen (default: `0.0.0.0:3000`)
- `redis.url`: Redis connection URL
- `solana.rpc_url` / `solana.network`: Solana network configuration
- `auth.authorized_wallets`: Wallet addresses allowed to manage rules/escalations
- `auth.mcp_api_keys`: API keys for MCP endpoint access
- `rate_limiting.max_concurrent_scans`: Concurrent MCP scan limit
- `rate_limiting.scans_per_hour_per_key`: Per-key hourly scan limit

Environment variables can override any config value. See `config.example.toml` for details.

### 2. Run

```bash
cargo run --bin parapet-api-core

# Or with custom config path:
cargo run --bin parapet-api-core -- --config /path/to/config.toml
```

### 3. Test

```bash
# Health check
curl http://localhost:3000/health

# MCP capabilities (requires API key)
curl -H "Authorization: Bearer YOUR_API_KEY" \
  http://localhost:3000/mcp/capabilities
```

## API Endpoints

### Core

- `GET /health` - Health check
- `POST /api/v1/auth/nonce` - Request auth nonce for wallet signatures

### Rules (Wallet-authenticated)

- `GET /api/v1/rules` - List dynamic rules
- `POST /api/v1/rules` - Create rule
- `PUT /api/v1/rules/:id` - Update rule
- `DELETE /api/v1/rules/:id` - Delete rule
- `POST /api/v1/rules/import` - Bulk import rules

### Escalations (Wallet-authenticated)

- `GET /api/v1/escalations` - List pending escalations
- `GET /api/v1/escalations/:id` - Get escalation details
- `POST /api/v1/escalations/:id/approve` - Approve transaction
- `POST /api/v1/escalations/:id/reject` - Reject transaction
- `GET /ws/escalations` - WebSocket for real-time events

### MCP (API Key authenticated)

- `GET /mcp/capabilities` - List available MCP tools
- `POST /mcp/tools/execute` - Execute MCP tool (wallet scanning, etc.)

## Architecture

### Library + Binary

This crate provides both:

- **Library** (`parapet_api_core`): Reusable router, state management, and config loading
- **Binary** (`parapet-api-core`): Standalone executable

The library defines an `ApiStateAccess` trait, allowing other services (like `api-platform`) to extend the core functionality.

### State Management

- `AppState`: Holds Redis, config, and rate limiter
- `ApiStateAccess` trait: Provides abstract state access for route handlers
- Graceful degradation: Redis wrapped in `Option`, 503 if unavailable

### Configuration Loading

1. Parse CLI arguments (`--config`)
2. Load TOML file from specified path (default: `./config.toml`)
3. Apply environment variable overrides
4. Initialize Tokio runtime with configured worker threads
5. Start server

## Development

### Building

```bash
# Check compilation
cargo check -p parapet-api-core

# Build release
cargo build --release -p parapet-api-core

# Run tests
cargo test -p parapet-api-core
```

### Adding Endpoints

See `AGENTS.md` for patterns and conventions.

## Deployment

```bash
# Build release binary
cargo build --release --bin parapet-api-core

# Copy binary and config
cp target/release/parapet-api-core /opt/parapet/
cp config.toml /opt/parapet/

# Run (from directory containing config.toml)
cd /opt/parapet && ./parapet-api-core
```

## Related

- `api-platform/`: Full-featured platform with user management and dashboard
- `mcp/`: Standalone STDIO MCP server for local AI assistants
- `proxy/`: Solana transaction proxy with rule engine
