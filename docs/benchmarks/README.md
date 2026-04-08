# Performance Benchmarks

This directory contains historical performance benchmark results for Parapet to track performance over time and prevent regressions.

## Directory Structure

```
benchmarks/
├── README.md                    # This file
├── rpc-perf-YYYY-MM-DD.md      # RPC proxy benchmark results
└── baseline.md                  # Current baseline for comparison
```

## Benchmark Types

### RPC Performance (rpc-perf)

Measures end-to-end latency for transaction processing through the Parapet proxy with rule engine evaluation.

**What it measures**:
- Transaction decode time
- Rule engine evaluation time
- Response encoding time
- HTTP overhead

**What it doesn't measure**:
- Network latency to Solana RPC (uses mock upstream)
- On-chain state queries
- Real blockhash validation

**When to run**:
- Before major releases
- After performance-related changes
- When adding new rules or analyzers
- Monthly for trend tracking

## Running Benchmarks

### Quick Smoke Test (Development)

```bash
cd parapet/
cargo run -p rpc-perf -- --iterations 100
```

**Time**: ~1 minute  
**Use**: Quick sanity check during development

### Standard Benchmark (Pre-Release)

```bash
cd parapet/
cargo run -p rpc-perf --release -- --iterations 500 --warmup 50
```

**Time**: ~6-7 minutes  
**Use**: Official benchmarks for comparison

### Extended Benchmark (Investigation)

```bash
cd parapet/
cargo run -p rpc-perf --release -- --iterations 1000 --warmup 100
```

**Time**: ~12-15 minutes  
**Use**: Detailed investigation of performance issues

### Concurrency Test

```bash
cd parapet/
cargo run -p rpc-perf --release -- --iterations 500 --concurrency 4
```

**Time**: ~3-4 minutes  
**Use**: Test lock contention and parallel performance

## Recording Results

### 1. Run the Benchmark

```bash
cd parapet/
cargo run -p rpc-perf --release -- --iterations 500 --warmup 50 | tee benchmark-output.txt
```

### 2. Create Dated Report

Copy the latest benchmark template and fill in results:

```bash
cp docs/benchmarks/rpc-perf-2026-04-07.md docs/benchmarks/rpc-perf-$(date +%Y-%m-%d).md
```

Edit the new file with your results.

### 3. Update Baseline (if appropriate)

If this is a new baseline (e.g., after major release), update `baseline.md`:

```bash
cp docs/benchmarks/rpc-perf-$(date +%Y-%m-%d).md docs/benchmarks/baseline.md
```

### 4. Commit Results

```bash
git add docs/benchmarks/
git commit -m "perf: add benchmark results for $(date +%Y-%m-%d)"
```

## Interpreting Results

### Healthy Performance

✅ **Good signs**:
- p50 latency: <0.5ms
- p95 latency: <0.8ms
- p99 latency: <1.0ms
- No outcome mismatches
- Consistent results across runs

### Warning Signs

⚠️ **Investigate if**:
- p50 increased by >10% vs baseline
- p95 increased by >15% vs baseline
- p99 increased by >20% vs baseline
- High variance between runs
- Any test case >1ms at p50

### Critical Issues

🚨 **Immediate action required if**:
- p50 increased by >25% vs baseline
- Any test case >2ms at p50
- p99 >5ms for any case
- Outcome mismatches detected
- Benchmark fails to complete

## Regression Analysis

### Comparing to Baseline

1. **Load current baseline**:
   ```bash
   cat docs/benchmarks/baseline.md
   ```

2. **Run new benchmark**:
   ```bash
   cargo run -p rpc-perf --release -- --iterations 500 --warmup 50
   ```

3. **Calculate percentage change**:
   ```
   Change % = ((New - Baseline) / Baseline) × 100
   ```

4. **Check against thresholds**:
   - <10% change: ✅ Acceptable
   - 10-25% change: ⚠️ Requires justification
   - >25% change: 🚨 Requires investigation and fix

### Example Comparison

