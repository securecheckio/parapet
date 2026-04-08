# Flowbits Performance Benchmark Results

**Date:** April 8, 2026  
**System:** Release mode, Criterion benchmarks

## Summary

All operations complete in **~50-200ns**, well below 1μs latency target.

## Per-Wallet Operations

| Operation | Median Time | Throughput |
|-----------|-------------|------------|
| `set()` | 113.67 ns | 8.8M ops/sec |
| `increment()` | 195.53 ns | 5.1M ops/sec |
| `is_set()` | 82.19 ns | 12.2M ops/sec |
| `get_counter()` | 100.74 ns | 9.9M ops/sec |

## Global Operations

| Operation | Median Time | Throughput |
|-----------|-------------|------------|
| `set_global()` | 83.55 ns | 12.0M ops/sec |
| `increment_global()` | 139.67 ns | 7.2M ops/sec |
| `is_set_global()` | 57.75 ns | 17.3M ops/sec |
| `get_counter_global()` | 62.93 ns | 15.9M ops/sec |

## Scaling Tests

### By Wallet Count
| Wallets | Median Time |
|---------|-------------|
| 10 | 93.03 ns |
| 100 | 91.66 ns |
| 1000 | 90.45 ns |

**Analysis:** No degradation with wallet count - HashMap O(1) lookup

### By Global Key Count
| Global Keys | Median Time |
|-------------|-------------|
| 100 | 147.71 ns |
| 1000 | 118.58 ns |
| 10000 | 120.20 ns |

**Analysis:** Minimal degradation, still sub-150ns

## Variable Interpolation

| Scenario | Median Time |
|----------|-------------|
| No variables | 13.63 ns |
| Single variable | 171.91 ns |
| Multiple variables | 441.60 ns |

## Realistic Scenarios

| Scenario | Median Time | Description |
|----------|-------------|-------------|
| AI agent velocity check | 337.07 ns | Track transaction rate per wallet |
| Enterprise lateral movement | 490.04 ns | Monitor cross-account activity |
| AI agent exfiltration | 560.93 ns | Multi-condition security check |

## Conclusion

**✅ Production Ready**

- Per-wallet ops: ~82-195ns (median)
- Global ops: ~58-140ns (median)  
- Realistic scenarios: ~300-560ns (all < 1μs)
- Zero scaling degradation up to 1000 wallets

Flowbits add **< 1μs latency per transaction** with negligible memory overhead.
