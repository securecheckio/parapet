import { FC, useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { apiService } from '../services/api';
import type { PricingPackage, TokenInfo } from '../services/api';
import QRCode from 'qrcode';

const Pricing: FC = () => {
  const navigate = useNavigate();
  const [packages, setPackages] = useState<PricingPackage[]>([]);
  const [tokenInfo, setTokenInfo] = useState<TokenInfo | null>(null);
  const [enabled, setEnabled] = useState(true);
  const [loading, setLoading] = useState(true);
  const [selectedPackage, setSelectedPackage] = useState<string | null>(null);
  const [paymentUrl, setPaymentUrl] = useState<string | null>(null);
  const [paymentId, setPaymentId] = useState<string | null>(null);
  const [qrCodeDataUrl, setQrCodeDataUrl] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    fetchPricing();
  }, []);

  const fetchPricing = async () => {
    try {
      const data = await apiService.getPricing();
      setEnabled(data.enabled);
      setPackages(data.packages);
      setTokenInfo(data.token_info || null);
    } catch (err) {
      setError('Failed to load pricing');
    } finally {
      setLoading(false);
    }
  };

  const handleSelectPackage = async (pkg: string) => {
    const apiKey = localStorage.getItem('securecheck_api_key');
    if (!apiKey) {
      navigate('/signup');
      return;
    }

    try {
      setError(null);
      const payment = await apiService.createPayment({
        api_key: apiKey,
        package: pkg,
        token_type: 'xlabs',
      });

      setSelectedPackage(pkg);
      setPaymentUrl(payment.payment_url);
      setPaymentId(payment.payment_id);

      // Generate QR code
      const qr = await QRCode.toDataURL(payment.payment_url, { width: 300 });
      setQrCodeDataUrl(qr);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to create payment');
    }
  };

  const handleVerifyPayment = async () => {
    const signature = prompt('Enter transaction signature from Phantom:');
    if (!signature || !paymentId) return;

    try {
      setError(null);
      const result = await apiService.verifyPayment({
        payment_id: paymentId,
        signature,
      });

      if (result.verified) {
        alert(`Payment confirmed! ${result.credits_purchased} credits added. New balance: ${result.new_balance}`);
        setSelectedPackage(null);
        setPaymentUrl(null);
        setPaymentId(null);
        setQrCodeDataUrl(null);
      } else {
        setError('Payment not verified');
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Verification failed');
    }
  };

  if (loading) {
    return (
      <div className="max-w-7xl mx-auto px-6 py-16">
        <div className="text-center text-white/60">Loading pricing...</div>
      </div>
    );
  }

  if (!enabled) {
    return (
      <div className="max-w-4xl mx-auto px-6 py-16">
        <div className="bg-green-500/10 border border-green-500/20 rounded-2xl p-12 text-center">
          <h1 className="text-4xl font-bold mb-4">Free Instance</h1>
          <p className="text-lg text-white/60">
            Payments are not enabled on this instance. You have unlimited requests with your API key.
          </p>
        </div>
      </div>
    );
  }

  if (paymentUrl && selectedPackage) {
    const pkg = packages.find(p => p.package === selectedPackage);
    return (
      <div className="max-w-3xl mx-auto px-6 py-16">
        <div className="bg-white/5 rounded-2xl border border-white/10 p-12">
          <h1 className="text-4xl font-bold mb-8 text-center">Complete Payment</h1>

          {error && (
            <div className="mb-8 p-4 bg-red-500/10 border border-red-500/20 rounded-lg text-red-400">
              {error}
            </div>
          )}

          <div className="bg-white/5 rounded-xl p-6 mb-8">
            <div className="grid grid-cols-2 gap-4 text-sm">
              <div>
                <div className="text-white/60">Package</div>
                <div className="font-semibold capitalize">{selectedPackage}</div>
              </div>
              <div>
                <div className="text-white/60">Credits</div>
                <div className="font-semibold">{pkg?.credits_formatted}</div>
              </div>
              <div>
                <div className="text-white/60">Price</div>
                <div className="font-semibold">{pkg?.token_amount_formatted} {tokenInfo?.symbol}</div>
              </div>
              <div>
                <div className="text-white/60">Token</div>
                <div className="font-semibold flex items-center gap-2">
                  {tokenInfo?.logo && <img src={tokenInfo.logo} alt="" className="w-5 h-5" />}
                  {tokenInfo?.symbol}
                </div>
              </div>
            </div>
          </div>

          {qrCodeDataUrl && (
            <div className="text-center mb-8">
              <p className="text-white/60 mb-4">Scan with your Solana wallet</p>
              <img src={qrCodeDataUrl} alt="Payment QR" className="mx-auto rounded-lg border border-white/10" />
            </div>
          )}

          <div className="space-y-3">
            <button
              onClick={() => navigator.clipboard.writeText(paymentUrl)}
              className="w-full px-6 py-3 bg-white/10 font-semibold rounded-lg hover:bg-white/20 transition-colors"
            >
              Copy Payment Link
            </button>
            <button
              onClick={() => window.open(paymentUrl, '_blank')}
              className="w-full px-6 py-3 bg-white/10 font-semibold rounded-lg hover:bg-white/20 transition-colors"
            >
              Open in Phantom
            </button>
            <button
              onClick={handleVerifyPayment}
              className="w-full px-6 py-3 bg-white text-black font-semibold rounded-lg hover:bg-white/90 transition-colors"
            >
              Verify Payment
            </button>
            <button
              onClick={() => {
                setSelectedPackage(null);
                setPaymentUrl(null);
                setPaymentId(null);
                setQrCodeDataUrl(null);
              }}
              className="w-full px-6 py-3 bg-red-500/10 text-red-400 font-semibold rounded-lg hover:bg-red-500/20 transition-colors"
            >
              Cancel
            </button>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="max-w-7xl mx-auto px-6 py-16">
      <div className="text-center mb-16">
        <h1 className="text-5xl font-bold mb-4">Buy Request Credits</h1>
        <p className="text-xl text-white/60">
          Pay with {tokenInfo?.name} tokens • Credits never expire
        </p>
      </div>

      {error && (
        <div className="mb-8 p-4 bg-red-500/10 border border-red-500/20 rounded-lg text-red-400 text-center">
          {error}
        </div>
      )}

      <div className="grid md:grid-cols-3 gap-8">
        {packages.map((pkg) => (
          <div
            key={pkg.package}
            className="bg-white/5 rounded-2xl border border-white/10 p-8 hover:border-white/20 transition-all"
          >
            <h3 className="text-2xl font-bold capitalize mb-2">{pkg.package}</h3>
            <div className="text-4xl font-bold mb-2">
              {pkg.token_amount_formatted}
              <span className="text-xl font-normal text-white/60 ml-2">{tokenInfo?.symbol}</span>
            </div>
            <div className="text-white/60 mb-6">{pkg.credits_formatted}</div>
            <ul className="space-y-3 mb-8 text-sm text-white/60">
              <li className="flex items-center gap-2">
                <span className="text-green-500">✓</span> Credits never expire
              </li>
              <li className="flex items-center gap-2">
                <span className="text-green-500">✓</span> Add to your balance
              </li>
              <li className="flex items-center gap-2">
                <span className="text-green-500">✓</span> Transaction security
              </li>
              <li className="flex items-center gap-2">
                <span className="text-green-500">✓</span> Rate limiting
              </li>
            </ul>
            <button
              onClick={() => handleSelectPackage(pkg.package)}
              className="w-full px-6 py-3 bg-white text-black font-semibold rounded-lg hover:bg-white/90 transition-colors"
            >
              Buy {pkg.package}
            </button>
          </div>
        ))}
      </div>
    </div>
  );
};

export default Pricing;
