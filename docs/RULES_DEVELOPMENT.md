# Rule development

Parapet loads **rule bundles**: JSON that describes when to `pass`, `alert`, or `block` a transaction using **conditions** over analyzer fields, optional **flowstate**, and optional third-party signals. This page is the **hub**; details live in focused docs (no duplicate long copies here).

| Topic | Document |
| --- | --- |
| Bundle structure, `RuleDefinition`, conditions, operators, analyzers | [RULES_FORMAT.md](RULES_FORMAT.md) |
| FlowState env, `flowstate` block, variable interpolation, examples | [RULES_FLOWSTATE.md](RULES_FLOWSTATE.md) |

## Loading bundles

Configure the Parapet proxy or scanner with your bundle via `RULES_PATH` or the process’s `--rules` flag. Paths are deployment-specific; version bundles like any other policy artifact.

## Operational habits

- Prefer incremental rollout: alert first, then block.
- Document threshold changes and allowlists with your bundle version.
- When a rule does not fire as expected, confirm required analyzers are registered, field names match `analyzer:field` conventions (see [RULES_FORMAT.md](RULES_FORMAT.md)), and flowstate are enabled if the rule depends on them ([RULES_FLOWSTATE.md](RULES_FLOWSTATE.md)).

## Program Vulnerability Rules

Use `program_analysis` fields to detect bytecode-level vulnerability patterns without hardcoding assessment in analyzer code:

- `program_analysis:missing_signer_check`
- `program_analysis:missing_owner_check`
- `program_analysis:arbitrary_cpi`
- `program_analysis:is_in_blocklist`
- `program_analysis:spl_token_related`
- `program_analysis:token_2022_related`

Reference bundle: `rpc-proxy/rules/policies/program-vulnerabilities.json`
