#!/usr/bin/env node

import { Command } from 'commander';
import { chromium } from 'playwright';
import { generateThrowawayKeypair, getPublicKeyString } from './keypair';
import { generatePhantomWalletStub } from './wallet-stub-phantom';
import { navigateAndCapture } from './navigator';
import { analyzeTransaction } from './analyzer';
import { PhishingReport, formatReportJSON, formatReportHuman } from './report';

const program = new Command();

program
  .name('sentinel')
  .description('Guardian agent that sacrifices itself to test suspicious sites')
  .version('0.1.0')
  .argument('<url>', 'URL of the suspected phishing site')
  .option('-f, --format <type>', 'Output format: json or human', 'json')
  .option('-t, --timeout <ms>', 'Navigation timeout in milliseconds', '30000')
  .option('-s, --max-steps <n>', 'Maximum navigation steps', '10')
  .option('--headless <bool>', 'Run browser in headless mode', 'true')
  .action(async (url: string, options) => {
    await assessPhishingSite(url, options);
  });

async function assessPhishingSite(
  url: string,
  options: {
    format: string;
    timeout: string;
    maxSteps: string;
    headless: string;
  }
) {
  const startTime = new Date().toISOString();
  const timeout = parseInt(options.timeout, 10);
  const maxSteps = parseInt(options.maxSteps, 10);
  const headless = options.headless !== 'false';
  
  // Validate required environment variables
  const rpcUrl = process.env.SOL_SHIELD_RPC_URL;
  if (!rpcUrl) {
    console.error('Error: SOL_SHIELD_RPC_URL environment variable is required');
    process.exit(1);
  }
  
  let browser;
  
  try {
    // Generate throwaway keypair
    const keypair = generateThrowawayKeypair();
    const publicKey = getPublicKeyString(keypair);
    
    if (options.format === 'human') {
      console.error(`[*] Generated throwaway wallet: ${publicKey}`);
      console.error(`[*] Launching browser...`);
    }
    
    // Launch browser
    browser = await chromium.launch({
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
    
    const page = await context.newPage();
    
    // Enable console logging
    page.on('console', msg => {
      if (options.format === 'human') {
        console.error(`[Browser Console] ${msg.type()}: ${msg.text()}`);
      }
    });
    
    // Inject Phantom wallet stub
    const walletStubCode = generatePhantomWalletStub(keypair);
    await page.addInitScript(walletStubCode);
    
    if (options.format === 'human') {
      console.error(`[*] Navigating to ${url}...`);
    }
    
    // Navigate to the site
    await page.goto(url, { waitUntil: 'domcontentloaded', timeout });
    
    if (options.format === 'human') {
      console.error(`[*] Attempting to trigger wallet interaction...`);
    }
    
    // Navigate and capture transaction
    const navResult = await navigateAndCapture(page, maxSteps, timeout);
    
    // Build report
    const report: PhishingReport = {
      url,
      scanned_at: startTime,
      throwaway_wallet: publicKey,
      transaction_captured: navResult.success,
      transaction_type: navResult.txType || undefined,
      navigation_method: navResult.method,
      navigation_steps: navResult.steps,
      programs_invoked: [],
      rules_matched: [],
      risk_level: 'unknown',
      verdict: 'NO_TRANSACTION',
    };
    
    // If transaction was captured, analyze it
    if (navResult.success && navResult.capturedTx) {
      if (options.format === 'human') {
        console.error(`[*] Transaction captured! Analyzing...`);
      }
      
      const analysis = await analyzeTransaction(
        navResult.capturedTx,
        publicKey,
        rpcUrl
      );
      
      if (analysis.success) {
        report.programs_invoked = analysis.programs;
        report.rules_matched = analysis.rulesMatched;
        report.risk_level = analysis.riskLevel;
        report.verdict = analysis.verdict;
        report.simulation_result = analysis.simulationResult;
      } else {
        report.error = analysis.error;
        report.verdict = 'ANALYSIS_FAILED';
      }
    } else {
      report.error = navResult.error;
    }
    
    // Close browser
    await browser.close();
    
    // Output report
    if (options.format === 'human') {
      console.log(formatReportHuman(report));
    } else {
      console.log(formatReportJSON(report));
    }
    
    // Exit with appropriate code
    if (report.verdict === 'MALICIOUS') {
      process.exit(2);
    } else if (report.verdict === 'SUSPICIOUS') {
      process.exit(1);
    } else {
      process.exit(0);
    }
    
  } catch (error) {
    if (browser) {
      await browser.close();
    }
    
    const report: PhishingReport = {
      url,
      scanned_at: startTime,
      throwaway_wallet: 'N/A',
      transaction_captured: false,
      programs_invoked: [],
      rules_matched: [],
      risk_level: 'unknown',
      verdict: 'ERROR',
      error: error instanceof Error ? error.message : String(error),
    };
    
    if (options.format === 'human') {
      console.error(`\n[ERROR] ${report.error}\n`);
      console.log(formatReportHuman(report));
    } else {
      console.log(formatReportJSON(report));
    }
    
    process.exit(3);
  }
}

program.parse();
