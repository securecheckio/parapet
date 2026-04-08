# Test Coverage Guide

## Overview

Parapet uses `cargo-llvm-cov` for test coverage reporting. This provides accurate, line-level coverage data for all Rust code.

## Quick Start

### Install Tools

```bash
# Install cargo-llvm-cov
cargo install cargo-llvm-cov

# Install WASM target (required for some tests)
rustup target add wasm32-unknown-unknown
```

Or use the Makefile:

```bash
make -f Makefile.coverage install-tools
```

### Generate Coverage Report

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

## Coverage Tools in Rust

### Option 1: cargo-llvm-cov (Recommended) ✅

**Pros:**
- Most accurate (uses LLVM instrumentation)
- Fast and reliable
- Built into Rust toolchain
- Supports all output formats (HTML, LCOV, JSON)
- Works with all Rust features

**Cons:**
- Requires llvm-tools-preview component

**Installation:**
```bash
cargo install cargo-llvm-cov
rustup component add llvm-tools-preview
```

**Usage:**
```bash
# HTML report
cargo llvm-cov --html --open

# LCOV report
cargo llvm-cov --lcov --output-path lcov.info

# JSON report
cargo llvm-cov --json --output-path coverage.json

# Summary
cargo llvm-cov --summary-only
```

### Option 2: cargo-tarpaulin

**Pros:**
- Easy to use
- Good for simple projects
- Integrates with Codecov

**Cons:**
- Linux-only
- Slower than llvm-cov
- Less accurate
- Doesn't work with all Rust features

**Installation:**
```bash
cargo install cargo-tarpaulin
```

**Usage:**
```bash
cargo tarpaulin --out Html --output-dir coverage
```

### Option 3: grcov

**Pros:**
- Mozilla's official tool
- Very accurate
- Cross-platform

**Cons:**
- More complex setup
- Requires manual profraw file handling
- Slower workflow

**Not recommended for Parapet** - Use cargo-llvm-cov instead.

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
- Codecov: https://codecov.io/gh/YOUR_ORG/parapet
- GitHub Actions artifacts: Download HTML report from workflow run

### Local Pre-commit Check

Add to `.git/hooks/pre-commit`:

```bash
#!/bin/bash
./coverage.sh --summary
```

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

### Exclude Non-testable Code

Add to `Cargo.toml`:

```toml
[package.metadata.coverage]
exclude = [
    "src/bin/*",  # Binary entry points
    "examples/*",  # Example code
]
```

Or use `#[cfg(not(tarpaulin_include))]` for specific functions:

```rust
#[cfg(not(tarpaulin_include))]
fn unreachable_panic_handler() {
    // This is only called in impossible situations
    panic!("This should never happen");
}
```

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
use sol_shield_core::*;

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

```bash
# Ubuntu/Debian
sudo apt-get install bc

# macOS
brew install bc
```

## Coverage Reports Location

- **HTML**: `target/llvm-cov/html/index.html`
- **LCOV**: `lcov.info`
- **JSON**: `coverage.json`
- **Summary**: Console output

## Continuous Monitoring

### Codecov Integration

1. Sign up at https://codecov.io
2. Add repository
3. Get token
4. Add to GitHub secrets: `CODECOV_TOKEN`
5. Coverage automatically uploaded on CI runs

### Coverage Badge

Add to README.md:

```markdown
[![codecov](https://codecov.io/gh/YOUR_ORG/parapet/branch/main/graph/badge.svg)](https://codecov.io/gh/YOUR_ORG/parapet)
```

### Track Coverage Over Time

Codecov provides:
- Coverage trends
- PR coverage diff
- File-level coverage
- Commit-level coverage

## Component-Specific Coverage Goals

### Core Library (`parapet-core`)

| Component | Current | Target | Priority |
|-----------|---------|--------|----------|
| Rule Engine | TBD | 90% | Critical |
| Analyzers | TBD | 85% | Critical |
| Instruction Padding | TBD | 95% | Critical |
| Performance Tracker | TBD | 80% | High |
| Dynamic Rules | TBD | 75% | Medium |

### RPC Proxy (`parapet-proxy`)

| Component | Current | Target | Priority |
|-----------|---------|--------|----------|
| RPC Handler | TBD | 85% | Critical |
| Server Init | TBD | 80% | High |
| Auth System | TBD | 90% | Critical |
| Escalations | TBD | 75% | Medium |

### Wallet Scanner (`parapet-scanner`)

| Component | Current | Target | Priority |
|-----------|---------|--------|----------|
| Scanner Core | TBD | 80% | High |
| History Analysis | TBD | 75% | Medium |
| CLI Tools | TBD | 60% | Low |

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
