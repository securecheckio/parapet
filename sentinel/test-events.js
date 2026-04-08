const { chromium } = require('playwright');

(async () => {
  const browser = await chromium.launch({ headless: false });
  const page = await browser.newPage();
  
  await page.addInitScript(() => {
    let connectCount = 0;
    window.solana = {
      isPhantom: true,
      isConnected: false,
      publicKey: null,
      
      connect: async function(opts) {
        connectCount++;
        console.log(`✅ CONNECT CALLED (${connectCount} times)`, opts);
        this.isConnected = true;
        this.publicKey = { toBase58: () => 'TestWallet123' };
        
        // Dispatch events
        window.dispatchEvent(new CustomEvent('wallet-connected', { detail: { publicKey: this.publicKey } }));
        
        return { publicKey: this.publicKey };
      },
      
      on: function(event, callback) {
        console.log(`Listener registered for: ${event}`);
        window.addEventListener(`solana-${event}`, callback);
      },
      
      signAndSendTransaction: async function(tx) {
        console.log('✅ SIGN AND SEND CALLED');
        throw new Error('User rejected');
      }
    };
    
    window.phantom = { solana: window.solana };
    
    // Dispatch ready event
    setTimeout(() => {
      window.dispatchEvent(new Event('phantom#initialized'));
      console.log('Dispatched phantom#initialized');
    }, 100);
  });
  
  await page.goto('https://bulktrade.me/coin');
  await page.waitForTimeout(2000);
  
  // Monitor what the page is doing
  await page.evaluate(() => {
    const original = window.solana.connect;
    window.solana.connect = async function(...args) {
      console.log('🔍 Site is calling connect with:', args);
      return await original.apply(this, args);
    };
  });
  
  console.log('=== Clicking Connect Wallet ===');
  await page.click('text=Connect Wallet');
  await page.waitForTimeout(1000);
  
  console.log('=== Clicking Phantom ===');
  await page.click('text=Phantom');
  await page.waitForTimeout(3000);
  
  const status = await page.evaluate(() => ({
    isConnected: window.solana?.isConnected,
    publicKey: window.solana?.publicKey?.toBase58?.(),
  }));
  console.log('Final status:', status);
  
  await page.waitForTimeout(5000);
  await browser.close();
})();
