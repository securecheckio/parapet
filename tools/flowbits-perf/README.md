# flowbits-perf

Criterion benchmarks for `FlowbitStateManager` in `parapet-core` (flowbits state only — not the RPC proxy).

**Run**

```bash
cargo bench -p flowbits-perf
```

**Implementation**

- Bench entry: [`benches/flowbits_perf.rs`](benches/flowbits_perf.rs)
- Flowbits logic under test: `parapet_core::rules::flowbits` in [`core/src/rules/flowbits/`](../../core/src/rules/flowbits/)
