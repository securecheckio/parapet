# BasicAnalyzer Field Reference

## Overview

The `BasicAnalyzer` extracts fundamental transaction metadata including signatures, program IDs, signers, fee payer, and transaction structure information.

**Analyzer Name**: `basic`

## Available Fields

### Transaction Identity


| Field       | Type   | Description                    | Variable Interpolation |
| ----------- | ------ | ------------------------------ | ---------------------- |
| `signature` | String | Transaction signature (base58) | ❌ No                   |
| `fee_payer` | String | Address of the fee payer       | ✅ Yes - `{fee_payer}`  |


**Example Values**:

```json
{
  "signature": "5VERv8NMvzbJMEkV8xnrLkEaWRtSz9CosKDYjCJjBRnbJLgp8uirBgmQpjKhoR4tjF3ZpRzrFmBV6UjKdiSZkQUW",
  "fee_payer": "7xKHnfHvPfVvFVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVV"
}
```

### Program Information


| Field                | Type          | Description                       | Variable Interpolation |
| -------------------- | ------------- | --------------------------------- | ---------------------- |
| `program_ids`        | Array[String] | List of all program IDs invoked   | ✅ Yes - `{program_id}` |
| `program_count`      | Number        | Number of unique programs invoked | ❌ No                   |
| `has_system_program` | Boolean       | True if System Program is invoked | ❌ No                   |
| `has_token_program`  | Boolean       | True if Token Program is invoked  | ❌ No                   |


**Example Values**:

```json
{
  "program_ids": [
    "11111111111111111111111111111111",
    "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",
    "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4"
  ],
  "program_count": 3,
  "has_system_program": true,
  "has_token_program": true
}
```

### Signer Information


| Field          | Type          | Description                       | Variable Interpolation |
| -------------- | ------------- | --------------------------------- | ---------------------- |
| `signers`      | Array[String] | List of all transaction signers   | ❌ No                   |
| `signer_count` | Number        | Number of signers                 | ❌ No                   |
| `is_multisig`  | Boolean       | True if transaction has >1 signer | ❌ No                   |


**Example Values**:

```json
{
  "signers": [
    "7xKHnfHvPfVvFVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVV",
    "3vQB7b6MrGQZaxCuFjFU9UKXesaokpe7yTrq1kPY2PnR"
  ],
  "signer_count": 2,
  "is_multisig": true
}
```

### Transaction Structure


| Field                    | Type   | Description                       | Variable Interpolation |
| ------------------------ | ------ | --------------------------------- | ---------------------- |
| `instruction_count`      | Number | Number of top-level instructions  | ❌ No                   |
| `account_count`          | Number | Number of accounts in transaction | ❌ No                   |
| `writable_account_count` | Number | Number of writable accounts       | ❌ No                   |
| `readonly_account_count` | Number | Number of readonly accounts       | ❌ No                   |


**Example Values**:

```json
{
  "instruction_count": 4,
  "account_count": 12,
  "writable_account_count": 6,
  "readonly_account_count": 6
}
```

### Transaction Metadata


| Field                | Type    | Description                                    | Variable Interpolation |
| -------------------- | ------- | ---------------------------------------------- | ---------------------- |
| `recent_blockhash`   | String  | Recent blockhash used                          | ❌ No                   |
| `uses_lookup_tables` | Boolean | True if transaction uses address lookup tables | ❌ No                   |


**Example Values**:

```json
{
  "recent_blockhash": "9sHcv6xwn9YkB8nxTUGKDwPwNnmqVp5oAXxU8Fdkm4J6",
  "uses_lookup_tables": false
}
```

## Variable Interpolation Support

### `{basic:fee_payer}` - Transaction Fee Payer

**Field**: `basic:fee_payer` (string)

**Use Case**: Track activity by specific wallet (less common - usually tracked automatically)

**Example**:

```json
{
  "flowstate": {
    "increment": ["activity_by:{basic:fee_payer}"],
    "ttl_seconds": 3600
  }
}
```

**Behavior**:

- Always present (every transaction has a fee payer)
- Creates unique flowstate per fee payer address
- Note: Per-wallet flowstate already track by fee payer automatically

### `{basic:program_ids[0]}` - Invoked Program

**Field**: `basic:program_ids` (array)

**Use Case**: Track interactions with specific programs

**Example**:

