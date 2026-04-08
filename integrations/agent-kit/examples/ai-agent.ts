/**
 * AI Agent example using SecureCheck with LangChain/Vercel AI
 */

import { SolanaAgentKit, createVercelAITools } from "solana-agent-kit";
import { Keypair } from "@solana/web3.js";
import SecureCheckPlugin from "../src/index.js";

async function main() {
  // Initialize agent with SecureCheck
  const keypair = Keypair.generate();
  const agent = new SolanaAgentKit(
    { publicKey: keypair.publicKey, signTransaction: async () => {} } as any,
    "https://api.mainnet-beta.solana.com",
    {
      OPENAI_API_KEY: process.env.OPENAI_API_KEY || "",
    }
  ).use(
    new SecureCheckPlugin({
      apiUrl: process.env.SECURECHECK_API_URL || "http://localhost:3001",
      enableAutoProtection: true, // Auto-block risky transactions
      minSecurityScore: 70, // Require 70+ score
    })
  );

  // Create tools for AI
  const tools = createVercelAITools(agent, agent.actions);

  console.log("🤖 SecureCheck AI Agent Ready");
  console.log("\nAvailable security tools:");
  console.log("  - checkTokenSecurity");
  console.log("  - checkInsiderRisk");
  console.log("  - checkLiquidityLock");
  console.log("  - performPreTradeChecks");
  console.log("  - validateTransaction");
  console.log("  - scanWallet");
  console.log("  - enableProtectedRPC");

  // Example AI workflow
  console.log("\n🔍 AI Agent: Analyzing token before trade...");

  const tokenToAnalyze = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"; // USDC

  // AI can call these autonomously
  const analysis = await agent.methods.performPreTradeChecks(
    agent,
    tokenToAnalyze
  );

  if (analysis.safe) {
    console.log("✅ AI Decision: Safe to trade");
    console.log(`   Token Score: ${analysis.tokenSecurity.score}/100`);
    console.log(
      `   Liquidity Locked: ${analysis.vaultAnalysis.lockedPercentage}%`
    );
    console.log(
      `   Insider Risk: ${analysis.insiderAnalysis.riskLevel}`
    );

    // AI can proceed with trade
    // await agent.methods.trade(...)
  } else {
    console.log("❌ AI Decision: Not safe to trade");
    console.log("\n   Reasons:");
    analysis.reasons.forEach((r) => console.log(`     - ${r}`));

    // AI decides to skip this token
  }

  // Example: Scan wallet for threats
  console.log("\n🔍 AI Agent: Scanning wallet for threats...");
  const walletReport = await agent.methods.scanWallet(
    agent,
    keypair.publicKey.toBase58(),
    {
      maxTransactions: 50,
      timeWindowDays: 7,
    }
  );

  console.log(`Security Score: ${walletReport.securityScore}/100`);
  console.log(`Threats Found: ${walletReport.stats.threatsFound}`);

  if (walletReport.threats.length > 0) {
    console.log("\n⚠️  Threats detected:");
    walletReport.threats.forEach((threat) => {
      console.log(`  [${threat.severity}] ${threat.description}`);
      console.log(`    → ${threat.recommendation}`);
    });
  }
}

main().catch(console.error);
