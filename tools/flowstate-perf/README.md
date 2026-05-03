# flowstate-perf

Criterion benchmarks for `FlowStateManager` in `parapet-core` (flowstate only — not the RPC proxy).

**Run**

```bash
cargo bench -p flowstate-perf
```

**Implementation**

- Bench entry: [`benches/flowstate_perf.rs`](benches/flowstate_perf.rs)
- FlowState logic under test: `parapet_core::rules::flowstate` in [`core/src/rules/flowstate/`](../../core/src/rules/flowstate/)
