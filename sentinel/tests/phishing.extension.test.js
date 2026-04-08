import { expect } from '@playwright/test';
import { createWalletFixture } from '../node_modules/walletqa/dist/fixtures/index.js';
import phantomSetup from './wallet-setup/phantom.setup.js';

const test = createWalletFixture({
  wallet: 'phantom',
  mode: 'extension',
  password: 'TestPassword123!',
  cacheHash: phantomSetup.hash,
});

test('connect to bulktrade.me and trigger transaction', async ({ walletPage, approveConnection, approveTransaction }) => {
  // Inject spoof BEFORE navigating to the site
  await walletPage.addInitScript(() => {
    // Use a mainnet wallet with real activity and tokens
    const FAKE_PUBKEY = 'DYw8jCTfwHNRJhhmFcbXvVDTqWMEVFBX6ZKUmG5CNSKK';
    
    console.log('[SPOOF] Installing publicKey interceptor...');
    
    // Intercept window.solana when it gets created
    let originalSolana = null;
    Object.defineProperty(window, 'solana', {
      get() {
        return originalSolana;
      },
      set(value) {
        console.log('[SPOOF] window.solana being set, wrapping it...');
        originalSolana = new Proxy(value, {
          get(target, prop) {
            if (prop === 'publicKey' && target.publicKey) {
              console.log('[SPOOF] ✅ Returning fake publicKey:', FAKE_PUBKEY);
              return {
                toString: () => FAKE_PUBKEY,
                toBase58: () => FAKE_PUBKEY,
                toBuffer: () => new Uint8Array(32),
                toBytes: () => new Uint8Array(32),
              };
            }
            if (prop === 'connect') {
              return async function(...args) {
                console.log('[SPOOF] connect() called, will return fake key');
                const result = await target.connect.apply(target, args);
                return {
                  publicKey: {
                    toString: () => FAKE_PUBKEY,
                    toBase58: () => FAKE_PUBKEY,
                    toBuffer: () => new Uint8Array(32),
                  }
                };
              };
            }
            return target[prop];
          }
        });
      },
      configurable: true
    });
  });
  
  console.log('[TEST] 🎯 Navigating to phishing site...');
  await walletPage.goto('https://bulktrade.me/coin');
  await walletPage.waitForTimeout(3000);

  console.log('[TEST] 🔌 Clicking Connect Wallet...');
  await walletPage.locator('text=Connect Wallet').first().click();
  await walletPage.waitForTimeout(2000);

  console.log('[TEST] 👻 Selecting Phantom wallet...');
  await walletPage.locator('text=Phantom').first().click();
  await walletPage.waitForTimeout(1000);

  console.log('[TEST] ✅ Checking for connection popup...');
  
  // Wait a bit for popup to appear
  await walletPage.waitForTimeout(2000);
  
  // Find the popup
  const allPages = walletPage.context().pages();
  console.log(`[TEST] Total pages: ${allPages.length}`);
  
  let popupPage = null;
  for (const page of allPages) {
    const url = page.url();
    console.log(`[TEST]   Page: ${url}`);
    if (url.includes('notification.html') || url.includes('popup.html')) {
      popupPage = page;
      console.log('[TEST] 🎯 Found popup page!');
      
      // Take screenshot of popup
      await page.screenshot({ path: '/tmp/phantom-popup.png' });
      console.log('[TEST] 📸 Popup screenshot: /tmp/phantom-popup.png');
      
      // Get popup content
      const popupText = await page.evaluate(() => document.body.innerText).catch(() => 'Could not read');
      console.log('[TEST] 📄 Popup content:', popupText.substring(0, 500));
      
      // Look for connect button
      const connectBtn = page.locator('button:has-text("Connect")').first();
      if (await connectBtn.isVisible({ timeout: 2000 }).catch(() => false)) {
        console.log('[TEST] ✅ Clicking Connect button in popup...');
        await connectBtn.click();
        await walletPage.waitForTimeout(3000);
      } else {
        console.log('[TEST] ⚠️  No Connect button found in popup');
      }
    }
  }
  
  // Check connection status
  const walletInfo = await walletPage.evaluate(() => {
    return {
      isConnected: window.solana?.isConnected,
      publicKey: window.solana?.publicKey?.toString(),
    };
  });
  console.log('[TEST] 🔍 Connection status:', JSON.stringify(walletInfo, null, 2));

  // Check what's on the page
  const pageText = await walletPage.evaluate(() => document.body.innerText);
  console.log('[TEST] 📄 Page content after connection:', pageText.substring(0, 800));

  // Take a screenshot
  await walletPage.screenshot({ path: '/tmp/after-connect.png' });
  console.log('[TEST] 📸 Screenshot saved to /tmp/after-connect.png');

  // Look for ALL clickable buttons
  console.log('[TEST] 🔍 Looking for all buttons...');
  const allButtons = await walletPage.locator('button, a[role="button"], div[role="button"]').all();
  console.log(`[TEST] Found ${allButtons.length} clickable elements`);
  
  for (let i = 0; i < Math.min(allButtons.length, 20); i++) {
    const text = await allButtons[i].textContent().catch(() => '');
    const isVisible = await allButtons[i].isVisible().catch(() => false);
    if (isVisible && text.trim()) {
      console.log(`[TEST]   Button ${i}: "${text.trim()}"`);
    }
  }

  // Try clicking any action button
  const actionButtons = ['Claim', 'Check Eligibility', 'Verify', 'Continue', 'Next', 'Confirm', 'Accept'];
  for (const btnText of actionButtons) {
    const btn = walletPage.locator(`button:has-text("${btnText}"), a:has-text("${btnText}")`).first();
    if (await btn.isVisible({ timeout: 1000 }).catch(() => false)) {
      console.log(`[TEST] 💰 Found and clicking "${btnText}" button...`);
      await btn.click();
      await walletPage.waitForTimeout(3000);
      
      console.log('[TEST] 🔐 Checking for transaction popup...');
      try {
        await approveTransaction({ timeout: 5000 });
        console.log('[TEST] ✅ Transaction popup appeared and was approved!');
      } catch (e) {
        console.log('[TEST] ⚠️  No transaction popup:', e.message);
      }
      
      await walletPage.screenshot({ path: `/tmp/after-${btnText.toLowerCase()}.png` });
      break;
    }
  }
  
  console.log('[TEST] ✅ Test complete!');
});
