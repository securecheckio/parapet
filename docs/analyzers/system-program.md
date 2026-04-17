# SystemProgramAnalyzer Field Reference

## Overview

The `SystemProgramAnalyzer` analyzes Solana's built-in System Program instructions, including SOL transfers, account creation, program assignment, memory allocation, and durable nonce operations.

**Analyzer Name**: `system`

## Available Fields

### SOL Transfers


| Field                   | Type          | Description                                       | Variable Interpolation |
| ----------------------- | ------------- | ------------------------------------------------- | ---------------------- |
| `has_sol_transfer`      | Boolean       | True if transaction contains SOL transfers        | ❌ No                   |
| `sol_transfer_count`    | Number        | Number of SOL transfer instructions               | ❌ No                   |
| `total_sol_transferred` | Number        | Total lamports transferred (sum of all transfers) | ❌ No                   |
| `max_sol_transfer`      | Number        | Largest single transfer amount in lamports        | ❌ No                   |
| `sol_recipients`        | Array[String] | List of recipient addresses for SOL transfers     | ✅ Yes - `{recipient}`  |


**Example Values**:

```json
{
  "has_sol_transfer": true,
  "sol_transfer_count": 2,
  "total_sol_transferred": 5000000000,
  "max_sol_transfer": 3000000000,
  "sol_recipients": ["7xKHnfHvPfVvFVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVV", "3vQB7b6MrGQZaxCuFjFU9UKXesaokpe7yTrq1kPY2PnR"]
}
```

### Account Operations


| Field                    | Type    | Description                                | Variable Interpolation |
| ------------------------ | ------- | ------------------------------------------ | ---------------------- |
| `creates_accounts`       | Boolean | True if transaction creates new accounts   | ❌ No                   |
| `account_creation_count` | Number  | Number of CreateAccount instructions       | ❌ No                   |
| `total_rent_required`    | Number  | Total lamports required for rent exemption | ❌ No                   |


**Example Values**:

```json
{
  "creates_accounts": true,
  "account_creation_count": 3,
  "total_rent_required": 6000000
}
```

### Program Assignment


| Field                       | Type    | Description                                   | Variable Interpolation |
| --------------------------- | ------- | --------------------------------------------- | ---------------------- |
| `assigns_program_ownership` | Boolean | True if transaction assigns program ownership | ❌ No                   |
| `assign_count`              | Number  | Number of Assign instructions                 | ❌ No                   |


**Example Values**:

```json
{
  "assigns_program_ownership": true,
  "assign_count": 1
}
```

### Memory Allocation


| Field            | Type   | Description                                      | Variable Interpolation |
| ---------------- | ------ | ------------------------------------------------ | ---------------------- |
| `allocate_count` | Number | Number of Allocate/AllocateWithSeed instructions | ❌ No                   |


**Example Values**:

```json
{
  "allocate_count": 2
}
```

### Durable Nonces


| Field                | Type              | Description                                  | Variable Interpolation    |
| -------------------- | ----------------- | -------------------------------------------- | ------------------------- |
| `uses_durable_nonce` | Boolean           | True if transaction uses a durable nonce     | ❌ No                      |
| `advances_nonce`     | Boolean           | True if transaction advances a nonce account | ❌ No                      |
| `nonce_account`      | String (Optional) | Address of the nonce account being advanced  | ✅ Yes - `{nonce_account}` |


**Example Values**:

```json
{
  "uses_durable_nonce": true,
  "advances_nonce": true,
  "nonce_account": "NonceAccount1111111111111111111111111111111"
}
```

**Note**: `nonce_account` is only present when `advances_nonce` is true.

### Security Indicators


| Field                | Type    | Description                                    | Variable Interpolation |
| -------------------- | ------- | ---------------------------------------------- | ---------------------- |
| `high_rent_spam`     | Boolean | True if creating >10 accounts (spam indicator) | ❌ No                   |
| `large_sol_transfer` | Boolean | True if any transfer >1 SOL (1B lamports)      | ❌ No                   |


**Example Values**:

```json
{
  "high_rent_spam": false,
  "large_sol_transfer": true
}
```

## Variable Interpolation Support

### `{system:sol_recipients[0]}` - SOL Transfer Recipient

**Field**: `system:sol_recipients` (array)

**Use Case**: Track transfers to specific addresses

**Example**:

```json
{
  "flowstate": {
    "increment": ["transfers_to:{system:sol_recipients[0]}"],
    "ttl_seconds": 86400
  }
}
```

