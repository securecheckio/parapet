# flowstate-perf

Criterion benchmarks for `FlowbitStateManager` in `parapet-core` (flowstate state only — not the RPC proxy).

**Run**

```bash
cargo bench -p flowstate-perf
```

**Implementation**

- Bench entry: [`benches/flowstate_perf.rs`](benches/flowstate_perf.rs)
- FlowState logic under test: `parapet_core::rules::flowstate` in [`core/src/rules/flowstate/`](../../core/src/rules/flowstate/)
