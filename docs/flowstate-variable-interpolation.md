# FlowState Variable Interpolation

## Overview

Variable interpolation allows flowstate to create dynamic flowstate names based on transaction data. This enables tracking of specific recipients, tokens, programs, or other transaction attributes.

## Syntax

Use `{analyzer:field_name}` to reference any analyzer field:

```json
{
  "flowstate": {
    "increment": ["transfers_to:{system:sol_recipients[0]}"]
  }
}
```

**Format**:
- `{analyzer:field_name}` - Direct field value (string, number, boolean)
- `{analyzer:field_name[0]}` - First element of array field
- `{analyzer:field_name[1]}` - Second element of array field
- `{analyzer:field_name[N]}` - Nth element of array field

**Examples**:
```json
{
  "flowstate": {
    "increment": [
      "transfers_to:{system:sol_recipients[0]}",
      "token_activity:{token_instructions:mints[0]}",
      "program_calls:{basic:program_ids[0]}",
      "fee_payer_activity:{basic:fee_payer}",
      "tx_size:{basic:instruction_count}"
    ]
  }
}
```

## Array Handling Rules

When a variable maps to an array field:

1. **Default Behavior**: Uses first element `[0]`
2. **Empty Arrays**: Skips flowstate operation (no flowstate created)
3. **Multiple Elements**: Only tracks first element (future: `[*]` for all)

### Example: Empty Array

```json
{
  "flowstate": {
    "increment": ["transfers_to:{recipient}"]
  }
}
```

**Transaction with no SOL transfers**:
- `system:sol_recipients` = `[]` (empty array)
- **Result**: Flowbit operation skipped (no increment)

**Transaction with 2 SOL transfers**:
- `system:sol_recipients` = `["7xK...9mP", "3vQ...2nR"]`
- **Result**: Flowbit `transfers_to:7xK...9mP` incremented (first element only)

## Use Cases

### 1. Track Transfers to Specific Recipients

**Goal**: Detect gradual exfiltration (repeated transfers to same address)

**Using direct field reference**:
```json
{
  "id": "track-recipient-transfers",
  "rule": {
    "action": "block",
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
    },
    "message": "Repeated transfers to same recipient (4+ in 24h)"
  }
}
```


**How it works**:
1. Transaction sends to `7xK...9mP`
2. Flowbit `transfers_to:7xK...9mP` = 1
3. Next transaction to same address: `transfers_to:7xK...9mP` = 2
4. Transaction to different address `3vQ...2nR`: `transfers_to:3vQ...2nR` = 1
5. Fourth transaction to `7xK...9mP`: `transfers_to:7xK...9mP` = 4 → **BLOCKED**

### 2. Track Token Drain Velocity (Global)

**Goal**: Detect mass compromise across all wallets

```json
{
  "id": "track-token-drain-velocity",
  "rule": {
    "action": "block",
    "conditions": {
      "all": [
        {"field": "token_instructions:has_transfer", "operator": "equals", "value": true},
        {"field": "flowstate_global:token_transfer_count:{token_instructions:mints[0]}", "operator": "greater_than", "value": 10}
      ]
    },
    "flowstate": {
      "scope": "global",
      "increment": ["token_transfer_count:{token_instructions:mints[0]}"],
      "ttl_seconds": 900
    },
    "message": "Unusual token velocity (10+ transfers in 15 min)"
  }
}
```

**How it works**:
1. Wallet A transfers USDC: `token_transfer_count:EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v` = 1
2. Wallet B transfers USDC: `token_transfer_count:EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v` = 2
3. ... (9 more transfers from different wallets)
4. 11th USDC transfer: **BLOCKED** (global velocity limit)

### 3. Track Lateral Movement (Cross-Wallet)

**Goal**: Detect when same recipient receives from multiple internal wallets

```json
{
  "id": "track-lateral-movement",
  "rule": {
    "action": "block",
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
    },
    "message": "Lateral movement: recipient received from 3+ wallets in 1h"
  }
}
```

**How it works**:
1. Wallet A sends to attacker: `suspicious_recipient:AttackerAddr` = 1
2. Wallet B sends to attacker: `suspicious_recipient:AttackerAddr` = 2
3. Wallet C sends to attacker: `suspicious_recipient:AttackerAddr` = 3 → **BLOCKED**

### 4. Track Durable Nonce Staleness

**Goal**: Prevent Drift Protocol-style attacks using stale nonces

