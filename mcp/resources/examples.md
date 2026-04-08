# Parapet Usage Examples

## Example 1: Pre-Transaction Wallet Check

**Scenario**: User wants to send tokens to a wallet

```typescript
// Agent receives request
User: "Send 100 USDC to wallet 7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU"

// Step 1: Scan the recipient wallet
scan_wallet({
  wallet_address: "7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU",
  format: "summary"
})

// Response indicates safe
Security Score: 15/100 (LOW RISK)
No active threats detected
Wallet has clean transaction history

// Agent proceeds
→ Build transaction
→ Send 100 USDC
→ "Transaction sent safely to verified wallet"
```

## Example 2: Token Safety Verification

**Scenario**: User asks about buying a token

```typescript
User: "Should I buy token BONK?"

// Step 1: Look up BONK mint address (from your knowledge or API)
const bonkMint = "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263";

// Step 2: Scan the token mint
scan_wallet({
  wallet_address: bonkMint,
  max_transactions: 50,
  format: "detailed"
})

// Response shows token details
Security Score: 25/100 (LOW RISK)
RugCheck Status: GOOD
- Mint authority: REVOKED ✅
- Freeze authority: REVOKED ✅
- Verified by OtterSec: Yes ✅

// Agent recommends
→ "BONK appears to be a legitimate token with good safety indicators:
   - Mint authority has been revoked (can't print more tokens)
   - Freeze authority revoked (can't freeze accounts)
   - Verified by security auditors
   However, remember that all crypto investments carry risk."
```

## Example 3: Detecting Active Threats

**Scenario**: Scanning a potentially compromised wallet

```typescript
scan_wallet({
  wallet_address: "SuspiciousWallet123...",
  time_window_days: 7,
  format: "detailed"
})

// Response shows critical threats
Security Score: 95/100 (CRITICAL RISK)

⚠️ ACTIVE THREATS DETECTED:
1. [CRITICAL] Unlimited token approval to DrainerProgram...
   - Token: USDC
   - Approved amount: UNLIMITED
   - Date: 2 days ago
   - Action: REVOKE IMMEDIATELY

2. [HIGH] Authority transfer on token ABC
   - New authority: UnknownAddress...
   - Indicates potential rug pull
   - Date: 1 day ago

📊 Recent Activity:
- 15 transactions in last 24 hours (unusual spike)
- 8 interactions with unverified programs
- 3 token burn instructions

// Agent response
→ "⚠️ CRITICAL WARNING: This wallet has ACTIVE SECURITY THREATS
   
   DO NOT interact with this wallet. It has:
   1. An unlimited token approval that could drain all USDC
   2. A compromised token authority
   3. Suspicious activity patterns
   
   If this is YOUR wallet:
   - Revoke the unlimited approval immediately
   - Transfer assets to a new wallet
   - Stop using this wallet for transactions"
```

## Example 4: Program Verification

**Scenario**: User wants to use a new DeFi protocol

```typescript
User: "Is SomeNewDeFi program safe to use? Program ID: SomeNew1..."

// Analyze the program
analyze_program({
  program_id: "SomeNew1...",
  network: "mainnet-beta"
})

// Response shows concerns
📋 PROGRAM ANALYSIS

Program: SomeNew1...
Network: mainnet-beta
Deployed: 5 days ago

⚠️ VERIFICATION STATUS:
- OtterSec: NOT VERIFIED
- Helius: Unknown Program
- Upgrade Authority: PRESENT (can be modified)

🔴 CONCERNS:
1. Program is only 5 days old
2. Not verified by security auditors
3. Has upgrade authority (can change functionality)
4. No established reputation

// Agent recommends
→ "⚠️ PROCEED WITH CAUTION
   
   This program has several risk factors:
   - Very recently deployed (5 days old)
   - Not audited or verified
   - Can be upgraded/changed by developers
   
   Recommendations:
   1. Start with small test transactions
   2. Wait for security audit
   3. Watch for community feedback
   4. Consider using established alternatives
   
   Only use if you fully trust the developers."
```

## Example 5: Trusted Program Verification

**Scenario**: Verifying a well-known program

```typescript
analyze_program({
  program_id: "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",
  network: "mainnet-beta"
})

// Response shows strong verification
📋 PROGRAM ANALYSIS

Program: TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA
Network: mainnet-beta
Name: SPL Token Program

✅ VERIFICATION STATUS:
- OtterSec: VERIFIED and AUDITED
- Helius: Core Solana Program
- Upgrade Authority: NONE (Immutable)

✅ SECURITY INDICATORS:
- Deployed: 2020 (3+ years active)
- Immutable program (cannot be changed)
- Core Solana infrastructure
- Widely used and trusted

// Agent confirms
→ "✅ SAFE TO USE
   
   This is the official SPL Token Program, verified by:
   - OtterSec security audit
   - Solana Foundation
   - 3+ years of proven track record
   
   This is a core Solana program used by all tokens.
   Safe to interact with."
```

## Example 6: Batch Analysis

**Scenario**: Analyzing multiple wallets for a group transaction

```typescript
const wallets = [
  "Wallet1...",
  "Wallet2...",
  "Wallet3..."
];

// Scan each wallet (can be done in parallel)
const results = await Promise.all(
  wallets.map(addr => scan_wallet({
    wallet_address: addr,
    max_transactions: 50,
    format: "summary"
  }))
);

// Check all are safe
const allSafe = results.every(r => 
  r.security_score < 30 && 
  r.active_threats.length === 0
);

if (allSafe) {
  → "All wallets verified safe for group transaction"
} else {
  const risky = results
    .filter(r => r.security_score >= 30)
    .map(r => r.wallet_address);
  
  → `Warning: ${risky.length} wallet(s) have elevated risk: ${risky.join(', ')}`
}
```

## Example 7: Periodic Wallet Monitoring

**Scenario**: Regular security checks on user's wallet

```typescript
// Check user's wallet daily
async function dailySecurityCheck(userWallet) {
  const result = await scan_wallet({
    wallet_address: userWallet,
    time_window_days: 1,  // Only check last day
    format: "detailed"
  });
  
  // Alert on new threats
  if (result.active_threats.length > 0) {
    return {
      alert: true,
      message: "⚠️ NEW SECURITY THREATS DETECTED",
      threats: result.active_threats,
      action_required: true
    };
  }
  
  // Alert on elevated risk
  if (result.security_score > 50) {
    return {
      alert: true,
      message: "⚠️ WALLET RISK INCREASED",
      score: result.security_score,
      action_required: false
    };
  }
  
  return {
    alert: false,
    message: "✅ Wallet security check passed",
    score: result.security_score
  };
}
```

## Best Practices Summary

1. **Always scan before sending**: Check recipient wallets before transfers
2. **Verify programs first**: Analyze programs before interacting
3. **Cache results**: Scans are expensive - cache for 5-10 minutes
4. **Handle slow scans**: Inform users that wallet scans take 5-10 minutes
5. **Explain risks clearly**: Don't just show scores - explain what they mean
6. **Provide actionable advice**: Tell users what to do about threats
7. **Use appropriate format**:
  - `summary` for quick checks
  - `detailed` for thorough analysis
  - `json` for programmatic processing
8. **Handle errors gracefully**: Network issues are common - retry with backoff

