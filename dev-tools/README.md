# Development Tools

## Code Coverage

Test coverage analysis using `cargo-llvm-cov`.

### Usage

```bash
# Quick summary
./dev-tools/coverage/coverage.sh --summary

# Generate HTML report
./dev-tools/coverage/coverage.sh --html

# Generate HTML and open in browser
./dev-tools/coverage/coverage.sh --html --open

# Coverage for specific package
./dev-tools/coverage/coverage.sh --package parapet-core

# Generate LCOV report (for CI)
./dev-tools/coverage/coverage.sh --lcov
```

### Files

- `coverage.sh` - Coverage analysis script
- `Makefile.coverage` - Make targets for coverage
- `.llvm-cov.toml` - LLVM coverage configuration

### Requirements

- `cargo-llvm-cov` (installed automatically by script)
- `wasm32-unknown-unknown` target (installed automatically)
