const { chromium } = require('playwright');

(async () => {
  const browser = await chromium.launch({ headless: false });
  const page = await browser.newPage();
  
  await page.addInitScript(async () => {
    const { createPhantom } = await import('https://cdn.jsdelivr.net/npm/@phantom/browser-injected-sdk@1.0.7/+esm');
    const { createSolanaPlugin } = await import('https://cdn.jsdelivr.net/npm/@phantom/browser-injected-sdk@1.0.7/solana/+esm');
    
    const phantom = createPhantom({
      plugins: [createSolanaPlugin()],
    });
    
    window.phantom = phantom;
    window.solana = phantom.solana;
    
    console.log('Phantom SDK loaded');
    console.log('phantom.solana keys:', Object.keys(phantom.solana));
    console.log('phantom.solana.isPhantom:', phantom.solana.isPhantom);
    console.log('phantom.solana.publicKey:', phantom.solana.publicKey);
  });
  
  await page.goto('https://bulktrade.me/coin');
  await page.waitForTimeout(5000);
  await browser.close();
})();
