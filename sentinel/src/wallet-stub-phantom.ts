/**
 * Phantom SDK-based wallet stub for Sentinel
 * 
 * This uses @phantom/browser-injected-sdk to create a fully-compatible
 * Phantom wallet that phishing sites will recognize, but intercepts
 * signing operations to capture transactions without actually signing.
 */

import { Keypair } from '@solana/web3.js';

export function generatePhantomWalletStub(keypair: Keypair): string {
  const publicKeyBase58 = keypair.publicKey.toBase58();
  const secretKeyArray = Array.from(keypair.secretKey);

  return `
(async function() {
  console.log('[Sentinel] Injecting Phantom SDK wallet...');
  
  // Import Phantom SDK from CDN
  const { createPhantom } = await import('https://cdn.jsdelivr.net/npm/@phantom/browser-injected-sdk@1.0.7/+esm');
  const { createSolanaPlugin } = await import('https://cdn.jsdelivr.net/npm/@phantom/browser-injected-sdk@1.0.7/solana/+esm');
  
  // Create keypair from secret key
  const secretKey = new Uint8Array(${JSON.stringify(secretKeyArray)});
  
  // Create Phantom instance with Solana plugin
  const phantom = createPhantom({
    plugins: [createSolanaPlugin({
      keypair: secretKey,
    })],
  });
  
  // Storage for captured transactions
  window.__capturedTx = null;
  window.__capturedTxType = null;
  window.__connectCalled = false;
  
  // Wrap the Solana plugin methods to capture transactions
  const originalSignAndSendTransaction = phantom.solana.signAndSendTransaction.bind(phantom.solana);
  const originalSignTransaction = phantom.solana.signTransaction.bind(phantom.solana);
  const originalSignAllTransactions = phantom.solana.signAllTransactions.bind(phantom.solana);
  const originalConnect = phantom.solana.connect.bind(phantom.solana);
  
  // Intercept connect to track calls
  phantom.solana.connect = async function(opts) {
    console.log('[Sentinel] ✅ connect() called with opts:', opts);
    window.__connectCalled = true;
    return await originalConnect(opts);
  };
  
  // Intercept signAndSendTransaction
  phantom.solana.signAndSendTransaction = async function(transaction, options) {
    console.log('[Sentinel] 🎯 signAndSendTransaction called!');
    
    // Capture the transaction
    const txBytes = transaction.serialize ? 
      transaction.serialize({ requireAllSignatures: false }) : 
      transaction;
    window.__capturedTx = Buffer.from(txBytes).toString('base64');
    window.__capturedTxType = 'signAndSendTransaction';
    
    console.log('[Sentinel] ✅ Transaction captured! Length:', window.__capturedTx.length);
    
    // Reject to prevent actual sending
    throw new Error('User rejected the request');
  };
  
  // Intercept signTransaction
  phantom.solana.signTransaction = async function(transaction) {
    console.log('[Sentinel] 🎯 signTransaction called!');
    
    const txBytes = transaction.serialize ? 
      transaction.serialize({ requireAllSignatures: false }) : 
      transaction;
    window.__capturedTx = Buffer.from(txBytes).toString('base64');
    window.__capturedTxType = 'signTransaction';
    
    console.log('[Sentinel] ✅ Transaction captured! Length:', window.__capturedTx.length);
    
    throw new Error('User rejected the request');
  };
  
  // Intercept signAllTransactions
  phantom.solana.signAllTransactions = async function(transactions) {
    console.log('[Sentinel] 🎯 signAllTransactions called! Count:', transactions.length);
    
    if (transactions.length > 0) {
      const txBytes = transactions[0].serialize ? 
        transactions[0].serialize({ requireAllSignatures: false }) : 
        transactions[0];
      window.__capturedTx = Buffer.from(txBytes).toString('base64');
      window.__capturedTxType = 'signAllTransactions';
      
      console.log('[Sentinel] ✅ First transaction captured! Length:', window.__capturedTx.length);
    }
    
    throw new Error('User rejected the request');
  };
  
  // Inject into window
  window.phantom = phantom;
  window.solana = phantom.solana;
  
  console.log('[Sentinel] ✅ Phantom wallet ready!');
  console.log('[Sentinel] Public key:', '${publicKeyBase58}');
  console.log('[Sentinel] isPhantom:', phantom.solana.isPhantom);
  
})().catch(err => {
  console.error('[Sentinel] Failed to inject Phantom wallet:', err);
});
`;
}
