#!/usr/bin/env node
/**
 * Sol-Shield MCP Server for OpenClaw
 * 
 * Provides secure Solana transaction tools with Sol-Shield protection
 * and human-in-the-loop escalation support.
 * 
 * Features:
 *   - Secure RPC proxy with transaction analysis
 *   - Human-in-the-loop approval for blocked transactions
 *   - Automatic retry with fresh blockhash after approval
 *   - Dual-path support (fast-forward or slow-path with rule creation)
 * 
 * Setup:
 *   1. Set SOL_SHIELD_RPC_URL (proxy endpoint, e.g., http://localhost:8899)
 *   2. Set SOL_SHIELD_API_URL (API endpoint, e.g., http://localhost:3001)
 *   3. Enable escalations in proxy: ENABLE_ESCALATIONS=true
 *   4. Set ESCALATION_APPROVER_WALLET in proxy config
 *   5. Add to OpenClaw MCP config
 * 
 * Environment Variables:
 *   - SOL_SHIELD_RPC_URL: Sol-Shield RPC proxy URL (default: http://localhost:8899)
 *   - SOL_SHIELD_API_URL: Sol-Shield API URL (default: http://localhost:3001)
 */

import { Connection, PublicKey, Transaction, SystemProgram, Keypair } from '@solana/web3.js';
import { Server } from '@modelcontextprotocol/sdk/server/index.js';
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';

const SOL_SHIELD_RPC = process.env.SOL_SHIELD_RPC_URL || 'http://localhost:8899';
const connection = new Connection(SOL_SHIELD_RPC, 'confirmed');

// MCP Server setup
const server = new Server(
  {
    name: 'parapet-solana',
    version: '1.0.0',
  },
  {
    capabilities: {
      tools: {},
    },
  }
);

// Tool: Simulate transaction with security analysis
server.setRequestHandler('tools/call', async (request) => {
  if (request.params.name === 'solana_simulate_safe') {
    const { transaction_base58 } = request.params.arguments as any;
    
    const sim = await connection.simulateTransaction(
      Transaction.from(Buffer.from(transaction_base58, 'base58'))
    );
    
    const shield = (sim.value as any).solShield;
    
    return {
      content: [
        {
          type: 'text',
          text: JSON.stringify({
            success: !sim.value.err,
            solShield: shield,
            logs: sim.value.logs
          }, null, 2)
        }
      ]
    };
  }
  
  if (request.params.name === 'solana_send_safe') {
    const { transaction_base58, signer_private_key, approve_escalations } = request.params.arguments as any;
    
    // Load signer
    const signer = Keypair.fromSecretKey(Buffer.from(signer_private_key, 'base58'));
    const tx = Transaction.from(Buffer.from(transaction_base58, 'base58'));
    
    try {
      // Attempt to send transaction
      const sig = await connection.sendTransaction(tx, [signer]);
      
      return {
        content: [{
          type: 'text',
          text: `✅ Transaction sent: ${sig}`
        }]
      };
    } catch (error: any) {
      // Check if this is an EscalationRequired error
      if (error.code === -32005 && error.data?.escalation_id) {
        const { escalation_id, canonical_hash, risk_score, message } = error.data;
        
        if (!approve_escalations) {
          return {
            content: [{
              type: 'text',
              text: `🚨 ESCALATION REQUIRED\n\nTransaction blocked and requires human approval.\n\nEscalation ID: ${escalation_id}\nCanonical Hash: ${canonical_hash}\nRisk Score: ${risk_score}\nReason: ${message}\n\nTo approve, set approve_escalations=true and retry after human approves via dashboard.`
            }]
          };
        }
        
        // Poll for escalation approval
        const API_URL = process.env.SOL_SHIELD_API_URL || 'http://localhost:3001';
        const MAX_WAIT = 120; // 2 minutes
        const POLL_INTERVAL = 3; // 3 seconds
        
        let elapsed = 0;
        while (elapsed < MAX_WAIT) {
          await new Promise(resolve => setTimeout(resolve, POLL_INTERVAL * 1000));
          elapsed += POLL_INTERVAL;
          
          // Check escalation status
          const statusResp = await fetch(`${API_URL}/api/v1/escalations/${escalation_id}/status`);
          if (!statusResp.ok) continue;
          
          const status = await statusResp.json();
          
          if (status.status === 'approved_fast_path' || status.status === 'forwarded') {
            // Transaction was already forwarded
            return {
              content: [{
                type: 'text',
                text: `✅ Transaction approved and forwarded (fast path)\n\nEscalation: ${escalation_id}\nApproval time: ${elapsed}s`
              }]
            };
          }
          
          if (status.status === 'approved' || status.status === 'approved_slow_path') {
            // Approval received, rebuild transaction with fresh blockhash and retry
            const { blockhash } = await connection.getLatestBlockhash();
            const newTx = Transaction.from(tx.serialize({ requireAllSignatures: false }));
            newTx.recentBlockhash = blockhash;
            newTx.sign(signer);
            
            const sig = await connection.sendTransaction(newTx, [signer]);
            
            return {
              content: [{
                type: 'text',
                text: `✅ Transaction approved and sent (slow path)\n\nSignature: ${sig}\nEscalation: ${escalation_id}\nApproval time: ${elapsed}s\n\nNote: Transaction was rebuilt with fresh blockhash.`
              }]
            };
          }
          
          if (status.status === 'denied') {
            return {
              content: [{
                type: 'text',
                text: `🚫 Transaction DENIED by human approver\n\nEscalation: ${escalation_id}\nWait time: ${elapsed}s`
              }]
            };
          }
          
          if (status.status === 'expired') {
            return {
              content: [{
                type: 'text',
                text: `⏱️  Escalation EXPIRED (no response)\n\nEscalation: ${escalation_id}\nWait time: ${elapsed}s`
              }]
            };
          }
        }
        
        return {
          content: [{
            type: 'text',
            text: `⏱️  Timeout waiting for approval\n\nEscalation: ${escalation_id}\nWaited: ${MAX_WAIT}s\n\nCheck dashboard for status.`
          }]
        };
      }
      
      // Other errors
      throw error;
    }
  }
  
  throw new Error('Unknown tool');
});

// List available tools
server.setRequestHandler('tools/list', async () => ({
  tools: [
    {
      name: 'solana_simulate_safe',
      description: 'Simulate Solana transaction with Sol-Shield security analysis',
      inputSchema: {
        type: 'object',
        properties: {
          transaction_base58: {
            type: 'string',
            description: 'Base58 encoded transaction'
          }
        },
        required: ['transaction_base58']
      }
    },
    {
      name: 'solana_send_safe',
      description: 'Send Solana transaction with Sol-Shield protection. Supports human-in-the-loop escalations for blocked transactions.',
      inputSchema: {
        type: 'object',
        properties: {
          transaction_base58: {
            type: 'string',
            description: 'Base58 encoded transaction'
          },
          signer_private_key: {
            type: 'string',
            description: 'Signer private key (base58)'
          },
          approve_escalations: {
            type: 'boolean',
            description: 'If true, polls for human approval when transaction is blocked and requires escalation. If false, returns escalation details immediately.',
            default: false
          }
        },
        required: ['transaction_base58', 'signer_private_key']
      }
    }
  ]
}));

// Start server
const transport = new StdioServerTransport();
await server.connect(transport);
