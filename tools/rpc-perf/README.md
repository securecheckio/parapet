# rpc-perf

Measures Parapet proxy + rule-engine latency using a mock upstream and real Solana transactions.

Each **test case** is a `(rule, transaction, expected-outcome)` triple. The transaction is purpose-built to satisfy the rule's conditions. The proxy decodes and evaluates it exactly as it would on mainnet — the only difference is the upstream is a local stub that returns a fake signature instead of submitting to Solana.

## What it measures

**Proxy overhead only** — decode → rule engine → forward to mock upstream → response. No network RTT to Solana, no blockhash validation, no on-chain state. This is the lower bound on latency that Parapet adds to every transaction.

## Running

From the workspace root (`parapet/`):

```bash
# All 7 cases, 100 iterations each (700 total), default rules
cargo run -p rpc-perf --

# More iterations for stable percentiles
cargo run -p rpc-perf -- --iterations 500 --warmup 50

# Specific cases only
cargo run -p rpc-perf -- --cases unlimited-approve-block,freeze-combo-block --iterations 200

# Concurrent load (tests RwLock / semaphore contention)
cargo run -p rpc-perf -- --iterations 200 --concurrency 4

# Custom rules file (absolute or relative to workspace root)
cargo run -p rpc-perf -- --rules-path proxy/rules/presets/comprehensive-protection.json

# Release build for lower-noise numbers
cargo run -p rpc-perf --release -- --iterations 500
```

## Test cases


| Case                       | Rule tested                         | Expected    |
| -------------------------- | ----------------------------------- | ----------- |
| `sol-transfer-pass`        | (none — no rules match)             | pass / 2xx  |
| `memo-alert`               | `rpc-perf-alert-memo`               | alert / 2xx |
| `large-sol-transfer-block` | `rpc-perf-block-large-sol-transfer` | block / 403 |
| `unlimited-approve-block`  | `rpc-perf-block-unlimited-approve`  | block / 403 |
| `freeze-combo-block`       | `rpc-perf-block-freeze-combo`       | block / 403 |
| `multi-ix-alert`           | `rpc-perf-alert-memo`               | alert / 2xx |
| `revoke-pass`              | `rpc-perf-pass-revoke-only`         | pass / 2xx  |


The harness reports **outcome mismatches** if the proxy returns a different HTTP status than expected — useful for catching rule regressions.

## Flags


| Flag                   | Default                         | Description                          |
| ---------------------- | ------------------------------- | ------------------------------------ |
| `--cases`              | all                             | Comma-separated list of cases to run |
| `--iterations`         | 100                             | Times each case is repeated          |
| `--warmup`             | 20                              | Unmeasured warmup iterations         |
| `--concurrency`        | 1                               | Parallel in-flight requests          |
| `--blocking-threshold` | 70                              | Rule-engine risk threshold (0–100)   |
| `--rules-path`         | `fixtures/realistic-rules.json` | Rules JSON file                      |
| `--seed`               | 42                              | RNG seed for shuffle                 |


## Automated test

```bash
cargo test -p rpc-perf
```

Runs unit tests (schedule logic, case registry) plus a smoke test that sends one pass transaction through a live proxy and asserts HTTP 2xx.

## Fixtures

- `fixtures/realistic-rules.json` — rules covering all 7 built-in test cases
- `fixtures/minimal-rules.json` — empty rules (used by smoke test)

