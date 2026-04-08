import { Page } from 'playwright';
import OpenAI from 'openai';

/**
 * Common patterns for wallet connection buttons
 */
const CONNECT_PATTERNS = [
  'button:has-text("Connect Wallet")',
  'button:has-text("Connect")',
  'button:has-text("connect wallet")',
  'button:has-text("connect")',
  '[class*="connect" i]:visible',
  '[id*="connect" i]:visible',
  'a:has-text("Connect Wallet")',
  'a:has-text("Connect")',
];

/**
 * Common patterns for claim/action buttons
 */
const CLAIM_PATTERNS = [
  'button:has-text("Claim")',
  'button:has-text("Claim Airdrop")',
  'button:has-text("claim")',
  'button:has-text("Mint")',
  'button:has-text("mint")',
  'button:has-text("Approve")',
  'button:has-text("approve")',
  'button:has-text("Sign")',
  'button:has-text("sign")',
  'button:has-text("Get")',
  'button:has-text("Collect")',
  '[class*="claim" i]:visible',
  '[class*="mint" i]:visible',
];

export interface NavigationResult {
  success: boolean;
  capturedTx: string | null;
  txType: string | null;
  method: 'opportunistic' | 'llm' | 'failed';
  steps: string[];
  error?: string;
}

/**
 * Navigate a phishing site and attempt to trigger a wallet transaction
 */
export async function navigateAndCapture(
  page: Page,
  maxSteps: number = 10,
  timeout: number = 30000
): Promise<NavigationResult> {
  const steps: string[] = [];
  
  // Set default timeout
  page.setDefaultTimeout(timeout);
  
  // Phase 1: Opportunistic pattern matching (no LLM)
  steps.push('Phase 1: Attempting opportunistic navigation');
  
  try {
    const opportunisticResult = await tryOpportunisticNavigation(page, steps);
    if (opportunisticResult.success) {
      return opportunisticResult;
    }
  } catch (error) {
    steps.push(`Opportunistic navigation error: ${error instanceof Error ? error.message : String(error)}`);
  }
  
  // Phase 2: LLM fallback (if configured)
  const llmApiKey = process.env.LLM_API_KEY;
  if (llmApiKey && llmApiKey !== 'none' && llmApiKey.trim() !== '') {
    steps.push('Phase 2: Attempting LLM-driven navigation');
    
    try {
      const llmResult = await tryLLMNavigation(page, maxSteps, steps);
      if (llmResult.success) {
        return llmResult;
      }
    } catch (error) {
      steps.push(`LLM navigation error: ${error instanceof Error ? error.message : String(error)}`);
    }
  } else {
    steps.push('Phase 2: Skipped (no LLM_API_KEY configured)');
  }
  
  // Failed to capture transaction
  return {
    success: false,
    capturedTx: null,
    txType: null,
    method: 'failed',
    steps,
    error: 'Could not trigger wallet transaction with available methods'
  };
}

/**
 * Phase 1: Try common patterns without LLM
 */
async function tryOpportunisticNavigation(
  page: Page,
  steps: string[]
): Promise<NavigationResult> {
  // Wait for page to be ready
  await page.waitForLoadState('domcontentloaded');
  await page.waitForTimeout(2000); // Let dynamic content load
  
  // Log what the wallet stub sees
  await page.evaluate(() => {
    const win = window as any;
    console.log('[Sentinel] window.solana present:', !!win.solana);
    console.log('[Sentinel] window.solana.isPhantom:', win.solana?.isPhantom);
  });
  
  // Try connect patterns first
  for (const pattern of CONNECT_PATTERNS) {
    try {
      const element = page.locator(pattern).first();
      if (await element.isVisible({ timeout: 1000 })) {
        steps.push(`Found connect button: ${pattern}`);
        await element.click({ timeout: 5000 });
        steps.push('Clicked connect button');
        
        // Wait for wallet popup or connection
        await page.waitForTimeout(3000);
        
        // Check for wallet selection modal
        const modalVisible = await page.locator('[class*="modal" i], [class*="wallet" i], [role="dialog"]').first().isVisible().catch(() => false);
        if (modalVisible) {
          steps.push('Wallet selection modal detected');
          
          // Try to click Phantom or first wallet option
          const walletOptions = [
            'button:has-text("Phantom")',
            'button:has-text("phantom")',
            '[class*="phantom" i]:visible',
            'li:has-text("Phantom")',
            'button[class*="wallet" i]:visible:first-child',
          ];
          
          for (const walletPattern of walletOptions) {
            try {
              const walletBtn = page.locator(walletPattern).first();
              if (await walletBtn.isVisible({ timeout: 1000 })) {
                steps.push(`Clicking wallet option: ${walletPattern}`);
                await walletBtn.click({ timeout: 5000 });
                await page.waitForTimeout(2000);
                break;
              }
            } catch (e) {
              continue;
            }
          }
        }
        
        // Log connection status
        const status = await page.evaluate(() => {
          const win = window as any;
          return {
            connected: win.solana?.isConnected,
            connectCalled: win.__connectCalled
          };
        });
        steps.push(`Wallet connected: ${status.connected}, connect() called: ${status.connectCalled}`);
        
        // Check if transaction was captured
        const captured = await checkCapturedTransaction(page);
        if (captured.success) {
          return { ...captured, method: 'opportunistic', steps };
        }
        
        break; // Found and clicked connect, move to claim patterns
      }
    } catch (error) {
      // Pattern didn't match, try next
      continue;
    }
  }
  
  // Try claim/action patterns - try ALL visible buttons, not just first match
  for (const pattern of CLAIM_PATTERNS) {
    try {
      const elements = page.locator(pattern);
      const count = await elements.count();
      
      for (let i = 0; i < count; i++) {
        try {
          const element = elements.nth(i);
          if (await element.isVisible({ timeout: 1000 })) {
            const text = await element.textContent().catch(() => 'unknown');
            steps.push(`Found action button [${i}]: ${pattern} - "${text}"`);
            await element.click({ timeout: 5000 });
            steps.push(`Clicked action button [${i}]`);
            
            // Wait for transaction
            await page.waitForTimeout(3000);
            
            // Check if transaction was captured
            const captured = await checkCapturedTransaction(page);
            if (captured.success) {
              return { ...captured, method: 'opportunistic', steps };
            }
          }
        } catch (error) {
          // This specific element failed, try next
          continue;
        }
      }
    } catch (error) {
      // Pattern didn't match at all, try next pattern
      continue;
    }
  }
  
  // No transaction captured
  return {
    success: false,
    capturedTx: null,
    txType: null,
    method: 'opportunistic',
    steps
  };
}

