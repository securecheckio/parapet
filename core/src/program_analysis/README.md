# Program Analysis

Three-tier analysis system for Solana programs, providing graduated depth of inspection based on risk tolerance and performance requirements.

## Analysis Tiers

### Tier 1: Superficial (10-50ms)
Fast reputation checks using external databases:
- Known-safe program lists
- Helius verification API
- OtterSec audit database
- Local cache lookups

**Use case:** High-throughput RPC proxies, bot protection

### Tier 2: Deep (200-1000ms)
Static analysis of program bytecode:
- Program bytecode fetching from Solana
- sBPF disassembly
- Semantic analysis (CPI patterns, account usage, instruction complexity)
- Signature extraction

**Use case:** Pre-trade analysis, wallet scanners, dApp security

### Tier 3: AI (2-10s)
LLM-powered semantic analysis:
- Natural language risk description
- Intent detection (mint, burn, transfer, upgrade)
- Vulnerability pattern matching
- Custom rule generation suggestions

**Use case:** In-depth audits, escalation investigation, threat research

## Components

### ProgramFetcher
Fetches deployed program bytecode from Solana RPC.

```rust
let fetcher = ProgramFetcher::new(rpc_url);
let bytecode = fetcher.fetch_program(&program_id).await?;
```

### ProgramDisassembler
Disassembles sBPF bytecode to human-readable assembly.

```rust
let disassembler = ProgramDisassembler::new()?;
let asm = disassembler.disassemble(&bytecode)?;
```

### SemanticAnalyzer
Extracts security-relevant patterns from disassembled code:
- CPI call counts and targets
- Account mutation patterns
- Signer requirement checks
- Instruction complexity scoring

```rust
let semantic = SemanticAnalyzer::new();
let analysis = semantic.analyze(&disassembly)?;
```

### AiAnalyzer (optional, feature-gated)
LLM analysis for high-risk programs:
- GPT-4, Claude, or local models
- Prompt engineering for security focus
- Structured output with risk scores

```rust
let ai = AiAnalyzer::new(AiProviderConfig::OpenAI { api_key });
let report = ai.analyze(&disassembly, &semantic_analysis).await?;
```

### ProgramCache
Redis-backed caching to avoid redundant analysis:
- TTL-based expiration
- Automatic invalidation on program upgrade
- Compressed storage for bytecode and results

## Usage

```rust
use parapet_core::program_analysis::{
    ProgramAnalysisService, AnalysisTier, AnalysisMode
};

// Create service
let service = ProgramAnalysisService::new(rpc_url);

// Superficial check (fast)
let result = service.analyze_program(
    &program_id,
    AnalysisTier::Superficial,
    AnalysisMode::Synchronous
).await?;

// Deep analysis (slower)
let result = service.analyze_program(
    &program_id,
    AnalysisTier::Deep,
    AnalysisMode::Synchronous
).await?;

// AI analysis (slowest, most detailed)
let result = service.analyze_program(
    &program_id,
    AnalysisTier::AI,
    AnalysisMode::Asynchronous  // Don't block RPC calls
).await?;
```

## Performance Characteristics

| Tier         | Latency      | Accuracy | Cache Hit Rate | Cost         |
|--------------|--------------|----------|----------------|--------------|
| Superficial  | 10-50ms      | 70%      | ~95%           | Free         |
| Deep         | 200-1000ms   | 85%      | ~80%           | RPC calls    |
| AI           | 2-10s        | 95%      | ~60%           | LLM API cost |

## Configuration

```bash
# RPC endpoint for program fetching
SOLANA_RPC_URL=https://api.mainnet-beta.solana.com

# Redis cache (optional but recommended)
REDIS_URL=redis://localhost:6379

# AI provider (optional, for Tier 3 analysis)
# Option 1: Generic AI config (recommended)
AI_API_KEY=sk-...
AI_BASE_URL=https://api.openai.com/v1
AI_MODEL=gpt-4
AI_MAX_TOKENS=4000
AI_TEMPERATURE=0.1

# Option 2: Provider-specific env vars (legacy)
OPENAI_API_KEY=sk-...
ANTHROPIC_API_KEY=sk-ant-...
GROQ_API_KEY=gsk-...

# Cache TTL (seconds)
PROGRAM_CACHE_TTL=86400  # 24 hours
```

### TOML Configuration

AI analyzer settings can also be configured via TOML file (e.g., `scanner/config.toml`):

```toml
[ai]
api_key = "sk-..."
base_url = "https://api.openai.com/v1"
model = "gpt-4"
max_tokens = 4000
temperature = 0.1
```

Environment variables override TOML settings when present.

## Integration with Rules Engine

Program analysis results are now exposed to rules via the `program_analysis` analyzer (raw evidence only):

```json
{
  "id": "block-missing-signer-check",
  "condition": {
    "all": [
      "program_analysis:missing_signer_check == true",
      "program_analysis:has_account_write == true"
    ]
  },
  "action": "block"
}
```

Key exposed fields:
- `program_analysis:missing_signer_check`
- `program_analysis:missing_owner_check`
- `program_analysis:arbitrary_cpi`
- `program_analysis:has_account_write`
- `program_analysis:has_cpi_call`
- `program_analysis:reads_account_data`
- `program_analysis:bytecode_hashes`
- `program_analysis:is_in_blocklist`
- `program_analysis:spl_token_related`
- `program_analysis:token_2022_related`

Hash blocklists can be provided locally and via remote feed polling:
- `blocked_hashes` in `rpc-proxy/config.toml`
- `blocked_program_feeds` with `feed_poll_interval_secs`

## Future Enhancements

- Worker queue for async analysis
- Distributed caching for multi-proxy deployments
- Custom AI prompt templates
- Community-sourced program reputation database
