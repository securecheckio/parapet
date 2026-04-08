import { FC, ReactNode, useState, useEffect } from 'react';
import { Link, useLocation, useNavigate } from 'react-router-dom';
import { useWallet } from '@solana/wallet-adapter-react';
import { WalletMultiButton } from '@solana/wallet-adapter-react-ui';
import { apiService } from '../services/api';

interface LayoutProps {
  children: ReactNode;
}

const Layout: FC<LayoutProps> = ({ children }) => {
  const location = useLocation();
  const navigate = useNavigate();
  const { connected } = useWallet();
  const [mobileMenuOpen, setMobileMenuOpen] = useState(false);
  const [wasConnected, setWasConnected] = useState(false);

  const isActive = (path: string) => location.pathname === path;

  // Handle wallet disconnect
  useEffect(() => {
    if (wasConnected && !connected) {
      // Wallet was disconnected
      console.log('Wallet disconnected, logging out...');
      const logout = async () => {
        try {
          await apiService.logout();
        } catch (error) {
          console.error('Logout error:', error);
        } finally {
          // Redirect to home page
          if (location.pathname === '/dashboard') {
            navigate('/');
          }
        }
      };
      logout();
    }
    setWasConnected(connected);
  }, [connected, wasConnected, location.pathname, navigate]);

  return (
    <div className="min-h-screen bg-slate-950">
      <nav className="border-b border-slate-800 bg-slate-950/80 backdrop-blur-sm sticky top-0 z-50">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 py-4">
          <div className="flex items-center justify-between">
            <Link to="/" className="flex items-center gap-3">
              <span className="text-lg sm:text-xl font-semibold text-slate-50">SecureCheck.io</span>
            </Link>

            {/* Desktop Navigation */}
            <div className="hidden md:flex items-center gap-6">
              <Link
                to="/dashboard"
                className={`text-sm font-medium transition-colors ${
                  isActive('/dashboard') ? 'text-slate-50' : 'text-slate-400 hover:text-slate-200'
                }`}
              >
                Dashboard
              </Link>
              <Link
                to="/learn"
                className={`text-sm font-medium transition-colors ${
                  isActive('/learn') || location.pathname.startsWith('/learn/') ? 'text-slate-50' : 'text-slate-400 hover:text-slate-200'
                }`}
              >
                Learn
              </Link>
              <Link
                to="/security"
                className={`text-sm font-medium transition-colors ${
                  isActive('/security') ? 'text-slate-50' : 'text-slate-400 hover:text-slate-200'
                }`}
              >
                How it works
              </Link>
              <Link
                to="/use-cases"
                className={`text-sm font-medium transition-colors ${
                  isActive('/use-cases') ? 'text-slate-50' : 'text-slate-400 hover:text-slate-200'
                }`}
              >
                Use Cases
              </Link>
              <WalletMultiButton className="!bg-slate-800 !hover:bg-slate-700 !border !border-slate-700" />
            </div>

            {/* Mobile Menu Button */}
            <button
              onClick={() => setMobileMenuOpen(!mobileMenuOpen)}
              className="md:hidden text-slate-300 p-2"
              aria-label="Toggle menu"
            >
              <svg className="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                {mobileMenuOpen ? (
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
                ) : (
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 6h16M4 12h16M4 18h16" />
                )}
              </svg>
            </button>
          </div>

          {/* Mobile Menu */}
          {mobileMenuOpen && (
            <div className="md:hidden mt-4 pb-4 space-y-4 border-t border-slate-800 pt-4">
              <div className="flex flex-col space-y-3">
                <Link
                  to="/dashboard"
                  className={`block text-sm font-medium transition-colors ${
                    isActive('/dashboard') ? 'text-slate-50' : 'text-slate-400'
                  }`}
                  onClick={() => setMobileMenuOpen(false)}
                >
                  Dashboard
                </Link>
                <Link
                  to="/learn"
                  className={`block text-sm font-medium transition-colors ${
                    isActive('/learn') || location.pathname.startsWith('/learn/') ? 'text-slate-50' : 'text-slate-400'
                  }`}
                  onClick={() => setMobileMenuOpen(false)}
                >
                  Learn
                </Link>
                <Link
                  to="/security"
                  className={`block text-sm font-medium transition-colors ${
                    isActive('/security') ? 'text-slate-50' : 'text-slate-400'
                  }`}
                  onClick={() => setMobileMenuOpen(false)}
                >
                  How it works
                </Link>
                <Link
                  to="/use-cases"
                  className={`block text-sm font-medium transition-colors ${
                    isActive('/use-cases') ? 'text-slate-50' : 'text-slate-400'
                  }`}
                  onClick={() => setMobileMenuOpen(false)}
                >
                  Use Cases
                </Link>
              </div>
              
              {/* Wallet Button for Mobile */}
              <div className="pt-3 border-t border-slate-800">
                <WalletMultiButton className="!w-full !bg-slate-800 !hover:bg-slate-700 !border !border-slate-700 !justify-center" />
              </div>
            </div>
          )}
        </div>
      </nav>

      <main>{children}</main>

      <footer className="border-t border-slate-800 mt-24 bg-slate-950">
        <div className="max-w-7xl mx-auto px-6 py-12">
          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-8">
            <div>
              <div className="mb-4">
                <span className="font-semibold text-slate-200">SecureCheck.io</span>
              </div>
              <p className="text-sm text-slate-400">
                Enterprise-grade transaction security for Solana
              </p>
            </div>
            <div>
              <h3 className="font-semibold text-slate-200 mb-4">Product</h3>
              <ul className="space-y-2 text-sm text-slate-400">
                <li><Link to="/security" className="hover:text-slate-200 transition-colors">How it works</Link></li>
                <li><Link to="/use-cases" className="hover:text-slate-200 transition-colors">Use Cases</Link></li>
                <li><Link to="/dashboard" className="hover:text-slate-200 transition-colors">Dashboard</Link></li>
                <li><Link to="/learn" className="hover:text-slate-200 transition-colors">Learn</Link></li>
              </ul>
            </div>
            <div>
              <h3 className="font-semibold text-slate-200 mb-4">Resources</h3>
              <ul className="space-y-2 text-sm text-slate-400">
                <li><a href="https://github.com/securecheckio/parapet" target="_blank" rel="noopener noreferrer" className="hover:text-slate-200 transition-colors">Documentation</a></li>
              </ul>
            </div>
            <div>
              <h3 className="font-semibold text-slate-200 mb-4">Contact</h3>
              <ul className="space-y-2 text-sm text-slate-400">
                <li><a href="https://x.com/securecheckio" target="_blank" rel="noopener noreferrer" className="hover:text-slate-200 transition-colors">Support</a></li>
                <li><a href="https://x.com/securecheckio" target="_blank" rel="noopener noreferrer" className="hover:text-slate-200 transition-colors">Partnerships</a></li>
              </ul>
            </div>
          </div>
          <div className="mt-12 pt-8 border-t border-slate-800">
            <div className="flex items-center justify-center gap-2 mb-4 text-sm text-slate-500">
              <span>Powered by</span>
              <a 
                href="https://www.helius.dev" 
                target="_blank" 
                rel="noopener noreferrer"
                className="text-blue-400 hover:text-blue-300 font-semibold transition-colors"
              >
                Helius
              </a>
            </div>
            <div className="text-center text-sm text-slate-500">
              © 2026 SecureCheck.io. All rights reserved.
            </div>
          </div>
        </div>
      </footer>
    </div>
  );
};

export default Layout;
