#!/usr/bin/env ts-node
/**
 * Sol-Shield Agent Integration Example (TypeScript)
 * 
 * This demonstrates how an AI agent should interact with Sol-Shield RPC proxy.
 * 
 * Installation:
 *   npm install @solana/web3.js
 *   npm install -g ts-node typescript
 * 
 * Usage:
 *   ts-node agent-integration-example.ts
 */

import { Connection, PublicKey, Transaction, SystemProgram, Keypair } from '@solana/web3.js';

// Type definitions for Sol-Shield metadata
interface ParapetWarning {
  severity: 'low' | 'medium' | 'high' | 'critical';
  message: string;
  ruleId: string;
  ruleName: string;
  weight: number;
}

interface ParapetMetadata {
  version: string;
  riskScore: number;
  structuralRisk: number;
  simulationRisk: number;
  decision: 'safe' | 'alert' | 'would_block';
  threshold: number;
  warnings: ParapetWarning[];
  analysis: {
    matchedRules: number;
    totalWeight: number;
    wouldBlock: boolean;
  };
}

interface EnrichedSimulationResult {
  err: any | null;
  logs: string[] | null;
  accounts: any[] | null;
  unitsConsumed: number;
  solShield?: ParapetMetadata;
}

/**
 * AI Agent with Sol-Shield integration
 */
class ParapetAgent {
  private connection: Connection;
  private maxRiskScore: number;
  
  constructor(rpcUrl: string, maxRiskScore: number = 50) {
    this.connection = new Connection(rpcUrl, 'confirmed');
    this.maxRiskScore = maxRiskScore;
  }
  
  /**
   * Health check - verify Sol-Shield is accessible
   */
  async healthCheck(): Promise<boolean> {
    try {
      const health = await fetch(`${this.connection.rpcEndpoint}/health`);
      return health.status === 200;
    } catch (error) {
      console.error('❌ Health check failed:', error);
      return false;
    }
  }
  
  /**
   * Simulate transaction and get Sol-Shield analysis
   */
  async simulateTransactionSafe(tx: Transaction): Promise<{
    success: boolean;
    solShield?: ParapetMetadata;
    logs?: string[];
    error?: string;
  }> {
    try {
      console.log('🔬 Simulating transaction...');
      
      const simulation = await this.connection.simulateTransaction(tx);
      const result = simulation.value as EnrichedSimulationResult;
      
      // Check for simulation failure
      if (result.err) {
        console.error('❌ Simulation failed:', result.err);
        return { success: false, error: 'Simulation failed' };
      }
      
      // Extract Sol-Shield metadata
      const solShield = result.solShield;
      
      if (!solShield) {
        console.warn('⚠️  No Sol-Shield metadata - RPC may not be protected');
        return { success: true, logs: result.logs || [] };
      }
      
      // Log analysis
      console.log(`📊 Risk Score: ${solShield.riskScore}`);
      console.log(`📊 Decision: ${solShield.decision}`);
      console.log(`📊 Threshold: ${solShield.threshold}`);
      
      if (solShield.warnings.length > 0) {
        console.warn(`⚠️  ${solShield.warnings.length} security warning(s):`);
        for (const warning of solShield.warnings) {
          console.warn(`   - [${warning.severity.toUpperCase()}] ${warning.message}`);
        }
      } else {
        console.log('✅ No security warnings');
      }
      
      return {
        success: true,
        solShield,
        logs: result.logs || []
      };
      
    } catch (error: any) {
      console.error('❌ Simulation error:', error.message);
      return { success: false, error: error.message };
    }
  }
  
  /**
   * Send transaction with Sol-Shield protection
   */
  async sendTransactionSafely(
    tx: Transaction,
    signers: Keypair[]
  ): Promise<{ success: boolean; signature?: string; error?: string }> {
    try {
      // Step 1: Simulate first
      const simulation = await this.simulateTransactionSafe(tx);
      
      if (!simulation.success) {
        return { success: false, error: 'Simulation failed' };
      }
      
      const { solShield } = simulation;
      
      // Step 2: Check if would be blocked
      if (solShield?.wouldBlock) {
        console.error('🚫 Transaction would be BLOCKED');
        this.logSecurityBlock(solShield);
        return { success: false, error: 'Blocked by security rules' };
      }
      
      // Step 3: Check against agent's risk tolerance
      if (solShield && solShield.riskScore > this.maxRiskScore) {
        console.warn(`⚠️  Risk ${solShield.riskScore} exceeds threshold ${this.maxRiskScore}`);
        this.logHighRisk(solShield);
        return { success: false, error: 'Risk score too high' };
      }
      
      // Step 4: Send transaction
      console.log('📤 Sending transaction...');
      const signature = await this.connection.sendTransaction(tx, signers);
      console.log('✅ Transaction sent:', signature);
      
      // Step 5: Confirm
      console.log('⏳ Confirming transaction...');
      await this.connection.confirmTransaction(signature);
      console.log('✅ Transaction confirmed');
      
      return { success: true, signature };
      
    } catch (error: any) {
      // Handle Sol-Shield specific errors
      if (error.code === -32004) {
        console.error('🚫 BLOCKED by Sol-Shield:', error.message);
        return { success: false, error: `Blocked: ${error.message}` };
      }
      
      if (error.code === -32005) {
        console.error('⏳ Rate limit exceeded');
        return { success: false, error: 'Rate limited' };
      }
      
      console.error('❌ Transaction failed:', error.message);
      return { success: false, error: error.message };
    }
  }
  