**Behavior**:

- Transaction with 2 recipients: Uses first recipient only
- Transaction with 0 recipients: Skips flowstate operation
- Creates unique flowstate per recipient address

### `{system:nonce_account}` - Durable Nonce Account

**Field**: `system:nonce_account` (string, optional)

**Use Case**: Track nonce advancement for staleness detection

**Example**:

```json
{
  "flowstate": {
    "scope": "global",
    "set": ["nonce_advanced:{system:nonce_account}"],
    "ttl_seconds": 1800
  }
}
```

**Behavior**:

- Only available when `advances_nonce` is true
- If field not present: Skips flowstate operation
- Creates unique flowstate per nonce account

## Common Use Cases

### 1. AI Agent Velocity Limiting

Detect runaway transaction behavior:

```json
{
  "conditions": {
    "field": "system:has_sol_transfer",
    "operator": "equals",
    "value": true
  },
  "flowstate": {
    "increment": ["transaction_count"],
    "ttl_seconds": 600
  }
}
```

### 2. Account Creation Spam Detection

Detect CreateAccount loops:

```json
{
  "conditions": {
    "field": "system:creates_accounts",
    "operator": "equals",
    "value": true
  },
  "flowstate": {
    "increment": ["account_creation_count"],
    "ttl_seconds": 300
  }
}
```

### 3. Gradual Exfiltration Detection

Track repeated transfers to same recipient:

```json
{
  "conditions": {
    "all": [
      {"field": "system:has_sol_transfer", "operator": "equals", "value": true},
      {"field": "flowstate:transfers_to:{system:sol_recipients[0]}", "operator": "greater_than", "value": 3}
    ]
  },
  "flowstate": {
    "scope": "perwallet",
    "increment": ["transfers_to:{system:sol_recipients[0]}"],
    "ttl_seconds": 86400
  }
}
```

### 4. Lateral Movement Detection

Detect same recipient receiving from multiple wallets:

```json
{
  "conditions": {
    "all": [
      {"field": "system:has_sol_transfer", "operator": "equals", "value": true},
      {"field": "flowstate_global:suspicious_recipient:{system:sol_recipients[0]}", "operator": "greater_than", "value": 2}
    ]
  },
  "flowstate": {
    "scope": "global",
    "increment": ["suspicious_recipient:{system:sol_recipients[0]}"],
    "ttl_seconds": 3600
  }
}
```

### 5. Durable Nonce Staleness Detection

Prevent Drift-style attacks with stale nonces:

```json
[
  {
    "conditions": {
      "field": "system:advances_nonce",
      "operator": "equals",
      "value": true
    },
    "flowstate": {
      "scope": "global",
      "set": ["nonce_advanced:{system:nonce_account}"],
      "ttl_seconds": 1800
    }
  },
  {
    "conditions": {
      "all": [
        {"field": "system:uses_durable_nonce", "operator": "equals", "value": true},
        {"field": "system:max_sol_transfer", "operator": "greater_than", "value": 1000000000},
        {"field": "flowstate_global:nonce_advanced:{system:nonce_account}", "operator": "isnotset"}
      ]
    }
  }
]
```

## Field Access in Rules

### Direct Field Access

```json
{
  "field": "system:has_sol_transfer",
  "operator": "equals",
  "value": true
}
```

### Array Field Access

```json
{
  "field": "system:sol_recipients",
  "operator": "contains",
  "value": "7xKHnfHvPfVvFVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVV"
}
```

### Numeric Comparisons

```json
{
  "field": "system:max_sol_transfer",
  "operator": "greater_than",
  "value": 1000000000
}
```

## Performance Notes

- **Analysis Time**: ~1ms per transaction
- **Memory**: Negligible (fields are computed on-demand)
- **Caching**: Fields are cached per transaction evaluation

## Related Analyzers

- **BasicAnalyzer**: Transaction-level metadata (fee payer, signatures, program IDs)
- **TokenInstructionAnalyzer**: SPL Token operations (transfers, approvals, mints)
- **InnerInstructionAnalyzer**: Cross-program invocation analysis

## See Also

- [FlowState](../RULES_FLOWSTATE.md#flowstate-variable-interpolation)
- [Rule development hub](../RULES_DEVELOPMENT.md)
- [Rule JSON format](../RULES_FORMAT.md)
- [Token Instructions Analyzer](token-instructions.md)
- [Basic Analyzer](basic.md)

