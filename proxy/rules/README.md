# Rules in this directory

**Production rule packs often live elsewhere** (object store, private repo, image layer). Point the proxy at them with **`RULES_PATH`** (or your deploy’s config).

This tree ships a **minimal** set of JSON bundles:

| Path | Purpose |
| --- | --- |
| **`presets/default-protection.json`** | Default when no `RULES_PATH` is set (see proxy `server.rs`). |
| **`presets/bot-essentials.json`** | Scanner / MCP / API fallbacks when resolving bundled rules. |
| **`presets/wallet-scan-enhanced.json`** | Same (wallet-oriented preset). |

**Additional sample presets, `examples/`, and `policies/` fragments** used by tests and docs live under **`proxy/tests/fixtures/rules/`** (same JSON files, moved — not duplicated).

| Topic | Notes |
| --- | --- |
| *(fingerprints)* | **Canonical** authority-change data is **`authority-change.json`** next to the **`InstructionDataAnalyzer`** source in `parapet-core` (embedded). Optional **override**: `…/fingerprints/authority-change.json` beside your rules bundle. Not rule JSON — **analyzer data**. |

If you ship your own rules, version and review them like application config.
