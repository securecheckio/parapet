const { chromium } = require('playwright');

(async () => {
  const browser = await chromium.launch({ headless: false });
  const page = await browser.newPage();
  
  await page.goto('https://rewards-solana.org/');
  await page.waitForTimeout(3000);
  
  const text = await page.evaluate(() => document.body.innerText);
  console.log('=== PAGE CONTENT ===');
  console.log(text);
  
  console.log('\n=== ALL BUTTONS ===');
  const buttons = await page.locator('button, a').all();
  for (let i = 0; i < Math.min(buttons.length, 30); i++) {
    const text = await buttons[i].textContent().catch(() => '');
    const isVisible = await buttons[i].isVisible().catch(() => false);
    if (isVisible && text.trim()) {
      console.log(`${i}: "${text.trim()}"`);
    }
  }
  
  await page.waitForTimeout(30000);
  await browser.close();
})();
