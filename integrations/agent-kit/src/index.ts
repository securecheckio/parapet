/**
 * @securecheck/solana-agent-kit-plugin
 * 
 * Sol-Shield security integration for Solana Agent Kit
 * 
 * @example
 * ```typescript
 * import { SolanaAgentKit } from "solana-agent-kit";
 * import SecureCheckPlugin from "@securecheck/solana-agent-kit-plugin";
 * 
 * const agent = new SolanaAgentKit(wallet, rpcUrl)
 *   .use(new SecureCheckPlugin({
 *     apiUrl: "http://localhost:3001",
 *     minSecurityScore: 60
 *   }));
 * 
 * // Check token before trading
 * const check = await agent.methods.checkTokenSecurity(
 *   "So11111111111111111111111111111111111111112"
 * );
 * 
 * if (check.isSafe) {
 *   await agent.methods.trade(...);
 * }
 * ```
 */

export { SecureCheckPlugin } from "./plugin.js";
export { SecureCheckClient } from "./client.js";
export * from "./types.js";

export { SecureCheckPlugin as default } from "./plugin.js";
