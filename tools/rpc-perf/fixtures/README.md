# RPC-Perf Test Fixtures

Rule sets for benchmarking proxy performance. These are **minimal test fixtures**, not production rules.

## Available Fixtures

### empty-rules.json

Empty rule set — measures baseline proxy overhead with no rules.

**Use case:** Establish minimum latency floor (smoke tests).

### baseline-rules.json

Core structural rules (pass / alert / block) aligned with built-in harness transactions.

**Use case:** Default rules for `rpc-perf send`.

### simulation-rules.json

Same structural rules as baseline plus one simulation-stage rule driven by mock `simulateTransaction` logs.

**Use case:** Default rules for `rpc-perf simulate`.

### stress-rules.json

Intentionally complex rules for worst-case nesting and analyzer fan-out.

**Use case:** Stress latency under heavy condition trees.

### mix-rules.json

Mixed action types for testing action distribution.

**Use case:** Validate action handling logic.

## Running Benchmarks

```bash
# Send path — default baseline-rules.json
cargo run --release -p rpc-perf -- send \
  --rules-path fixtures/baseline-rules.json \
  --iterations 200

# Simulate path — default simulation-rules.json
cargo run --release -p rpc-perf -- simulate \
  --rules-path fixtures/simulation-rules.json \
  --iterations 200

# Baseline (no rules)
cargo run --release -p rpc-perf -- send \
  --rules-path fixtures/empty-rules.json \
  --iterations 200
```

## Performance Targets

| Fixture   | Target p50 | Target p99 | Notes              |
| --------- | ---------- | ---------- | ------------------ |
| empty     | <0.1ms     | <0.2ms     | Proxy baseline     |
| baseline  | <0.5ms     | <1.0ms     | Typical harness    |
| stress    | <2.0ms     | <5.0ms     | Worst-case rules   |

## Why Small Fixtures?

Large rule sets (200+ rules) are **proprietary** and would leak detection strategies. These fixtures test **rule engine performance**, not rule coverage.

The proxy's performance depends on:

1. Rule complexity (nesting depth, condition count)
2. Analyzer overhead (field extraction cost)
3. Not on rule count alone (evaluation short-circuits on decisive paths)

Therefore, a small set of well-crafted complex rules is more valuable for benchmarking than hundreds of trivial rules.
