const { chromium } = require('playwright');

(async () => {
  const browser = await chromium.launch({ headless: false });
  const page = await browser.newPage();
  
  // Inject a fake wallet BEFORE the page loads
  await page.addInitScript(() => {
    // Use a WHALE wallet with tons of tokens and SOL (43k+ SOL, 892 tokens)
    const FAKE_PUBKEY = 'GJRs4FwHtemZ5ZE9x3FNvJ8TMwitKTh21yxdRPqn7npE';
    
    console.log('[FAKE WALLET] Installing fake Phantom wallet...');
    
    window.__capturedTransaction = null;
    
    window.solana = {
      isPhantom: true,
      isConnected: false,
      publicKey: null,
      
      connect: async function(opts) {
        console.log('[FAKE WALLET] ✅✅✅ connect() WAS CALLED! ✅✅✅');
        console.log('[FAKE WALLET] Options:', opts);
        
        // Small delay to mimic real wallet
        await new Promise(resolve => setTimeout(resolve, 500));
        
        this.isConnected = true;
        this.publicKey = {
          toString: () => FAKE_PUBKEY,
          toBase58: () => FAKE_PUBKEY,
          toBuffer: () => new Uint8Array(32),
          toBytes: () => new Uint8Array(32),
        };
        
        console.log('[FAKE WALLET] Set isConnected =', this.isConnected);
        console.log('[FAKE WALLET] Set publicKey =', this.publicKey.toString());
        
        // Dispatch multiple connect events
        window.dispatchEvent(new CustomEvent('connect', { detail: { publicKey: this.publicKey } }));
        window.dispatchEvent(new Event('solana#connect'));
        
        if (this.emit) {
          this.emit('connect', this.publicKey);
        }
        
        return { publicKey: this.publicKey };
      },
      
      disconnect: async function() {
        console.log('[FAKE WALLET] disconnect() called');
        this.isConnected = false;
        this.publicKey = null;
      },
      
      signTransaction: async function(transaction) {
        console.log('[FAKE WALLET] 🎯 signTransaction() called!');
        console.log('[FAKE WALLET] Transaction type:', transaction.constructor.name);
        
        // Serialize the transaction to base64
        let txBytes;
        if (transaction.serialize) {
          txBytes = transaction.serialize({ requireAllSignatures: false, verifySignatures: false });
        } else if (transaction instanceof Uint8Array) {
          txBytes = transaction;
        } else {
          txBytes = new Uint8Array(transaction);
        }
        
        const txBase64 = btoa(String.fromCharCode(...txBytes));
        window.__capturedTransaction = txBase64;
        
        console.log('[FAKE WALLET] ✅ CAPTURED TRANSACTION!');
        console.log('[FAKE WALLET] Length:', txBase64.length);
        console.log('[FAKE WALLET] Base64:', txBase64.substring(0, 100) + '...');
        
        // Reject to prevent actual signing
        throw new Error('User rejected the request');
      },
      
      signAllTransactions: async function(transactions) {
        console.log('[FAKE WALLET] 🎯 signAllTransactions() called! Count:', transactions.length);
        
        if (transactions.length > 0) {
          return this.signTransaction(transactions[0]);
        }
        
        throw new Error('User rejected the request');
      },
      
      signAndSendTransaction: async function(transaction, options) {
        console.log('[FAKE WALLET] 🎯 signAndSendTransaction() called!');
        
        // Capture the transaction first
        await this.signTransaction(transaction);
        
        // Then reject
        throw new Error('User rejected the request');
      },
      
      signMessage: async function(message) {
        console.log('[FAKE WALLET] signMessage() called');
        throw new Error('User rejected the request');
      },
    };
    
    // Also expose as phantom
    window.phantom = { solana: window.solana };
    
    // Dispatch ready events
    setTimeout(() => {
      window.dispatchEvent(new Event('solana#initialized'));
      window.dispatchEvent(new Event('phantom#initialized'));
      console.log('[FAKE WALLET] Ready events dispatched');
    }, 100);
    
    // Add event emitter capability
    window.solana.on = function(event, callback) {
      console.log('[FAKE WALLET] Listener registered for:', event);
      window.addEventListener(`solana-${event}`, callback);
    };
    
    window.solana.emit = function(event, data) {
      window.dispatchEvent(new CustomEvent(`solana-${event}`, { detail: data }));
    };
    
    console.log('[FAKE WALLET] ✅ Fake wallet installed with pubkey:', FAKE_PUBKEY);
  });
  
  console.log('Navigating to phishing site...');
  // Try multiple phishing sites
  const sites = [
    'https://rewards-solana.org/',
    'https://www.solanamysterybox.com',
    'https://xtrialsolana.xyz',
  ];
  
  let siteLoaded = false;
  for (const site of sites) {
    try {
      console.log(`Trying ${site}...`);
      await page.goto(site, { timeout: 10000 });
      await page.waitForTimeout(2000);
      
      // Check if page loaded
      const content = await page.content();
      if (content.length > 1000) {
        console.log(`✅ Loaded: ${site}`);
        siteLoaded = true;
        break;
      }
    } catch (e) {
      console.log(`❌ Failed: ${site} - ${e.message}`);
    }
  }
  
  if (!siteLoaded) {
    console.log('All sites failed, exiting');
    await browser.close();
    return;
  }
  
  await page.waitForTimeout(2000);

  console.log('Clicking Connect Wallet...');
  await page.click('text=Connect Wallet');
  await page.waitForTimeout(2000);

  console.log('Clicking Phantom...');
  await page.click('text=Phantom');
  await page.waitForTimeout(1000);
  
  // Manually trigger connect since the site isn't calling it
  console.log('Manually triggering connect()...');
  await page.evaluate(async () => {
    if (window.solana && window.solana.connect) {
      try {
        const result = await window.solana.connect();
        console.log('[MANUAL] Connect result:', result);
      } catch (e) {
        console.log('[MANUAL] Connect error:', e.message);
      }
    }
  });
  await page.waitForTimeout(3000);

  // Check connection status
  const status = await page.evaluate(() => ({
    isConnected: window.solana?.isConnected,
    publicKey: window.solana?.publicKey?.toString(),
  }));
  console.log('Connection status:', status);

  // Get page content
  const pageText = await page.evaluate(() => document.body.innerText);
  console.log('\n=== Page content after connection ===');
  console.log(pageText.substring(0, 500));

  // Look for any action buttons
  console.log('\n=== Looking for action buttons ===');
  const buttons = await page.locator('button, a[role="button"]').all();
  for (let i = 0; i < Math.min(buttons.length, 20); i++) {
    const text = await buttons[i].textContent().catch(() => '');
    const isVisible = await buttons[i].isVisible().catch(() => false);
    if (isVisible && text.trim()) {
      console.log(`Button ${i}: "${text.trim()}"`);
      
      // Try clicking anything that looks like an action
      if (text.match(/claim|verify|check|continue|next|confirm/i)) {
        console.log(`\n>>> Clicking button: "${text.trim()}" <<<`);
        await buttons[i].click();
        await page.waitForTimeout(5000);
        
        // Check if transaction was captured
        const captured = await page.evaluate(() => window.__capturedTransaction);
        if (captured) {
          console.log('\n🎉🎉🎉 TRANSACTION CAPTURED! 🎉🎉🎉');
          console.log('Base64 length:', captured.length);
          console.log('First 200 chars:', captured.substring(0, 200));
          
          // Save to file
          const fs = require('fs');
          fs.writeFileSync('/tmp/captured-transaction.txt', captured);
          console.log('Saved to /tmp/captured-transaction.txt');
        }
        
        break;
      }
    }
  }

  console.log('\nWaiting 30s for manual inspection...');
  await page.waitForTimeout(30000);

  await browser.close();
})();
