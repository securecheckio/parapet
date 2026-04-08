import { VersionedTransaction, Connection, PublicKey } from '@solana/web3.js';

export interface ProgramInfo {
  address: string;
  known: boolean;
  name?: string;
}

export interface RuleMatch {
  id: string;
  action: string;
  message: string;
  severity?: string;
}

export interface AnalysisResult {
  success: boolean;
  programs: ProgramInfo[];
  rulesMatched: RuleMatch[];
  riskLevel: string;
  verdict: string;
  simulationResult?: any;
  error?: string;
}

/**
 * Known Solana programs
 */
const KNOWN_PROGRAMS: Record<string, string> = {
  '11111111111111111111111111111111': 'System Program',
  'TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA': 'Token Program',
  'TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb': 'Token-2022 Program',
  'ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL': 'Associated Token Program',
  'ComputeBudget111111111111111111111111111111': 'Compute Budget Program',
  'Memo1UhkJRfHyvLMcVucJwxXeuD728EqVDDwQDxFMNo': 'Memo Program',
  'MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr': 'Memo Program v2',
};

/**
 * Analyze a captured transaction
 */
export async function analyzeTransaction(
  capturedTx: string,
  throwawayWallet: string,
  rpcUrl: string
): Promise<AnalysisResult> {
  try {
    // Decode the transaction
    const txBuffer = Buffer.from(capturedTx, 'base64');
    const transaction = VersionedTransaction.deserialize(txBuffer);
    
    // Extract program IDs
    const programs = extractPrograms(transaction);
    
    // Create connection to parapet proxy
    const connection = new Connection(rpcUrl, 'confirmed');
    
    // Simulate the transaction to get parapet analysis
    const simulationResult = await simulateWithParapet(
      connection,
      transaction,
      throwawayWallet
    );
    
    // Parse parapet metadata from simulation result
    const analysis = parseAnalysisFromSimulation(simulationResult, programs);
    
    return {
      success: true,
      ...analysis
    };
  } catch (error) {
    return {
      success: false,
      programs: [],
      rulesMatched: [],
      riskLevel: 'unknown',
      verdict: 'ANALYSIS_FAILED',
      error: error instanceof Error ? error.message : String(error)
    };
  }
}

/**
 * Extract program IDs from a transaction
 */
function extractPrograms(transaction: VersionedTransaction): ProgramInfo[] {
  const programs: ProgramInfo[] = [];
  const seen = new Set<string>();
  
  try {
    // Get all account keys
    const message = transaction.message;
    const accountKeys = message.staticAccountKeys || [];
    
    // Extract program IDs (programs are typically at the end of account keys)
    // In Solana, instructions reference programs by index
    const instructions = message.compiledInstructions || [];
    
    for (const ix of instructions) {
      const programIdIndex = ix.programIdIndex;
      if (programIdIndex < accountKeys.length) {
        const programId = accountKeys[programIdIndex].toBase58();
        
        if (!seen.has(programId)) {
          seen.add(programId);
          programs.push({
            address: programId,
            known: programId in KNOWN_PROGRAMS,
            name: KNOWN_PROGRAMS[programId]
          });
        }
      }
    }
    
    // If no programs found via instructions, check all account keys
    // (some might be program accounts)
    if (programs.length === 0) {
      for (const key of accountKeys) {
        const address = key.toBase58();
        if (address in KNOWN_PROGRAMS && !seen.has(address)) {
          seen.add(address);
          programs.push({
            address,
            known: true,
            name: KNOWN_PROGRAMS[address]
          });
        }
      }
    }
  } catch (error) {
    console.error('Error extracting programs:', error);
  }
  
  return programs;
}

/**
 * Simulate transaction with parapet proxy
 */
async function simulateWithParapet(
  connection: Connection,
  transaction: VersionedTransaction,
  signerPubkey: string
): Promise<any> {
  try {
    // Simulate the transaction
    // The parapet proxy will intercept this and inject solShield metadata
    const result = await connection.simulateTransaction(transaction, {
      sigVerify: false,
      commitment: 'confirmed',
      replaceRecentBlockhash: true,
    });
    
    return result;
  } catch (error) {
    console.error('Simulation error:', error);
    throw error;
  }
}

/**
 * Parse analysis from simulation result with solShield metadata
 */
function parseAnalysisFromSimulation(
  simulationResult: any,
  programs: ProgramInfo[]
): Omit<AnalysisResult, 'success'> {
  // Check if solShield metadata is present
  const solShield = simulationResult.value?.solShield;
  
  if (solShield) {
    // Parse parapet analysis
    const rulesMatched: RuleMatch[] = [];
    
    if (solShield.matched_rules) {
      for (const rule of solShield.matched_rules) {
        rulesMatched.push({
          id: rule.id || rule.rule_id || 'unknown',
          action: rule.action || 'unknown',
          message: rule.message || '',
          severity: rule.severity
        });
      }
    }
    
    // Determine risk level
    let riskLevel = 'low';
    let verdict = 'SAFE';
    
    if (solShield.action === 'block' || solShield.decision === 'block') {
      riskLevel = 'critical';
      verdict = 'MALICIOUS';
    } else if (solShield.action === 'alert' || solShield.decision === 'alert') {
      riskLevel = 'high';
      verdict = 'SUSPICIOUS';
    } else if (rulesMatched.length > 0) {
      riskLevel = 'medium';
      verdict = 'SUSPICIOUS';
    }
    
    // Check for unknown programs
    const unknownPrograms = programs.filter(p => !p.known);
    if (unknownPrograms.length > 0 && riskLevel === 'low') {
      riskLevel = 'medium';
      verdict = 'SUSPICIOUS';
    }
    
    return {
      programs,
      rulesMatched,
      riskLevel,
      verdict,
      simulationResult: solShield
    };
  } else {
    // No solShield metadata - analyze manually
    const unknownPrograms = programs.filter(p => !p.known);
    
    let riskLevel = 'low';
    let verdict = 'UNKNOWN';
    
    if (unknownPrograms.length > 0) {
      riskLevel = 'high';
      verdict = 'SUSPICIOUS';
    } else if (programs.length === 0) {
      riskLevel = 'unknown';
      verdict = 'ANALYSIS_INCOMPLETE';
    } else {
      verdict = 'SAFE';
    }
    
    return {
      programs,
      rulesMatched: [],
      riskLevel,
      verdict,
      simulationResult: simulationResult.value
    };
  }
}