**Baseline** (2026-04-07):
- sol-transfer-pass p50: 0.341ms

**New benchmark** (2026-04-15):
- sol-transfer-pass p50: 0.375ms

**Analysis**:
```
Change = ((0.375 - 0.341) / 0.341) × 100 = +10.0%
Status: ⚠️ Warning - investigate cause
```

## Common Performance Issues

### Issue: Increased p50 latency

**Possible causes**:
- New rules added (expected)
- Inefficient rule conditions
- Added synchronous I/O
- Increased allocation/copying

**Investigation**:
```bash
# Profile the code
cargo flamegraph -p rpc-perf -- --iterations 100

# Check for new allocations
cargo build --release
heaptrack target/release/rpc-perf -- --iterations 100
```

### Issue: Increased p99 latency (but p50 stable)

**Possible causes**:
- Lock contention
- Garbage collection pauses
- Cache misses
- Background tasks interfering

**Investigation**:
```bash
# Test with concurrency
cargo run -p rpc-perf --release -- --iterations 500 --concurrency 8

# Check for lock contention
cargo build --features parking_lot/deadlock_detection
```

### Issue: High variance between runs

**Possible causes**:
- System load
- Thermal throttling
- Background processes
- Non-deterministic behavior

**Investigation**:
- Run on dedicated benchmark machine
- Close other applications
- Run multiple times and take median
- Check for non-deterministic code paths

## Benchmark History

### Baseline: April 7, 2026

- **Version**: 0.1.0
- **p50 (pass)**: 0.341ms
- **p50 (alert)**: 0.346ms
- **p50 (block)**: 0.246ms
- **p95 (all)**: 0.345-0.489ms
- **p99 (all)**: 0.403-0.583ms
- **Notes**: Initial baseline, mock upstream, 7 test cases

### Future Benchmarks

Add new entries here as benchmarks are run:

```markdown
### YYYY-MM-DD: Description

- **Version**: X.Y.Z
- **p50 (pass)**: X.XXXms
- **Change**: +X.X% vs baseline
- **Notes**: What changed, why
```

## Automation

### CI Integration (Future)

```yaml
# .github/workflows/benchmark.yml
name: Performance Benchmark

on:
  pull_request:
    paths:
      - 'core/**'
      - 'proxy/**'
      - 'rpc-perf/**'

jobs:
  benchmark:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Run benchmark
        run: |
          cargo run -p rpc-perf --release -- --iterations 500
      - name: Compare to baseline
        run: |
          # Parse results and compare to baseline
          # Fail if regression >10%
```

### Performance Dashboard (Future)

Track trends over time with a dashboard:
- Plot p50/p95/p99 over time
- Highlight regressions
- Correlate with commits/PRs
- Alert on threshold violations

## Best Practices

### Before Committing Code

1. ✅ Run quick smoke test (`--iterations 100`)
2. ✅ Check results are reasonable
3. ✅ If performance-sensitive change, run full benchmark
4. ✅ Document expected performance impact in PR

### Before Releasing

1. ✅ Run full benchmark (`--iterations 500 --warmup 50`)
2. ✅ Compare to baseline
3. ✅ Document any regressions with justification
4. ✅ Update baseline if appropriate
5. ✅ Commit benchmark results

### Monthly Maintenance

1. ✅ Run full benchmark on main branch
2. ✅ Compare to previous month
3. ✅ Investigate any gradual degradation
4. ✅ Update performance targets if needed

## Questions?

- **How often should I benchmark?** Before major changes and releases
- **What if I see a regression?** Investigate cause, document justification, or fix
- **Should I update the baseline?** Only after major releases or intentional changes
- **What about real-world performance?** These are lower-bound tests; production adds network latency

## See Also

- [flowbits-performance-results.md](../flowbits-performance-results.md) - Flowbits system benchmarks
- [TEST_COVERAGE.md](../TEST_COVERAGE.md) - Test coverage documentation
- [rpc-perf README](../../rpc-perf/README.md) - Benchmark tool documentation
