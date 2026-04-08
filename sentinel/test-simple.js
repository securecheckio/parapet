const { chromium } = require('playwright');

(async () => {
  const browser = await chromium.launch({ headless: false });
  const page = await browser.newPage();
  
  // Simple stub that works
  await page.addInitScript(() => {
    window.solana = {
      isPhantom: true,
      isConnected: false,
      publicKey: null,
      connect: async function() {
        console.log('✅✅✅ CONNECT CALLED ✅✅✅');
        this.isConnected = true;
        this.publicKey = { toBase58: () => 'TestWallet123' };
        return { publicKey: this.publicKey };
      },
      signAndSendTransaction: async function(tx) {
        console.log('✅ SIGN AND SEND CALLED');
        throw new Error('User rejected');
      }
    };
    window.phantom = { solana: window.solana };
  });
  
  await page.goto('https://bulktrade.me/coin');
  await page.waitForTimeout(2000);
  
  console.log('Step 1: Click Connect Wallet button');
  await page.click('text=Connect Wallet');
  await page.waitForTimeout(1000);
  
  console.log('Step 2: Look for wallet selection modal');
  const pageContent = await page.content();
  console.log('Modal visible?', pageContent.includes('Phantom') || pageContent.includes('wallet'));
  
  // Try to find and click Phantom in modal
  const allButtons = await page.locator('button, a, div[role="button"]').all();
  console.log(`Found ${allButtons.length} clickable elements`);
  
  for (let i = 0; i < allButtons.length; i++) {
    const text = await allButtons[i].textContent().catch(() => '');
    const isVisible = await allButtons[i].isVisible().catch(() => false);
    if (isVisible && text && (text.includes('Phantom') || text.includes('phantom'))) {
      console.log(`Found Phantom button: "${text}"`);
      await allButtons[i].click();
      console.log('Clicked Phantom option');
      await page.waitForTimeout(2000);
      break;
    }
  }
  
  const connected = await page.evaluate(() => window.solana?.isConnected);
  console.log('Connected:', connected);
  
  await page.waitForTimeout(5000);
  await browser.close();
})();
