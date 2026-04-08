# SecureCheck Plugin for Solana Agent Kit

Real-time security analysis for Solana AI agents. Integrates Parapet's security engine with Solana Agent Kit to provide token analysis, transaction validation, and rugpull protection.

## Features

- **Token Security Checks** - Rugcheck integration with risk scoring (0-100)
- **Insider Trading Detection** - Identify wash trading and holder inflation
- **Liquidity Analysis** - Verify locked liquidity and rugpull risk
- **Transaction Validation** - Pre-send security analysis
- **Wallet Scanning** - Historical threat detection
- **Protected RPC Gateway** - Route transactions through security layer

## Installation

```bash
npm install @securecheck/solana-agent-kit-plugin
```

## Quick Start

```typescript
import { SolanaAgentKit } from "solana-agent-kit";
import SecureCheckPlugin from "@securecheck/solana-agent-kit-plugin";

const agent = new SolanaAgentKit(wallet, rpcUrl)
  .use(new SecureCheckPlugin({
    apiUrl: "http://localhost:3001",
    minSecurityScore: 60
  }));

// Check token before trading
const check = await agent.methods.checkTokenSecurity(
  "So11111111111111111111111111111111111111112"
);

if (check.isSafe) {
  await agent.methods.trade(/* ... */);
} else {
  console.log("⚠️ Security warning:", check.data.risks);
}
```

## API Reference

### `checkTokenSecurity(agent, mintAddress)`

Analyze token security using Rugcheck data.

**Returns:**
```typescript
{
  isSafe: boolean;
  score: number; // 0-100
  riskLevel: string; // Poor, Fair, Good, Excellent
  data: {
    risks: Array<{ name, description, level, score }>;
    marketCap?: number;
    topHoldersPercentage?: number;
    liquidity?: number;
  }
}
```

### `checkInsiderRisk(agent, mintAddress)`

Detect wash trading and insider networks.

**Returns:**
```typescript
{
  isHighRisk: boolean;
  data: {
    tradeNetworks: number;
    insiderConcentration: number;
    warnings: string[];
  }
}
```

### `checkLiquidityLock(agent, mintAddress)`

Verify liquidity locks and rugpull risk.

**Returns:**
```typescript
{
  isRugpullRisk: boolean;
  data: {
    hasLockedLiquidity: boolean;
    lockedPercentage: number;
    rugpullRisk: string; // Low, Medium, High, Critical
  }
}
```

### `performPreTradeChecks(agent, mintAddress)`

Comprehensive security check combining all analyses.

**Returns:**
```typescript
{
  safe: boolean;
  reasons: string[];
  tokenSecurity: TokenSecurityData;
  insiderAnalysis: InsiderAnalysis;
  vaultAnalysis: VaultAnalysis;
}
```

### `validateTransaction(agent, transaction)`

Analyze transaction before sending.

**Returns:**
```typescript
{
  shouldProceed: boolean;
  data: {
    shouldBlock: boolean;
    riskLevel: "LOW" | "MEDIUM" | "HIGH" | "CRITICAL";
    findings: Array<Finding>;
  }
}
```

### `scanWallet(agent, walletAddress, options?)`

Scan wallet for security threats.

### `enableProtectedRPC(agent, apiKey)`

Route all transactions through SecureCheck gateway.

## Configuration

```typescript
new SecureCheckPlugin({
  apiUrl: "http://localhost:3001",        // Parapet API endpoint
  apiKey: "your-api-key",                 // Optional API key
  rpcUrl: "https://api.mainnet-beta.solana.com",
  enableAutoProtection: false,             // Auto-block high-risk transactions
  minSecurityScore: 50                     // Minimum acceptable score (0-100)
})
```

## Usage Examples

### Pre-Trade Security Check

```typescript
const result = await agent.methods.performPreTradeChecks(
  agent,
  "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v" // USDC
);

if (result.safe) {
  console.log("✅ Token passed all security checks");
  await agent.methods.trade(/* ... */);
} else {
  console.log("❌ Security concerns:");
  result.reasons.forEach(r => console.log(`  - ${r}`));
}
```

### Transaction Validation

```typescript
const tx = /* prepare transaction */;

const validation = await agent.methods.validateTransaction(agent, tx);

if (validation.shouldProceed) {
  await agent.connection.sendTransaction(tx);
} else {
  console.log("🚫 Transaction blocked:", validation.data.explanation);
}
```

### Enable Protected Mode

```typescript
await agent.methods.enableProtectedRPC(agent, "your-api-key");
// All subsequent transactions now go through SecureCheck gateway
```

## Requirements

- **Parapet API** running (see [parapet](../parapet/README.md))
- Node.js 18+
- Solana Agent Kit 2.0+

## Running Parapet Locally

```bash
cd parapet/api
cargo run
```

API will be available at `http://localhost:3001`

## License

Apache-2.0

## Links

- [Parapet Documentation](../parapet/README.md)
- [Solana Agent Kit](https://github.com/sendaifun/solana-agent-kit)
- [SecureCheck](https://github.com/securecheckio/securecheck)
