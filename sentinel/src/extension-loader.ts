/**
 * Loads the Phantom wallet extension into Playwright browser
 */

import { chromium, BrowserContext } from 'playwright';
import * as path from 'path';
import * as fs from 'fs';
import * as os from 'os';

export async function launchBrowserWithPhantom(headless: boolean = true): Promise<{ context: BrowserContext; extensionPath: string | null }> {
  // Try to find Phantom extension
  const homeDir = os.homedir();
  const possiblePaths = [
    path.join(homeDir, '.config/BraveSoftware/Brave-Browser/Default/Extensions/bfnaelmomeimhlpmgjnjophhpkkoljpa'),
    path.join(homeDir, '.config/google-chrome/Default/Extensions/bfnaelmomeimhlpmgjnjophhpkkoljpa'),
    path.join(homeDir, '.config/chromium/Default/Extensions/bfnaelmomeimhlpmgjnjophhpkkoljpa'),
  ];

  let extensionPath: string | null = null;
  
  for (const basePath of possiblePaths) {
    if (fs.existsSync(basePath)) {
      // Find the version directory (e.g., 26.10.0_0)
      const versions = fs.readdirSync(basePath);
      if (versions.length > 0) {
        extensionPath = path.join(basePath, versions[0]);
        console.error(`[*] Found Phantom extension at: ${extensionPath}`);
        break;
      }
    }
  }

  if (!extensionPath) {
    console.error('[!] Warning: Phantom extension not found, falling back to custom stub');
    // Launch without extension
    const browser = await chromium.launch({
      headless,
      args: [
        '--disable-blink-features=AutomationControlled',
        '--disable-web-security',
      ]
    });
    
    const context = await browser.newContext({
      viewport: { width: 1280, height: 720 },
      userAgent: 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36',
    });
    
    return { context, extensionPath: null };
  }

  // Launch with extension - requires persistent context
  const userDataDir = path.join(os.tmpdir(), `sentinel-${Date.now()}`);
  
  const context = await chromium.launchPersistentContext(userDataDir, {
    headless: false, // Extensions don't work in headless mode
    args: [
      `--disable-extensions-except=${extensionPath}`,
      `--load-extension=${extensionPath}`,
      '--disable-blink-features=AutomationControlled',
    ],
    viewport: { width: 1280, height: 720 },
    userAgent: 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36',
  });

  return { context, extensionPath };
}
