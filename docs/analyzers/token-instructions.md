# TokenInstructionAnalyzer Field Reference

## Overview

The `TokenInstructionAnalyzer` analyzes SPL Token Program instructions, including token transfers, approvals, mints, burns, and account operations.

**Analyzer Name**: `token_instructions`

## Available Fields

### Token Transfers

| Field | Type | Description | Variable Interpolation |
|-------|------|-------------|------------------------|
| `has_transfer` | Boolean | True if transaction contains token transfers | ❌ No |
| `transfer_count` | Number | Number of token transfer instructions | ❌ No |
| `total_transfer_amount` | Number | Total tokens transferred (sum, raw amount) | ❌ No |
| `max_transfer_amount` | Number | Largest single transfer amount (raw) | ❌ No |

**Example Values**:
```json
{
  "has_transfer": true,
  "transfer_count": 3,
  "total_transfer_amount": 5000000,
  "max_transfer_amount": 3000000
}
```

### Token Mints

| Field | Type | Description | Variable Interpolation |
|-------|------|-------------|------------------------|
| `has_mint` | Boolean | True if transaction mints new tokens | ❌ No |
| `mint_count` | Number | Number of mint instructions | ❌ No |
| `mints` | Array[String] | List of token mint addresses involved | ✅ Yes - `{mint}` |

**Example Values**:
```json
{
  "has_mint": true,
  "mint_count": 1,
  "mints": ["EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"]
}
```

### Token Burns

| Field | Type | Description | Variable Interpolation |
|-------|------|-------------|------------------------|
| `has_burn` | Boolean | True if transaction burns tokens | ❌ No |
| `burn_count` | Number | Number of burn instructions | ❌ No |

**Example Values**:
```json
{
  "has_burn": true,
  "burn_count": 2
}
```

### Token Approvals

| Field | Type | Description | Variable Interpolation |
|-------|------|-------------|------------------------|
| `has_approve` | Boolean | True if transaction approves delegates | ❌ No |
| `approve_count` | Number | Number of approve instructions | ❌ No |
| `delegates` | Array[String] | List of delegate addresses approved | ✅ Yes - `{delegate}` |

**Example Values**:
```json
{
  "has_approve": true,
  "approve_count": 1,
  "delegates": ["DelegateAddress1111111111111111111111111111"]
}
```

### Token Account Operations

| Field | Type | Description | Variable Interpolation |
|-------|------|-------------|------------------------|
| `has_close_account` | Boolean | True if transaction closes token accounts | ❌ No |
| `close_account_count` | Number | Number of close account instructions | ❌ No |
| `has_freeze` | Boolean | True if transaction freezes token accounts | ❌ No |
| `freeze_count` | Number | Number of freeze instructions | ❌ No |
| `has_thaw` | Boolean | True if transaction thaws token accounts | ❌ No |
| `thaw_count` | Number | Number of thaw instructions | ❌ No |

**Example Values**:
```json
{
  "has_close_account": true,
  "close_account_count": 1,
  "has_freeze": false,
  "freeze_count": 0,
  "has_thaw": false,
  "thaw_count": 0
}
```

### Authority Operations

| Field | Type | Description | Variable Interpolation |
|-------|------|-------------|------------------------|
| `has_set_authority` | Boolean | True if transaction changes token authorities | ❌ No |
| `set_authority_count` | Number | Number of set authority instructions | ❌ No |

**Example Values**:
```json
{
  "has_set_authority": true,
  "set_authority_count": 1
}
```

## Variable Interpolation Support

### `{token_instructions:mints[0]}` - Token Mint Address

**Field**: `token_instructions:mints` (array)

**Use Case**: Track token-specific activity (drain velocity, mint patterns)

**Example**:
```json
{
  "flowbits": {
    "scope": "global",
    "increment": ["token_transfer_count:{token_instructions:mints[0]}"],
    "ttl_seconds": 900
  }
}
```

**Behavior**:
- Transaction with multiple mints: Uses first mint only
- Transaction with no mints: Skips flowbit operation
- Creates unique flowbit per token mint address

### `{token_instructions:delegates[0]}` - Approved Delegate Address

**Field**: `token_instructions:delegates` (array)

**Use Case**: Track delegation patterns (drainer detection)

