# Parapet proxy

HTTP JSON-RPC proxy: analyzes transactions and applies JSON rules.

## Run

```bash
cargo run --release
```

## Configuration

**Default approach: TOML config files**

1. Copy `config.toml.example` to `config.toml`
2. Customize settings for your deployment
3. Use environment variables only for secrets (API keys, passwords)

```bash
cp config.toml.example config.toml
nano config.toml  # Edit settings
```

**Environment variables** override TOML values when set (non-exhaustive):

- `HELIUS_API_KEY`, `JUPITER_API_KEY` — analyzer secrets (prefer env, not TOML)
- `UPSTREAM_RPC_URL` — single upstream URL, or use `UPSTREAM_RPC_URLS` (comma-separated) for failover
- `UPSTREAM_STRATEGY`, `UPSTREAM_SMART_MAX_SLOT_LAG` — optional smart routing (multi-URL)
- `ALLOWED_RPC_METHODS`, `BLOCKED_RPC_METHODS` — optional JSON-RPC method policy (comma-separated)
- `REDIS_URL`, `PROXY_PORT`, `RULES_PATH` — common overrides

See **[Upstream RPC](#upstream-rpc-multi-url-and-method-policy)** below and [`docs/OPERATIONS_GUIDE.md`](../docs/OPERATIONS_GUIDE.md#multi-upstream-rpc-proxy-and-api) for full guidance.

**Without `config.toml`:** Falls back to environment-only mode (not recommended for production).

**Analyzers:** Copy `analyzers.toml.example` → `analyzers.toml` to enable/disable specific analyzers. Default: all enabled.

**Rules:** Set `rules_path` in config.toml or `RULES_PATH` env var. Files under `rules/` are samples/fixtures.

**Deploy:** `../deployments/proxy-only/` or `../deployments/full-stack/`

## RPC surface

- **Supported:** `sendTransaction` / `sendRawTransaction`, `simulateTransaction`, read-only HTTP JSON-RPC.
- **Not supported:** WebSocket subscriptions (use another RPC for subs).

## Repo layout


| Path                              | Purpose                                           |
| --------------------------------- | ------------------------------------------------- |
| `config.toml.example`             | Template (committed)                              |
| `config.toml`                     | Your settings (gitignored)                        |
| `rules/`                          | Sample + test rule JSON (see `rules/README.md`)   |
| `config/known-safe-programs.json` | Default safe-program list                         |
| `analyzers.toml.example`          | Template → copy to `analyzers.toml` to customize  |
| `analyzers.toml`                  | Your analyzer toggles (gitignored); omit = all on |


## `config.toml` reference

Defaults match `proxy/src/config.rs` unless noted. See `config.toml.example` for a filled-in template.

**With a config file loaded:** Values come from TOML. Environment variables **still override** the file when set for the same keys (for example `UPSTREAM_RPC_URL`, `UPSTREAM_RPC_URLS`, `PROXY_PORT`, `REDIS_URL`, `RULES_PATH`, and the other upstream / security vars listed below).

**Without `config.toml`:** Everything is taken from environment variables (`config::Config::from_env`). Set **`UPSTREAM_RPC_URL`** *or* **`UPSTREAM_RPC_URLS`**. `[[rule_feeds.sources]]` entries are only available via TOML, not env-only mode.

### `[server]`


| Key            | Type    | Default     | Description       |
| -------------- | ------- | ----------- | ----------------- |
| `port`         | integer | `8899`      | HTTP listen port  |
| `bind_address` | string  | `"0.0.0.0"` | IPv4 bind address |


Env-only: `PROXY_PORT`, `BIND_ADDRESS`.

### Upstream RPC (multi-URL and method policy)

The proxy uses the shared **`parapet-upstream`** stack: one HTTP client per URL, retries on transient errors, per-URL circuit breakers, and optional **failover** (default) or **`smart`** routing across multiple URLs.

**Rules:**

- Set **either** `[upstream].url` **or** one or more **`[[upstream.endpoint]]`** entries — not both (config validation fails if both are used).
- With **`[[upstream.endpoint]]`**, omit `url`. Sort order for failover is by **`priority`** (lower = preferred). Each row may override HTTP fields; omitted fields inherit from `[upstream]` defaults.

**TOML keys on `[upstream]` (shared defaults + strategy):**

| Key | Type | Default | Description |
| --- | ---- | ------- | ----------- |
| `url` | string | empty | Single Solana JSON-RPC URL (single-URL mode) |
| `strategy` | string | unset | Multi-URL only: omit or `"failover"` for priority order; `"smart"` for latency/slot-aware selection |
| `smart_max_slot_lag` | integer | `20` | With `smart`, max tolerated slot lag between endpoints |
| `max_concurrent` | integer | `10` | Default max concurrent upstream requests per client |
| `delay_ms` | integer | `100` | Default delay between upstream calls (ms) |
| `timeout_secs` | integer | `30` | Default per-request timeout |
| `max_retries` | integer | `3` | Default retries on transient failures |
| `retry_base_delay_ms` | integer | `100` | Default exponential backoff base (ms) |
| `circuit_breaker_threshold` | integer | `5` | Default failures before circuit opens |
| `circuit_breaker_timeout_secs` | integer | `60` | Default cool-down before half-open |

**`[[upstream.endpoint]]` (repeatable, multi-URL mode):**

| Key | Type | Default | Description |
| --- | ---- | ------- | ----------- |
| `url` | string | *(required)* | Endpoint JSON-RPC URL |
| `priority` | integer | `0` | Lower = tried first on failover |
| `max_concurrent` | integer | *(inherits)* | Override for this endpoint only |
| `delay_ms` | integer | *(inherits)* | Override |
| `timeout_secs` | integer | *(inherits)* | Override |
| `max_retries` | integer | *(inherits)* | Override |
| `retry_base_delay_ms` | integer | *(inherits)* | Override |
| `circuit_breaker_threshold` | integer | *(inherits)* | Override |
| `circuit_breaker_timeout_secs` | integer | *(inherits)* | Override |

**Environment (overrides TOML when set):**

`UPSTREAM_RPC_URL`, `UPSTREAM_RPC_URLS` (comma-separated), `UPSTREAM_STRATEGY`, `UPSTREAM_SMART_MAX_SLOT_LAG`, `UPSTREAM_MAX_CONCURRENT`, `UPSTREAM_DELAY_MS`, `UPSTREAM_TIMEOUT_SECS`, `UPSTREAM_MAX_RETRIES`, `UPSTREAM_RETRY_BASE_DELAY_MS`, `UPSTREAM_CIRCUIT_BREAKER_THRESHOLD`, `UPSTREAM_CIRCUIT_BREAKER_TIMEOUT_SECS`.

**Env-only mode:** same upstream variables; **`UPSTREAM_RPC_URL` or `UPSTREAM_RPC_URLS` is required.**

Copy **`config.toml.example`** for commented single- vs multi-upstream examples.

### `[network]`


| Key       | Type   | Default          | Description             |
| --------- | ------ | ---------------- | ----------------------- |
| `network` | string | `"mainnet-beta"` | Cluster name (metadata) |


Env-only: `SOLANA_NETWORK`.

### `[security]`


| Key                          | Type             | Default | Description                                                              |
| ---------------------------- | ---------------- | ------- | ------------------------------------------------------------------------ |
| `default_blocking_threshold` | integer (0–255)  | `70`    | Cumulative risk at or above → block                                      |
| `rules_path`                 | string           | —       | Path to rules JSON bundle                                                |
| `rule_action_override`       | string           | —       | If set, force every rule’s action (e.g. `"alert"`)                       |
| `blocked_programs`           | array of strings | —       | Program IDs to always block (TOML only; env-only mode does not set this) |
| `allowed_methods`            | array of strings | `[]`    | If non-empty, only these JSON-RPC methods are accepted                   |
| `blocked_methods`            | array of strings | `[]`    | Methods always rejected (evaluated before `allowed_methods`)             |


Env-only: `DEFAULT_BLOCKING_THRESHOLD`, `RULE_ACTION_OVERRIDE`, `RULES_PATH` (and `RULES_PATH` overrides TOML when a file is loaded), `ALLOWED_RPC_METHODS`, `BLOCKED_RPC_METHODS` (comma-separated method names).

### `[auth]`


| Key               | Type             | Default  | Description                                              |
| ----------------- | ---------------- | -------- | -------------------------------------------------------- |
| `mode`            | string           | `"none"` | One of `none`, `api_key`, `wallet_allowlist`             |
| `api_keys`        | string           | —        | Key material for API-key auth (format depends on server) |
| `allowed_wallets` | array of strings | —        | Allowed signers when `mode = "wallet_allowlist"`         |


Env-only: `AUTH_MODE`, `API_KEYS`, `ALLOWED_WALLETS` (comma-separated list).

### `[usage]`


| Key                          | Type    | Default | Description                               |
| ---------------------------- | ------- | ------- | ----------------------------------------- |
| `enabled`                    | boolean | `false` | Track per-wallet usage / quotas           |
| `default_requests_per_month` | integer | `10000` | Default monthly request budget per wallet |


Env-only: `ENABLE_USAGE_TRACKING`, `DEFAULT_REQUESTS_PER_MONTH`.

### `[redis]`


| Key   | Type   | Default | Description                                  |
| ----- | ------ | ------- | -------------------------------------------- |
| `url` | string | —       | `redis://…` for shared cache and usage state |


Env-only: `REDIS_URL` (also overrides TOML when a file is loaded).

### `[wasm]`


| Key               | Type   | Default         | Description                                       |
| ----------------- | ------ | --------------- | ------------------------------------------------- |
| `analyzers_path`  | string | `"./analyzers"` | Directory for WASM analyzers (if feature enabled) |
| `analyzer_config` | string | —               | Optional JSON config path                         |


Env-only: `WASM_ANALYZERS_PATH`, `WASM_ANALYZER_CONFIG`.

### `[escalations]`


| Key               | Type    | Default | Description            |
| ----------------- | ------- | ------- | ---------------------- |
| `enabled`         | boolean | `false` | Enable escalation flow |
| `approver_wallet` | string  | —       | Approver wallet pubkey |


Env-only: `ENABLE_ESCALATIONS`, `ESCALATION_APPROVER_WALLET`.

### `[rule_feeds]`


| Key                    | Type    | Default | Description                                           |
| ---------------------- | ------- | ------- | ----------------------------------------------------- |
| `enabled`              | boolean | `false` | Poll remote rule feeds                                |
| `poll_interval`        | integer | `3600`  | Seconds between poll cycles                           |
| `default_min_interval` | integer | `300`   | Default minimum seconds between hits to the same feed |


Env-only: `RULES_FEED_ENABLED`, `RULES_FEED_POLL_INTERVAL`, `RULES_FEED_MIN_INTERVAL`. Feed URLs use `[[rule_feeds.sources]]` in TOML only.

### `[[rule_feeds.sources]]` (repeatable)


| Key            | Type    | Default      | Description                                                                                  |
| -------------- | ------- | ------------ | -------------------------------------------------------------------------------------------- |
| `url`          | string  | *(required)* | JSON rule feed URL                                                                           |
| `name`         | string  | —            | Label for logs                                                                               |
| `priority`     | integer | `0`          | Ordering / preference                                                                        |
| `min_interval` | integer | —            | Optional per-source min spacing (seconds); falls back to `[rule_feeds].default_min_interval` |


