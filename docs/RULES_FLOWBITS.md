# Parapet rules — flowbits

Stateful counters and flags across transactions, `**flowbits**` blocks on rules, environment variables, and **variable interpolation** in flowbit names. JSON shape for rules and conditions is in [RULES_FORMAT.md](RULES_FORMAT.md).

## Flowbits overview

Flowbits track per-wallet or global counters and flags across transactions. See the hub [RULES_DEVELOPMENT.md](RULES_DEVELOPMENT.md) for loading bundles and operational habits.

**Parapet environment variables** (today’s implementation uses the `**PARAPET_FLOWBITS_*`** prefix in `parapet-core`; treat these as Parapet settings until renamed in code):


| Variable                             | Role                                   |
| ------------------------------------ | -------------------------------------- |
| `PARAPET_FLOWBITS_ENABLED`         | Master switch                          |
| `PARAPET_FLOWBITS_MAX_WALLETS`     | Cap tracked wallets                    |
| `PARAPET_FLOWBITS_DEFAULT_TTL`     | Default seconds for flowbit expiry     |
| `PARAPET_FLOWBITS_MAX_GLOBAL_KEYS` | Cap global keys for cross-wallet rules |


Start with `action: alert`, tune from traffic, then tighten to `block`.

## Flowbits variable interpolation

Interpolation lets flowbit **names** include values from the current transaction (dynamic keys per recipient, mint, program, etc.).

### Syntax

Use `{analyzer:field_name}` to reference analyzer fields:

```json
{
  "flowbits": {
    "increment": ["transfers_to:{system:sol_recipients[0]}"]
  }
}
```

- `{analyzer:field_name}` — scalar value (string, number, boolean)
- `{analyzer:field_name[0]}` — first element of an array field
- `{analyzer:field_name[N]}` — Nth element

```json
{
  "flowbits": {
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

### Array handling

1. If a variable maps to an array field, default behavior uses the first element `[0]`.
2. **Empty arrays:** the flowbit operation is skipped (no increment/set).
3. **Multiple elements:** only the first index is used unless you list multiple templates (e.g. `[0]` and `[1]`); a future wildcard `[*]` may track all elements.

Example: `system:sol_recipients` empty → skip; two recipients → only `sol_recipients[0]` participates unless you add another increment line for `[1]`.

### Scope: per-wallet vs global

**Per-wallet** (`"scope": "perwallet"`): separate counters per fee payer.

**Global** (`"scope": "global"`): one counter namespace across all wallets.

### Flowbit operations

**Set** — boolean flag for a TTL:

```json
{
  "flowbits": {
    "set": ["flag_name:{variable}"],
    "ttl_seconds": 3600
  }
}
```

**Increment** — counter:

```json
{
  "flowbits": {
    "increment": ["counter_name:{variable}"],
    "ttl_seconds": 3600
  }
}
```

**Unset** — remove a flag:

```json
{
  "flowbits": {
    "unset": ["flag_name:{variable}"]
  }
}
```

Conditions that **test** flowbit state should use the **flowbit condition** shape (`{ "flowbit": "...", ... }`) with the **fully resolved** flowbit name (same string that would be produced after interpolation when incrementing). See the flowbit condition variant under **Conditions** in [RULES_FORMAT.md](RULES_FORMAT.md).

### Illustrative patterns

**Recipient transfer counting (per-wallet):**

```json
{
  "id": "track-recipient-transfers",
  "rule": {
    "action": "block",
    "conditions": {
      "all": [
        {"field": "system:has_sol_transfer", "operator": "equals", "value": true},
        {"field": "flowbit:transfers_to:{system:sol_recipients[0]}", "operator": "greater_than", "value": 3}
      ]
    },
    "flowbits": {
      "scope": "perwallet",
      "increment": ["transfers_to:{system:sol_recipients[0]}"],
      "ttl_seconds": 86400
    },
    "message": "Repeated transfers to same recipient (4+ in 24h)"
  }
}
```

**Token velocity (global):**

```json
{
  "id": "track-token-drain-velocity",
  "rule": {
    "action": "block",
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
    "message": "Unusual token velocity (10+ transfers in 15 min)"
  }
}
```

**Lateral movement (global recipient):**

```json
{
  "id": "track-lateral-movement",
  "rule": {
    "action": "block",
    "conditions": {
      "all": [
        {"field": "system:has_sol_transfer", "operator": "equals", "value": true},
        {"field": "flowbit_global:suspicious_recipient:{system:sol_recipients[0]}", "operator": "greater_than", "value": 2}
      ]
    },
    "flowbits": {
      "scope": "global",
      "increment": ["suspicious_recipient:{system:sol_recipients[0]}"],
      "ttl_seconds": 3600
    },
    "message": "Lateral movement: recipient received from 3+ wallets in 1h"
  }
}
```

**Nonce advancement + stale transfer (two-rule pattern):**

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
      "flowbits": {
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
          {
            "not": {
              "flowbit": "nonce_advanced:<same_resolved_name_as_set_rule>"
            }
          }
        ]
      },
      "message": "Stale nonce detected (>30 min old) on high-value transfer"
    }
  }
]
```

