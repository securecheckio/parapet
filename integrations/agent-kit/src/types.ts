/**
 * Parapet Security Plugin Types
 */

export interface TokenSecurityData {
  tokenAddress: string;
  score: number;
  riskLevel: string;
  risks: RiskItem[];
  marketCap?: number;
  topHoldersPercentage?: number;
  liquidity?: number;
  tokenAgeDays?: number;
}

export interface RiskItem {
  name: string;
  description: string;
  level: string;
  score: number;
}

export interface InsiderAnalysis {
  tokenAddress: string;
  tradeNetworks: number;
  transferNetworks: number;
  totalNetworks: number;
  totalInsiders: number;
  insiderConcentration: number;
  riskLevel: string;
  riskScore: number;
  warnings: string[];
}

export interface VaultAnalysis {
  tokenAddress: string;
  hasLockedLiquidity: boolean;
  totalLockers: number;
  lockedPercentage: number;
  unlockDate?: string;
  rugpullRisk: string;
  lockers: VaultLocker[];
}

export interface VaultLocker {
  lockerType: string;
  lockedAmount: number;
  unlockDate?: string;
  percentageOfSupply: number;
}

export interface TransactionAnalysis {
  shouldBlock: boolean;
  shouldWarn: boolean;
  riskLevel: "LOW" | "MEDIUM" | "HIGH" | "CRITICAL";
  riskScore: number;
  findings: Finding[];
  explanation: string;
}

export interface Finding {
  severity: "INFO" | "LOW" | "MEDIUM" | "HIGH" | "CRITICAL";
  category: string;
  title: string;
  description: string;
  recommendation?: string;
}

export interface WalletScanReport {
  wallet: string;
  securityScore: number;
  riskLevel: string;
  stats: {
    timeRangeDays: number;
    transactionsAnalyzed: number;
    threatsFound: number;
    criticalCount: number;
    highCount: number;
    mediumCount: number;
    lowCount: number;
  };
  threats: Threat[];
}

export interface Threat {
  severity: "LOW" | "MEDIUM" | "HIGH" | "CRITICAL";
  threatType: string;
  description: string;
  recommendation: string;
}

export interface SecureCheckConfig {
  apiUrl?: string;
  apiKey?: string;
  rpcUrl?: string;
  enableAutoProtection?: boolean;
  minSecurityScore?: number;
}
