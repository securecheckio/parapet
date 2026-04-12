# Parapet — Architecture

*Companion to the project design record in `[OVERVIEW.md](OVERVIEW.md)`.*

Mermaid diagrams for stakeholders and engineers. Render in GitHub, Notion, or any Mermaid-capable viewer.

---

## 1. High-level system context

```mermaid
flowchart TB
  subgraph clients [Clients]
    W[Wallet / Bot]
    A[AI Agent]
    D[dApp Backend]
  end

  subgraph parapet [Parapet workspace]
    PXY[parapet-proxy]
    CORE[parapet-core]
    SCAN[parapet-scanner]
    API[parapet-api-core]
    PLAT[parapet-platform]
    MCP[parapet-mcp]
  end

  subgraph external [External]
    RPC[Upstream Solana HTTP RPC]
    TPI[Optional intel: Helius Jupiter Rugcheck OtterSec]
    REDIS[(Redis)]
    PG[(PostgreSQL)]
  end

  W --> PXY
  A --> PXY
  A --> API
  D --> PXY
  SCAN --> CORE
  PXY --> CORE
  API --> CORE
  MCP --> CORE
  PLAT --> API
  PXY --> RPC
  SCAN --> RPC
  API --> REDIS
  PLAT --> PG
  CORE --> TPI
```



---

## 2. Perimeter IDS/IPS (RPC proxy)

```mermaid
flowchart LR
  C[Client] -->|JSON-RPC| PXY[parapet-proxy]
  PXY --> CORE[parapet-core rules and analyzers]
  CORE -->|risk score + decision| PXY
  PXY -->|forward if allowed| UP[Upstream RPC]
  PXY -->|block or alert per policy| C
```



---

## 3. Dual deployment mental model

Same engine and rules; placement of **parapet-proxy** changes the trust boundary (client path vs server/gateway path).

```mermaid
flowchart TB
  subgraph clientSide [Client-side deployment]
    W[Wallet / Trading stack] --> P1[parapet-proxy]
    P1 --> S[Solana network]
  end
  subgraph serverSide [Server-side deployment]
    APP[Backend / Gateway] --> P2[parapet-proxy]
    P2 --> S
  end
```



---

## 4. Transaction decision and escalation

```mermaid
flowchart TD
  T[Transaction or simulate request] --> E[parapet-core evaluation]
  E -->|clear allow| OK[Allow forward]
  E -->|clear block| BL[Block or alert]
  E -->|borderline / policy| ESC[Escalation queue in api-core]
  ESC --> H[Human approve / reject]
  H --> OK
  H --> BL
```



---

## 5. Risk register alignment (conceptual)

```mermaid
flowchart LR
  RR[Risk register categories] --> MAP[Map to analyzers + rules]
  MAP --> COV[Coverage report: addressed vs gap]
  COV --> ROAD[Roadmap and premium rule backlog]
```



---

## 6. Optional third-party enrichment

```mermaid
flowchart LR
  CORE[parapet-core analyzers] --> T1[Helius]
  CORE --> T2[Jupiter]
  CORE --> T3[Rugcheck]
  CORE --> T4[OtterSec]
  T1 --> CORE
  T2 --> CORE
  T3 --> CORE
  T4 --> CORE
```



---

*Diagrams describe architecture; refer to crate READMEs for exact endpoints and configuration.*