Use the **same** resolved flowbit name string the `set` step uses (after interpolation). If your deployment compares flowbit counters via `field` + `operator` instead, that path must match how `parapet-core` merges flowbit values into the evaluation map for your version.

**Unknown program interaction:**

```json
{
  "id": "track-unknown-program",
  "rule": {
    "action": "alert",
    "conditions": {
      "all": [
        {"field": "basic:program_ids", "operator": "not_in", "value": ["<known_programs>"]},
        {"field": "flowbit:program_interaction:{basic:program_ids[0]}", "operator": "greater_than", "value": 2}
      ]
    },
    "flowbits": {
      "scope": "perwallet",
      "increment": ["program_interaction:{basic:program_ids[0]}"],
      "ttl_seconds": 604800
    },
    "message": "Repeated interaction with unknown program"
  }
}
```

**Generic field patterns:**

```json
{
  "flowbits": {
    "increment": [
      "signer_activity:{basic:signers[0]}",
      "high_value_transfers:{system:max_sol_transfer}",
      "complex_tx:{basic:instruction_count}",
      "inner_calls:{inner_instruction:cpi_depth}"
    ]
  }
}
```

Numeric and boolean values are stringified for flowbit names.

### Discovering fields for templates

1. Analyzer docs under `docs/analyzers/`
2. Validation / logs for unknown fields
3. `fields()` in the analyzer’s Rust source under `parapet-core`

### Interpolation errors and behavior


| Situation                                                | Result                   |
| -------------------------------------------------------- | ------------------------ |
| Invalid template (not `analyzer:field` or indexed array) | Warn; flowbit op skipped |
| Field missing for this transaction                       | Warn; op skipped         |
| Empty array for indexed access                           | Op skipped (no warn)     |
| Unsupported type for string conversion                   | Warn; op skipped         |


Strings, numbers, and booleans interpolate; complex objects / null do not.

### Performance and memory

Rough overhead: sub-millisecond per interpolated name. Each distinct interpolated name costs on the order of ~100 bytes (per-wallet or global). Cap TTLs and key cardinality via env limits and allowlists.

### Debugging

- `RUST_LOG=parapet_core::rules::engine=debug` on the binary you run.
- Look for lines about interpolated names, unknown variables, or missing fields.

### Advanced

Multiple indices in one rule:

```json
{
  "flowbits": {
    "increment": [
      "transfers_to:{system:sol_recipients[0]}",
      "transfers_to:{system:sol_recipients[1]}"
    ]
  }
}
```

Numeric/boolean fields in names (e.g. `tx_size:{basic:instruction_count}`, `multisig:{basic:is_multisig}`) produce discrete keys per value.

### Planned behavior (not guaranteed shipped)

- Array wildcard `[*]` to fan out to all elements
- Nested paths such as `{simulation:metadata.event_type}`

