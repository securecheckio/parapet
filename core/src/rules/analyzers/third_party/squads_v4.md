# SquadsV4Analyzer

Analyzes Squads Protocol v4 multisig transactions to detect governance changes and security concerns.

**Feature Flag:** None (always enabled)

## Risks Detected

- **Governance attacks** - Multiple governance changes in single transaction
- **Threshold manipulation** - Lowering multisig approval threshold
- **Member removal** - Removing signers from multisig
- **Spending limit removal** - Disabling spending controls
- **Unauthorized execution** - Transactions executed without visible proposal approval

## Fields

### Detection
- `squads_v4:is_squads_transaction` - Transaction interacts with Squads v4
- `squads_v4:squads_instruction_count` - Number of Squads instructions

### Instruction Types
- `squads_v4:has_multisig_create` - Creating new multisig
- `squads_v4:has_proposal_create` - Creating proposal
- `squads_v4:has_proposal_approve` - Approving proposal
- `squads_v4:has_proposal_reject` - Rejecting proposal
- `squads_v4:has_vault_transaction_execute` - Executing vault transaction
- `squads_v4:has_config_transaction_execute` - Executing config change

### Governance Operations
- `squads_v4:has_governance_change` - Any governance modification
- `squads_v4:has_member_add` - Adding multisig member
- `squads_v4:has_member_remove` - Removing multisig member
- `squads_v4:has_threshold_change` - Changing approval threshold
- `squads_v4:has_spending_limit_remove` - Removing spending limits

### Security Analysis
- `squads_v4:security_concerns` - Array of detected concerns
- `squads_v4:has_security_concerns` - Any concerns detected
- `squads_v4:concern_count` - Number of security concerns
- `squads_v4:governance_change_count` - Count of governance changes

## Performance

**Latency:** <1ms (on-chain instruction parsing, no API calls)

## Use Cases

- Detect multisig governance attacks
- Alert on threshold manipulation
- Monitor member removal attempts
- Track spending limit changes
- Validate proposal approval flow

## Example Rules

**Alert on Governance Changes:**
```json
{
  "action": "alert",
  "conditions": {
    "field": "squads_v4:has_governance_change",
    "operator": "equals",
    "value": true
  },
  "message": "Squads multisig governance change detected"
}
```

**Block Multiple Governance Changes:**
```json
{
  "action": "block",
  "conditions": {
    "field": "squads_v4:governance_change_count",
    "operator": "greater_than",
    "value": 1
  },
  "message": "Multiple governance changes in single transaction"
}
```

**Alert on Threshold Changes:**
```json
{
  "action": "alert",
  "conditions": {
    "field": "squads_v4:has_threshold_change",
    "operator": "equals",
    "value": true
  },
  "message": "Multisig approval threshold being modified"
}
```

**Block Unauthorized Execution:**
```json
{
  "action": "block",
  "conditions": {
    "all": [
      {"field": "squads_v4:has_security_concerns", "operator": "equals", "value": true},
      {"field": "squads_v4:concern_count", "operator": "greater_than", "value": 0}
    ]
  },
  "message": "Security concerns detected in Squads transaction"
}
```

## Security Concerns Detected

- `multiple_governance_changes` - More than 2 governance operations
- `threshold_change_detected` - Approval threshold being modified
- `member_removal_detected` - Signer being removed
- `spending_limit_removal` - Spending controls being disabled
- `execution_without_visible_proposal` - Transaction executed without proposal

## Program Details

- **Program ID:** `SQDS4ep65T869zMMBKyuUq6aD6EgTu8psMjkvj52pCf`
- **Protocol:** Squads Protocol v4 (Multisig wallet)
- **Instruction Format:** Anchor-based (discriminator in first byte)

## Testing

Unit tests: `core/src/rules/analyzers/third_party/squads_v4.rs`
