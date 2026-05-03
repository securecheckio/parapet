# Parapet rules — JSON format

Structure of rule bundles, condition trees, operators, and how **analyzers** expose fields. For **flowstate** (state + interpolation), see [Rule development — hub](RULES_DEVELOPMENT.md) → [RULES_FLOWSTATE.md](RULES_FLOWSTATE.md).

## Rule bundle file

The engine accepts either:

- A **JSON array** of rule objects: `[ { ... }, { ... } ]`, or  
- A **single** rule object: `{ ... }` (treated as one-element logic depending on loader).

Each element is one `**RuleDefinition`** (see below).

## `RuleDefinition` (top-level JSON object)


| Field         | Required | Meaning                                                                         |
| ------------- | -------- | ------------------------------------------------------------------------------- |
| `version`     | yes      | Schema / bundle version string (e.g. `"1.0"`).                                  |
| `id`          | yes      | Stable id for this rule.                                                        |
| `name`        | yes      | Human-readable name.                                                            |
| `description` | no       | Longer text.                                                                    |
| `author`      | no       | Who wrote the rule.                                                             |
| `enabled`     | yes      | If `false`, the rule is skipped.                                                |
| `tags`        | no       | String array (e.g. categorization).                                             |
| `rule`        | yes      | The `**Rule`** payload (action, conditions, message, optional flowstate).        |
| `metadata`    | no       | Arbitrary JSON object (e.g. `weight`, `missing_field_behavior`, network hints). |


## `rule` object


| Field        | Required | Meaning                                                                     |
| ------------ | -------- | --------------------------------------------------------------------------- |
| `action`     | yes      | `"block"`, `"alert"`, or `"pass"` (lowercase).                              |
| `conditions` | yes      | A **condition tree** (see below).                                           |
| `message`    | yes      | Message attached when the rule matches.                                     |
| `flowstate`   | no       | Optional `set` / `unset` / `increment` actions and `scope` / `ttl_seconds`. |


## Conditions (`conditions`)

Conditions are serialized as an **untagged** enum: the JSON shape determines the variant.

### 1. Simple condition (analyzer field comparison)

An object with `**field`**, `**operator`**, and `**value**`:

```json
{
  "field": "system:has_sol_transfer",
  "operator": "equals",
  "value": true
}
```

- `**field**`: string referencing a field the engine can resolve (see [Analyzers and field names](#analyzers-and-field-names)).
- `**operator**`: snake-case string; see [Comparison operators](#comparison-operators).
- `**value**`: any JSON value appropriate to the operator (number, string, boolean, array for `in` / `not_in`, etc.).

### 2. Flowstate condition

An object with a `**flowstate**` string (the flowstate **name**, after any interpolation used when the flowstate was set/incremented), plus optional modifiers:

- **Presence / expiry**: omit `count_operator` and `within_seconds` → engine checks whether the flowstate is set (and not expired) for the current wallet.
- **Time window**: `"within_seconds": <seconds>` → true if the flowstate was set within that window.
- **Counter**: `"count_operator": "<op>"`, `"count_value": <n>` → compares the counter using the same operator names as simple conditions (`equals`, `greater_than`, …).

```json
{
  "flowstate": "transfers_to:7xKXtg2C...",
  "count_operator": "greater_than",
  "count_value": 3
}
```

Use this when the condition depends **only** on Parapet flowstate state, not on a raw analyzer field. Naming and interpolation for flowstate are covered in [RULES_FLOWSTATE.md](RULES_FLOWSTATE.md).

### 3. Compound condition

An object with `**all`**, `**any`**, and/or `**not**`:

```json
{
  "all": [
    { "field": "system:has_sol_transfer", "operator": "equals", "value": true },
    { "field": "system:max_sol_transfer", "operator": "greater_than", "value": 1000000 }
  ]
}
```

- `**all**`: every nested condition must be true.  
- `**any**`: at least one nested condition must be true.  
- `**not**`: single nested condition must be false.

Nesting is allowed.

## Comparison operators

Operators are compared as `**snake_case**` in JSON, matching `ComparisonOperator` in `parapet-core`:

`equals`, `not_equals`, `greater_than`, `less_than`, `greater_than_or_equal`, `less_than_or_equal`, `in`, `not_in`, `contains`.

## Analyzers and field names

Parapet’s rule engine is driven by **analyzers**: implementations of `TransactionAnalyzer` in `parapet-core` (`core/src/rules/analyzer.rs` and `core/src/rules/analyzers/`).

Each analyzer:

1. Has a `**name()`** used as a **namespace** (e.g. `system`, `basic`, `token_instructions`).
2. Declares `**fields()`**: the logical field names it produces (e.g. `has_sol_transfer`, `sol_recipients`).
3. Populates a key/value map when analyzing a transaction.

The **registry** merges outputs into a single flat map. Keys are exposed to rules as:

- `**"analyzer:field"`** — canonical form (e.g. `system:has_sol_transfer`, `token_instructions:has_transfer`).
- The same value may also appear under an **unprefixed** duplicate where unambiguous—prefer **prefixed** names in new rules to avoid clashes.

**What to use in `conditions.field`:** reference fields that exist for your deployed analyzer set. Lists and descriptions live under `**docs/analyzers/`** per domain; the authoritative list for a given analyzer is its `fields()` implementation in Rust.

**Lazy evaluation:** only analyzers needed for fields referenced by your enabled rules are run.

## Third-party analyzers

Integration analyzers (external APIs, extra latency, optional features/env) are documented **next to their implementations**, not in this rules guide. Start at the index: `[core/src/rules/analyzers/third_party/README.md](../core/src/rules/analyzers/third_party/README.md)`. Each supported analyzer has a sibling `**.md`** file in that directory (field namespaces, example conditions, API keys, Cargo features, rate limits). In rules, use the same `analyzer:field` convention as for core analyzers.