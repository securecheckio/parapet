# Parapet — Project design record (stakeholder summary)

**Project:** Parapet (open-source Solana transaction security)  
**Audience:** Investors, advisors, core team, contributors  
**Document type:** Living record of **design intent**, **architecture**, and **repository alignment** (not a sprint or hackathon artifact).

**Related:** [Architecture diagrams](architecture.md)

---

## Executive summary

Parapet is documented here as a full-stack, Rust-first security platform: a shared detection engine, an RPC perimeter proxy (IDS/IPS-style), wallet scanning, headless APIs for automation and escalations, a local MCP server, and an optional multi-tenant platform layer. The design is **systematic**—risk taxonomy, rule format documentation, flowbits, optional ecosystem analyzers, and deployment paths—so the product story is a **coherent security program** with clear perimeter control and a path to monetize **premium rules and threat intelligence** (not metered public API pricing on the open-source core).

---

## What the system comprises


| Area                 | Deliverable                                                                                                                            |
| -------------------- | -------------------------------------------------------------------------------------------------------------------------------------- |
| **Core engine**      | `parapet-core`: JSON rules, pluggable analyzers, flowbits, risk scoring                                                                |
| **Perimeter**        | `parapet-proxy`: JSON-RPC proxy, rules, thresholds, optional Redis, deployment examples                                                |
| **Scanner**          | `parapet-scanner`: CLI wallet analysis; output **human** / **json** / **brief**; optional Rugcheck, Jupiter, Helius, OtterSec features |
| **Automation API**   | `parapet-api-core`: MCP HTTP, dynamic rules (Redis), **escalations** (approve/reject), WebSocket events, API keys                      |
| **Platform**         | `parapet-platform`: Extends core API with PostgreSQL, dashboard, payments, analytics, push, learning                                   |
| **Developer UX**     | `parapet-mcp`: stdio MCP for Cursor/Claude-style workflows                                                                             |
| **Quality & perf**   | `rpc-perf`, `flowbits-perf`; workspace tests and coverage discipline (`docs/TEST_COVERAGE.md`)                                         |
| **Risk methodology** | `tools/risk-register`: risk categories CSV + README; alignment with rules repo for mappings                                            |


---

## Strategic decisions (why this architecture)

1. **Parapet = perimeter** — The RPC proxy is positioned as **IDS/IPS for Solana traffic**: the control point before transactions hit the network or critical paths.
2. **Dual deployment** — Same stack supports **client-side** (wallets, agents, trading) and **server-side** (gateway / program-adjacent) deployments—one engine, expanded addressable surface.
3. **Snort-style economics** — Open engine + proxy + scanner **drive adoption**; revenue targets **custom/premium rules and intel**, consistent with the main README’s rules licensing and separate `parapet-rules` repo positioning.
4. **Risk register** — Threat categories and analyzer alignment are first-class so we can state **what is covered and what is not**, and prioritize the roadmap honestly.
5. **Human-in-the-loop** — Escalations in `api-core` address **borderline** decisions and reduce false-positive pain without abandoning automation.
6. **Ecosystem integrations** — Optional Helius, Jupiter, Rugcheck, OtterSec analyzers show we **compose** with existing intel instead of reinventing it.

---

## Scope and maturity indicators

- **Workspace scope:** Core + proxy + scanner + api-core + api-platform + MCP + perf tools + risk-register tooling (see root `Cargo.toml` members).
- **Documentation surface:** User, developer, operations, rules format, flowbits, test coverage—suitable for onboarding operators and contributors.
- **End-to-end design:** From **local MCP** → **headless API** → **perimeter proxy** → **optional SaaS platform**.

---

## Roadmap (illustrative)

- Deepen **rule evaluation metrics** (labeled corpora, regression dashboards) where product and compliance need them.
- Expand **reporting** integrations for regulated users (building on JSON exports and platform analytics).
- Continue **risk-register ↔ rules** coverage tracking with the `parapet-rules` pipeline.

---

*This documentation describes the Parapet security platform.*

---

## Repository alignment (accuracy check)

Verified against the codebase and docs:

- **Workspace members** include `core`, `proxy`, `scanner`, `api-core`, `api-platform`, `mcp`, `tools/rpc-perf`, `tools/flowbits-perf`, `reference/gateway` (see workspace `Cargo.toml`).
- **Scanner CLI** documents output formats **human**, **json**, and **brief** (not PDF/CSV in-tree); JSON supports automation and downstream reporting pipelines.
- **api-core** documents **escalation** endpoints and **WebSocket** for escalation events.
- **Flowbits** and **rule format** are documented under `docs/` (`RULES_FLOWBITS.md`, `RULES_FORMAT.md`).
- **Third-party analyzers** (Helius, Jupiter, Rugcheck, OtterSec) are implemented as optional features in `parapet-core` and referenced in scanner/MCP docs.
- **Risk register** tooling lives under `tools/risk-register/` (categories CSV + README); detailed rule-to-risk mappings reference the external `parapet-rules` repository.

Aspirational items in the engineering brief (e.g. full precision/recall dashboards for every rule) should be tracked as roadmap items where not yet automated in CI.