# Risk Categories

High-level categorization of security threats Parapet addresses.

## Available Data

**risk-categories.csv** - Threat categories tracked by Parapet analyzers

## Categories

| Category | Description |
|----------|-------------|
| Authority Control | Account ownership and permission changes |
| Token Delegation | SPL token approval and delegation attacks |
| Burn & Close | Suspicious token burn and account closure |
| Token Freeze | Token freeze authority exploitation |
| Phishing Simulation | Transaction simulation-based phishing |
| Program Reputation | Unknown or malicious program detection |
| Transaction Pattern | Abnormal transaction structure |
| Token Safety | Token-level risks (rugpulls, liquidity) |
| Account Mutation | Suspicious account state changes |

## Usage

```bash
# View categories
cat risk-categories.csv

# Count risks by category
cut -d',' -f2 risk-categories.csv | sort | uniq -c
```

## Detailed Risk Database

The complete risk inventory, analyzer mappings, rule coverage, and test cases are maintained in the [parapet-rules](https://github.com/securecheckio/parapet-rules) repository.

This open-source codebase provides the analyzer framework and category definitions. Specific detection rules and risk mappings are part of SecureCheck's consulting offerings.