```json
{
  "flowstate": {
    "scope": "perwallet",
    "increment": ["program_interaction:{basic:program_ids[0]}"],
    "ttl_seconds": 604800
  }
}
```

**Behavior**:

- Transaction with multiple programs: Uses first program only
- Transaction with no programs: Skips flowstate operation (rare)
- Creates unique flowstate per program ID

## Common Use Cases

### 1. Unknown Program Interaction Detection

Track repeated interaction with unverified programs:

```json
{
  "conditions": {
    "all": [
      {"field": "basic:program_ids", "operator": "not_in", "value": [
        "11111111111111111111111111111111",
        "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",
        "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4"
      ]},
      {"field": "flowstate:program_interaction:{basic:program_ids[0]}", "operator": "greater_than", "value": 2}
    ]
  },
  "flowstate": {
    "scope": "perwallet",
    "increment": ["program_interaction:{basic:program_ids[0]}"],
    "ttl_seconds": 604800
  },
  "message": "AI Agent: Repeated interaction with unknown program"
}
```

### 2. Multisig Transaction Monitoring

Track multisig activity:

```json
{
  "conditions": {
    "field": "basic:is_multisig",
    "operator": "equals",
    "value": true
  },
  "flowstate": {
    "increment": ["multisig_activity"],
    "ttl_seconds": 86400
  }
}
```

### 3. Complex Transaction Detection

Detect unusually complex transactions:

```json
{
  "conditions": {
    "any": [
      {"field": "basic:instruction_count", "operator": "greater_than", "value": 10},
      {"field": "basic:account_count", "operator": "greater_than", "value": 20}
    ]
  },
  "message": "Alert: Complex transaction detected"
}
```

### 4. Program Allowlist Enforcement

Block transactions to non-allowlisted programs:

```json
{
  "conditions": {
    "field": "basic:program_ids",
    "operator": "not_in",
    "value": [
      "11111111111111111111111111111111",
      "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",
      "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"
    ]
  },
  "message": "Blocked: Transaction invokes non-allowlisted program"
}
```

### 5. Address Lookup Table Detection

Monitor usage of address lookup tables:

```json
{
  "conditions": {
    "field": "basic:uses_lookup_tables",
    "operator": "equals",
    "value": true
  },
  "message": "Info: Transaction uses address lookup tables"
}
```

## Field Access in Rules

### Direct Field Access

```json
{
  "field": "basic:signature",
  "operator": "exists"
}
```

### Array Field Access

```json
{
  "field": "basic:program_ids",
  "operator": "contains",
  "value": "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4"
}
```

### Numeric Comparisons

```json
{
  "field": "basic:instruction_count",
  "operator": "greater_than",
  "value": 5
}
```

## Known Program IDs

### Core Programs

```json
{
  "System Program": "11111111111111111111111111111111",
  "Token Program": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",
  "Token-2022 Program": "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb",
  "Associated Token Program": "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL",
  "Memo Program": "MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr",
  "Compute Budget": "ComputeBudget111111111111111111111111111111"
}
```

### Popular DEXs

```json
{
  "Jupiter": "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4",
  "Orca Whirlpool": "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc",
  "Raydium": "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8",
  "Serum": "9xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVFin"
}
```

## Performance Notes

- **Analysis Time**: <1ms per transaction
- **Memory**: Negligible (fields are computed on-demand)
- **Caching**: Fields are cached per transaction evaluation

## Security Considerations

### High-Risk Patterns

1. **Unknown programs**: Transactions to unverified programs
2. **High instruction count**: Complex transactions (>10 instructions)
3. **Many writable accounts**: Potential for widespread state changes
4. **Single signer + many programs**: Possible malicious automation

### Allowlist Recommendations

Maintain allowlists for:

- Known safe programs (DEXs, lending protocols, etc.)
- Trusted multisig addresses
- Legitimate program upgrade authorities

## Related Analyzers

- **SystemProgramAnalyzer**: System Program instruction analysis
- **TokenInstructionAnalyzer**: Token Program instruction analysis
- **ProgramComplexityAnalyzer**: Advanced program interaction patterns

## See Also

- [FlowState](../RULES_FLOWSTATE.md#flowstate-variable-interpolation)
- [Rule development hub](../RULES_DEVELOPMENT.md)
- [Rule JSON format](../RULES_FORMAT.md)
- [System Program Analyzer](system-program.md)
- [Token Instructions Analyzer](token-instructions.md)

