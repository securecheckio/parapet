# Parapet Security Analysis Guide

## What is Parapet?

Parapet is a comprehensive Solana security analysis system that protects wallets and programs from threats like:

- Token drains and unlimited delegations
- Malicious program interactions
- Authority hijacks and compromised accounts
- Rug pulls and scam tokens
- Suspicious Cross-Program Invocations (CPIs)

## Available Tools

### 1. scan_wallet

Performs comprehensive security analysis on a Solana wallet address.

**When to use:**

- Before interacting with a new wallet
- To audit a wallet's transaction history
- To detect active threats (unlimited delegations, compromised authorities)
- To verify wallet safety for trading or transfers

**What it checks:**

- **Active Threats**: Unlimited token delegations, compromised authorities
- **Transaction History**: Up to 100 recent transactions analyzed
- **Program Interactions**: All programs the wallet has interacted with
- **Token Verification**: Integration with RugCheck, Jupiter, Helius
- **Risk Scoring**: Overall security score from 0-100

**Parameters:**

- `wallet_address` (required): Solana wallet address (base58)
- `max_transactions` (optional): Number of transactions to analyze (default: 100)
- `time_window_days` (optional): Days to look back (default: 30)
- `format` (optional): Output format - "summary", "detailed", or "json"

**Response format:**

```
🔍 WALLET SCAN REPORT

Wallet: 7xKXtg2CW87...
Security Score: 85/100 (MEDIUM RISK)
Threats Detected: 2

⚠️ ACTIVE THREATS:
- Unlimited token delegation to SomeProgram...
- Compromised authority detected on token ABC...

📊 TRANSACTION ANALYSIS:
- 45 transactions scanned (last 30 days)
- 12 unique programs interacted with
- 3 suspicious patterns detected

🎯 RECOMMENDATIONS:
1. Revoke unlimited delegation to SomeProgram
2. Stop interacting with token ABC
3. Review recent transactions for anomalies
```

### 2. analyze_program

Analyzes a Solana program for security and verification status.

**When to use:**

- Before interacting with a new program
- To verify program legitimacy
- To check if a program is verified by security auditors
- To get program metadata and explorer links

**What it checks:**

- **On-chain Data**: Program account, upgrade authority, executability
- **OtterSec Verification**: Audited and verified programs
- **Helius Identity**: Program reputation and metadata
- **Explorer Links**: Direct links for manual review

**Parameters:**

- `program_id` (required): Solana program ID (base58)
- `network` (optional): "mainnet-beta", "devnet", or "testnet" (default: mainnet-beta)

**Response format:**

```
📋 PROGRAM ANALYSIS

Program: TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA
Network: mainnet-beta

✅ VERIFICATION STATUS:
- OtterSec: Verified and Audited
- Helius: Known Program (SPL Token)

📊 ON-CHAIN DATA:
- Executable: Yes
- Upgrade Authority: None (Immutable)
- Owner: BPFLoaderUpgradeab1e...

🔗 EXPLORER LINKS:
- Solana Explorer: https://explorer.solana.com/...
- SolanaFM: https://solana.fm/...
```

## Understanding Risk Scores

Parapet uses a risk scoring system from 0-100:

- **0-30**: LOW RISK - Safe to interact with
- **31-60**: MEDIUM RISK - Caution advised, review warnings
- **61-85**: HIGH RISK - Dangerous, avoid unless verified
- **86-100**: CRITICAL RISK - Severe threats detected, DO NOT INTERACT

Risk scores are calculated by combining:

1. **Active Threats**: Immediate dangers (unlimited delegations, compromised accounts)
2. **Transaction Patterns**: Suspicious behavior in transaction history
3. **Program Reputation**: Interactions with unverified or malicious programs
4. **Token Analysis**: RugCheck scores for tokens held/interacted with

## Best Practices for AI Agents

### 1. Always Scan Before Interaction

```
Before sending tokens to a wallet:
→ scan_wallet(recipient_address)
→ Check risk score < 30
→ Verify no active threats
→ Proceed with transaction

Before interacting with a program:
→ analyze_program(program_id)
→ Check OtterSec verification
→ Review on-chain data
→ Proceed with interaction
```

### 2. Interpret Results Carefully

- **Security Score < 30**: Generally safe
- **Security Score 30-60**: Review warnings, use caution
- **Security Score > 60**: Recommend human review
- **Active Threats Present**: Strongly advise against interaction

### 3. Provide Context to Users

Don't just report scores - explain the risks:

- "This wallet has an unlimited token delegation that could drain funds"
- "This program is not verified by OtterSec auditors"
- "Recent transactions show interactions with known scam programs"

### 4. Handle Scan Duration

Wallet scans can take 5-10 minutes for 100 transactions because they:

- Fetch and analyze each transaction individually
- Query external APIs (RugCheck, Helius, Jupiter)
- Perform deep CPI (Cross-Program Invocation) analysis

Inform users about the expected duration upfront.

## Common Workflows

### Workflow 1: Verify Recipient Before Transfer

```
User: "Send 10 SOL to wallet ABC123..."

Agent:
1. scan_wallet("ABC123...")
2. If score < 30 and no active threats:
   → Proceed with transfer
3. If score 30-60:
   → Warn user, ask for confirmation
4. If score > 60:
   → Refuse and explain risks
```

### Workflow 2: Token Safety Check

```
User: "Should I buy token XYZ?"

Agent:
1. Look up token mint address
2. scan_wallet(token_mint_address)
3. Check RugCheck score in response
4. Check for:
   - Rug pull indicators
   - Mint authority status
   - Freeze authority status
5. Provide recommendation based on findings
```

### Workflow 3: Program Due Diligence

```
User: "Is this DeFi program safe to use?"

Agent:
1. analyze_program(program_id)
2. Check OtterSec verification
3. Check upgrade authority status:
   - None = Immutable (good)
   - Present = Can be changed (risk)
4. Cross-reference with Helius identity
5. Provide assessment
```

## Error Handling

Common errors you may encounter:

- **Invalid Address**: Check address format (base58, 32-44 chars)
- **Network Timeout**: RPC endpoint may be slow, retry
- **Rate Limiting**: Too many requests, implement backoff
- **Resource Not Found**: Wallet has no transaction history

Always handle errors gracefully and inform the user.

## Integration Tips

1. **Cache Results**: Wallet scans are expensive - cache for 5-10 minutes
2. **Set Timeouts**: Allow 5-10 minutes for wallet scans
3. **Batch When Possible**: If analyzing multiple programs, do them in parallel
4. **Use Appropriate Format**:
  - `summary` for quick user feedback
  - `detailed` for thorough analysis
  - `json` for programmatic parsing

## Getting Help

If you encounter issues or need clarification:

- Check the error message carefully
- Verify input parameters are correct
- Ensure network connectivity to Solana RPC
- Check Solana RPC endpoint is accessible

