# parapet-upstream

Shared Solana JSON-RPC upstream client with failover, circuit breakers, and pluggable routing strategies.

## Overview

`parapet-upstream` is a standalone library that provides robust HTTP client infrastructure for forwarding JSON-RPC requests to Solana RPC endpoints. It's used by the Parapet RPC proxy to handle communication with upstream Solana nodes.

## Features

### 🔄 Failover Support
- **Priority-ordered failover** - Automatically tries backup endpoints when primary fails
- **Multiple upstream URLs** - Configure fallback RPC endpoints for high availability
- **Graceful degradation** - Continues working even if some endpoints are down

### 🛡️ Circuit Breakers
- **Per-endpoint protection** - Prevents cascading failures
- **Automatic recovery** - Circuit breakers transition through Closed → Open → Half-Open states
- **Configurable thresholds** - Set failure count and timeout duration
- **Smart retry logic** - Tests recovered endpoints before full restoration

### 📊 Smart Routing (Optional)
- **Latency-based selection** - Prefers faster endpoints automatically
- **Slot-lag filtering** - Ensures RPC nodes are in sync (< N slots behind)
- **Real-time metrics** - Tracks success rate and average latency per endpoint
- **Adaptive behavior** - Routing improves over time as metrics accumulate

### ⚙️ Configuration
- **Pluggable strategies** - Swap routing logic via `UpstreamProvider` trait
- **HTTP client tuning** - Configurable timeouts, retries, connection pooling
- **Environment-driven** - Parse URLs from env vars or config files

---

## Architecture

### Core Components

```
┌─────────────────────────────────────────┐
│         UpstreamProvider Trait          │
│   (forward, get_account, get_url)      │
└─────────────────────────────────────────┘
                    ▲
                    │
        ┌───────────┴────────────┐
        │                        │
┌───────────────┐      ┌─────────────────────┐
│ UpstreamClient│      │ FailoverUpstreamProvider│
│   (single)    │      │   (multiple + failover) │
└───────────────┘      └─────────────────────┘
        │                        │
        │                        │
   ┌────┴────┐          ┌────────┴────────┐
   │ Circuit │          │ SmartUpstreamProvider│
   │ Breaker │          │  (latency + slot-lag)│
   └─────────┘          └──────────────────┘
```

### Circuit Breaker States

```
     ┌─────────┐
     │ CLOSED  │──┐
     └─────────┘  │ failures >= threshold
           ▲      │
  success  │      ▼
           │  ┌────────┐
     ┌─────────┤  OPEN  │
     │     │   └────────┘
     │     │        │
     │     │        │ timeout expires
     │     │        ▼
     │  ┌──────────────┐
     └──│  HALF_OPEN   │
        └──────────────┘
         (test request)
```

---

## Usage

### Single Endpoint (Basic)

```rust
use parapet_upstream::{UpstreamClient, UpstreamHttpSettings, UpstreamProvider};

let client = UpstreamClient::new("https://api.mainnet-beta.solana.com".to_string());

let request = JsonRpcRequest {
    jsonrpc: "2.0".to_string(),
    id: serde_json::json!(1),
    method: "getHealth".to_string(),
    params: vec![],
};

let response = client.forward(&request).await?;
```

### Multiple Endpoints with Failover

```rust
use parapet_upstream::{build_upstream_stack, UpstreamHttpSettings};

let urls = vec![
    "https://api.mainnet-beta.solana.com".to_string(),
    "https://solana-api.projectserum.com".to_string(),
];

let settings = UpstreamHttpSettings {
    timeout_secs: 30,
    max_retries: 3,
    ..Default::default()
};

let provider = build_upstream_stack(urls, settings)?;
let response = provider.forward(&request).await?;
```

### Smart Routing (Latency + Slot-Aware)

```rust
use parapet_upstream::{build_upstream_stack_with_strategy, UpstreamHttpSettings};

let urls = vec![
    "https://api.mainnet-beta.solana.com".to_string(),
    "https://solana-api.projectserum.com".to_string(),
    "https://rpc.ankr.com/solana".to_string(),
];

let settings = UpstreamHttpSettings::default();
let max_slot_lag = 50; // Allow up to 50 slots behind leader

let provider = build_upstream_stack_with_strategy(
    urls,
    settings,
    Some("smart"),  // Use smart routing
    max_slot_lag,
)?;

let response = provider.forward(&request).await?;
```