```json
[
  {
    "id": "track-nonce-advancement",
    "rule": {
      "action": "pass",
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
    }
  },
  {
    "id": "detect-stale-nonce",
    "rule": {
      "action": "block",
      "conditions": {
        "all": [
          {"field": "system:uses_durable_nonce", "operator": "equals", "value": true},
          {"field": "system:max_sol_transfer", "operator": "greater_than", "value": 1000000000},
          {"field": "flowstate_global:nonce_advanced:{system:nonce_account}", "operator": "isnotset"}
        ]
      },
      "message": "Stale nonce detected (>30 min old) on high-value transfer"
    }
  }
]
```

**How it works**:
1. Transaction advances nonce `NonceAcc123`: `nonce_advanced:NonceAcc123` = true (TTL: 30 min)
2. High-value transfer uses `NonceAcc123`: Flowbit is set → **ALLOWED**
3. 31 minutes later, transfer uses `NonceAcc123`: Flowbit expired → **BLOCKED**

### 5. Track Unknown Program Interaction

**Goal**: Alert on repeated interaction with unverified programs

```json
{
  "id": "track-unknown-program",
  "rule": {
    "action": "alert",
    "conditions": {
      "all": [
        {"field": "basic:program_ids", "operator": "not_in", "value": ["<known_programs>"]},
        {"field": "flowstate:program_interaction:{basic:program_ids[0]}", "operator": "greater_than", "value": 2}
      ]
    },
    "flowstate": {
      "scope": "perwallet",
      "increment": ["program_interaction:{basic:program_ids[0]}"],
      "ttl_seconds": 604800
    },
    "message": "Repeated interaction with unknown program"
  }
}
```

### 6. Track by Any Analyzer Field

**Goal**: Use any analyzer field for tracking

**Examples**:
```json
{
  "flowstate": {
    "increment": [
      "signer_activity:{basic:signers[0]}",
      "high_value_transfers:{system:max_sol_transfer}",
      "complex_tx:{basic:instruction_count}",
      "inner_calls:{inner_instruction:cpi_depth}"
    ]
  }
}
```

**Note**: Numeric and boolean fields are converted to strings for flowstate names.

## Scope: Per-Wallet vs Global

### Per-Wallet Scope (`"scope": "perwallet"`)

Tracks flowstate separately for each wallet (fee payer).

**Use cases**:
- AI agent behavior monitoring
- Individual wallet exfiltration detection
- Per-wallet velocity limits

**Example**:
```json
{
  "flowstate": {
    "scope": "perwallet",
    "increment": ["transfers_to:{recipient}"]
  }
}
```

**Behavior**:
- Wallet A → Recipient X: `transfers_to:RecipientX` (for Wallet A) = 1
- Wallet B → Recipient X: `transfers_to:RecipientX` (for Wallet B) = 1
- **Independent counters per wallet**

### Global Scope (`"scope": "global"`)

Tracks flowstate across all wallets.

**Use cases**:
- Enterprise lateral movement detection
- Cross-wallet token drain velocity
- Durable nonce staleness (shared nonces)

**Example**:
```json
{
  "flowstate": {
    "scope": "global",
    "increment": ["suspicious_recipient:{recipient}"]
  }
}
```

**Behavior**:
- Wallet A → Recipient X: `suspicious_recipient:RecipientX` (global) = 1
- Wallet B → Recipient X: `suspicious_recipient:RecipientX` (global) = 2
- **Shared counter across all wallets**

## Flowbit Operations

### Set

Sets a boolean flowstate (true).

```json
{
  "flowstate": {
    "set": ["flag_name:{variable}"],
    "ttl_seconds": 3600
  }
}
```

**Use case**: Track that an event occurred (e.g., nonce advancement)

### Increment

Increments a counter flowstate.

```json
{
  "flowstate": {
    "increment": ["counter_name:{variable}"],
    "ttl_seconds": 3600
  }
}
```

**Use case**: Count occurrences (e.g., transfer count, interaction count)

### Unset

Removes a flowstate.

```json
{
  "flowstate": {
    "unset": ["flag_name:{variable}"]
  }
}
```

**Use case**: Clear a flag after resolution

## Checking FlowState in Conditions

### Check if Set

```json
{
  "field": "flowstate:name",
  "operator": "isset"
}
```

### Check if Not Set

```json
{
  "field": "flowstate:name",
  "operator": "isnotset"
}
```

### Check Counter Value

```json
{
  "field": "flowstate:counter_name",
  "operator": "greater_than",
  "value": 5
}
```

