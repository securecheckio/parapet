const { chromium } = require('playwright');
const path = require('path');

(async () => {
  // Use the walletqa cached profile and extension
  const cacheDir = path.join(__dirname, '.walletqa/cache/phantom/360ccdf8120f');
  const extensionPath = path.join(__dirname, '.walletqa/extensions/phantom/latest');
  
  console.log('Launching browser with Phantom extension...');
  console.log('Cache:', cacheDir);
  console.log('Extension:', extensionPath);
  
  const context = await chromium.launchPersistentContext(cacheDir, {
    headless: false,
    args: [
      `--disable-extensions-except=${extensionPath}`,
      `--load-extension=${extensionPath}`,
      '--disable-blink-features=AutomationControlled',
    ],
  });

  // Wait for extension to load
  await context.waitForEvent('page', { timeout: 10000 }).catch(() => null);
  await new Promise(resolve => setTimeout(resolve, 2000));
  
  // Check if we need to unlock the wallet
  const initialPages = context.pages();
  for (const p of initialPages) {
    const url = p.url();
    console.log('Initial page:', url);
    
    if (url.includes('onboarding.html') || url.includes('popup.html')) {
      console.log('Unlocking wallet...');
      // Try to find and fill password
      const passwordInput = p.locator('input[type="password"]').first();
      if (await passwordInput.isVisible({ timeout: 5000 }).catch(() => false)) {
        await passwordInput.fill('TestPassword123!');
        await p.locator('button[type="submit"]').first().click();
        await p.waitForTimeout(2000);
      }
    }
  }
  
  const page = await context.newPage();
  
  console.log('Navigating to phishing site...');
  await page.goto('https://bulktrade.me/coin');
  await page.waitForTimeout(3000);

  console.log('Clicking Connect Wallet...');
  await page.click('text=Connect Wallet');
  await page.waitForTimeout(2000);

  console.log('Clicking Phantom...');
  await page.click('text=Phantom');
  await page.waitForTimeout(2000);

  // Look for Phantom popup window
  const pages = context.pages();
  console.log(`Total pages: ${pages.length}`);
  
  for (let i = 0; i < pages.length; i++) {
    const url = pages[i].url();
    console.log(`Page ${i}: ${url}`);
    
    if (url.includes('notification.html') || url.includes('popup.html')) {
      console.log('Found Phantom popup!');
      const popupPage = pages[i];
      await popupPage.waitForTimeout(2000);
      
      // Click Connect/Approve button
      const connectBtn = popupPage.locator('button:has-text("Connect")').first();
      if (await connectBtn.isVisible({ timeout: 5000 }).catch(() => false)) {
        console.log('Clicking Connect in popup...');
        await connectBtn.click();
        await page.waitForTimeout(2000);
      }
    }
  }

  console.log('Waiting to see if connected...');
  await page.waitForTimeout(5000);

  console.log('Looking for Claim button...');
  const claimBtn = page.locator('text=Claim').first();
  if (await claimBtn.isVisible({ timeout: 5000 }).catch(() => false)) {
    console.log('Clicking Claim...');
    await claimBtn.click();
    await page.waitForTimeout(5000);
    
    // Check for new popups (transaction approval)
    const newPages = context.pages();
    console.log(`Total pages after claim: ${newPages.length}`);
    
    for (let i = 0; i < newPages.length; i++) {
      const url = newPages[i].url();
      console.log(`Page ${i}: ${url}`);
      
      if (url.includes('notification.html')) {
        console.log('Found transaction approval popup!');
        const txPopup = newPages[i];
        await txPopup.waitForTimeout(2000);
        
        // Take screenshot
        await txPopup.screenshot({ path: '/tmp/phantom-tx-approval.png' });
        console.log('Screenshot saved to /tmp/phantom-tx-approval.png');
        
        // Get transaction details
        const txDetails = await txPopup.evaluate(() => document.body.innerText);
        console.log('Transaction details:', txDetails);
      }
    }
  }

  console.log('Waiting 30s for manual inspection...');
  await page.waitForTimeout(30000);

  await context.close();
})();
