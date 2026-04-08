const { chromium } = require('playwright');

(async () => {
  const browser = await chromium.launch({ headless: false });
  const context = await browser.newContext();
  const page = await context.newPage();
  
  // Inject wallet stub
  await page.addInitScript(() => {
    console.log('[TEST] Injecting wallet stub...');
    window.__connectCalled = false;
    window.__capturedTx = null;
    
    window.solana = {
      isPhantom: true,
      isConnected: false,
      publicKey: null,
      
      connect: async function() {
        console.log('[TEST] ✅✅✅ CONNECT WAS CALLED! ✅✅✅');
        window.__connectCalled = true;
        this.isConnected = true;
        this.publicKey = { toBase58: () => 'TestWallet123' };
        return { publicKey: this.publicKey };
      },
      
      signTransaction: async function(tx) {
        console.log('[TEST] ✅✅✅ SIGN TRANSACTION CALLED! ✅✅✅');
        window.__capturedTx = 'TRANSACTION_CAPTURED';
        throw new Error('User rejected');
      }
    };
    
    window.phantom = { solana: window.solana };
    console.log('[TEST] Wallet stub ready');
  });
  
  await page.goto('https://bulktrade.me/coin');
  console.log('[TEST] Page loaded');
  
  await page.waitForTimeout(3000);
  
  // Check what we have
  const status = await page.evaluate(() => {
    return {
      solanaPresent: !!window.solana,
      isPhantom: window.solana?.isPhantom,
      connectCalled: window.__connectCalled
    };
  });
  console.log('[TEST] Status:', status);
  
  // Try clicking connect
  console.log('[TEST] Looking for Connect Wallet button...');
  const buttons = await page.locator('button, a').all();
  console.log(`[TEST] Found ${buttons.length} clickable elements`);
  
  for (let i = 0; i < buttons.length; i++) {
    const text = await buttons[i].textContent();
    if (text && text.toLowerCase().includes('connect')) {
      console.log(`[TEST] Found connect button [${i}]: "${text}"`);
      await buttons[i].click();
      console.log('[TEST] Clicked!');
      await page.waitForTimeout(3000);
      
      const afterClick = await page.evaluate(() => {
        return {
          connectCalled: window.__connectCalled,
          capturedTx: window.__capturedTx
        };
      });
      console.log('[TEST] After click:', afterClick);
      break;
    }
  }
  
  await page.waitForTimeout(5000);
  await browser.close();
})();
