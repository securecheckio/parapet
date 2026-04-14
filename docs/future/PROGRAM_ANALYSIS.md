# Program Analysis & Bytecode Detection

## Overview

Add deep program analysis capability to detect malicious programs through disassembly and semantic analysis. Enables blacklist curation through automated threat detection.

## Architecture

### Two New Analyzers

**1. DisassemblyAnalyzer** (Fast, Deterministic)

- Replace basic byte pattern matching with proper solana-sbpf disassembly
- Extract metrics: entropy, complexity, CFG analysis, instruction patterns
- Expose fields: `disasm:entropy`, `disasm:complexity`, `disasm:has_sol_invoke`, etc.
- Performance: 200-500ms (cached forever after first analysis)

**2. AISecurityAnalyzer** (Semantic Analysis)

- Feed disassembly results to LLM
- Detect semantic vulnerabilities: unchecked signers, arbitrary CPI, missing validations
- Expose fields: `ai:has_unchecked_signer`, `ai:exploit_pattern`, `ai:confidence`, etc.
- Performance: 2-10s (cached forever, run once per program)

### Integration with Existing System

**Reuse existing reputation analyzers:**

- `OtterSecVerifiedAnalyzer` - source verification
- `HeliusIdentityAnalyzer` - program recognition
- `RugcheckAnalyzer` - token rugpull detection

**Rules combine all signals:**

```json
{
  "conditions": {
    "all": [
      {"field": "disasm:entropy", "op": "greater_than", "value": 0.9},
      {"field": "ai:has_unchecked_signer", "op": "equals", "value": true},
      {"field": "ottersec:verified", "op": "equals", "value": false}
    ]
  },
  "severity": "critical"
}
```

## Detection Rules

### Pattern-Based Detection

Rules identify malicious programs through combinations of:

**Anti-Analysis Patterns:**

- High entropy (>0.9) = obfuscation
- Excessive complexity = anti-analysis
- Dead code injection = size padding

**Missing Security Checks:**

- Unchecked signer validation
- Missing account owner checks
- No PDA derivation validation
- Missing account type discriminators

**Dangerous Operations:**

- Arbitrary CPI (user-controlled program IDs)
- Transfers without authorization
- Mint/burn without authority checks
- Close account without rent return

**Known Exploit Patterns:**

- Invoke-then-close sequence
- Self-invocation (reentrancy)
- Log suppression with sensitive operations
- Sysvar manipulation attempts

### Two-Tier Detection

**Tier 1: Blacklist (1ms)**

- Simple program ID lookup
- Known malicious programs blocked instantly

**Tier 2: Pattern Analysis (200-500ms)**

- Disassembly + AI semantic analysis
- Rules evaluate indicators
- If malicious → promote to blacklist for future speed

## Security Modes

**Paranoid Mode (Maximum Security)**

- Unknown program → analyze immediately, block transaction until complete
- Willing to wait 2-5s for unknown programs
- Use case: treasuries, high-value wallets

**Balanced Mode (Default)**

- Unknown program + suspicious transaction → analyze
- Unknown program + normal transaction → allow, queue for later
- Use case: typical users, normal DeFi activity

**Permissive Mode (Speed First)**

- Only blacklist blocking
- Unknown programs allowed and logged
- Analysis on explicit request or batch processing
- Use case: trading bots, high-frequency users

## Caching Strategy

**Programs are immutable** - cache analysis results forever:

- First encounter: full analysis (slow)
- All future transactions: cache lookup (1-2ms)
- Pre-warm cache for popular programs at startup
- Redis-backed for shared cache across instances

## Local Blacklist Management

**Simple file-based blacklist:**

```
# blacklist.txt
ABC123... # scam detected 2026-04-13
DEF456... # rugpull pattern
```

**User workflow:**

1. Transaction flagged: "Unknown program XYZ789"
2. Run: `./parapet program scan XYZ789`
3. Review analysis report
4. Add to blacklist: `./parapet blacklist add XYZ789 "reason"`
5. Future transactions blocked instantly

**Composable blacklists:**

- Personal blacklist
- Community shared lists
- SecureCheck official list
- Import/export for sharing

## Future Extensions

### Community Analysis Network

- Users submit signed program analysis
- Consensus emerges from multiple reviews
- Trust-weighted scoring (analyst reputation)
- Economic incentives for accurate analysis
- Decentralized threat intelligence

### Automated Pattern Learning

- ML identifies common patterns in flagged programs
- Generate rule suggestions
- Human review before production deployment
- Continuous improvement of detection

### On-Chain Verification

- Publish analysis merkle roots on-chain
- Cryptographic proof of inclusion
- Dispute resolution mechanism
- Decentralized audit trail

## Implementation Phases

**Phase 1: Foundation (MVP)**

- Implement DisassemblyAnalyzer with solana-sbpf
- Implement AISecurityAnalyzer with structured output
- CLI: `./parapet program scan <ID>`
- Local blacklist file management
- Basic detection rules

**Phase 2: Rule Development**

- Comprehensive rule set for known patterns
- Test against known malicious programs
- Tune thresholds to minimize false positives
- Document rule coverage in risk register

**Phase 3: Integration**

- Wire into scanner for batch analysis
- Add to proxy with availability status field
- MCP tool for AI agent access
- Dashboard for pending analysis queue

**Phase 4: Community (Future)**

- Signed analysis submissions
- Trust scoring and consensus
- Incentive mechanisms
- Decentralized storage options

## Success Metrics

- **Coverage:** % of transactions using analyzed programs
- **Accuracy:** False positive rate on legitimate programs
- **Performance:** 95%+ cache hit rate after warm-up
- **Detection:** New malicious programs identified per week
- **Community:** Active analysts contributing reviews

## References

- solana-sbpf: [https://github.com/anza-xyz/sbpf](https://github.com/anza-xyz/sbpf)
- Existing disassembler: `core/src/program_analysis/disassembler.rs`
- Existing AI analyzer: `core/src/program_analysis/ai_analyzer.rs`
- Risk register: `tools/risk-register/`