---

## Configuration

### UpstreamHttpSettings

```rust
pub struct UpstreamHttpSettings {
    pub timeout_secs: u64,           // HTTP request timeout (default: 30)
    pub max_retries: usize,          // Retry attempts (default: 3)
    pub circuit_failure_threshold: usize,  // Failures before opening circuit (default: 5)
    pub circuit_timeout_secs: u64,   // Time before half-open (default: 60)
}
```

### Environment Variables

Parse upstream URLs from environment:

```bash
UPSTREAM_RPC_URL="https://api.mainnet-beta.solana.com"
# or multiple
UPSTREAM_RPC_URLS="https://primary.com,https://fallback1.com,https://fallback2.com"
```

```rust
use parapet_upstream::parse_upstream_urls_list;

let urls = parse_upstream_urls_list(&env_var)?;
```

---

## Circuit Breaker Behavior

### States

1. **CLOSED** (Normal Operation)
   - All requests pass through
   - Failures are counted
   - Transitions to OPEN after `failure_threshold` consecutive failures

2. **OPEN** (Endpoint Down)
   - All requests are rejected immediately (fail-fast)
   - No traffic sent to failing endpoint
   - After `timeout_duration`, transitions to HALF_OPEN

3. **HALF_OPEN** (Testing Recovery)
   - Single test request allowed
   - Success → CLOSED (endpoint recovered)
   - Failure → OPEN (still down, reset timer)

### Example

```rust
let breaker = CircuitBreaker::new(
    5,   // Open after 5 failures
    60,  // Wait 60 seconds before testing
);

// Normal operation
if breaker.call_permitted().await {
    match make_request().await {
        Ok(_) => breaker.record_success().await,
        Err(_) => breaker.record_failure().await,
    }
}
```

---

## Smart Routing Strategy

The `SmartUpstreamProvider` dynamically routes requests based on:

1. **Circuit breaker state** - Skip unhealthy endpoints
2. **Slot lag** - Filter out nodes that are too far behind
3. **Average latency** - Prefer faster endpoints
4. **Priority order** - Tie-breaker when latencies are equal

### Metrics Tracked

- **Latency sum** - Total response time per endpoint
- **Success count** - Number of successful requests
- **Last slot** - Latest observed slot height
- **Average latency** - `latency_sum / success_count`

### Slot-Lag Filtering

Prevents routing to stale RPC nodes:

```rust
let max_slot_lag = 50;
let provider = SmartUpstreamProvider::new(configs, max_slot_lag);
```

- Calls `getSlot` on all endpoints periodically
- Only routes to nodes within `max_slot_lag` of the leader
- Leader = endpoint with highest observed slot

---

## Advanced: Custom Strategies

Implement the `UpstreamProvider` trait for custom routing logic:

```rust
use async_trait::async_trait;
use parapet_upstream::{UpstreamProvider, JsonRpcRequest, JsonRpcResponse};

pub struct MyCustomStrategy {
    clients: Vec<UpstreamClient>,
    // your state here
}

#[async_trait]
impl UpstreamProvider for MyCustomStrategy {
    async fn forward(&self, request: &JsonRpcRequest) -> anyhow::Result<JsonRpcResponse> {
        // Your custom routing logic
        todo!()
    }

    async fn get_account(&self, pubkey: &str) -> anyhow::Result<Option<Vec<u8>>> {
        // Your custom account fetching
        todo!()
    }

    async fn get_multiple_accounts(&self, pubkeys: &[String]) 
        -> anyhow::Result<Vec<Option<Vec<u8>>>> {
        // Your custom batch account fetching
        todo!()
    }

    fn get_upstream_url(&self) -> String {
        // Return primary URL for logging
        self.clients[0].upstream_url.clone()
    }
}
```

---

## Testing

Run tests:

```bash
cd upstream
cargo test
```

Run with logging:

```bash
RUST_LOG=debug cargo test -- --nocapture
```

---

## Dependencies

- **tokio** - Async runtime
- **reqwest** - HTTP client
- **serde/serde_json** - JSON serialization
- **anyhow** - Error handling
- **async-trait** - Trait async support

---

## License

MIT - See LICENSE file in repository root
