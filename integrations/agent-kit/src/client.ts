/**
 * Parapet API Client
 * HTTP client for interacting with Parapet security services
 */

import type {
  TokenSecurityData,
  InsiderAnalysis,
  VaultAnalysis,
  TransactionAnalysis,
  WalletScanReport,
  SecureCheckConfig,
} from "./types.js";

export class SecureCheckClient {
  private apiUrl: string;
  private apiKey?: string;
  private rpcUrl?: string;

  constructor(config: SecureCheckConfig = {}) {
    this.apiUrl = config.apiUrl || "http://localhost:3001";
    this.apiKey = config.apiKey;
    this.rpcUrl = config.rpcUrl || "https://api.mainnet-beta.solana.com";
  }

  private async fetch(
    endpoint: string,
    options: RequestInit = {}
  ): Promise<Response> {
    const headers: HeadersInit = {
      "Content-Type": "application/json",
      ...options.headers,
    };

    if (this.apiKey) {
      headers["Authorization"] = `Bearer ${this.apiKey}`;
    }

    const response = await fetch(`${this.apiUrl}${endpoint}`, {
      ...options,
      headers,
    });

    if (!response.ok) {
      const error = await response.text();
      throw new Error(`SecureCheck API error: ${response.status} - ${error}`);
    }

    return response;
  }

  /**
   * Check token security using Rugcheck integration
   */
  async checkTokenSecurity(mintAddress: string): Promise<TokenSecurityData> {
    // This would call your MCP or a dedicated REST endpoint
    // For now, we'll use the MCP HTTP interface
    const response = await this.fetch("/mcp/message", {
      method: "POST",
      body: JSON.stringify({
        jsonrpc: "2.0",
        id: Date.now(),
        method: "tools/call",
        params: {
          name: "check_token_security",
          arguments: {
            mint_address: mintAddress,
          },
        },
      }),
    });

    const data = await response.json();
    return data.result.content[0].text
      ? JSON.parse(data.result.content[0].text)
      : data.result;
  }

  /**
   * Analyze insider trading risks
   */
  async checkInsiderRisk(mintAddress: string): Promise<InsiderAnalysis> {
    const response = await this.fetch("/mcp/message", {
      method: "POST",
      body: JSON.stringify({
        jsonrpc: "2.0",
        id: Date.now(),
        method: "tools/call",
        params: {
          name: "check_insider_risk",
          arguments: {
            mint_address: mintAddress,
          },
        },
      }),
    });

    const data = await response.json();
    return data.result.content[0].text
      ? JSON.parse(data.result.content[0].text)
      : data.result;
  }

  /**
   * Check liquidity vault locks
   */
  async checkLiquidityLock(mintAddress: string): Promise<VaultAnalysis> {
    const response = await this.fetch("/mcp/message", {
      method: "POST",
      body: JSON.stringify({
        jsonrpc: "2.0",
        id: Date.now(),
        method: "tools/call",
        params: {
          name: "check_liquidity_lock",
          arguments: {
            mint_address: mintAddress,
          },
        },
      }),
    });

    const data = await response.json();
    return data.result.content[0].text
      ? JSON.parse(data.result.content[0].text)
      : data.result;
  }

  /**
   * Analyze a transaction before sending
   */
  async validateTransaction(
    transaction: string | object
  ): Promise<TransactionAnalysis> {
    const txData =
      typeof transaction === "string" ? transaction : JSON.stringify(transaction);

    const response = await this.fetch("/mcp/message", {
      method: "POST",
      body: JSON.stringify({
        jsonrpc: "2.0",
        id: Date.now(),
        method: "tools/call",
        params: {
          name: "validate_transaction",
          arguments: {
            transaction: txData,
          },
        },
      }),
    });

    const data = await response.json();
    return data.result.content[0].text
      ? JSON.parse(data.result.content[0].text)
      : data.result;
  }

  /**
   * Scan a wallet for security threats
   */
  async scanWallet(
    walletAddress: string,
    options: {
      maxTransactions?: number;
      timeWindowDays?: number;
      format?: "summary" | "detailed" | "json";
    } = {}
  ): Promise<WalletScanReport> {
    const response = await this.fetch("/mcp/message", {
      method: "POST",
      body: JSON.stringify({
        jsonrpc: "2.0",
        id: Date.now(),
        method: "tools/call",
        params: {
          name: "scan_wallet",
          arguments: {
            wallet_address: walletAddress,
            rpc_url: this.rpcUrl,
            max_transactions: options.maxTransactions || 100,
            time_window_days: options.timeWindowDays || 30,
            format: options.format || "json",
          },
        },
      }),
    });

    const data = await response.json();
    const content = data.result.content[0].text;

    // If format was summary or detailed, parse the markdown
    if (options.format === "json") {
      return JSON.parse(content);
    }

    // Return summary as simple report
    return {
      wallet: walletAddress,
      securityScore: 0,
      riskLevel: "UNKNOWN",
      stats: {
        timeRangeDays: options.timeWindowDays || 30,
        transactionsAnalyzed: 0,
        threatsFound: 0,
        criticalCount: 0,
        highCount: 0,
        mediumCount: 0,
        lowCount: 0,
      },
      threats: [],
    };
  }

  /**
   * Check API health
   */
  async healthCheck(): Promise<boolean> {
    try {
      const response = await fetch(`${this.apiUrl}/health`);
      return response.ok;
    } catch {
      return false;
    }
  }
}
