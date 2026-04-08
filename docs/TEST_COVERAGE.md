# Test Coverage Guide

## Overview

Parapet uses `cargo-llvm-cov` for test coverage reporting. This provides accurate, line-level coverage data for all Rust code.

**Working directory:** run every command in this guide from the **workspace root** — the directory that contains the workspace `Cargo.toml`, `coverage.sh`, and `Makefile.coverage` (typically the `parapet` directory).

**Prerequisites:** the workspace must compile and tests must run with `cargo test --workspace --all-features` (that is what coverage invokes). If that fails, coverage will fail for the same reason.

## Quick Start

### Install Tools

```bash
cargo install cargo-llvm-cov
rustup component add llvm-tools-preview
rustup target add wasm32-unknown-unknown
```

Or:

```bash
make -f Makefile.coverage install-tools
```

The Makefile target installs `cargo-llvm-cov` and the WASM target only; add `**llvm-tools-preview**` yourself if needed (`rustup component add llvm-tools-preview`).

### Generate Coverage Report

`coverage.sh` builds the WASM mock when `core/tests/wasm_mock` exists, runs `cargo llvm-cov` with `--workspace --all-features`, then **checks a 70% total line threshold** (using `grep -oP` and `bc` where available). If coverage is below threshold, the script exits with a non-zero status even when the report was generated.

```bash
# Generate HTML report
./coverage.sh --html

# Generate and open in browser
./coverage.sh --html --open

# Generate LCOV report (for CI/Codecov)
./coverage.sh --lcov

# Show summary only
./coverage.sh --summary
```

Or use the Makefile:

```bash
# HTML report
make -f Makefile.coverage coverage

# Open in browser
make -f Makefile.coverage coverage-open

# Summary only
make -f Makefile.coverage coverage-summary
```

## cargo-llvm-cov (what Parapet uses)

