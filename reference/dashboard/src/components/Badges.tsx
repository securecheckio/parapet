import { FC, useEffect, useState } from 'react';
import { Award, Shield, ShieldCheck } from 'lucide-react';
import { apiService } from '../services/api';
import { useWallet } from '@solana/wallet-adapter-react';

interface Badge {
  id: string;
  name: string;
  description: string;
  icon: string;
  earned_at: string;
}

const iconMap: Record<string, any> = {
  'shield': Shield,
  'shield-check': ShieldCheck,
  'award': Award,
};

export const BadgesSection: FC = () => {
  const { connected } = useWallet();
  const [badges, setBadges] = useState<Badge[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    if (connected) {
      loadBadges();
    } else {
      setLoading(false);
    }
  }, [connected]);

  const loadBadges = async () => {
    try {
      const data = await apiService.getMyBadges();
      setBadges(data);
    } catch (error) {
      console.log('Failed to load badges');
    } finally {
      setLoading(false);
    }
  };

  if (!connected) {
    return (
      <div className="bg-white/5 border border-white/10 rounded-lg p-6 text-center">
        <Award className="mx-auto mb-3 text-slate-500" size={32} />
        <p className="text-slate-400 text-sm">Connect your wallet to view earned badges</p>
      </div>
    );
  }

  if (loading) {
    return (
      <div className="bg-white/5 border border-white/10 rounded-lg p-6 text-center">
        <p className="text-slate-400 text-sm">Loading badges...</p>
      </div>
    );
  }

  if (badges.length === 0) {
    return (
      <div className="bg-white/5 border border-white/10 rounded-lg p-6 text-center">
        <Award className="mx-auto mb-3 text-slate-500" size={32} />
        <p className="text-slate-400 text-sm mb-2">No badges earned yet</p>
        <p className="text-slate-500 text-xs">Complete courses to earn your first badge!</p>
      </div>
    );
  }

  return (
    <div>
      <h3 className="text-xl font-bold text-white mb-4">Your Badges</h3>
      <div className="grid sm:grid-cols-2 lg:grid-cols-3 gap-4">
        {badges.map((badge) => {
          const Icon = iconMap[badge.icon] || Award;
          return (
            <div
              key={badge.id}
              className="bg-gradient-to-br from-blue-500/10 to-purple-500/10 border border-blue-500/30 rounded-lg p-5 hover:border-blue-500/50 transition-all"
            >
              <div className="flex items-start gap-3">
                <div className="p-2 bg-blue-500/20 rounded-lg">
                  <Icon className="text-blue-400" size={24} />
                </div>
                <div className="flex-1">
                  <h4 className="font-semibold text-white mb-1">{badge.name}</h4>
                  <p className="text-xs text-slate-400 mb-2">{badge.description}</p>
                  <p className="text-xs text-slate-500">
                    Earned {new Date(badge.earned_at).toLocaleDateString()}
                  </p>
                </div>
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
};

export default BadgesSection;
