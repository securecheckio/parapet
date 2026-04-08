import { FC } from 'react';
import { Link } from 'react-router-dom';

const Security: FC = () => {
  return (
    <div className="max-w-7xl mx-auto px-4 sm:px-6 py-8 sm:py-12 lg:py-16">
      {/* Hero */}
      <div className="text-center mb-12 sm:mb-16">
        <h1 className="text-3xl sm:text-4xl lg:text-5xl font-bold mb-4 text-white">How it works</h1>
        <p className="text-base sm:text-lg text-white/70 max-w-2xl mx-auto">
          Defense-in-depth protection that analyzes transactions before they reach the blockchain
        </p>
      </div>

      {/* Architecture */}
      <div className="mb-16 sm:mb-20">
        <h2 className="text-2xl sm:text-3xl font-bold mb-8 text-white">Architecture</h2>
        <div className="bg-white/5 border border-white/10 rounded-lg p-8">
          <div className="space-y-6">
            <div className="flex items-start gap-4">
              <div className="text-blue-400 font-mono text-sm mt-1">1</div>
              <div>
                <div className="text-white font-semibold mb-1">Your wallet/dApp sends a transaction</div>
                <div className="text-white/70 text-sm">Instead of connecting to a Solana RPC, you connect to the SecureCheck RPC proxy</div>
              </div>
            </div>
            <div className="flex items-start gap-4">
              <div className="text-purple-400 font-mono text-sm mt-1">2</div>
              <div>
                <div className="text-white font-semibold mb-1">RPC proxy analyzes the transaction</div>
                <div className="text-white/70 text-sm">Sub-50ms security scan checks patterns, addresses, and authorities against threat database</div>
              </div>
            </div>
            <div className="flex items-start gap-4">
              <div className="text-green-400 font-mono text-sm mt-1">3</div>
              <div>
                <div className="text-white font-semibold mb-1">Forward or block based on risk</div>
                <div className="text-white/70 text-sm">Safe transactions pass through to your RPC and then to Solana. Threats are blocked before reaching the blockchain with detailed explanations.</div>
              </div>
            </div>
          </div>
        </div>
      </div>

      {/* Performance */}
      <div className="mb-16 sm:mb-20">
        <h2 className="text-2xl sm:text-3xl font-bold mb-8 text-white">Built for speed</h2>
        <div className="grid sm:grid-cols-3 gap-6">
          <div className="bg-white/5 border border-white/10 rounded-lg p-6">
            <div className="text-3xl font-bold text-blue-400 mb-2">&lt;50ms</div>
            <div className="text-white/70 text-sm">Average analysis time with zero UX impact</div>
          </div>
          <div className="bg-white/5 border border-white/10 rounded-lg p-6">
            <div className="text-3xl font-bold text-purple-400 mb-2">Cached Intelligence</div>
            <div className="text-white/70 text-sm">Analysis augmented with cached threat data, no blocking lookups</div>
          </div>
          <div className="bg-white/5 border border-white/10 rounded-lg p-6">
            <div className="text-3xl font-bold text-green-400 mb-2">Rust</div>
            <div className="text-white/70 text-sm">Memory-safe, high-performance analysis engine</div>
          </div>
        </div>
      </div>

      {/* Pluggable Framework */}
      <div className="mb-16 sm:mb-20">
        <h2 className="text-2xl sm:text-3xl font-bold mb-8 text-white">Pluggable framework</h2>
        <div className="bg-white/5 border border-white/10 rounded-lg p-8">
          <p className="text-white/70 mb-6">
            Security policies are defined in JSON files, making them auditable, verifiable, and easy to customize.
          </p>
          
          <div className="space-y-6">
            <div>
              <div className="flex items-center gap-2 mb-3">
                <span className="text-blue-400 font-mono text-sm">1.</span>
                <span className="text-white font-semibold">Rules</span>
                <span className="text-white/50 text-sm">- Define conditions and actions</span>
              </div>
              <div className="bg-black/40 rounded p-6 font-mono text-xs text-white/90 overflow-x-auto">
                <div className="text-white/50">// rules.json</div>
                <div>&#123;</div>
                <div className="pl-2">"id": "unlimited-delegation",</div>
                <div className="pl-2">"name": "Block Unlimited Delegations",</div>
                <div className="pl-2">"enabled": true,</div>
                <div className="pl-2">"rule": &#123;</div>
                <div className="pl-4">"action": "block",</div>
                <div className="pl-4">"conditions": &#123;</div>
                <div className="pl-6">"field": "delegation_is_unlimited",</div>
                <div className="pl-6">"operator": "equals",</div>
                <div className="pl-6">"value": true</div>
                <div className="pl-4">&#125;,</div>
                <div className="pl-4">"message": "BLOCKED: Unlimited token delegation"</div>
                <div className="pl-2">&#125;</div>
                <div>&#125;</div>
              </div>
            </div>

            <div>
              <div className="flex items-center gap-2 mb-3">
                <span className="text-purple-400 font-mono text-sm">2.</span>
                <span className="text-white font-semibold">Analyzers</span>
                <span className="text-white/50 text-sm">- Provide security data fields</span>
              </div>
              <div className="bg-black/40 rounded p-6 font-mono text-xs text-white/90">
                <div className="text-white/50">// Core analyzers:</div>
                <div className="pl-2">• SecurityAnalyzer - Risk scoring, delegation detection</div>
                <div className="pl-2">• TokenInstructionAnalyzer - SPL Token operations</div>
                <div className="pl-2">• BasicAnalyzer - Transaction metadata</div>
                <div className="mt-2 text-white/50">// Third-party analyzers:</div>
                <div className="pl-2">• HelixIdentityAnalyzer - Scammer address checks</div>
                <div className="pl-2">• OttersecVerifiedAnalyzer - Program verification</div>
              </div>
            </div>
          </div>

          <div className="mt-6 text-sm text-white/60">
            Add custom analyzers, adjust risk thresholds, or define project-specific rules without changing code.
          </div>
        </div>
      </div>

      {/* Deployment Options */}
      <div className="mb-16 sm:mb-20">
        <h2 className="text-2xl sm:text-3xl font-bold mb-8 text-white">Deployment options</h2>
        <div className="grid md:grid-cols-2 gap-8">
          <div className="bg-white/5 border border-white/10 rounded-lg p-8">
            <h3 className="text-xl font-bold text-white mb-4">Hosted</h3>
            <p className="text-white/70 mb-4 text-sm">
              Point your wallet to our managed RPC. Get started in 30 seconds with zero setup.
            </p>
            <div className="bg-black/40 rounded p-4 mb-4 font-mono text-sm text-blue-400">
              rpc.securecheck.io
            </div>
            <Link
              to="/signup"
              className="inline-block w-full px-6 py-3 bg-blue-600 text-white font-semibold rounded-lg hover:bg-blue-700 transition-colors text-center text-sm"
            >
              Get API Key
            </Link>
          </div>
          <div className="bg-white/5 border border-white/10 rounded-lg p-8">
            <h3 className="text-xl font-bold text-white mb-4">Self-hosted</h3>
            <p className="text-white/70 mb-4 text-sm">
              Deploy your own proxy with one-click DigitalOcean setup or run locally.
            </p>
            <div className="bg-black/40 rounded p-4 mb-4 font-mono text-sm text-green-400">
              terraform apply
            </div>
            <a
              href="https://github.com/securecheckio/sol-shield"
              target="_blank"
              rel="noopener noreferrer"
              className="inline-block w-full px-6 py-3 bg-white/10 border border-white/20 text-white font-semibold rounded-lg hover:bg-white/20 transition-colors text-center text-sm"
            >
              View on GitHub
            </a>
          </div>
        </div>
      </div>

      {/* Integration */}
      <div className="mb-16 sm:mb-20">
        <h2 className="text-2xl sm:text-3xl font-bold mb-8 text-white">Integration</h2>
        <div className="bg-white/5 border border-white/10 rounded-lg p-8">
          <div className="space-y-6">
            <div>
              <h3 className="text-white font-semibold mb-2">Wallets</h3>
              <p className="text-white/70 text-sm">Update your RPC endpoint setting to point to SecureCheck. No code changes needed.</p>
            </div>
            <div>
              <h3 className="text-white font-semibold mb-2">dApps & Web Apps</h3>
              <p className="text-white/70 text-sm">Pass the RPC URL when initializing your Solana connection or wallet adapter.</p>
            </div>
            <div>
              <h3 className="text-white font-semibold mb-2">Backend Services & Bots</h3>
              <p className="text-white/70 text-sm">Use as a library in Rust applications or connect via HTTP RPC for any language.</p>
            </div>
            <div>
              <h3 className="text-white font-semibold mb-2">🦞 AI Agents</h3>
              <p className="text-white/70 text-sm">Autonomous agents using existing skills can leverage the same RPC endpoint for built-in security.</p>
            </div>
          </div>
        </div>
      </div>

      {/* Output Formats */}
      <div className="mb-16 sm:mb-20">
        <h2 className="text-2xl sm:text-3xl font-bold mb-8 text-white">Forensic output formats</h2>
        <div className="bg-white/5 border border-white/10 rounded-lg p-8">
          <p className="text-white/70 mb-6">
            Export transaction analysis data in multiple formats for compliance, audit trails, and regulatory reporting.
          </p>
          <div className="grid sm:grid-cols-3 gap-6">
            <div>
              <div className="text-blue-400 font-semibold mb-2">JSON-LS</div>
              <p className="text-white/70 text-sm">JSON Lines format for log analysis and data pipelines</p>
            </div>
            <div>
              <div className="text-purple-400 font-semibold mb-2">ISO 20022</div>
              <p className="text-white/70 text-sm">International financial messaging standard for institutional reporting</p>
            </div>
            <div>
              <div className="text-green-400 font-semibold mb-2">XBRL-JSON</div>
              <p className="text-white/70 text-sm">eXtensible Business Reporting Language for regulatory compliance</p>
            </div>
          </div>
        </div>
      </div>

      {/* CTA */}
      <div className="bg-white/5 border border-white/20 rounded-lg p-8 sm:p-10 text-center">
        <h2 className="text-2xl sm:text-3xl font-bold mb-3 text-white">Start testing</h2>
        <p className="text-sm sm:text-base text-white/70 mb-8">
          Free access for early testers
        </p>
        <Link
          to="/signup"
          className="inline-block px-8 sm:px-10 py-3 sm:py-4 bg-blue-600 text-white font-bold rounded-lg hover:bg-blue-700 transition-colors text-base sm:text-lg"
        >
          Join Testing Program
        </Link>
      </div>
    </div>
  );
};

export default Security;