### Check Within Time Window

```json
{
  "field": "flowstate:name",
  "operator": "isset_within",
  "value": 300  // seconds
}
```

## Discovering Available Fields

To find available fields for interpolation:

1. **Check analyzer documentation**: See `docs/analyzers/` for field lists
2. **Use rule validation**: Invalid field references are logged
3. **Check analyzer source**: Look at `fields()` method in analyzer code

**Example**: To find all fields from `SystemProgramAnalyzer`:
```rust
// In analyzers/core/system_program.rs
fn fields(&self) -> Vec<String> {
    vec![
        "has_sol_transfer",
        "sol_recipients",  // Array field - use [0], [1], etc.
        "max_sol_transfer",
        "nonce_account",
        // ... etc
    ]
}
```

## Error Handling

### Invalid Variable Format

If variable doesn't use `analyzer:field_name` format:
```
WARN: Invalid variable format 'foo' - use 'analyzer:field_name' or 'analyzer:field_name[index]'
```
**Result**: Flowbit operation skipped

### Field Not Found

If the mapped field doesn't exist:
```
WARN: Field system:sol_recipients not found for variable {recipient}
```
**Result**: Flowbit operation skipped

### Empty Array

If the array field is empty:
```
// No warning - expected behavior
```
**Result**: Flowbit operation skipped

### Unsupported Type

If the field value cannot be converted to string:
```
WARN: Field analyzer:field has unsupported type for interpolation
```
**Result**: Flowbit operation skipped

**Note**: Strings, numbers, and booleans are automatically converted to strings. Complex types (objects, null) are not supported.

## Performance Considerations

### Interpolation Overhead

- **Regex matching**: ~0.1ms per template
- **Field lookup**: ~0.05ms per variable
- **String replacement**: ~0.05ms per variable
- **Total**: <0.5ms per flowstate name

### Memory Impact

Each unique interpolated flowstate name consumes:
- Per-wallet: ~100 bytes
- Global: ~100 bytes

**Example**: Tracking 1000 unique recipients = ~100KB memory

### Best Practices

1. **Limit Variable Usage**: Only use variables when necessary
2. **Set Reasonable TTLs**: Avoid accumulating stale flowstate
3. **Monitor Unique Keys**: Track number of unique flowstate names created
4. **Use Allowlists**: Filter out known-good values before incrementing

## Debugging

### Enable Debug Logging

```bash
RUST_LOG=sol_shield_core::rules::engine=debug parapet-proxy
```

### Check Interpolation

Look for log messages:
```
DEBUG: Interpolated flowstate name: transfers_to:7xK...9mP
WARN: Unknown variable in flowstate template: {invalid}
WARN: Field not found for variable {recipient}
```

### Inspect Flowbit State

Add a debug endpoint to your proxy:
```rust
// GET /debug/flowstate/{wallet}
// Returns current flowstate state for wallet
```

## Examples

See `parapet/proxy/rules/presets/` for complete examples:
- `ai-agent-advanced.json` - Per-wallet variable interpolation
- `enterprise-cross-wallet.json` - Global variable interpolation

## Advanced Features

### Multiple Array Elements

Track multiple recipients/mints/programs:
```json
{
  "flowstate": {
    "increment": [
      "transfers_to:{system:sol_recipients[0]}",
      "transfers_to:{system:sol_recipients[1]}"
    ]
  }
}
```

**Note**: Each array index creates a separate flowstate. Empty array slots are skipped.

### Numeric Field Interpolation

Use numeric fields in flowstate names:
```json
{
  "flowstate": {
    "increment": ["tx_size:{basic:instruction_count}"]
  }
}
```

**Result**: Creates flowstate like `tx_size:5`, `tx_size:10`, etc.

### Boolean Field Interpolation

Use boolean fields in flowstate names:
```json
{
  "flowstate": {
    "increment": ["multisig:{basic:is_multisig}"]
  }
}
```

**Result**: Creates flowstate like `multisig:true`, `multisig:false`

## Future Enhancements

### Array Wildcard (`[*]`)

Track all elements in an array (planned):
```json
{
  "flowstate": {
    "increment": ["transfers_to:{system:sol_recipients[*]}"]
  }
}
```

**Behavior**: Would create flowstate for each recipient in transaction

### Nested Field Access

Support nested object fields (planned):
```json
{
  "flowstate": {
    "increment": ["event:{simulation:metadata.event_type}"]
  }
}
```
