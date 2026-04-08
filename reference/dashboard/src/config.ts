// API Configuration
export const API_URL = import.meta.env.VITE_API_URL || 'http://localhost:3001';

// RPC URL - Production uses public community RPC, development uses local
// Can be overridden by API response
export const DEFAULT_RPC_URL = import.meta.env.VITE_RPC_URL || 
  (import.meta.env.MODE === 'production' 
    ? 'https://rpc.securecheck.io' 
    : 'http://localhost:8899');

// Export as RPC_URL for backward compatibility
export const RPC_URL = DEFAULT_RPC_URL;

// Solana network (mainnet-beta)
export const SOLANA_NETWORK = 'mainnet-beta';
