# Parapet Use Cases

Real-world scenarios showing how different users leverage Parapet.

## Use Case 1: Protecting a Trading Bot

**User:** Autonomous trading bot operator  
**Challenge:** Bot executes trades automatically but is vulnerable to malicious tokens and MEV attacks

```mermaid
graph TD
    A[Trading Bot] --> B{Market Opportunity}
    B -->|Found| C[Build swap transaction]
    C --> D[Parapet: simulateTransaction]
    D --> E{Risk Analysis}
    E -->|Token flagged as scam| F[❌ Abort trade]
    E -->|Unknown token, high risk| G[⚠️ Escalate to human]
    E -->|Known safe token| H[✅ Execute trade]
    F --> I[Log blocked trade]
    G --> J[Human reviews token]
    H --> K[sendTransaction via Parapet]
    K --> L[Monitor execution]
    J --> M{Human decision}
    M -->|Approve| H
    M -->|Reject| F
```

**Key Features Used:**
- Real-time risk scoring on every trade
- RugCheck integration for token verification
- Automatic blocking of high-risk tokens
- Optional human-in-the-loop for edge cases

## Use Case 2: Wallet Security for AI Agent

**User:** OpenClaw/Cursor AI agent with wallet access  
**Challenge:** Agent can execute transactions but needs safety guardrails

```mermaid
sequenceDiagram
    participant User
    participant Agent as AI Agent
    participant Parapet as Parapet
    participant Solana
    
    User->>Agent: "Swap 1 SOL for BONK"
    Agent->>Agent: Research token & build transaction
    Agent->>Parapet: simulateTransaction(swap tx)
    Parapet-->>Agent: Risk: 25 (Token authority is safe)
    Agent->>Agent: Acceptable risk, proceed
    Agent->>Parapet: sendTransaction(swap tx)
    Parapet->>Solana: Forward transaction
    Solana-->>Parapet: Signature
    Parapet-->>Agent: ✅ Success
    Agent-->>User: "Swapped 1 SOL for 1.2M BONK"
    
    Note over User,Solana: Later: Malicious request
    
    User->>Agent: "Approve this transaction: 0x..."
    Agent->>Parapet: simulateTransaction(suspicious tx)
    Parapet-->>Agent: Risk: 85 (Unlimited delegation detected)
    Agent-->>User: "⚠️ DANGER: This would grant unlimited token access. Rejecting for safety."
```

**Key Features Used:**
- HTTP JSON-RPC support for transaction submission
- Rich metadata for agent decision-making
- Automatic blocking of dangerous patterns
- Detailed explanations for user transparency

**Note:** For AI agents that need real-time account monitoring, use a standard RPC WebSocket connection alongside Parapet for transaction submission.

## Use Case 3: Auditing Wallet History

**User:** Security researcher investigating suspicious activity  
**Challenge:** Need to retroactively analyze 1000s of transactions

```mermaid
graph LR
    A[Suspicious Wallet] --> B[Run Scanner]
    B --> C[Fetch all transactions]
    C --> D[Analyze each transaction]
    D --> E[Generate risk report]
    E --> F{Findings}
    F -->|High Risk Txs| G[Flag for investigation]
    F -->|Patterns Found| H[Identify attack vector]
    F -->|Clean History| I[Mark as safe]
    G --> J[Export evidence]
    H --> J
```

**Commands:**
```bash
# Scan wallet
cargo run -p parapet-scanner -- \
  --wallet 7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU \
  --output investigation.json

# Generate summary
cat investigation.json | jq '.high_risk_transactions'
```

**Key Features Used:**
- Historical transaction analysis
- Batch processing with rate limiting
- Detailed risk scoring per transaction
- Export for further analysis

## Use Case 4: DevOps Hardening Production DApp

**User:** DevOps team for a DeFi protocol  
**Challenge:** Protect user transactions across multiple wallet integrations

