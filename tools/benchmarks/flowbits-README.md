# Flowbits Performance Benchmarks

Validates that flowbits add <1μs latency per transaction with no scaling degradation.

## Running the Benchmark

```bash
cargo bench --bench flowbits_performance
```

Results saved to `flowbits-YYYYMMDD.txt` with summary in `flowbits-YYYYMMDD-summary.md`.

## What's Tested

### 1. Core Operations
- **Per-wallet**: `set()`, `increment()`, `is_set()`, `get_counter()`
- **Global**: Same operations on shared state
- **Target**: All ops < 200ns

### 2. Scaling
- Wallet count: 10 → 100 → 1000 wallets
- Global keys: 100 → 1k → 10k keys
- **Target**: No O(n) degradation (HashMap should be O(1))

### 3. Variable Interpolation
- Static names vs. dynamic (`transfers_to:{recipient}`)
- **Target**: < 500ns for single variable

### 4. Realistic Attack Detection
- **AI agent velocity**: Rate limit per wallet
- **Lateral movement**: Track recipient across wallets
- **Gradual exfiltration**: Per-recipient tracking
- **Target**: All scenarios < 1μs

## Why This Matters

Flowbits run on **every transaction** in the RPC proxy. If they're slow or degrade with scale, they become a bottleneck.

**Critical validations:**
1. ✅ Sub-microsecond latency (0.05-0.5% of 1ms budget)
2. ✅ O(1) HashMap behavior confirmed (no slowdown at 1000 wallets)
3. ✅ Real-world patterns all < 1μs

## Benchmark Source

Implementation: `../../core/benches/flowbits_performance.rs`

Uses Criterion framework:
- 100+ samples per test
- Statistical outlier detection
- Compiler optimization prevention (`black_box`)
- Warmup period for stable measurements

## Interpreting Results

```
time:   [81.015 ns 82.191 ns 83.751 ns]
         ^min       ^median    ^max
```

Focus on **median time** - the middle value represents typical performance.

**Acceptable thresholds:**
- Per-wallet ops: < 200ns ✅
- Global ops: < 300ns ✅
- Realistic scenarios: < 1000ns (1μs) ✅
