/**
 * Basic usage example for SecureCheck plugin
 */

import { SolanaAgentKit, createVercelAITools } from "solana-agent-kit";
import { Keypair } from "@solana/web3.js";
import SecureCheckPlugin from "../src/index.js";

async function main() {
  // Initialize agent
  const keypair = Keypair.generate(); // In production, load from secure storage
  const agent = new SolanaAgentKit(
    { publicKey: keypair.publicKey, signTransaction: async () => {} } as any,
    "https://api.mainnet-beta.solana.com"
  ).use(
    new SecureCheckPlugin({
      apiUrl: "http://localhost:3001",
      minSecurityScore: 60,
    })
  );

  // Example 1: Check token security before trading
  console.log("\n🔍 Checking SOL security...");
  const solCheck = await agent.methods.checkTokenSecurity(
    agent,
    "So11111111111111111111111111111111111111112"
  );

  console.log(`Score: ${solCheck.score}/100`);
  console.log(`Risk Level: ${solCheck.riskLevel}`);
  console.log(`Safe to trade: ${solCheck.isSafe ? "✅ Yes" : "❌ No"}`);

  // Example 2: Comprehensive pre-trade checks
  console.log("\n🔍 Performing comprehensive checks...");
  const checks = await agent.methods.performPreTradeChecks(
    agent,
    "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v" // USDC
  );

  if (checks.safe) {
    console.log("✅ All checks passed!");
  } else {
    console.log("⚠️ Security concerns found:");
    checks.reasons.forEach((reason) => console.log(`  - ${reason}`));
  }

  // Example 3: Check for insider trading
  console.log("\n🔍 Checking for insider trading...");
  const insiderCheck = await agent.methods.checkInsiderRisk(
    agent,
    "So11111111111111111111111111111111111111112"
  );

  if (insiderCheck.isHighRisk) {
    console.log("⚠️ High insider risk detected!");
    console.log(
      `Insider concentration: ${insiderCheck.data.insiderConcentration}%`
    );
    insiderCheck.data.warnings.forEach((w) => console.log(`  - ${w}`));
  } else {
    console.log("✅ No significant insider activity");
  }

  // Example 4: Health check
  console.log("\n🔍 Checking SecureCheck API status...");
  const healthy = await agent.methods.healthCheck(agent);
  console.log(`API Status: ${healthy ? "✅ Online" : "❌ Offline"}`);
}

main().catch(console.error);
