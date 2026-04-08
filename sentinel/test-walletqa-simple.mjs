import { test as base } from '@playwright/test';
import { createWalletFixture } from 'walletqa/fixtures';
import phantomSetup from './tests/wallet-setup/phantom.setup.js';

const test = createWalletFixture({
  wallet: 'phantom',
  mode: 'extension',
  password: 'TestPassword123!',
  cacheHash: phantomSetup.hash,
});

test('connect to bulktrade phishing site', async ({ walletPage, approveConnection }) => {
  console.log('Navigating to phishing site...');
  await walletPage.goto('https://bulktrade.me/coin');
  await walletPage.waitForTimeout(3000);

  console.log('Clicking Connect Wallet...');
  await walletPage.locator('text=Connect Wallet').first().click();
  await walletPage.waitForTimeout(2000);

  console.log('Clicking Phantom...');
  await walletPage.locator('text=Phantom').first().click();
  await walletPage.waitForTimeout(1000);

  console.log('Approving connection...');
  await approveConnection();
  await walletPage.waitForTimeout(3000);

  console.log('SUCCESS! Wallet connected. Looking for Claim button...');
  const claimBtn = walletPage.locator('text=Claim').first();
  if (await claimBtn.isVisible({ timeout: 5000 }).catch(() => false)) {
    console.log('Clicking Claim...');
    await claimBtn.click();
    await walletPage.waitForTimeout(10000);
  }

  console.log('Test complete');
});
