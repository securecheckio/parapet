import { ProgramInfo, RuleMatch } from './analyzer';

export interface PhishingReport {
  url: string;
  scanned_at: string;
  throwaway_wallet: string;
  transaction_captured: boolean;
  transaction_type?: string;
  navigation_method?: 'opportunistic' | 'llm' | 'failed';
  navigation_steps?: string[];
  programs_invoked: ProgramInfo[];
  rules_matched: RuleMatch[];
  risk_level: string;
  verdict: string;
  simulation_result?: any;
  error?: string;
}

/**
 * Format report as JSON for stdout
 */
export function formatReportJSON(report: PhishingReport): string {
  return JSON.stringify(report, null, 2);
}

/**
 * Format report as human-readable text
 */
export function formatReportHuman(report: PhishingReport): string {
  const lines: string[] = [];
  
  lines.push('='.repeat(60));
  lines.push('PHISHING SITE ASSESSMENT');
  lines.push('='.repeat(60));
  lines.push('');
  lines.push(`URL:          ${report.url}`);
  lines.push(`Scanned at:   ${report.scanned_at}`);
  lines.push(`Wallet:       ${report.throwaway_wallet}`);
  lines.push(`Risk Level:   ${report.risk_level.toUpperCase()}`);
  lines.push(`Verdict:      ${report.verdict}`);
  lines.push('');
  
  if (report.transaction_captured) {
    lines.push('TRANSACTION INTERCEPTED');
    lines.push('-'.repeat(60));
    lines.push(`Type: ${report.transaction_type || 'unknown'}`);
    lines.push(`Method: ${report.navigation_method || 'unknown'}`);
    lines.push('');
    
    if (report.programs_invoked.length > 0) {
      lines.push('Programs invoked:');
      for (const program of report.programs_invoked) {
        const status = program.known ? '✓' : '⚠';
        const name = program.name ? ` (${program.name})` : ' (UNKNOWN)';
        lines.push(`  ${status} ${program.address}${name}`);
      }
      lines.push('');
    }
    
    if (report.rules_matched.length > 0) {
      lines.push('RULES ENGINE');
      lines.push('-'.repeat(60));
      for (const rule of report.rules_matched) {
        lines.push(`  [${rule.action.toUpperCase()}] ${rule.id}`);
        lines.push(`    ${rule.message}`);
      }
      lines.push('');
    }
  } else {
    lines.push('NO TRANSACTION CAPTURED');
    lines.push('-'.repeat(60));
    if (report.error) {
      lines.push(`Error: ${report.error}`);
    }
    lines.push('');
  }
  
  if (report.navigation_steps && report.navigation_steps.length > 0) {
    lines.push('NAVIGATION STEPS');
    lines.push('-'.repeat(60));
    for (const step of report.navigation_steps) {
      lines.push(`  ${step}`);
    }
    lines.push('');
  }
  
  lines.push('='.repeat(60));
  
  return lines.join('\n');
}