Parapet standardizes on **[cargo-llvm-cov](https://github.com/taiki-e/cargo-llvm-cov)** for coverage. CI, `coverage.sh`, and `Makefile.coverage` all invoke it.

Typical direct invocations (same engine as the script; you usually prefer `./coverage.sh` or the Makefile for consistency):

```bash
cargo llvm-cov --workspace --all-features --html --open
cargo llvm-cov --workspace --all-features --lcov --output-path lcov.info
cargo llvm-cov --workspace --all-features --json --output-path coverage.json
cargo llvm-cov --workspace --all-features --summary-only
```

## Per-Component Coverage

### Core Library

```bash
./coverage.sh --package parapet-core --html
```

Or:

```bash
make -f Makefile.coverage coverage-core
```

### RPC Proxy

```bash
./coverage.sh --package parapet-proxy --html
```

Or:

```bash
make -f Makefile.coverage coverage-proxy
```

### Wallet Scanner

```bash
./coverage.sh --package parapet-scanner --html
```

Or:

```bash
make -f Makefile.coverage coverage-scanner
```

## Coverage Thresholds

Current thresholds:

- **Minimum**: 70% overall coverage
- **Target**: 80%+ for critical components
- **Goal**: 90%+ for core security analyzers

### Critical Components (Target: 90%+)

- `core/src/rules/engine.rs` - Rule evaluation engine
- `core/src/rules/analyzers/core/` - Core security analyzers
- `core/src/rules/analyzers/core/instruction_padding.rs` - Padding detection
- `proxy/src/rpc_handler.rs` - RPC request handling

### Important Components (Target: 80%+)

- `core/src/rules/analyzer.rs` - Analyzer registry
- `core/src/rules/types.rs` - Rule types
- `proxy/src/server.rs` - Server initialization
- `scanner/src/lib.rs` - Scanner core

### Supporting Components (Target: 70%+)

- `core/src/enrichment/` - External data enrichment
- `core/src/program_analysis/` - Program analysis
- Examples and utilities

## CI/CD Integration

### GitHub Actions

Coverage is automatically generated on every push/PR via `.github/workflows/coverage.yml`.

**What it does:**

1. Runs all tests with coverage instrumentation
2. Generates LCOV report
3. Uploads to Codecov
4. Generates HTML report (artifact)
5. Checks coverage threshold (fails if < 70%)

**Viewing Reports:**

- Codecov: use your repository’s Codecov project URL (configure GitHub org/repo in Codecov; the workflow uses `secrets.CODECOV_TOKEN`).
- GitHub Actions artifacts: download the `coverage-report` / `coverage-summary` artifacts from the workflow run (`branches: main` and `master`).

### Local Pre-commit Check

Avoid wiring `./coverage.sh --summary` into pre-commit unless you accept a **full workspace coverage run** plus the **70% threshold check** on every commit (slow and brittle if the tree does not compile). Prefer running coverage in CI or manually before release.

## Reading Coverage Reports

### HTML Report

Open `target/llvm-cov/html/index.html` in a browser.

**Color coding:**

- 🟢 Green: Line executed
- 🔴 Red: Line not executed
- 🟡 Yellow: Partially executed (branches)

**Key metrics:**

- **Line Coverage**: % of lines executed
- **Function Coverage**: % of functions called
- **Branch Coverage**: % of conditional branches taken

### LCOV Report

Used by Codecov and other tools. Format:

```
TN:
SF:src/rules/engine.rs
FN:42,RuleEngine::evaluate
FNDA:10,RuleEngine::evaluate
DA:42,10
DA:43,10
DA:44,5
end_of_record
```

- `SF`: Source file
- `FN`: Function name
- `FNDA`: Function execution count
- `DA`: Line number and execution count

## Improving Coverage

### Identify Uncovered Code

```bash
# Generate HTML report
./coverage.sh --html --open

# Look for red lines in the report
# Focus on critical paths first
```

### Add Tests for Uncovered Code

Example: If `instruction_padding.rs` line 150 is uncovered:

```rust
#[tokio::test]
async fn test_uncovered_edge_case() {
    // Test the specific condition that triggers line 150
    let analyzer = InstructionPaddingAnalyzer::new();
    let tx = create_edge_case_transaction();
    let result = analyzer.analyze(&tx).await.unwrap();
    assert_eq!(result["has_suspicious_padding"], json!(true));
}
```

### Focus on Critical Paths

Priority order:

1. **Error handling**: Ensure all error paths are tested
2. **Security checks**: All security validations must be covered
3. **Edge cases**: Boundary conditions, empty inputs, max values
4. **Integration points**: RPC handlers, analyzer registration

### Exclude paths from reports

Use `cargo llvm-cov` options (e.g. ignore filters) as documented in the [cargo-llvm-cov README](https://github.com/taiki-e/cargo-llvm-cov); align any excludes with how CI and `coverage.sh` invoke the tool.

## Coverage Best Practices

### 1. Test Behavior, Not Implementation

❌ Bad:

```rust
#[test]
fn test_internal_function() {
    let result = internal_helper_function();
    assert_eq!(result, 42);
}
```

✅ Good:

```rust
#[test]
fn test_public_api_behavior() {
    let analyzer = MyAnalyzer::new();
    let result = analyzer.analyze(&transaction).await.unwrap();
    assert_eq!(result["field"], json!(expected_value));
}
```

### 2. Test Edge Cases

```rust
#[test]
fn test_empty_input() { /* ... */ }

#[test]
fn test_max_size_input() { /* ... */ }

#[test]
fn test_invalid_input() { /* ... */ }

#[test]
fn test_concurrent_access() { /* ... */ }
```

### 3. Test Error Paths

```rust
#[test]
fn test_error_handling() {
    let result = function_that_can_fail(invalid_input);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().to_string(), "Expected error message");
}
```

### 4. Integration Tests

Place in `tests/` directory:

```rust
// tests/integration_test.rs
use parapet_core::rules::{AnalyzerRegistry, RuleEngine};

#[tokio::test]
async fn test_end_to_end_flow() {
    let registry = AnalyzerRegistry::new();
    let engine = RuleEngine::new(registry);
    // Test complete flow
}
```

## Troubleshooting

### "cargo-llvm-cov not found"

```bash
cargo install cargo-llvm-cov
```

### "llvm-tools-preview not installed"

```bash
rustup component add llvm-tools-preview
```

### Coverage lower than expected

1. Check for untested error paths
2. Look for `#[cfg(test)]` code that's not executed
3. Check for unreachable code
4. Review async code (ensure `.await` is tested)

### WASM tests failing

```bash
rustup target add wasm32-unknown-unknown
cd core/tests/wasm_mock
cargo build --target wasm32-unknown-unknown --release
```

### "bc: command not found" (in coverage.sh)

The coverage script and `.github/workflows/coverage.yml` threshold step use `bc` for floating-point comparison.

```bash
# Ubuntu/Debian
sudo apt-get install bc

# macOS
brew install bc
```

### Threshold shows `0%` or check always fails locally

`coverage.sh` parses the summary with `grep -oP`, which requires **GNU grep** (`grep (GNU grep)`). BSD grep on macOS does not support `-P`; use a Linux environment, CI, or adjust parsing to match your `cargo llvm-cov --summary-only` output.

## Coverage Reports Location

- **HTML**: `target/llvm-cov/html/index.html`
- **LCOV**: `lcov.info`
- **JSON**: `coverage.json`
- **Summary**: Console output

## Continuous Monitoring

### Codecov Integration

1. Sign up at [https://codecov.io](https://codecov.io)
2. Add repository
3. Get token
4. Add to GitHub secrets: `CODECOV_TOKEN`
5. Coverage automatically uploaded on CI runs

### Coverage Badge

Add to README.md (replace with your Codecov badge URL):

```markdown
[![codecov](https://codecov.io/gh/<org>/<repo>/branch/main/graph/badge.svg)](https://codecov.io/gh/<org>/<repo>)
```

### Track Coverage Over Time

Codecov provides:

- Coverage trends
- PR coverage diff
- File-level coverage
- Commit-level coverage

## Component-Specific Coverage Goals

### Core Library (`parapet-core`)


| Component           | Current | Target | Priority |
| ------------------- | ------- | ------ | -------- |
| Rule Engine         | TBD     | 90%    | Critical |
| Analyzers           | TBD     | 85%    | Critical |
| Instruction Padding | TBD     | 95%    | Critical |
| Performance Tracker | TBD     | 80%    | High     |
| Dynamic Rules       | TBD     | 75%    | Medium   |


### RPC Proxy (`parapet-proxy`)


| Component   | Current | Target | Priority |
| ----------- | ------- | ------ | -------- |
| RPC Handler | TBD     | 85%    | Critical |
| Server Init | TBD     | 80%    | High     |
| Auth System | TBD     | 90%    | Critical |
| Escalations | TBD     | 75%    | Medium   |


### Wallet Scanner (`parapet-scanner`)


| Component        | Current | Target | Priority |
| ---------------- | ------- | ------ | -------- |
| Scanner Core     | TBD     | 80%    | High     |
| History Analysis | TBD     | 75%    | Medium   |
| CLI Tools        | TBD     | 60%    | Low      |


## Next Steps

1. **Baseline**: Run `./coverage.sh --summary` to establish current coverage
2. **Identify Gaps**: Review HTML report for uncovered critical code
3. **Add Tests**: Focus on security-critical components first
4. **Monitor**: Set up Codecov and track coverage trends
5. **Enforce**: Add coverage checks to CI (already configured)

## Resources

- [cargo-llvm-cov docs](https://github.com/taiki-e/cargo-llvm-cov)
- [Rust testing guide](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [Codecov documentation](https://docs.codecov.com/)