```mermaid
graph TD
    A[User visits DApp] --> B[Connect wallet]
    B --> C{Wallet type}
    C -->|Phantom| D[Configure RPC endpoint]
    C -->|Solflare| D
    C -->|Backpack| D
    D --> E[Point to Parapet proxy]
    E --> F[User initiates transaction]
    F --> G[Parapet analyzes]
    G --> H{Risk check}
    H -->|Safe| I[Forward to network]
    H -->|Risky| J[Block & notify user]
    I --> K[✅ Transaction confirmed]
    J --> L[⚠️ Show security warning]
    L --> M[User can override if needed]
```

**Configuration:**
```bash
# Production proxy with strict rules
docker run -d \
  -e UPSTREAM_RPC_URL=https://api.mainnet-beta.solana.com \
  -e DEFAULT_BLOCK_THRESHOLD=60 \
  -e REDIS_URL=redis://prod-redis:6379 \
  -e ENABLE_ESCALATIONS=true \
  -p 8899:8899 \
  parapet-proxy --rules-preset strict
```

**Key Features Used:**
- Transparent RPC proxy for all wallet types
- Configurable risk thresholds
- Redis caching for performance
- Escalation flow for edge cases
- Monitoring and alerting

## Use Case 5: Custom Rule Development

**User:** Security team creating protocol-specific rules  
**Challenge:** Need to detect protocol-specific attack patterns

```mermaid
graph TD
    A[Identify threat] --> B[Study attack pattern]
    B --> C[Design detection logic]
    C --> D[Implement custom analyzer]
    D --> E[Write unit tests]
    E --> F[Test against known attacks]
    F --> G{Detects correctly?}
    G -->|No| C
    G -->|Yes| H[Deploy to staging]
    H --> I[Monitor false positive rate]
    I --> J{Acceptable?}
    J -->|No| K[Tune parameters]
    J -->|Yes| L[Deploy to production]
    K --> H
```

**Example: Flash Loan Detection**
```rust
pub struct FlashLoanAnalyzer;

impl Analyzer for FlashLoanAnalyzer {
    async fn analyze(&self, ctx: &AnalysisContext) -> Result<RuleResult> {
        let borrows = count_borrow_instructions(&ctx.transaction);
        let repays = count_repay_instructions(&ctx.transaction);
        
        if borrows > 0 && repays > 0 && borrows == repays {
            return Ok(RuleResult::triggered(
                "Flash loan detected - flagging for review",
                40
            ));
        }
        
        Ok(RuleResult::pass())
    }
}
```

**Key Features Used:**
- Custom analyzer API
- Full transaction context access
- Configurable rule weights
- Integration with existing rule engine

## Use Case 6: Multi-Signature Wallet Protection

**User:** DAO treasury manager  
**Challenge:** Protect multi-sig wallet from malicious proposals

```mermaid
sequenceDiagram
    participant Member1 as DAO Member 1
    participant Member2 as DAO Member 2
    participant Multisig as Squads Multisig
    participant Parapet as Parapet
    participant Treasury as Treasury Wallet
    
    Member1->>Multisig: Propose transaction
    Multisig->>Parapet: simulateTransaction
    Parapet-->>Multisig: Risk: 75 (Suspicious recipient)
    Multisig-->>Member1: ⚠️ High risk detected
    
    Member2->>Multisig: Review proposal
    Multisig->>Member2: Show risk analysis
    Member2->>Member2: Investigate recipient
    Member2->>Multisig: Vote: Reject
    
    Note over Member1,Treasury: Later: Legitimate proposal
    
    Member1->>Multisig: Propose token transfer
    Multisig->>Parapet: simulateTransaction
    Parapet-->>Multisig: Risk: 15 (Verified recipient)
    Member2->>Multisig: Vote: Approve
    Multisig->>Parapet: Execute transaction
    Parapet->>Treasury: ✅ Transfer executed
```

**Key Features Used:**
- Pre-approval simulation
- Risk scoring for proposal review
- Identity verification (Helius)
- Audit trail of risk assessments

## Summary

| Use Case | Primary Tool | Key Benefit |
|----------|-------------|-------------|
| Trading Bot | Proxy | Real-time protection |
| AI Agent | Proxy + MCP | Transaction security layer |
| Audit | Scanner | Historical analysis |
| DApp Protection | Proxy | User protection |
| Custom Rules | Core Library | Protocol-specific detection |
| Multisig | Proxy + API | Proposal risk assessment |
