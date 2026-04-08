import { FC, useState, useEffect } from 'react';
import { useWallet } from '@solana/wallet-adapter-react';
import { useNavigate } from 'react-router-dom';
import bs58 from 'bs58';
import { apiService } from '../services/api';

const Signup: FC = () => {
  const { publicKey, signMessage, connected } = useWallet();
  const navigate = useNavigate();
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [step, setStep] = useState<'connect' | 'sign' | 'complete'>('connect');

  useEffect(() => {
    if (connected && publicKey) {
      setStep('sign');
    } else {
      setStep('connect');
    }
  }, [connected, publicKey]);

  useEffect(() => {
    // Check if user is already logged in (has session)
    const checkSession = async () => {
      try {
        const user = await apiService.getCurrentUser();
        if (user) {
          // User is already logged in, redirect to dashboard
          navigate('/dashboard');
        }
      } catch {
        // Not logged in, continue with signup flow
      }
    };
    checkSession();
  }, [navigate]);

  const handleSignMessage = async () => {
    if (!publicKey || !signMessage) {
      setError('Wallet not connected');
      return;
    }

    try {
      setLoading(true);
      setError(null);

      const message = `Sign this message to login to SecureCheck.\n\nWallet: ${publicKey.toString()}\nTimestamp: ${Date.now()}`;
      const encodedMessage = new TextEncoder().encode(message);
      const signature = await signMessage(encodedMessage);

      // Use session-based login instead of API key signup
      await apiService.login({
        wallet_address: publicKey.toString(),
        message,
        signature: bs58.encode(signature),
      });

      // Redirect to dashboard (session cookie is now set)
      navigate('/dashboard');
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to sign in');
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="max-w-3xl mx-auto px-4 sm:px-6 py-8 sm:py-12 md:py-16">
      <div className="bg-white/5 rounded-2xl border border-white/10 p-6 sm:p-8 md:p-12">
        <div className="text-center mb-8 sm:mb-12">
          <h1 className="text-2xl sm:text-3xl md:text-4xl font-bold mb-4">Get Started</h1>
          <p className="text-white/60 text-sm sm:text-base">
            Connect your Solana wallet to receive your API key instantly
          </p>
        </div>

        {error && (
          <div className="mb-8 p-4 bg-red-500/10 border border-red-500/20 rounded-lg text-red-400">
            {error}
          </div>
        )}

        <div className="space-y-8">
          {/* Step 1: Connect Wallet */}
          <div className={`${step === 'connect' ? '' : 'opacity-50'}`}>
            <div className="flex items-center gap-3 mb-4">
              <div className={`w-8 h-8 rounded-full flex items-center justify-center font-bold ${
                step !== 'connect' ? 'bg-green-500' : 'bg-white/10'
              }`}>
                {step !== 'connect' ? '✓' : '1'}
              </div>
              <h2 className="text-lg sm:text-xl font-semibold">Connect Wallet</h2>
            </div>
            {connected && publicKey ? (
              <div className="ml-11 p-3 sm:p-4 bg-white/5 rounded-lg border border-white/10">
                <div className="text-xs sm:text-sm text-white/60 mb-2">Connected Wallet</div>
                <div className="font-mono text-xs sm:text-sm break-all">{publicKey.toString()}</div>
              </div>
            ) : (
              <div className="ml-11 text-white/60 text-sm sm:text-base">
                Click the "Select Wallet" button in the top right to connect your Solana wallet
              </div>
            )}
          </div>

          {/* Step 2: Sign Message */}
          <div className={`${step === 'sign' ? '' : 'opacity-50'}`}>
            <div className="flex items-center gap-3 mb-4">
              <div className={`w-8 h-8 rounded-full flex items-center justify-center font-bold ${
                step === 'complete' ? 'bg-green-500' : 'bg-white/10'
              }`}>
                {step === 'complete' ? '✓' : '2'}
              </div>
              <h2 className="text-lg sm:text-xl font-semibold">Sign Message</h2>
            </div>
            {step === 'sign' && (
              <div className="ml-11 space-y-4">
                <p className="text-white/60 text-xs sm:text-sm">
                  Sign a message to prove you own this wallet. This is free and doesn't create a transaction.
                </p>
                <button
                  onClick={handleSignMessage}
                  disabled={loading}
                  className="px-6 py-3 bg-white text-black font-semibold rounded-lg hover:bg-white/90 transition-colors disabled:opacity-50 disabled:cursor-not-allowed w-full sm:w-auto text-sm sm:text-base"
                >
                  {loading ? 'Signing...' : 'Sign Message'}
                </button>
              </div>
            )}
          </div>

        </div>
      </div>

      {/* Self-hosted option */}
      <div className="mt-8 text-center">
        <p className="text-white/60 text-sm">
          Or{' '}
          <a
            href="https://github.com/securecheckio/parapet"
            target="_blank"
            rel="noopener noreferrer"
            className="text-blue-400 hover:text-blue-300 font-semibold transition-colors"
          >
            self-host your own RPC proxy
          </a>
        </p>
      </div>
    </div>
  );
};

export default Signup;
