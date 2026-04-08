# Core Analyzers

Built-in transaction analyzers that extract security-relevant fields from Solana transactions. These are the foundation of the rules engine and operate entirely on-chain data without external API calls.

## Overview

Core analyzers are **fast, deterministic, and dependency-free**. They parse transaction structures, decode instructions, and extract fields that rules can match against.

**Performance:** <1ms per transaction (pure in-memory parsing)

## Available Analyzers

### basic
Fundamental transaction metadata from Solana SDK.

**Fields:**
- `instruction_count` - Number of instructions
- `account_keys_count` - Total accounts involved
- `writable_accounts_count` - Mutable accounts
- `signers_count` - Required signatures
- `amount` - Extracted transfer amount
- `program_ids` - Unique programs invoked

**Use cases:** Complexity limits, multi-program detection, amount thresholds

---

### security
High-level risk assessment and known-threat detection.

**Fields:**
- `risk_score` - Computed risk level (0-100)
- `risk_level` - Categorical: low, medium, high, critical
- `delegation_detected` - Token delegation found
- `delegation_is_unlimited` - u64::MAX approval (drainer signature)
- `delegation_count` - Number of delegations

**Use cases:** Wallet drainer detection, risk-based routing, delegation alerts

---

### token_instructions
SPL Token program instruction parsing.

**Fields:**
- `has_transfer` - Any token movement
- `has_approve` - Delegation instructions
- `approve_count` - Number of approvals
- `unlimited_approve_count` - u64::MAX approvals
- `has_revoke` - Revoke delegation
- `has_freeze` - Freeze account
- `has_mint` - Mint new tokens
- `has_burn` - Burn tokens

**Use cases:** Token permission tracking, drainer detection, mint/burn monitoring

---

### system_program
Native SOL transfer and account operations.

**Fields:**
- `has_sol_transfer` - Native SOL movement
- `sol_transfer_count` - Number of transfers
- `total_sol_transferred` - Sum in lamports
- `max_sol_transfer` - Largest single transfer
- `large_sol_transfer` - Any transfer > 1 SOL

**Use cases:** Drain detection, large transfer alerts, suspicious SOL movements

---

### instruction_data
Raw instruction data pattern matching.

**Fields:**
- `instruction_data:matched_names` - Detected patterns
- `instruction_data:has_match` - Any pattern matched
- `instruction_data:has_authority_change` - Authority modification detected
- `instruction_data:match_count` - Total matches
- `instruction_data:categories` - Pattern categories matched

**Use cases:** Fingerprinting malicious instructions, authority change detection

---

### instruction_padding
Detects obfuscation via excessive instruction padding.

**Fields:**
- `has_excessive_padding` - Padding > threshold
- `max_padding_ratio` - Worst padding ratio
- `padded_instruction_count` - Number of padded instructions

**Use cases:** Evasion detection, obfuscation alerts

---

### program_complexity
Static analysis of invoked programs.

**Fields:**
- `unique_program_count` - Distinct programs called
- `has_unverified_programs` - Unknown programs detected
- `verified_program_count` - Known-safe programs
- `cpi_count` - Cross-program invocation count

**Use cases:** Multi-program attack detection, unverified program blocking

---

### transaction_logs
Parses on-chain logs for errors and warnings.

**Fields:**
- `has_errors` - Any error logs
- `has_warnings` - Any warning logs
- `error_count` - Number of errors
- `log_messages` - Raw log array

**Use cases:** Simulation failure detection, debugging

---

### canonical_tx
Transaction identity and deduplication.

**Fields:**
- `canonical_hash` - Deterministic tx fingerprint
- `blockhash` - Recent blockhash used
- `has_recent_blockhash` - Valid blockhash present

**Use cases:** Deduplication, replay detection, escalation tracking

## Analyzer Lifecycle

```rust
// 1. Analyzer is created with config
let analyzer = BasicAnalyzer::new();

// 2. Called by rules engine for each transaction
let result = analyzer.analyze(&transaction, &context)?;

// 3. Fields extracted and stored
let fields: HashMap<String, Value> = result.fields;

// 4. Rules match against fields
// Example: "basic:instruction_count > 10"
```

## Adding Custom Analyzers

Core analyzers are part of the library. To add a custom analyzer:

1. **For production**: Submit PR to this repository
2. **For experiments**: Use WASM analyzers (see `core/WASM_ANALYZERS.md`)
3. **For proprietary**: Fork and maintain your own core library

## Configuration

Core analyzers have no external dependencies and require no configuration. They operate entirely on the transaction data provided.

For analyzers that need configuration (e.g., thresholds), use rule parameters:

```json
{
  "id": "high-instruction-count",
  "condition": "basic:instruction_count > 20",
  "action": "alert",
  "metadata": {
    "threshold": 20
  }
}
```

## Performance Benchmarks

```
Analyzer               Avg Latency    P99 Latency
-------------------------------------------------
basic                  0.05ms         0.12ms
security               0.08ms         0.18ms
token_instructions     0.12ms         0.25ms
system_program         0.10ms         0.22ms
instruction_data       0.15ms         0.35ms
instruction_padding    0.08ms         0.18ms
program_complexity     0.20ms         0.45ms
transaction_logs       0.10ms         0.23ms
canonical_tx           0.05ms         0.11ms
```

Total overhead for all core analyzers: **~0.5ms per transaction**

## Documentation

Each analyzer has a dedicated markdown file with:
- Detailed field descriptions
- Risk scenarios detected
- Example rules
- Performance characteristics

See `*.md` files in this directory.