**Example**:
```json
{
  "flowbits": {
    "scope": "perwallet",
    "increment": ["delegate_approvals:{token_instructions:delegates[0]}"],
    "ttl_seconds": 86400
  }
}
```

**Behavior**:
- Transaction with multiple approvals: Uses first delegate only
- Transaction with no approvals: Skips flowbit operation
- Creates unique flowbit per delegate address

## Common Use Cases

### 1. Token Drain Velocity Detection

Detect mass token transfers across wallets:

```json
{
  "conditions": {
    "all": [
      {"field": "token_instructions:has_transfer", "operator": "equals", "value": true},
      {"field": "flowbit_global:token_transfer_count:{token_instructions:mints[0]}", "operator": "greater_than", "value": 10}
    ]
  },
  "flowbits": {
    "scope": "global",
    "increment": ["token_transfer_count:{token_instructions:mints[0]}"],
    "ttl_seconds": 900
  },
  "message": "Enterprise: Unusual token transfer velocity - 10+ transfers in 15 min"
}
```

### 2. Delegate Approval Tracking

Track suspicious delegate approvals:

```json
{
  "conditions": {
    "all": [
      {"field": "token_instructions:has_approve", "operator": "equals", "value": true},
      {"field": "token_instructions:delegates", "operator": "not_in", "value": ["<known_dex_programs>"]},
      {"field": "flowbit:delegate_approvals:{token_instructions:delegates[0]}", "operator": "greater_than", "value": 0}
    ]
  },
  "flowbits": {
    "scope": "perwallet",
    "increment": ["delegate_approvals:{token_instructions:delegates[0]}"],
    "ttl_seconds": 86400
  },
  "message": "Warning: Multiple approvals to unknown delegate"
}
```

### 3. Token Mint Activity Monitoring

Track minting patterns:

```json
{
  "conditions": {
    "field": "token_instructions:has_mint",
    "operator": "equals",
    "value": true
  },
  "flowbits": {
    "scope": "global",
    "increment": ["mint_activity:{token_instructions:mints[0]}"],
    "ttl_seconds": 3600
  }
}
```

### 4. Token Account Closure Detection

Detect suspicious account closures:

```json
{
  "conditions": {
    "all": [
      {"field": "token_instructions:has_close_account", "operator": "equals", "value": true},
      {"field": "token_instructions:close_account_count", "operator": "greater_than", "value": 5}
    ]
  },
  "message": "Warning: Closing multiple token accounts (5+) in single transaction"
}
```

### 5. Authority Change Detection

Alert on authority changes:

```json
{
  "conditions": {
    "field": "token_instructions:has_set_authority",
    "operator": "equals",
    "value": true
  },
  "message": "Alert: Token authority change detected"
}
```

## Field Access in Rules

### Direct Field Access

```json
{
  "field": "token_instructions:has_transfer",
  "operator": "equals",
  "value": true
}
```

### Array Field Access

```json
{
  "field": "token_instructions:mints",
  "operator": "contains",
  "value": "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"
}
```

### Numeric Comparisons

```json
{
  "field": "token_instructions:transfer_count",
  "operator": "greater_than",
  "value": 10
}
```

## Token Program Versions

This analyzer supports:
- **Token Program**: `TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA`
- **Token-2022 Program**: `TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb`

Both programs are analyzed with the same field structure.

## Performance Notes

- **Analysis Time**: ~2ms per transaction
- **Memory**: Negligible (fields are computed on-demand)
- **Caching**: Fields are cached per transaction evaluation

## Security Considerations

### High-Risk Patterns

1. **Approve + Transfer in same tx**: Potential drainer pattern
2. **Multiple CloseAccount**: Possible account cleanup after drain
3. **SetAuthority + Transfer**: Authority takeover + drain
4. **Large transfer amounts**: Potential whale wallet compromise

### Allowlist Recommendations

Maintain allowlists for:
- Known DEX programs (delegates)
- Legitimate token mints
- Trusted authority addresses

## Related Analyzers

- **SystemProgramAnalyzer**: SOL transfers and account operations
- **BasicAnalyzer**: Transaction-level metadata
- **SimulationTokenAnalyzer**: Pre/post token balance changes

## See Also

- [Variable Interpolation Guide](../flowbits-variable-interpolation.md)
- [Configuration Guide](../flowbits-configuration-guide.md)
- [System Program Analyzer](system-program.md)
- [Basic Analyzer](basic.md)
