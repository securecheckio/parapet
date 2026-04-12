# Parapet Risk Scoring System

## Risk Score Ranges

Parapet calculates risk scores from 0-100:


| Range  | Level    | Meaning               | Recommended Action         |
| ------ | -------- | --------------------- | -------------------------- |
| 0-30   | LOW      | Safe to interact with | Proceed normally           |
| 31-60  | MEDIUM   | Caution advised       | Review warnings carefully  |
| 61-85  | HIGH     | Dangerous             | Avoid unless verified safe |
| 86-100 | CRITICAL | Severe threats        | DO NOT INTERACT            |


## How Scores Are Calculated

Risk scores combine multiple factors weighted by severity:

### 1. Active Threats (Highest Weight)

**Unlimited Token Delegations** (+40-60 points)

- Wallet has granted unlimited approval to a program
- Could allow complete token drain
- Immediate action required

**Compromised Authority** (+50-80 points)

- Token or account authority has been transferred to suspicious address
- Indicates potential takeover or rug pull
- Critical threat level

**Active Drainer Detection** (+80-100 points)

- Wallet interacted with known token drainer
- Pattern matches malicious behavior
- Maximum threat level

### 2. Transaction Pattern Analysis (Medium Weight)

**Suspicious Programs** (+10-30 points per program)

- Interactions with unverified programs
- Programs with poor reputation
- Recently deployed programs (< 30 days)

**High-Risk Instructions** (+5-20 points each)

- SetAuthority instructions
- CloseAccount instructions
- Burn instructions with suspicious patterns

**Complex CPI Chains** (+10-25 points)

- Deep nested Cross-Program Invocations (3+ levels)
- Unusual program interaction patterns
- Potential for hidden behavior

### 3. Token Analysis (Medium Weight)

**RugCheck Scores** (Mapped to risk points)

- RugCheck "danger": +60 points
- RugCheck "warning": +30 points
- RugCheck "good": +0 points

**Token Metadata Flags**

- Mutable metadata: +10 points
- Freeze authority present: +15 points
- Unknown/suspicious mint: +20 points
- Low liquidity: +15 points

### 4. Program Reputation (Lower Weight)

**Verification Status**

- Not OtterSec verified: +5-10 points
- No Helius identity: +5 points
- Recently deployed: +10 points
- Upgrade authority present: +5 points

## Example Calculation

**Wallet Analysis:**

```
Base Score: 0

+ Unlimited delegation to UnknownProgram: +50
+ Interaction with unverified DeFi program: +15
+ Token XYZ has RugCheck "warning": +30
+ 2 recent SetAuthority instructions: +20
+ Complex CPI chain (4 levels deep): +15

Total Risk Score: 130 (capped at 100)
Final Score: 100 (CRITICAL RISK)
```

## Threat Severity Levels

Threats are classified by severity:

### LOW Severity

- Interactions with popular programs (Raydium, Jupiter, etc.)
- Standard token transfers
- Normal authority transfers between owned accounts

### MEDIUM Severity

- Interactions with unverified programs
- Mutable token metadata
- Freeze authority present
- Recent account creations

### HIGH Severity

- Unlimited token approvals
- Token burns with suspicious timing
- Complex nested CPIs
- Interactions with recent programs

### CRITICAL Severity

- Active token drainers detected
- Compromised authorities
- Known malicious programs
- Rug pull indicators

## Special Cases

### Immutable Programs (Lower Risk)

Programs with no upgrade authority are considered more trustworthy:

- Risk reduced by 10-20 points
- Still analyzed for other factors

### Verified Programs (Lower Risk)

OtterSec verified programs receive reduced risk:

- Risk reduced by 15-25 points
- Verification does not eliminate all risk

### Old Wallets (Context Matters)

Wallets with long history (> 6 months) and many transactions:

- Individual transaction weight reduced
- Overall pattern more important

### Fresh Wallets (Higher Risk)

New wallets (< 7 days old):

- Individual threats weighted higher
- Less transaction history for context

## Using Risk Scores in Decision Making

### For AI Agents

```javascript
if (riskScore < 30) {
    // Safe to proceed
    return "This wallet appears safe to interact with";
} else if (riskScore < 60) {
    // Caution - check specific warnings
    return "Caution advised. Review these warnings: " + warnings;
} else if (riskScore < 85) {
    // High risk - recommend against
    return "HIGH RISK: Strongly recommend NOT interacting with this wallet";
} else {
    // Critical - refuse
    return "CRITICAL RISK: Refusing to interact. Active threats detected.";
}
```

### Combining Multiple Factors

Don't rely solely on the score:

1. Check risk score
2. Review specific threats detected
3. Consider threat severity levels
4. Look at transaction patterns
5. Verify program reputations
6. Make informed decision

### When to Override Scores

Some situations may warrant overriding the score:

- **False Positives**: Popular programs may trigger warnings
- **User Intent**: User explicitly trusts the destination
- **Context**: Specific use case justifies the risk

Always explain the risk to users before proceeding.

## Continuous Improvement

The risk scoring system evolves based on:

- New threat patterns discovered
- False positive feedback
- Security research updates
- Community reports

Scores should be interpreted as guidance, not absolute truth.