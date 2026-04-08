# Parapet Risk Inventory Database

Machine-readable database of all security risks detected by Parapet.

## Generated Files

- **risk-inventory.csv** - Main database with 67 risks
- **risk-categories.csv** - Risk categories and statistics
- **analyzer-coverage.csv** - Which analyzers detect which risks
- **rule-coverage.csv** - Which rules detect which risks

## Risk Inventory Schema

| Column | Description |
|--------|-------------|
| risk_code | Unique identifier (RISK-001, RISK-002, etc.) |
| risk_category | High-level category |
| risk_name | Human-readable name |
| risk_description | Detailed description |
| severity | CRITICAL, HIGH, MEDIUM, or LOW |
| rule_ids | Comma-separated rule IDs that detect this risk |
| analyzer_fields | Fields used for detection |
| analyzer_names | Analyzers providing the fields |
| detection_type | signature, heuristic, or pattern |
| action_types | Possible actions (block, alert, pass) |
| attack_vector | How the attack works |
| false_positive_rate | Estimated FP rate |
| mitigation | How Parapet mitigates this risk |
| references | Related documentation |

## Usage

### Find all CRITICAL risks
```bash
grep "CRITICAL" risk-inventory.csv
```

### Find risks without rules
```bash
awk -F',' '$6 == ""' risk-inventory.csv
```

### Count risks by category
```bash
cut -d',' -f2 risk-inventory.csv | sort | uniq -c
```

### Find unused analyzer fields
Compare analyzer-coverage.csv fields with rule-coverage.csv to identify gaps.

## Statistics

- **Total Risks**: 67
- **Analyzers**: 20
- **Rules**: 200
- **Categories**: 9

## Severity Distribution

- **CRITICAL**: 20
- **HIGH**: 15
- **MEDIUM**: 15
- **LOW**: 17

## Next Steps

1. **Gap Analysis**: Identify risks without detection rules
2. **Test Coverage**: Create test transactions for each risk
3. **False Positive Testing**: Validate rules against benign transactions
4. **Continuous Updates**: Add new risks as they're discovered

## Regeneration

To regenerate this database:

```bash
python3 generate_risk_inventory.py
```

Generated: generate_risk_inventory.py