  /**
   * Log blocked transaction for security audit
   */
  private logSecurityBlock(solShield: ParapetMetadata) {
    console.error('\n🔒 SECURITY BLOCK EVENT');
    console.error(`   Timestamp: ${new Date().toISOString()}`);
    console.error(`   Risk Score: ${solShield.riskScore}`);
    console.error(`   Threshold: ${solShield.threshold}`);
    console.error(`   Warnings:`);
    for (const warning of solShield.warnings) {
      console.error(`     - [${warning.severity}] ${warning.message}`);
      console.error(`       Rule: ${warning.ruleName} (${warning.ruleId})`);
      console.error(`       Weight: ${warning.weight}`);
    }
    console.error('');
  }
  
  /**
   * Log high-risk transaction for review
   */
  private logHighRisk(solShield: ParapetMetadata) {
    console.warn('\n⚠️  HIGH RISK EVENT');
    console.warn(`   Timestamp: ${new Date().toISOString()}`);
    console.warn(`   Risk Score: ${solShield.riskScore}`);
    console.warn(`   Agent Threshold: ${this.maxRiskScore}`);
    console.warn(`   Status: Escalated for review`);
    console.warn('');
  }
  
  /**
   * Log security warnings for audit trail
   */
  private logWarnings(warnings: ParapetWarning[]) {
    console.log('\n📝 Security Warnings:');
    for (const warning of warnings) {
      console.log(`   [${warning.severity.toUpperCase()}] ${warning.message}`);
      console.log(`      Rule: ${warning.ruleName} (weight: ${warning.weight})`);
    }
    console.log('');
  }
}

/**
 * Demo: Test basic integration
 */
async function demo() {
  const RPC_URL = process.env.SOL_SHIELD_RPC_URL || 'http://localhost:8899';
  
  console.log('🤖 Sol-Shield Agent Integration Demo');
  console.log('=====================================\n');
  
  // Create agent
  const agent = new ParapetAgent(RPC_URL, 50);
  
  // Test 1: Health check
  console.log('Test 1: Health Check');
  console.log('────────────────────────────────────────');
  const isHealthy = await agent.healthCheck();
  if (isHealthy) {
    console.log('✅ Sol-Shield is running and accessible\n');
  } else {
    console.error('❌ Sol-Shield is not accessible\n');
    process.exit(1);
  }
  
  // Test 2: Test pass-through RPC methods
  console.log('Test 2: Pass-through RPC Methods');
  console.log('────────────────────────────────────────');
  try {
    const slot = await agent['connection'].getSlot();
    console.log(`✅ Current slot: ${slot}`);
    
    const blockHeight = await agent['connection'].getBlockHeight();
    console.log(`✅ Block height: ${blockHeight}\n`);
  } catch (error: any) {
    console.error('❌ Pass-through test failed:', error.message);
  }
  
  console.log('═══════════════════════════════════════════');
  console.log('✅ Basic Integration Tests Complete');
  console.log('═══════════════════════════════════════════\n');
  
  console.log('📋 Integration Status:');
  console.log('   [✅] RPC connectivity');
  console.log('   [✅] Health check');
  console.log('   [✅] Pass-through methods');
  console.log('   [⏳] Transaction simulation (requires test transaction)');
  console.log('   [⏳] Transaction sending (requires test transaction + signer)\n');
  
  console.log('🎯 To Test Transaction Analysis:');
  console.log('   1. Build a test transaction using your agent logic');
  console.log('   2. Call agent.simulateTransactionSafe(tx)');
  console.log('   3. Review the solShield metadata');
  console.log('   4. Call agent.sendTransactionSafely(tx, signers) if safe\n');
  
  console.log('📚 See AI_AGENT_INTEGRATION_GUIDE.md for complete guide\n');
}

// Run demo
demo().catch(console.error);
