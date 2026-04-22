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

**Environment variables** override TOML values when set:
- `HELIUS_API_KEY` - API key for Helius analyzer (secret, use env var)
- `JUPITER_API_KEY` - API key for Jupiter analyzer (secret, use env var)
- `UPSTREAM_RPC_URL` - Override upstream RPC endpoint
- `REDIS_URL` - Override Redis connection
- `PROXY_PORT` - Override server port
- `RULES_PATH` - Override rules file path

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

**With a config file loaded:** Values come from TOML. These environment variables **still override** the file when set: `PROXY_PORT`, `UPSTREAM_RPC_URL`, `REDIS_URL`, `RULES_PATH`.

**Without `config.toml`:** Everything is taken from environment variables (`config::Config::from_env`). `UPSTREAM_RPC_URL` is required. `[[rule_feeds.sources]]` entries are only available via TOML, not env-only mode.

### `[server]`


| Key            | Type    | Default     | Description       |
| -------------- | ------- | ----------- | ----------------- |
| `port`         | integer | `8899`      | HTTP listen port  |
| `bind_address` | string  | `"0.0.0.0"` | IPv4 bind address |


Env-only: `PROXY_PORT`, `BIND_ADDRESS`.

### `[upstream]`


| Key                            | Type    | Default              | Description                       |
| ------------------------------ | ------- | -------------------- | --------------------------------- |
| `url`                          | string  | *(required in TOML)* | Upstream Solana JSON-RPC URL      |
| `max_concurrent`               | integer | `10`                 | Max concurrent upstream requests  |
| `delay_ms`                     | integer | `100`                | Delay between upstream calls (ms) |
| `timeout_secs`                 | integer | `30`                 | Per-request timeout               |
| `max_retries`                  | integer | `3`                  | Retries on transient failures     |
| `retry_base_delay_ms`          | integer | `100`                | Exponential backoff base (ms)     |
| `circuit_breaker_threshold`    | integer | `5`                  | Failures before circuit opens     |
| `circuit_breaker_timeout_secs` | integer | `60`                 | Cool-down before half-open        |


Env-only: `UPSTREAM_RPC_URL`, `UPSTREAM_MAX_CONCURRENT`, `UPSTREAM_DELAY_MS`, `UPSTREAM_TIMEOUT_SECS`, `UPSTREAM_MAX_RETRIES`, `UPSTREAM_RETRY_BASE_DELAY_MS`, `UPSTREAM_CIRCUIT_BREAKER_THRESHOLD`, `UPSTREAM_CIRCUIT_BREAKER_TIMEOUT_SECS`.

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


Env-only: `DEFAULT_BLOCKING_THRESHOLD`, `RULE_ACTION_OVERRIDE`, `RULES_PATH` (and `RULES_PATH` overrides TOML when a file is loaded).

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


