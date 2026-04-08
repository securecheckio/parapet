# Tools

Performance benchmarking and security analysis utilities for Parapet.

## Directories

### `rpc-perf/`

RPC proxy + rule engine latency benchmarking tool.

Measures end-to-end proxy performance using real Solana transactions against a mock upstream. Validates the sub-millisecond analysis claim.

**Usage:**

```bash
cargo run -p rpc-perf --release -- --iterations 500
```

**Results:** Documented in `docs/benchmarks/` (see `rpc-perf-2026-04-15.md`)

---

### `flowbits-perf/`

Flowbits state manager performance benchmarks.

Criterion-based microbenchmarks for stateful detection (flowbits) operations. Tests counter updates, TTL expiration, and cross-transaction state tracking.

**Usage:**

```bash
cargo bench -p flowbits-perf
```

---

### `risk-register/`

Security threat categorization and coverage tracking.

CSV database of threat categories (Authority Control, Token Delegation, Phishing, etc.) that Parapet detects. Used to map rules to risk categories and identify coverage gaps.

**Files:**

- `risk-categories.csv` - Threat taxonomy
- Analysis tools for coverage reporting


## Purpose

These tools support:

- **Performance validation** - Prove <1ms analysis claims
- **Regression detection** - Compare against baselines
- **Security coverage** - Track which threats are detected
- **Development** - Profile optimization opportunities

