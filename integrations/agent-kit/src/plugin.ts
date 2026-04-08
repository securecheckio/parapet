/**
 * Sol-Shield Security Plugin for Solana Agent Kit
 * Provides real-time security analysis for Solana transactions and tokens
 */

import { PublicKey } from "@solana/web3.js";
import { SecureCheckClient } from "./client.js";
import type {
  SecureCheckConfig,
  TokenSecurityData,
  InsiderAnalysis,
  VaultAnalysis,
  TransactionAnalysis,
  WalletScanReport,
} from "./types.js";

export class SecureCheckPlugin {
  private client: SecureCheckClient;
  private config: SecureCheckConfig;

  constructor(config: SecureCheckConfig = {}) {
    this.config = {
      minSecurityScore: 50,
      enableAutoProtection: false,
      ...config,
    };
    this.client = new SecureCheckClient(config);
  }

  /**
   * Check if a token is safe to trade
   * Returns security score, risk level, and detailed risks
   */
  async checkTokenSecurity(
    agent: any,
    mintAddress: string
  ): Promise<{
    isSafe: boolean;
    score: number;
    riskLevel: string;
    data: TokenSecurityData;
  }> {
    // Validate address
    try {
      new PublicKey(mintAddress);
    } catch {
      throw new Error(`Invalid Solana address: ${mintAddress}`);
    }

    const data = await this.client.checkTokenSecurity(mintAddress);

    const isSafe =
      data.score >= (this.config.minSecurityScore || 50) &&
      data.riskLevel !== "Poor";

    return {
      isSafe,
      score: data.score,
      riskLevel: data.riskLevel,
      data,
    };
  }

  /**
   * Detect insider trading and wash trading patterns
   */
  async checkInsiderRisk(
    agent: any,
    mintAddress: string
  ): Promise<{
    isHighRisk: boolean;
    data: InsiderAnalysis;
  }> {
    const data = await this.client.checkInsiderRisk(mintAddress);

    const isHighRisk =
      data.riskLevel === "Critical" ||
      data.riskLevel === "High" ||
      data.insiderConcentration > 50;

    return {
      isHighRisk,
      data,
    };
  }

  /**
   * Check if liquidity is locked (rugpull protection)
   */
  async checkLiquidityLock(
    agent: any,
    mintAddress: string
  ): Promise<{
    isRugpullRisk: boolean;
    data: VaultAnalysis;
  }> {
    const data = await this.client.checkLiquidityLock(mintAddress);

    const isRugpullRisk =
      !data.hasLockedLiquidity ||
      data.rugpullRisk === "Critical" ||
      data.rugpullRisk === "High";

    return {
      isRugpullRisk,
      data,
    };
  }

  /**
   * Validate a transaction before sending
   * Can automatically block high-risk transactions
   */
  async validateTransaction(
    agent: any,
    transaction: string | object
  ): Promise<{
    shouldProceed: boolean;
    data: TransactionAnalysis;
  }> {
    const data = await this.client.validateTransaction(transaction);

    const shouldProceed = this.config.enableAutoProtection
      ? !data.shouldBlock
      : true;

    return {
      shouldProceed,
      data,
    };
  }

  /**
   * Comprehensive security check before trading
   * Combines all security checks
   */
  async performPreTradeChecks(
    agent: any,
    mintAddress: string
  ): Promise<{
    safe: boolean;
    reasons: string[];
    tokenSecurity: TokenSecurityData;
    insiderAnalysis: InsiderAnalysis;
    vaultAnalysis: VaultAnalysis;
  }> {
    const [tokenCheck, insiderCheck, vaultCheck] = await Promise.all([
      this.checkTokenSecurity(agent, mintAddress),
      this.checkInsiderRisk(agent, mintAddress),
      this.checkLiquidityLock(agent, mintAddress),
    ]);

    const reasons: string[] = [];
    let safe = true;

    if (!tokenCheck.isSafe) {
      safe = false;
      reasons.push(
        `Low security score: ${tokenCheck.score}/100 (${tokenCheck.riskLevel})`
      );
    }

    if (insiderCheck.isHighRisk) {
      safe = false;
      reasons.push(
        `High insider risk: ${insiderCheck.data.riskLevel} (${insiderCheck.data.insiderConcentration.toFixed(1)}% insider concentration)`
      );
    }

    if (vaultCheck.isRugpullRisk) {
      safe = false;
      reasons.push(
        `Rugpull risk: ${vaultCheck.data.rugpullRisk} (${vaultCheck.data.lockedPercentage.toFixed(1)}% locked)`
      );
    }

    return {
      safe,
      reasons,
      tokenSecurity: tokenCheck.data,
      insiderAnalysis: insiderCheck.data,
      vaultAnalysis: vaultCheck.data,
    };
  }

  /**
   * Scan a wallet for security threats
   */
  async scanWallet(
    agent: any,
    walletAddress: string,
    options?: {
      maxTransactions?: number;
      timeWindowDays?: number;
    }
  ): Promise<WalletScanReport> {
    return this.client.scanWallet(walletAddress, {
      ...options,
      format: "json",
    });
  }

  /**
   * Enable protected mode - routes all RPC through SecureCheck gateway
   */
  async enableProtectedRPC(
    agent: any,
    apiKey: string
  ): Promise<{ protected: boolean; gatewayUrl: string }> {
    const gatewayUrl = `${this.config.apiUrl?.replace("3001", "8899") || "http://localhost:8899"}`;

    // Update agent's connection to use secured gateway
    // This assumes agent has a connection property
    if (agent.connection) {
      const { Connection } = await import("@solana/web3.js");
      agent.connection = new Connection(
        `${gatewayUrl}?apiKey=${apiKey}`,
        "confirmed"
      );
    }

    return {
      protected: true,
      gatewayUrl,
    };
  }

  /**
   * Set security threshold (0-100)
   */
  async setRiskThreshold(agent: any, threshold: number): Promise<void> {
    if (threshold < 0 || threshold > 100) {
      throw new Error("Threshold must be between 0 and 100");
    }
    this.config.minSecurityScore = threshold;
  }

  /**
   * Check if SecureCheck API is available
   */
  async healthCheck(agent: any): Promise<boolean> {
    return this.client.healthCheck();
  }
}

export default SecureCheckPlugin;
