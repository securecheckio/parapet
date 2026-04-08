const { chromium } = require('playwright');

(async () => {
  const browser = await chromium.launch({ headless: false });
  const page = await browser.newPage();
  
  // Inject wallet stub
  await page.addInitScript(() => {
    window.__connectCalled = false;
    window.solana = {
      isPhantom: true,
      isConnected: false,
      publicKey: null,
      connect: async function() {
        console.log('✅✅✅ CONNECT CALLED! ✅✅✅');
        window.__connectCalled = true;
        this.isConnected = true;
        this.publicKey = { toBase58: () => 'TestWallet123' };
        return { publicKey: this.publicKey };
      },
      signTransaction: async function(tx) {
        console.log('✅ SIGN TX CALLED!');
        throw new Error('User rejected');
      }
    };
    window.phantom = { solana: window.solana };
    setTimeout(() => {
      window.dispatchEvent(new Event('solana#initialized'));
      window.dispatchEvent(new Event('phantom#initialized'));
    }, 100);
  });
  
  await page.goto('https://bulktrade.me/coin');
  await page.waitForTimeout(2000);
  
  console.log('=== Step 1: Click Connect Wallet ===');
  await page.click('text=Connect Wallet');
  await page.waitForTimeout(2000);
  
  // Take screenshot to see what appeared
  await page.screenshot({ path: '/tmp/after-connect-click.png' });
  console.log('Screenshot saved to /tmp/after-connect-click.png');
  
  // Check for modal or wallet options
  const visibleText = await page.evaluate(() => document.body.innerText);
  console.log('=== Visible text after click ===');
  console.log(visibleText.substring(0, 500));
  
  // Look for Phantom button in modal
  console.log('\n=== Looking for Phantom option ===');
  const phantomButton = page.locator('text=Phantom').first();
  if (await phantomButton.isVisible({ timeout: 1000 }).catch(() => false)) {
    console.log('Found Phantom button, clicking...');
    await phantomButton.click();
    await page.waitForTimeout(2000);
    
    const status = await page.evaluate(() => window.__connectCalled);
    console.log('Connect called:', status);
  } else {
    console.log('No Phantom button found');
  }
  
  await page.waitForTimeout(5000);
  await browser.close();
})();
