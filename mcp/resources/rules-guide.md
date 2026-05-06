# Parapet custom rules (concise)

Ephemeral rules go in the **`custom_rules`** argument on `scan_wallet` and `check_transaction` as a JSON array of rule objects. Always call **`list_analyzers`** first to see which `namespace:field` keys exist on **this** deployment.

---

## Quick start

```json
{
  "version": "1.0",
  "id": "example-alert-large-sol",
  "name": "Alert large SOL move",
  "enabled": true,
  "rule": {
    "action": "alert",
    "conditions": {
      "field": "system:max_sol_transfer",
      "operator": "greater_than",
      "value": 1000000000
    },
    "message": "SOL transfer above threshold"
  }
}
```

---

## Rule object (`RuleDefinition`)

| Field | Required | Notes |
|-------|----------|--------|
| `version` | yes | e.g. `"1.0"` |
| `id` | yes | Stable string |
| `name` | yes | Display name |
| `enabled` | yes | `false` skips rule |
| `rule` | yes | `action`, `conditions`, `message` |
| `description`, `author`, `tags`, `metadata` | no | `metadata.weight` etc. |

Inside **`rule`**: `action` is `"pass"` \| `"alert"` \| `"block"`; **`conditions`** is a tree; **`message`** is shown when matched.

---

## Conditions (three shapes)

**Simple** — compare one field:

```json
{ "field": "system:max_sol_transfer", "operator": "greater_than", "value": 1000000000 }
```

**Compound** — AND / OR / NOT:

```json
{ "all": [ { "field": "system:has_sol_transfer", "operator": "equals", "value": true }, { "field": "system:max_sol_transfer", "operator": "greater_than", "value": 500000000 } ] }
```

**FlowState** — requires `PARAPET_FLOWSTATE_ENABLED` for full effect:

```json
{ "flowstate": "my_counter_name", "count_operator": "greater_than", "count_value": 3 }
```

See deploy docs for flowstate naming and interpolation.

---

## Operators

| Operators | Value type |
|-----------|------------|
| `equals`, `not_equals` | any JSON |
| `greater_than`, `less_than`, `greater_than_or_equal`, `less_than_or_equal` | number (or numeric string) |
| `in`, `not_in` | JSON array |
| `contains` | string or array |
| `isnotset`, `exists` | per engine |

---

## Field discovery

1. Call **`list_analyzers`** → markdown list of `namespace` and fields.  
2. In rules use **`namespace:field`** (recommended).

Third-party analyzers (rugcheck, helius, jupiter, ottersec, …) appear only when enabled (features + API keys). **`list_analyzers`** is authoritative.

---

## Templates (copy-paste)

### 1. Block large SOL transfer

```json
{
  "version": "1.0",
  "id": "block-large-sol",
  "name": "Block large SOL",
  "enabled": true,
  "rule": {
    "action": "block",
    "conditions": {
      "field": "system:max_sol_transfer",
      "operator": "greater_than",
      "value": 10000000000
    },
    "message": "SOL transfer exceeds limit"
  },
  "metadata": { "weight": 80 }
}
```

### 2. Alert unlimited token approvals

```json
{
  "version": "1.0",
  "id": "alert-unlimited-approve",
  "name": "Unlimited approve",
  "enabled": true,
  "rule": {
    "action": "alert",
    "conditions": {
      "field": "token_instructions:unlimited_approve_count",
      "operator": "greater_than",
      "value": 0
    },
    "message": "Unlimited delegation present"
  }
}
```

### 3. Compound — SOL transfer AND large amount

```json
{
  "version": "1.0",
  "id": "compound-sol",
  "name": "SOL transfer and size",
  "enabled": true,
  "rule": {
    "action": "alert",
    "conditions": {
      "all": [
        {
          "field": "system:has_sol_transfer",
          "operator": "equals",
          "value": true
        },
        {
          "field": "system:max_sol_transfer",
          "operator": "greater_than",
          "value": 500000000
        }
      ]
    },
    "message": "SOL transfer over 0.5 SOL"
  }
}
```

### 4. FlowState-style condition (counters)

Use only if flowstate is enabled in your environment.

```json
{
  "version": "1.0",
  "id": "flow-repeat",
  "name": "Repeated activity",
  "enabled": true,
  "rule": {
    "action": "alert",
    "conditions": {
      "flowstate": "example_flow_key",
      "count_operator": "greater_than",
      "count_value": 2
    },
    "message": "Flowstate threshold exceeded"
  }
}
```

### 5. Third-party example — Rugcheck (`rugcheck:is_rugged`)

**Only if** analyzer `rugcheck` appears in **`list_analyzers`**:

```json
{
  "version": "1.0",
  "id": "rugcheck-example",
  "name": "Rugcheck rugged flag",
  "enabled": true,
  "rule": {
    "action": "alert",
    "conditions": {
      "field": "rugcheck:is_rugged",
      "operator": "equals",
      "value": true
    },
    "message": "Rugcheck flagged token activity"
  }
}
```

---

## Validation failures

- Unknown **`field`** → not returned by **`list_analyzers`** for this server.  
- **Operator / value** mismatch → e.g. `greater_than` needs a number.  
- Malformed JSON → fix structure to match tables above.

---

## Related resources

- **`parapet://guide`** — overview; links custom rules workflow.  
- **`parapet://examples`** — short workflow: list_analyzers → this guide → tools.
