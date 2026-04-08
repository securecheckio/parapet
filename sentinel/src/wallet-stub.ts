/**
 * Generate the wallet stub code that will be injected into the page
 * This mimics the Phantom wallet interface but captures transactions without signing
 */
export function generateWalletStub(publicKeyBase58: string): string {
  return `
(function() {
  'use strict';
  
  // Prevent multiple injections
  if (window.__solanaStubInjected) {
    return;
  }
  window.__solanaStubInjected = true;
  
  console.log('[SecureCheck] Wallet stub injected');
  
  // Storage for captured transaction
  window.__capturedTx = null;
  window.__capturedTxType = null;
  
  // Create a fake PublicKey class
  class PublicKey {
    constructor(value) {
      this._bn = value;
    }
    
    toBase58() {
      return '${publicKeyBase58}';
    }
    
    toString() {
      return this.toBase58();
    }
    
    toBuffer() {
      // Return a dummy buffer
      return new Uint8Array(32);
    }
    
    equals(other) {
      return other && other.toBase58() === this.toBase58();
    }
  }
  
  // Create the fake Solana wallet object
  const solanaWallet = {
    isPhantom: true,
    isConnected: false,
    publicKey: null,
    
    // Auto-connect when requested
    connect: async function(opts) {
      console.log('[SecureCheck] ✅ connect() WAS CALLED!', opts);
      window.__connectCalled = true;
      
      // Small delay to mimic real wallet
      await new Promise(resolve => setTimeout(resolve, 100));
      
      this.isConnected = true;
      this.publicKey = new PublicKey('${publicKeyBase58}');
      
      // Dispatch connect event
      window.dispatchEvent(new CustomEvent('connect', { 
        detail: { publicKey: this.publicKey }
      }));
      
      // Also dispatch on the solana object itself
      if (this.on) {
        window.dispatchEvent(new Event('solana#connect'));
      }
      
      console.log('[SecureCheck] Connected! Public key:', this.publicKey.toBase58());
      
      return { publicKey: this.publicKey };
    },
    
    // Disconnect
    disconnect: async function() {
      console.log('[SecureCheck] disconnect() called');
      this.isConnected = false;
      this.publicKey = null;
      
      window.dispatchEvent(new Event('disconnect'));
    },
    
    // Capture signTransaction - the main interception point
    signTransaction: async function(transaction) {
      console.log('[SecureCheck] signTransaction() called - CAPTURING');
      
      try {
        // Serialize the transaction to capture it
        let serialized;
        if (transaction.serialize) {
          // Try to serialize (may fail if not fully populated)
          try {
            serialized = transaction.serialize({ 
              requireAllSignatures: false,
              verifySignatures: false 
            });
          } catch (e) {
            console.log('[SecureCheck] Could not serialize, trying message');
            // Fallback: capture the message
            if (transaction.message) {
              serialized = transaction.message.serialize();
            }
          }
        }
        
        if (serialized) {
          // Convert to base64
          const base64 = btoa(String.fromCharCode.apply(null, serialized));
          window.__capturedTx = base64;
          window.__capturedTxType = 'signTransaction';
          console.log('[SecureCheck] Transaction captured:', base64.substring(0, 50) + '...');
        } else {
          // Store the raw object as fallback
          window.__capturedTx = JSON.stringify(transaction);
          window.__capturedTxType = 'signTransaction';
          console.log('[SecureCheck] Transaction object captured (could not serialize)');
        }
      } catch (error) {
        console.error('[SecureCheck] Error capturing transaction:', error);
        window.__capturedTx = 'ERROR: ' + error.message;
        window.__capturedTxType = 'signTransaction';
      }
      
      // Reject with user cancellation so the site doesn't get a signed tx
      throw new Error('User rejected the request');
    },
    
    // Capture signAllTransactions
    signAllTransactions: async function(transactions) {
      console.log('[SecureCheck] signAllTransactions() called - CAPTURING');
      
      try {
        const captured = [];
        for (const tx of transactions) {
          let serialized;
          if (tx.serialize) {
            try {
              serialized = tx.serialize({ 
                requireAllSignatures: false,
                verifySignatures: false 
              });
              const base64 = btoa(String.fromCharCode.apply(null, serialized));
              captured.push(base64);
            } catch (e) {
              captured.push('ERROR: ' + e.message);
            }
          }
        }
        
        window.__capturedTx = JSON.stringify(captured);
        window.__capturedTxType = 'signAllTransactions';
        console.log('[SecureCheck] Multiple transactions captured:', captured.length);
      } catch (error) {
        console.error('[SecureCheck] Error capturing transactions:', error);
        window.__capturedTx = 'ERROR: ' + error.message;
        window.__capturedTxType = 'signAllTransactions';
      }
      
      throw new Error('User rejected the request');
    },
    
    // Capture signAndSendTransaction
    signAndSendTransaction: async function(transaction, options) {
      console.log('[SecureCheck] signAndSendTransaction() called - CAPTURING');
      
      // Same capture logic as signTransaction
      try {
        let serialized;
        if (transaction.serialize) {
          try {
            serialized = transaction.serialize({ 
              requireAllSignatures: false,
              verifySignatures: false 
            });
            const base64 = btoa(String.fromCharCode.apply(null, serialized));
            window.__capturedTx = base64;
            window.__capturedTxType = 'signAndSendTransaction';
            console.log('[SecureCheck] Transaction captured:', base64.substring(0, 50) + '...');
          } catch (e) {
            window.__capturedTx = 'ERROR: ' + e.message;
            window.__capturedTxType = 'signAndSendTransaction';
          }
        }
      } catch (error) {
        console.error('[SecureCheck] Error capturing transaction:', error);
        window.__capturedTx = 'ERROR: ' + error.message;
        window.__capturedTxType = 'signAndSendTransaction';
      }
      
      throw new Error('User rejected the request');
    },
    
    // Capture signMessage
    signMessage: async function(message, encoding) {
      console.log('[SecureCheck] signMessage() called - CAPTURING');
      
      window.__capturedTx = typeof message === 'string' ? message : btoa(String.fromCharCode.apply(null, message));
      window.__capturedTxType = 'signMessage';
      
      throw new Error('User rejected the request');
    },
    
    // Event listeners (for compatibility)
    on: function(event, callback) {
      window.addEventListener(event, callback);
    },
    
    off: function(event, callback) {
      window.removeEventListener(event, callback);
    }
  };
  
  // Inject into window
  Object.defineProperty(window, 'solana', {
    value: solanaWallet,
    writable: false,
    configurable: false
  });
  
  // Also expose as phantom for sites that check specifically for Phantom
  Object.defineProperty(window, 'phantom', {
    value: { solana: solanaWallet },
    writable: false,
    configurable: false
  });
  
  console.log('[SecureCheck] Wallet stub ready. Public key:', '${publicKeyBase58}');
  
  // Dispatch wallet ready events that wallet adapters listen for
  setTimeout(() => {
    window.dispatchEvent(new Event('solana#initialized'));
    window.dispatchEvent(new Event('phantom#initialized'));
    console.log('[SecureCheck] Dispatched wallet ready events');
  }, 100);
})();
  `.trim();
}
