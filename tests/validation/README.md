# Parapet Validation Tests

Validation scripts to prove key features work correctly.

## Scripts

### validate-rules.sh
Validates all JSON rule files are syntactically correct.

```bash
./validate-rules.sh
```

### validate-performance.sh
Validates that transaction analysis meets the <50ms performance requirement.

```bash
./validate-performance.sh
```

### validate-drain-detection.sh
Tests that drain detection capabilities are functional:
- Unlimited delegation detection
- Delegation analyzer exists
- Related tests pass

```bash
./validate-drain-detection.sh
```

### validate-authority-changes.sh
Tests that authority change detection is working:
- Authority change analyzer exists
- Authority protection rules exist

```bash
./validate-authority-changes.sh
```

## Running All Validations

```bash
cd tests/validation
for script in validate-*.sh; do
    echo "Running $script..."
    ./$script
    echo ""
done
```

## Purpose

These validation scripts provide quick sanity checks that core security features are:
1. Present in the codebase
2. Configured properly
3. Tested adequately

They complement the full test suite and integration tests.
