# RPC-Perf Test Fixtures

Rule sets for benchmarking proxy performance. These are **minimal test fixtures**, not production rules.

## Available Fixtures

### minimal-rules.json
Empty rule set - measures baseline proxy overhead with no rules.

**Use case:** Establish minimum latency floor

### realistic-rules.json (90 lines)
5 basic rules testing different actions (pass, alert, block):
- Block large SOL transfers
- Block unlimited token approvals
- Block freeze + approve combo
- Alert on memo instructions
- Pass revoke-only transactions

**Use case:** Typical production rule evaluation

### stress-test-rules.json (133 lines)
5 intentionally complex rules targeting worst-case performance:
- Deep nested `all`/`any` conditions (3+ levels)
- Multiple analyzer field evaluations (7+ fields per rule)
- Array contains operations across multiple fields
- Threshold cascades with 8+ conditions
- Weighted risk aggregation with nested logic

**Use case:** Verify performance under complex rule engine load

### mix-rules.json (37 lines)
Mixed action types for testing action distribution.

**Use case:** Validate action handling logic

## Running Benchmarks

```bash
# Quick test with realistic rules
cargo run --release -p rpc-perf -- \
  --rules fixtures/realistic-rules.json \
  --iterations 200

# Stress test with complex rules
cargo run --release -p rpc-perf -- \
  --rules fixtures/stress-test-rules.json \
  --iterations 500

# Baseline (no rules)
cargo run --release -p rpc-perf -- \
  --rules fixtures/minimal-rules.json \
  --iterations 200
```

## Performance Targets

| Fixture | Target p50 | Target p99 | Notes |
|---------|------------|------------|-------|
| minimal | <0.1ms | <0.2ms | Proxy baseline |
| realistic | <0.5ms | <1.0ms | Typical production |
| stress-test | <2.0ms | <5.0ms | Worst-case complex rules |

## Why Small Fixtures?

Large rule sets (200+ rules) are **proprietary** and would leak detection strategies. These fixtures test **rule engine performance**, not rule coverage.

The proxy's performance depends on:
1. Rule complexity (nesting depth, condition count)
2. Analyzer overhead (field extraction cost)
3. Not on rule count (rules evaluated sequentially until match)

Therefore, 5-10 well-crafted complex rules are more valuable for benchmarking than 200 simple rules.