/**
 * Phase 2: LLM-driven navigation using accessibility tree
 */
async function tryLLMNavigation(
  page: Page,
  maxSteps: number,
  steps: string[]
): Promise<NavigationResult> {
  const llm = new OpenAI({
    apiKey: process.env.LLM_API_KEY ?? 'none',
    baseURL: process.env.LLM_BASE_URL,
  });
  const model = process.env.LLM_MODEL ?? 'gpt-4o';
  
  for (let step = 0; step < maxSteps; step++) {
    steps.push(`LLM step ${step + 1}/${maxSteps}`);
    
    // Get page content as text (accessibility tree alternative)
    const pageText = await page.evaluate(() => {
      return document.body.innerText;
    });
    const accessibilityText = pageText.substring(0, 2000); // Limit to 2000 chars
    
    // Get page URL for context
    const url = page.url();
    
    // Ask LLM what to click
    const prompt = `You are a security researcher analyzing a potential phishing site. Your goal is to trigger a wallet transaction request.

Current URL: ${url}
Step: ${step + 1}/${maxSteps}

Page accessibility tree:
${accessibilityText}

What single element should be clicked next to progress toward triggering a wallet transaction?
Reply with JSON only: { "selector": "playwright selector", "reason": "brief explanation" }

If no suitable element exists, reply: { "selector": null, "reason": "explanation" }`;

    try {
      const response = await llm.chat.completions.create({
        model,
        messages: [{ role: 'user', content: prompt }],
        temperature: 0.3,
        max_tokens: 200,
      });
      
      const content = response.choices[0]?.message?.content?.trim();
      if (!content) {
        steps.push('LLM returned empty response');
        continue;
      }
      
      // Parse JSON response
      const jsonMatch = content.match(/\{[\s\S]*\}/);
      if (!jsonMatch) {
        steps.push(`LLM response not JSON: ${content.substring(0, 100)}`);
        continue;
      }
      
      const decision = JSON.parse(jsonMatch[0]);
      
      if (!decision.selector) {
        steps.push(`LLM: ${decision.reason}`);
        break; // No more actions to take
      }
      
      steps.push(`LLM suggests: ${decision.selector} - ${decision.reason}`);
      
      // Try to click the suggested element
      try {
        await page.locator(decision.selector).first().click({ timeout: 5000 });
        steps.push('Clicked suggested element');
        
        // Wait for any transaction
        await page.waitForTimeout(3000);
        
        // Check if transaction was captured
        const captured = await checkCapturedTransaction(page);
        if (captured.success) {
          return { ...captured, method: 'llm', steps };
        }
      } catch (clickError) {
        steps.push(`Could not click: ${clickError instanceof Error ? clickError.message : String(clickError)}`);
      }
    } catch (llmError) {
      steps.push(`LLM error: ${llmError instanceof Error ? llmError.message : String(llmError)}`);
      break;
    }
  }
  
  return {
    success: false,
    capturedTx: null,
    txType: null,
    method: 'llm',
    steps
  };
}

/**
 * Check if a transaction was captured by the wallet stub
 */
async function checkCapturedTransaction(page: Page): Promise<{
  success: boolean;
  capturedTx: string | null;
  txType: string | null;
}> {
  try {
    const result = await page.evaluate(() => {
      const win = window as any;
      return {
        tx: win.__capturedTx,
        type: win.__capturedTxType
      };
    });
    
    if (result.tx) {
      return {
        success: true,
        capturedTx: result.tx,
        txType: result.type
      };
    }
  } catch (error) {
    // Evaluation failed
  }
  
  return {
    success: false,
    capturedTx: null,
    txType: null
  };
}

