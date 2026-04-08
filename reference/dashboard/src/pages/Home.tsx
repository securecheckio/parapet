import { FC } from 'react';
import { Link } from 'react-router-dom';
import LogoCarousel from '../components/LogoCarousel';

const Home: FC = () => {

  return (
    <div className="relative">
      {/* Hero Section */}
      <div className="relative overflow-hidden">
        <div className="max-w-7xl mx-auto px-6 sm:px-6 lg:px-8 py-12 sm:py-20 lg:py-32">
          <div className="text-center">
            <div className="inline-flex items-center gap-2 border border-blue-500/30 bg-blue-500/10 rounded-full px-4 py-2 mb-4 sm:mb-6">
              <span className="text-lg">🦞</span>
              <span className="text-xs sm:text-sm text-blue-400 uppercase tracking-wide font-medium">Protection for the Agent Economy</span>
            </div>
            <h1 className="text-3xl sm:text-4xl md:text-5xl lg:text-6xl xl:text-7xl font-bold mb-4 sm:mb-6 lg:mb-8 leading-tight sm:leading-[1.1] text-white max-w-4xl mx-auto px-2">
              Transaction security for Solana
            </h1>
            <p className="text-sm sm:text-base lg:text-lg xl:text-xl text-white/70 mb-6 sm:mb-8 lg:mb-12 max-w-2xl sm:max-w-3xl mx-auto leading-relaxed px-4">
              Defense-in-depth protection that analyzes transactions before they reach the blockchain. Firewall protection for your wallet.
            </p>
            <div className="flex flex-col sm:flex-row items-center justify-center gap-3 sm:gap-4 px-4">
              <Link
                to="/signup"
                className="w-full sm:w-auto px-6 sm:px-8 py-3 sm:py-4 bg-gradient-to-r from-blue-600 to-purple-600 text-white font-semibold rounded-lg hover:from-blue-700 hover:to-purple-700 transition-all hover:scale-[1.02] text-center shadow-lg shadow-blue-500/20 text-sm sm:text-base"
              >
                Start for Free
              </Link>
              <Link
                to="/security"
                className="w-full sm:w-auto px-6 sm:px-8 py-3 sm:py-4 bg-white/10 font-semibold rounded-lg hover:bg-white/20 transition-all border border-white/20 text-center text-sm sm:text-base"
              >
                See How It Works
              </Link>
            </div>
            <p className="text-xs sm:text-sm text-white/50 mt-4 sm:mt-6 px-4">Test mode • Seeking early users to help test and verify this tool</p>
            
            {/* Logo Carousel */}
            <div className="mt-12 sm:mt-16 lg:mt-20">
              <LogoCarousel />
            </div>
          </div>
        </div>
      </div>

      {/* How It Works */}
      <div className="bg-white/[0.02] border-y border-white/10">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-16 sm:py-20 lg:py-24">
          <div className="text-center mb-12 sm:mb-16">
            <div className="inline-flex items-center gap-2 border border-white/20 rounded-full px-4 py-2 mb-6">
              <span className="text-xs text-white/50 uppercase tracking-wide">How It Works</span>
            </div>
            <h2 className="text-3xl sm:text-4xl lg:text-5xl font-bold mb-4 text-white">Three steps to safety</h2>
            <p className="text-base sm:text-lg text-white/70 max-w-3xl mx-auto">
              Invisible protection that analyzes every transaction in under 50ms
            </p>
          </div>

          <div className="grid md:grid-cols-3 gap-8">
            <div className="bg-white/5 border-l-4 border-blue-500 p-8 rounded-lg">
              <div className="text-blue-400 font-bold text-lg mb-2">1. Intercept</div>
              <p className="text-white/70">Your wallet sends transactions to SecureCheck's RPC proxy before passing through to your existing RPC</p>
            </div>
            <div className="bg-white/5 border-l-4 border-purple-500 p-8 rounded-lg">
              <div className="text-purple-400 font-bold text-lg mb-2">2. Analyze</div>
              <p className="text-white/70">Sub-50ms security scan checks patterns, addresses, and authorities against our threat database</p>
            </div>
            <div className="bg-white/5 border-l-4 border-green-500 p-8 rounded-lg">
              <div className="text-green-400 font-bold text-lg mb-2">3. Forward or Block</div>
              <p className="text-white/70">Safe transactions pass to your RPC. Threats are blocked with detailed explanations.</p>
            </div>
          </div>
        </div>
      </div>

      {/* What We Protect Against */}
      <div className="bg-zinc-900 border-y border-white/10">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-16 sm:py-20 lg:py-24">
          <div className="text-center mb-12 sm:mb-16">
            <div className="inline-flex items-center gap-2 border border-white/20 rounded-full px-4 py-2 mb-6">
              <span className="text-xs text-white/50 uppercase tracking-wide">Protection</span>
            </div>
            <h2 className="text-3xl sm:text-4xl lg:text-5xl font-bold mb-4 text-white">Stop threats before they cost you</h2>
            <p className="text-base sm:text-lg text-white/70 max-w-3xl mx-auto">
              Real-time protection against the attacks draining millions from Solana users every month
            </p>
          </div>

          <div className="grid sm:grid-cols-2 lg:grid-cols-3 gap-6 sm:gap-8">
            <ThreatCard
              title="Wallet Drainers"
              description="Stop unlimited token approvals (u64::MAX) that give attackers full access to drain your tokens"
            />
            <ThreatCard
              title="Phishing Addresses"
              description="Block transactions to 5,100+ known scammer addresses from Helius database"
            />
            <ThreatCard
              title="Fake Tokens"
              description="Verify program source code through OtterSec to catch unverified or malicious contracts"
            />
            <ThreatCard
              title="Authority Hijacking"
              description="Alert on mint authority, freeze authority, or ownership changes"
            />
            <ThreatCard
              title="Suspicious Patterns"
              description="Detect multiple delegations, account draining patterns, unusual fee structures, and suspicious activity"
            />
            <ThreatCard
              title="Community & Custom Rules"
              description="Community-driven rules for emerging threats, customized policies for your needs, and support for additional analyzers"
            />
          </div>
        </div>
      </div>

      {/* Solutions Section */}
      <div className="bg-zinc-900 border-y border-white/10">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-12 sm:py-16 lg:py-20">
          <div className="mb-8 sm:mb-12 lg:mb-16">
            <div className="inline-block border-l-4 border-purple-500 pl-4 sm:pl-6 mb-4 sm:mb-6">
              <h2 className="text-2xl sm:text-3xl lg:text-4xl xl:text-5xl font-bold text-white">Solutions for any business</h2>
            </div>
            <p className="text-sm sm:text-base lg:text-lg text-white/70 max-w-3xl pl-0 sm:pl-10">
              Purpose-built security infrastructure for every type of Solana application
            </p>
          </div>

        <div className="grid sm:grid-cols-2 lg:grid-cols-3 gap-6 sm:gap-8">
          <SolutionCard
            title="Wallets"
            description="Protect users with real-time transaction analysis and threat detection before they sign."
          />
          <SolutionCard
            title="DeFi Protocols"
            description="Add an extra security layer to protect users from malicious programs and drainer attacks."
          />
          <SolutionCard
            title="NFT Marketplaces"
            description="Verify token authenticity and protect against fake mints and authorization exploits."
          />
          <SolutionCard
            title="Exchanges"
            description="Screen all transactions for compliance and security before processing on-chain."
          />
          <SolutionCard
            title="dApps"
            description="Build user trust with transparent, auditable security checks on every transaction."
          />
          <SolutionCard
            title="Trading Bots"
            description="Protect automated strategies from interacting with malicious contracts and tokens."
          />
        </div>
        </div>
      </div>

      {/* CTA Section */}
      <div className="bg-zinc-900 border-y-4 border-blue-500">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-12 sm:py-16 lg:py-20 text-center">
          <h2 className="text-2xl sm:text-3xl lg:text-4xl xl:text-5xl font-bold mb-3 sm:mb-4 lg:mb-6 text-white">Help us test and verify</h2>
          <p className="text-sm sm:text-base lg:text-lg text-white/70 mb-6 sm:mb-8 lg:mb-10 max-w-2xl mx-auto px-4">
            We're looking for early users to help test and validate this security tool.
          </p>
          <div className="flex flex-col sm:flex-row items-center justify-center gap-3 sm:gap-4 px-4">
            <Link
              to="/signup"
              className="w-full sm:w-auto px-8 sm:px-10 py-3 sm:py-4 bg-blue-600 text-white font-bold rounded hover:bg-blue-700 transition-colors text-center text-base sm:text-lg"
            >
              Join Testing Program
            </Link>
            <Link
              to="/security"
              className="w-full sm:w-auto px-8 sm:px-10 py-3 sm:py-4 border-2 border-white/30 font-semibold rounded hover:border-white/50 hover:bg-white/5 transition-all text-center text-base sm:text-lg"
            >
              View Security Features
            </Link>
          </div>
        </div>
      </div>
    </div>
  );
};

interface SolutionCardProps {
  title: string;
  description: string;
}

const SolutionCard: FC<SolutionCardProps> = ({ title, description }) => (
  <div className="bg-black/30 border border-white/10 rounded p-6 sm:p-8 hover:border-purple-500/50 transition-colors">
    <h3 className="text-xl sm:text-2xl font-bold mb-3 text-white">{title}</h3>
    <p className="text-white/60 text-sm sm:text-base leading-relaxed">{description}</p>
  </div>
);

interface ThreatCardProps {
  title: string;
  description: string;
}

const ThreatCard: FC<ThreatCardProps> = ({ title, description }) => (
  <div className="bg-white/5 border border-white/10 rounded-lg p-6 sm:p-8 hover:bg-white/[0.07] transition-colors">
    <h3 className="font-bold text-xl sm:text-2xl text-white mb-3">{title}</h3>
    <p className="text-sm sm:text-base text-white/70 leading-relaxed">{description}</p>
  </div>
);

export default Home;
