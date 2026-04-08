import { FC, useState, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { useWallet } from '@solana/wallet-adapter-react';
import { apiService } from '../services/api';
import type { UserStats, SecurityEvent, ActiveRulesResponse, GlobalStats } from '../services/api';
import { RPC_URL } from '../config';
import { subscribeToPushNotifications, sendSubscriptionToServer } from '../services/push';

const Dashboard: FC = () => {
  console.log('🔥🔥🔥 DASHBOARD COMPONENT LOADED 🔥🔥🔥');
  const navigate = useNavigate();
  const { publicKey } = useWallet();
  const [stats, setStats] = useState<UserStats | null>(null);
  const [globalStats, setGlobalStats] = useState<GlobalStats | null>(null);
  const [events, setEvents] = useState<SecurityEvent[]>([]);
  const [loading, setLoading] = useState(true);
  console.log('🔥 Initial events state:', events.length);
  const [error, setError] = useState<string | null>(null);
  const [regenerateApiKey, setRegenerateApiKey] = useState(false);
  const [newApiKey, setNewApiKey] = useState<string | null>(null);
  const [currentApiKey] = useState<string | null>(() => localStorage.getItem('securecheck_api_key'));
  const [rpcUrl, setRpcUrl] = useState<string>(RPC_URL);
  const [sessionWallet, setSessionWallet] = useState<string | null>(null);
  const [rules, setRules] = useState<ActiveRulesResponse | null>(null);
  const [showRules, setShowRules] = useState(false);
  const [currentPage, setCurrentPage] = useState(1);
  const [showSecurityDropdown, setShowSecurityDropdown] = useState(false);
  const [customThreshold, setCustomThreshold] = useState<number | null>(null);
  const [updatingThreshold, setUpdatingThreshold] = useState(false);
  const [totalEvents, setTotalEvents] = useState(0);
  const [loadingMore, setLoadingMore] = useState(false);
  const [wsConnected, setWsConnected] = useState(false);
  const [solanaNetwork, setSolanaNetwork] = useState<string>('devnet');
  const [showNetworkDetails, setShowNetworkDetails] = useState(false);
  const [lastGlobalStatsUpdate, setLastGlobalStatsUpdate] = useState<number>(Date.now());
  const eventsPerPage = 10;

  useEffect(() => {
    const fetchStats = async () => {
      try {
        console.log('🔥 fetchStats starting...');
        // Check if user is logged in (has session)
        const currentUser = await apiService.getCurrentUser().catch((err) => {
          console.error('🔥 getCurrentUser FAILED:', err);
          return null;
        });
        
        console.log('🔥 currentUser:', currentUser);
        
        if (!currentUser) {
          console.log('🔥 NO USER - redirecting to signup');
          navigate('/signup');
          return;
        }
        
        console.log('🔥 User authenticated, fetching events...');

        // Store the wallet address from the session
        setSessionWallet(currentUser.wallet_address);

      const [userStats, securityEvents, systemStatus, activeRules, networkInfo, networkStats] = await Promise.all([
        apiService.getMyStats(),
        apiService.getMyEvents(50, 0),
        apiService.getStatus().catch(() => ({ 
          auth_api: true, 
          rpc_proxy: true, 
          payment_system: false,
          rpc_url: undefined 
        })),
        apiService.getActiveRules().catch(() => null),
        apiService.getNetworkInfo().catch(() => ({ network: 'devnet' })),
        apiService.getGlobalStats().catch(() => null),
      ]);
      
      // Deduplicate events by ID
      const uniqueEvents = Array.from(
        new Map(securityEvents.map(e => [e.id, e])).values()
      );
      
      setStats(userStats);
      setGlobalStats(networkStats);
      setEvents(uniqueEvents);
      setTotalEvents(uniqueEvents.length);
      setRules(activeRules);
      setSolanaNetwork(networkInfo.network);
      setCurrentPage(1);
      
      // Use RPC URL from API if available, otherwise use config default
      if (systemStatus?.rpc_url) {
        setRpcUrl(systemStatus.rpc_url);
      }
      
      setError(null);
      } catch (error) {
        console.error('Failed to fetch stats:', error);
        if (error instanceof Error && error.message.includes('Unauthorized')) {
          navigate('/signup');
        } else {
          setError('Failed to load dashboard. Please try again.');
        }
      } finally {
        setLoading(false);
      }
    };

    fetchStats();
  }, [navigate]);

  // Smart polling - only when needed and tab is visible
  useEffect(() => {
    let interval: NodeJS.Timeout | null = null;
    let isTabVisible = !document.hidden;

    // Handle tab visibility changes
    const handleVisibilityChange = () => {
      const wasHidden = !isTabVisible;
      isTabVisible = !document.hidden;
      
      // When tab becomes visible after being hidden, refresh immediately
      if (isTabVisible && wasHidden && !wsConnected) {
        refreshData();
      }
    };

    const refreshData = async () => {
      try {
        const [userStats, securityEvents, networkStats] = await Promise.all([
          apiService.getMyStats(),
          apiService.getMyEvents(50, 0),
          apiService.getGlobalStats().catch(() => null),
        ]);
        
        const uniqueEvents = Array.from(
          new Map(securityEvents.map(e => [e.id, e])).values()
        );
        
        setStats(userStats);
        if (networkStats) {
          setGlobalStats(networkStats);
          setLastGlobalStatsUpdate(Date.now());
        }
        setEvents(uniqueEvents);
        setTotalEvents(uniqueEvents.length);
      } catch (error) {
        console.error('Failed to refresh data:', error);
      }
    };

    // Only poll if WebSocket is not connected AND tab is visible
    const startPolling = () => {
      if (interval) clearInterval(interval);
      
      interval = setInterval(() => {
        if (!wsConnected && isTabVisible) {
          refreshData();
        }
      }, 60000); // Poll every 60 seconds as fallback
    };

    // Listen for visibility changes
    document.addEventListener('visibilitychange', handleVisibilityChange);

    // Start polling if WebSocket is disconnected
    if (!wsConnected) {
      startPolling();
    }

    return () => {
      if (interval) clearInterval(interval);
      document.removeEventListener('visibilitychange', handleVisibilityChange);
    };
  }, [wsConnected]);

  // Refresh global stats when network details are expanded (if stale)
  useEffect(() => {
    if (showNetworkDetails && globalStats) {
      const timeSinceLastUpdate = Date.now() - lastGlobalStatsUpdate;
      // Only refresh if data is older than 30 seconds
      if (timeSinceLastUpdate > 30000) {
        apiService.getGlobalStats()
          .then(networkStats => {
            setGlobalStats(networkStats);
            setLastGlobalStatsUpdate(Date.now());
          })
          .catch(err => {
            console.error('Failed to refresh global stats:', err);
          });
      }
    }
  }, [showNetworkDetails]);

  // WebSocket connection for real-time updates
  useEffect(() => {
    console.log('🔌 Initializing WebSocket connection...');
    
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const wsUrl = `${protocol}//${window.location.hostname}:3001/dashboard/ws`;
    console.log('🌐 Connecting to:', wsUrl);
    
    let ws: WebSocket | null = null;
    let reconnectTimeout: NodeJS.Timeout;

    const connect = () => {
      try {
        ws = new WebSocket(wsUrl);
        
        ws.onopen = () => {
          console.log('✅ WebSocket connected successfully');
          setWsConnected(true);
        };

        ws.onmessage = (event) => {
          try {
            const update = JSON.parse(event.data);
            console.log('📨 WebSocket update received:', update);

            if (update.type === 'stats_update') {
              // Update credits immediately from WebSocket message
              setStats(prev => prev ? {
                ...prev,
                credits_balance: update.credits_balance,
                credits_used_this_month: update.credits_used_this_month,
                // If the WebSocket message includes transaction counts, use them
                total_requests: update.total_requests ?? prev.total_requests,
                total_blocked: update.total_blocked ?? prev.total_blocked,
                total_warnings: update.total_warnings ?? prev.total_warnings,
              } : null);
            } else if (update.type === 'new_event') {
              console.log('🔔 New security event detected, refreshing feed...');
              
              // Show browser notification if enabled and event is critical
              if (stats?.notifications_enabled && Notification.permission === 'granted') {
                const event = update.event;
                if (event) {
                  const isBlocked = event.outcome === 'blocked';
                  const isCritical = event.severity === 'critical' || event.severity === 'high';
                  
                  // Show notification for blocked or critical events
                  if (isBlocked || isCritical) {
                    const title = isBlocked ? '🚫 Transaction Blocked' : '⚠️ Security Alert';
                    const body = event.summary || event.message || 'Security event detected';
                    
                    const notification = new Notification(title, {
                      body: body,
                      icon: '/favicon.ico',
                      badge: '/favicon.ico',
                      tag: event.id, // Prevent duplicate notifications
                      requireInteraction: isBlocked, // Keep blocked notifications visible
                    });
                    
                    // Click to focus window
                    notification.onclick = () => {
                      window.focus();
                      notification.close();
                    };
                    
                    console.log('📢 Browser notification sent:', title, body);
                  }
                }
              }
              
              // Fetch events and updated stats on new event
              Promise.all([
                apiService.getMyEvents(50, 0),
                apiService.getMyStats(),
                apiService.getGlobalStats().catch(() => null),
              ]).then(([securityEvents, userStats, networkStats]) => {
                console.log(`✅ Fetched ${securityEvents.length} events after new_event notification`);
                const uniqueEvents = Array.from(
                  new Map(securityEvents.map(e => [e.id, e])).values()
                );
                setEvents(uniqueEvents);
                setTotalEvents(uniqueEvents.length);
                setStats(userStats);
                if (networkStats) setGlobalStats(networkStats);
              }).catch(err => {
                console.error('❌ Failed to refresh after new_event:', err);
              });
            }
          } catch (err) {
            console.error('Failed to parse WebSocket message:', err);
          }
        };

        ws.onerror = (error) => {
          console.error('❌ WebSocket error:', error);
          setWsConnected(false);
        };

        ws.onclose = () => {
          console.log('🔌 WebSocket disconnected, reconnecting in 5s...');
          setWsConnected(false);
          reconnectTimeout = setTimeout(connect, 5000);
        };
      } catch (err) {
        console.error('Failed to create WebSocket:', err);
        reconnectTimeout = setTimeout(connect, 5000);
      }
    };

    connect();

    return () => {
      if (ws) {
        ws.close();
      }
      if (reconnectTimeout) {
        clearTimeout(reconnectTimeout);
      }
    };
  }, []); // Empty dependency array - connect once on mount

  // Detect wallet changes and logout automatically
  useEffect(() => {
    if (sessionWallet && publicKey) {
      const currentWallet = publicKey.toString();
      if (currentWallet !== sessionWallet) {
        console.log('Wallet changed, logging out...');
        handleLogout();
      }
    }
  }, [publicKey, sessionWallet]);

  const handleLogout = async () => {
    try {
      await apiService.logout();
    } catch (error) {
      console.error('Logout error:', error);
    } finally {
      navigate('/signup');
    }
  };

  const copyApiKey = () => {
    if (newApiKey) {
      navigator.clipboard.writeText(newApiKey);
    }
  };

  const handleRegenerateApiKey = async () => {
    try {
      const response = await apiService.regenerateMyApiKey();
      setNewApiKey(response.api_key);
      setRegenerateApiKey(false);
      // Refresh stats
      const userStats = await apiService.getMyStats();
      setStats(userStats);
    } catch (error) {
      console.error('Failed to regenerate API key:', error);
      setError('Failed to regenerate API key. Please try again.');
    }
  };

  const formatNumber = (num: number) => {
    return num.toLocaleString();
  };

  const loadMoreEvents = async () => {
    if (loadingMore) return;
    
    setLoadingMore(true);
    try {
      // Fetch next batch of events
      const moreEvents = await apiService.getMyEvents(50, events.length);
      if (moreEvents.length > 0) {
        // Filter out any duplicates by ID to prevent React key errors
        setEvents(prev => {
          const existingIds = new Set(prev.map(e => e.id));
          const uniqueNewEvents = moreEvents.filter(e => !existingIds.has(e.id));
          return [...prev, ...uniqueNewEvents];
        });
        setTotalEvents(prev => prev + moreEvents.length);
      }
    } catch (error) {
      console.error('Failed to load more events:', error);
    } finally {
      setLoadingMore(false);
    }
  };

  const exportToCSV = async () => {
    if (events.length === 0) return;

    // If we have fewer events than total, fetch all events first
    let allEvents = events;
    if (events.length < totalEvents) {
      try {
        // Fetch all events for export (up to 10,000)
        allEvents = await apiService.getMyEvents(10000, 0);
      } catch (error) {
        console.error('Failed to fetch all events for export:', error);
        // Fall back to exporting what we have
      }
    }

    // CSV headers
    const headers = ['ID', 'Type', 'Severity', 'Threat Category', 'Description', 'Wallet', 'Method', 'Amount', 'Programs', 'Risk Score', 'Rule Matches', 'Signature', 'Created At'];
    
    // Convert events to CSV rows
    const rows = allEvents.map(event => [
      event.id,
      event.event_type,
      event.severity,
      event.threat_category || 'N/A',
      event.description || event.summary || 'N/A',
      event.wallet || 'N/A',
      event.method || 'N/A',
      event.amount || 'N/A',
      event.programs?.join('; ') || 'N/A',
      event.risk_score?.toString() || 'N/A',
      event.rule_matches?.toString() || 'N/A',
      event.signature || 'N/A',
      new Date(event.created_at).toLocaleString()
    ]);

    // Combine headers and rows
    const csvContent = [
      headers.join(','),
      ...rows.map(row => row.map(cell => {
        // Escape commas and quotes in cell content
        const cellStr = String(cell).replace(/"/g, '""');
        return cellStr.includes(',') || cellStr.includes('"') || cellStr.includes('\n') 
          ? `"${cellStr}"` 
          : cellStr;
      }).join(','))
    ].join('\n');

    // Create blob and download
    const blob = new Blob([csvContent], { type: 'text/csv;charset=utf-8;' });
    const link = document.createElement('a');
    const url = URL.createObjectURL(blob);
    link.setAttribute('href', url);
    link.setAttribute('download', `security-events-${new Date().toISOString().split('T')[0]}.csv`);
    link.style.visibility = 'hidden';
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);
  };

  if (loading) {
    return (
      <div className="max-w-7xl mx-auto px-6 py-16">
        <div className="flex items-center justify-center">
          <div className="flex flex-col items-center gap-4">
            <div className="w-10 h-10 border-3 border-slate-700 border-t-blue-500 rounded-full animate-spin"></div>
            <div className="text-slate-400 text-sm font-medium">Loading dashboard...</div>
          </div>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="max-w-7xl mx-auto px-6 py-16">
        <div className="bg-red-950/30 border border-red-800/40 rounded-lg p-6 shadow-lg">
          <div className="flex items-start gap-3">
            <svg className="w-5 h-5 text-red-500 mt-0.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
            </svg>
            <div className="flex-1">
              <p className="text-red-400 text-sm mb-4">{error}</p>
              <button 
                onClick={() => window.location.reload()}
                className="px-4 py-2 bg-red-600 hover:bg-red-700 text-white text-sm font-medium rounded-md transition-colors shadow-sm"
              >
                Retry
              </button>
            </div>
          </div>
        </div>
      </div>
    );
  }

  if (!stats) {
    return null;
  }

  return (
    <div className="max-w-7xl mx-auto px-3 sm:px-6 py-6 sm:py-10 md:py-12">
      {/* Header */}
      <div className="mb-6 sm:mb-8">
        <div className="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-3 sm:gap-4">
          <div className="flex-1 min-w-0">
            <h1 className="text-xl sm:text-2xl md:text-3xl font-semibold text-slate-50 mb-2">Security Dashboard</h1>
            {stats && (
              <div className="flex items-center gap-2 sm:gap-3 flex-wrap">
                <p className="text-slate-400 text-xs sm:text-sm font-mono truncate">
                  {stats.wallet_address.slice(0, 6)}...{stats.wallet_address.slice(-6)}
                </p>
                <div className="flex items-center gap-1.5 sm:gap-2 px-2 sm:px-2.5 py-1 bg-slate-800/50 rounded-md border border-slate-700/50 flex-shrink-0">
                  <div className={`w-1.5 h-1.5 rounded-full ${wsConnected ? 'bg-emerald-500 animate-pulse' : 'bg-red-500'}`} />
                  <span className="text-xs font-medium text-slate-300">{wsConnected ? 'Live' : 'Offline'}</span>
                </div>
              </div>
            )}
          </div>

          {/* Security Level Control */}
          {stats && (
            <div className="flex items-center gap-2 sm:gap-3 text-sm relative">
              <span className="text-slate-400 font-medium hidden sm:inline">Security Level:</span>
              <button
                onClick={() => setShowSecurityDropdown(!showSecurityDropdown)}
                className={`px-3 py-1.5 rounded-md font-medium flex items-center gap-2 transition-all border shadow-sm text-left ${
                  stats.blocking_threshold === 50 ? 'bg-red-950/40 text-red-400 border-red-800/50 hover:bg-red-950/60' :
                  stats.blocking_threshold === 85 ? 'bg-emerald-950/40 text-emerald-400 border-emerald-800/50 hover:bg-emerald-950/60' :
                  stats.blocking_threshold === 70 ? 'bg-blue-950/40 text-blue-400 border-blue-800/50 hover:bg-blue-950/60' :
                  'bg-violet-950/40 text-violet-400 border-violet-800/50 hover:bg-violet-950/60'
                }`}
              >
                {stats.blocking_threshold === 50 ? 'Strict' : 
                 stats.blocking_threshold === 85 ? 'Relaxed' : 
                 stats.blocking_threshold === 70 ? 'Balanced' :
                 `Custom (${stats.blocking_threshold})`}
                <svg className={`w-3.5 h-3.5 transition-transform ${showSecurityDropdown ? 'rotate-180' : ''}`} fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 9l-7 7-7-7" />
                </svg>
              </button>
              
              {showSecurityDropdown && (
                <>
                  <div 
                    className="fixed inset-0 z-[100]" 
                    onClick={(e) => {
                      e.stopPropagation();
                      setShowSecurityDropdown(false);
                    }} 
                  />
                  <div className="absolute top-full right-0 mt-2 bg-slate-900 border border-slate-700 rounded-lg shadow-2xl py-2 z-[200] min-w-[280px]">
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        apiService.updateBlockingThreshold(50).then(() => {
                          setStats(prev => prev ? { ...prev, blocking_threshold: 50 } : null);
                          setShowSecurityDropdown(false);
                          setCustomThreshold(null);
                        }).catch(err => {
                          console.error('Failed to set Strict:', err);
                          alert('Failed to update. Please try again.');
                        });
                      }}
                      className={`w-full px-4 py-2.5 text-left hover:bg-slate-800 transition-colors ${
                        stats.blocking_threshold === 50 ? 'bg-slate-800/50' : ''
                      }`}
                    >
                      <div className="flex items-center justify-between">
                        <div>
                          <div className="text-sm font-medium text-slate-100">Strict</div>
                          <div className="text-xs text-slate-400 mt-0.5">Maximum protection • Blocks at 50/100 risk</div>
                        </div>
                        {stats.blocking_threshold === 50 && (
                          <svg className="w-5 h-5 text-blue-500" fill="currentColor" viewBox="0 0 20 20">
                            <path fillRule="evenodd" d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z" clipRule="evenodd" />
                          </svg>
                        )}
                      </div>
                    </button>
                    
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        apiService.updateBlockingThreshold(70).then(() => {
                          setStats(prev => prev ? { ...prev, blocking_threshold: 70 } : null);
                          setShowSecurityDropdown(false);
                          setCustomThreshold(null);
                        }).catch(err => {
                          console.error('Failed to set Balanced:', err);
                          alert('Failed to update. Please try again.');
                        });
                      }}
                      className={`w-full px-4 py-2.5 text-left hover:bg-slate-800 transition-colors ${
                        stats.blocking_threshold === 70 ? 'bg-slate-800/50' : ''
                      }`}
                    >
                      <div className="flex items-center justify-between">
                        <div>
                          <div className="text-sm font-medium text-slate-100">Balanced</div>
                          <div className="text-xs text-slate-400 mt-0.5">Recommended • Blocks at 70/100 risk</div>
                        </div>
                        {stats.blocking_threshold === 70 && (
                          <svg className="w-5 h-5 text-blue-500" fill="currentColor" viewBox="0 0 20 20">
                            <path fillRule="evenodd" d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z" clipRule="evenodd" />
                          </svg>
                        )}
                      </div>
                    </button>
                    
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        apiService.updateBlockingThreshold(85).then(() => {
                          setStats(prev => prev ? { ...prev, blocking_threshold: 85 } : null);
                          setShowSecurityDropdown(false);
                          setCustomThreshold(null);
                        }).catch(err => {
                          console.error('Failed to set Relaxed:', err);
                          alert('Failed to update. Please try again.');
                        });
                      }}
                      className={`w-full px-4 py-2.5 text-left hover:bg-slate-800 transition-colors ${
                        stats.blocking_threshold === 85 ? 'bg-slate-800/50' : ''
                      }`}
                    >
                      <div className="flex items-center justify-between">
                        <div>
                          <div className="text-sm font-medium text-slate-100">Relaxed</div>
                          <div className="text-xs text-slate-400 mt-0.5">Minimal interference • Blocks at 85/100 risk</div>
                        </div>
                        {stats.blocking_threshold === 85 && (
                          <svg className="w-5 h-5 text-blue-500" fill="currentColor" viewBox="0 0 20 20">
                            <path fillRule="evenodd" d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z" clipRule="evenodd" />
                          </svg>
                        )}
                      </div>
                    </button>
                    
                    <div className="border-t border-slate-700 my-2" />
                    
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        setCustomThreshold(stats?.blocking_threshold || 50);
                        setShowSecurityDropdown(false);
                      }}
                      className="w-full px-4 py-2.5 text-left hover:bg-slate-800 transition-colors"
                    >
                      <div className="flex items-center justify-between">
                        <div>
                          <div className="text-sm font-medium text-slate-100">Custom</div>
                          <div className="text-xs text-slate-400 mt-0.5">Set your own risk threshold (0-100)</div>
                        </div>
                        <svg className="w-5 h-5 text-slate-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 6V4m0 2a2 2 0 100 4m0-4a2 2 0 110 4m-6 8a2 2 0 100-4m0 4a2 2 0 110-4m0 4v2m0-6V4m6 6v10m6-2a2 2 0 100-4m0 4a2 2 0 110-4m0 4v2m0-6V4" />
                        </svg>
                      </div>
                    </button>
                    
                    <div className="border-t border-slate-700 my-2" />
                    
                    <div className="px-4 py-2.5">
                      <div className="flex items-center justify-between">
                        <div>
                          <div className="text-sm font-medium text-slate-100">Browser Notifications</div>
                          <div className="text-xs text-slate-400 mt-0.5">Alert when threshold exceeded</div>
                        </div>
                        <button
                          onClick={async (e) => {
                            e.stopPropagation();
                            try {
                              const enablingNotifications = !stats?.notifications_enabled;
                              
                              if (enablingNotifications) {
                                console.log('🔔 Enabling notifications...');
                                console.log('🔔 Current permission:', Notification.permission);
                                
                                // Request notification permission if not granted
                                if (Notification.permission !== 'granted') {
                                  console.log('🔔 Requesting permission...');
                                  const permission = await Notification.requestPermission();
                                  console.log('🔔 Permission result:', permission);
                                  
                                  if (permission !== 'granted') {
                                    alert('Notification permission denied. Please enable notifications in your browser settings.');
                                    return;
                                  }
                                }
                                
                                // Subscribe to push notifications
                                console.log('🔔 Subscribing to push notifications...');
                                const pushSubscription = await subscribeToPushNotifications();
                                console.log('🔔 Push subscription:', pushSubscription ? 'success' : 'failed');
                                
                                if (pushSubscription) {
                                  console.log('🔔 Sending subscription to server...');
                                  const sent = await sendSubscriptionToServer(pushSubscription);
                                  console.log('🔔 Server registration:', sent ? 'success' : 'failed');
                                  if (!sent) {
                                    console.error('Failed to register push subscription with server');
                                    alert('Failed to register push notifications with server. Please try again.');
                                    return;
                                  }
                                } else {
                                  alert('Failed to create push subscription. Please check console for errors.');
                                  return;
                                }
                                
                                console.log('🔔 Push notifications fully enabled!');
                              }
                              
                              await apiService.toggleNotifications(enablingNotifications);
                              setStats(prev => prev ? { ...prev, notifications_enabled: enablingNotifications } : null);
                            } catch (err) {
                              console.error('❌ Failed to toggle notifications:', err);
                              alert(`Failed to update notifications: ${err instanceof Error ? err.message : String(err)}`);
                            }
                          }}
                          className={`relative inline-flex h-5 w-9 items-center rounded-full transition-colors ${
                            stats?.notifications_enabled ? 'bg-blue-600' : 'bg-slate-700'
                          }`}
                        >
                          <span className={`inline-block h-3 w-3 transform rounded-full bg-white transition-transform ${
                            stats?.notifications_enabled ? 'translate-x-5' : 'translate-x-1'
                          }`} />
                        </button>
                      </div>
                    </div>
                  </div>
                </>
              )}
            </div>
          )}
        </div>
      </div>

      {/* Network-Wide Security Insights - Ultra-Compact Banner */}
      {globalStats && (
        <div className="bg-gradient-to-r from-blue-950/30 via-slate-900/50 to-blue-950/30 border border-blue-900/30 rounded-lg mb-6 shadow-sm overflow-hidden">
          {/* Single-Line Banner View */}
          <div className="px-4 py-3 sm:px-6 sm:py-4">
            <div className="flex items-center justify-between gap-4">
              <div className="flex items-center gap-3 sm:gap-4 flex-1 min-w-0">
                <div className="flex items-center gap-2 flex-shrink-0">
                  <div className="p-1.5 bg-blue-600/20 rounded border border-blue-500/30">
                    <svg className="w-4 h-4 text-blue-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M3.055 11H5a2 2 0 012 2v1a2 2 0 002 2 2 2 0 012 2v2.945M8 3.935V5.5A2.5 2.5 0 0010.5 8h.5a2 2 0 012 2 2 2 0 104 0 2 2 0 012-2h1.064M15 20.488V18a2 2 0 012-2h3.064M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
                    </svg>
                  </div>
                  <div className="flex items-center gap-2">
                    <span className="text-sm font-semibold text-slate-200">Network Security</span>
                    <div className="w-1.5 h-1.5 rounded-full bg-blue-500 animate-pulse" />
                  </div>
                </div>
                
                {/* Inline Stats */}
                <div className="hidden sm:flex items-center gap-4 text-sm flex-1">
                  <div className="flex items-center gap-1.5">
                    <span className="text-slate-400">{formatNumber(globalStats.total_requests)}</span>
                    <span className="text-slate-500">analyzed</span>
                  </div>
                  <div className="w-px h-4 bg-slate-700" />
                  <div className="flex items-center gap-1.5">
                    <span className="text-red-400 font-semibold">{formatNumber(globalStats.total_blocked)}</span>
                    <span className="text-slate-500">blocked</span>
                    <span className="text-slate-600">({((globalStats.total_blocked / globalStats.total_requests) * 100).toFixed(1)}%)</span>
                  </div>
                  <div className="w-px h-4 bg-slate-700" />
                  <div className="flex items-center gap-1.5">
                    <span className="text-emerald-400 font-semibold">
                      {((globalStats.total_requests - globalStats.total_blocked - globalStats.total_warnings) / globalStats.total_requests * 100).toFixed(1)}%
                    </span>
                    <span className="text-slate-500">clean</span>
                  </div>
                </div>

                {/* Mobile Compact Stats */}
                <div className="flex sm:hidden items-center gap-2 text-xs">
                  <span className="text-slate-400">{formatNumber(globalStats.total_requests)}</span>
                  <span className="text-slate-600">•</span>
                  <span className="text-emerald-400 font-semibold">
                    {((globalStats.total_requests - globalStats.total_blocked - globalStats.total_warnings) / globalStats.total_requests * 100).toFixed(1)}% clean
                  </span>
                </div>
              </div>

              <button
                onClick={() => setShowNetworkDetails(!showNetworkDetails)}
                className="flex items-center gap-1.5 px-3 py-1.5 bg-slate-800/50 hover:bg-slate-800 border border-slate-700 rounded-md text-xs font-medium text-slate-300 hover:text-slate-100 transition-all flex-shrink-0"
              >
                <span className="hidden sm:inline">{showNetworkDetails ? 'Less' : 'Details'}</span>
                <svg 
                  className={`w-3.5 h-3.5 transition-transform ${showNetworkDetails ? 'rotate-180' : ''}`} 
                  fill="none" 
                  stroke="currentColor" 
                  viewBox="0 0 24 24"
                >
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 9l-7 7-7-7" />
                </svg>
              </button>
            </div>
          </div>

          {/* Expanded Details View */}
          {showNetworkDetails && (
            <div className="border-t border-slate-800 p-4 sm:p-6 space-y-6">
              <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4 sm:gap-6">
            {/* Total Network Requests */}
            <div className="bg-slate-900/50 border border-slate-800 rounded-lg p-5">
              <div className="flex items-center gap-3 mb-3">
                <div className="p-2 bg-blue-600/10 rounded-md">
                  <svg className="w-5 h-5 text-blue-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13 10V3L4 14h7v7l9-11h-7z" />
                  </svg>
                </div>
                <div className="text-xs text-slate-500 font-semibold uppercase tracking-wide">Network Activity</div>
              </div>
              <div className="text-3xl font-semibold text-slate-50 mb-1">{formatNumber(globalStats.total_requests)}</div>
              <div className="text-xs text-slate-400">Total transactions analyzed</div>
              <div className="mt-3 pt-3 border-t border-slate-800">
                <div className="flex items-center gap-2 text-xs">
                  <div className="w-1.5 h-1.5 rounded-full bg-blue-500 animate-pulse" />
                  <span className="text-slate-400">{globalStats.requests_per_second.toFixed(1)} req/s</span>
                </div>
              </div>
            </div>

            {/* Threats Blocked */}
            <div className="bg-slate-900/50 border border-slate-800 rounded-lg p-5">
              <div className="flex items-center gap-3 mb-3">
                <div className="p-2 bg-red-600/10 rounded-md">
                  <svg className="w-5 h-5 text-red-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M18.364 18.364A9 9 0 005.636 5.636m12.728 12.728A9 9 0 015.636 5.636m12.728 12.728L5.636 5.636" />
                  </svg>
                </div>
                <div className="text-xs text-slate-500 font-semibold uppercase tracking-wide">Threats Blocked</div>
              </div>
              <div className="text-3xl font-semibold text-red-400 mb-1">{formatNumber(globalStats.total_blocked)}</div>
              <div className="text-xs text-slate-400">Malicious transactions stopped</div>
              <div className="mt-3 pt-3 border-t border-slate-800">
                <div className="text-xs text-slate-400">
                  {((globalStats.total_blocked / globalStats.total_requests) * 100).toFixed(2)}% block rate
                </div>
              </div>
            </div>

            {/* Warnings Issued */}
            <div className="bg-slate-900/50 border border-slate-800 rounded-lg p-5">
              <div className="flex items-center gap-3 mb-3">
                <div className="p-2 bg-amber-600/10 rounded-md">
                  <svg className="w-5 h-5 text-amber-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
                  </svg>
                </div>
                <div className="text-xs text-slate-500 font-semibold uppercase tracking-wide">Warnings Issued</div>
              </div>
              <div className="text-3xl font-semibold text-amber-400 mb-1">{formatNumber(globalStats.total_warnings)}</div>
              <div className="text-xs text-slate-400">Suspicious activity flagged</div>
              <div className="mt-3 pt-3 border-t border-slate-800">
                <div className="text-xs text-slate-400">
                  {((globalStats.total_warnings / globalStats.total_requests) * 100).toFixed(2)}% warning rate
                </div>
              </div>
            </div>

            {/* Clean Transactions */}
            <div className="bg-slate-900/50 border border-slate-800 rounded-lg p-5">
              <div className="flex items-center gap-3 mb-3">
                <div className="p-2 bg-emerald-600/10 rounded-md">
                  <svg className="w-5 h-5 text-emerald-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
                  </svg>
                </div>
                <div className="text-xs text-slate-500 font-semibold uppercase tracking-wide">Clean Transactions</div>
              </div>
              <div className="text-3xl font-semibold text-emerald-400 mb-1">
                {formatNumber(globalStats.total_requests - globalStats.total_blocked - globalStats.total_warnings)}
              </div>
              <div className="text-xs text-slate-400">Passed all security checks</div>
              <div className="mt-3 pt-3 border-t border-slate-800">
                <div className="text-xs text-slate-400">
                  {(((globalStats.total_requests - globalStats.total_blocked - globalStats.total_warnings) / globalStats.total_requests) * 100).toFixed(2)}% clean rate
                </div>
              </div>
            </div>
          </div>

          {/* Network Security Bar */}
          <div className="mt-6 pt-6 border-t border-slate-800">
            <div className="flex items-center justify-between mb-3">
              <span className="text-sm font-medium text-slate-300">Network Security Distribution</span>
              <span className="text-xs text-slate-500">Last 24 hours</span>
            </div>
            <div className="relative h-3 bg-slate-900 rounded-full overflow-hidden border border-slate-800">
              <div 
                className="absolute left-0 top-0 bottom-0 bg-emerald-500 transition-all"
                style={{ 
                  width: `${((globalStats.total_requests - globalStats.total_blocked - globalStats.total_warnings) / globalStats.total_requests) * 100}%` 
                }}
              />
              <div 
                className="absolute top-0 bottom-0 bg-amber-500 transition-all"
                style={{ 
                  left: `${((globalStats.total_requests - globalStats.total_blocked - globalStats.total_warnings) / globalStats.total_requests) * 100}%`,
                  width: `${(globalStats.total_warnings / globalStats.total_requests) * 100}%` 
                }}
              />
              <div 
                className="absolute right-0 top-0 bottom-0 bg-red-500 transition-all"
                style={{ 
                  width: `${(globalStats.total_blocked / globalStats.total_requests) * 100}%` 
                }}
              />
            </div>
            <div className="flex items-center justify-between mt-3 text-xs">
              <div className="flex items-center gap-2">
                <div className="w-2 h-2 rounded-full bg-emerald-500" />
                <span className="text-slate-400">Clean</span>
              </div>
              <div className="flex items-center gap-2">
                <div className="w-2 h-2 rounded-full bg-amber-500" />
                <span className="text-slate-400">Warned</span>
              </div>
              <div className="flex items-center gap-2">
                <div className="w-2 h-2 rounded-full bg-red-500" />
                <span className="text-slate-400">Blocked</span>
              </div>
            </div>
          </div>

              <div className="p-4 bg-blue-950/30 border border-blue-900/30 rounded-lg">
                <div className="flex items-start gap-3">
                  <svg className="w-5 h-5 text-blue-400 mt-0.5 flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
                  </svg>
                  <div>
                    <h4 className="text-sm font-semibold text-slate-200 mb-1">Collective Security Intelligence</h4>
                    <p className="text-sm text-slate-400 leading-relaxed">
                      SecureCheck protects the entire network by sharing threat intelligence across all users. When a new threat is detected, 
                      all users benefit from enhanced protection within seconds.
                    </p>
                  </div>
                </div>
              </div>
            </div>
          )}
        </div>
      )}

      {/* Custom Threshold Input */}
      {customThreshold !== null && (
        <div className="bg-slate-900 border border-slate-700 rounded-lg p-5 mb-6 shadow-lg">
          <div className="flex items-center justify-between mb-4">
            <div>
              <h3 className="text-sm font-semibold text-slate-100">Custom Risk Threshold</h3>
              <p className="text-xs text-slate-400 mt-0.5">Adjust the risk score that triggers transaction blocking</p>
            </div>
            <button
              onClick={() => setCustomThreshold(null)}
              className="text-slate-400 hover:text-slate-300 text-sm font-medium"
            >
              Cancel
            </button>
          </div>
          <div className="flex items-center gap-4">
            <input
              type="range"
              min="30"
              max="95"
              step="5"
              value={customThreshold}
              onChange={(e) => setCustomThreshold(parseInt(e.target.value))}
              className="flex-1 h-2 bg-slate-800 rounded-lg appearance-none cursor-pointer accent-blue-500"
            />
            <span className="text-xl font-semibold text-slate-100 min-w-[70px] text-center">{customThreshold}<span className="text-slate-400 text-sm">/100</span></span>
            <button
              onClick={async () => {
                setUpdatingThreshold(true);
                try {
                  console.log('Applying custom threshold:', customThreshold);
                  await apiService.updateBlockingThreshold(customThreshold);
                  console.log('API call successful, updating state');
                  setStats(prev => {
                    const updated = prev ? { ...prev, blocking_threshold: customThreshold } : null;
                    console.log('Updated stats:', updated);
                    return updated;
                  });
                  setCustomThreshold(null);
                  setShowSecurityDropdown(false);
                  console.log('Custom threshold applied:', customThreshold);
                } catch (err) {
                  console.error('Failed to update threshold:', err);
                  alert('Failed to update threshold. Please try again.');
                } finally {
                  setUpdatingThreshold(false);
                }
              }}
              disabled={updatingThreshold}
              className="px-4 py-2 bg-blue-600 hover:bg-blue-700 disabled:bg-blue-600/50 disabled:cursor-not-allowed rounded-md text-sm font-medium transition-colors whitespace-nowrap shadow-sm"
            >
              {updatingThreshold ? 'Applying...' : 'Apply'}
            </button>
          </div>
          <div className="flex justify-between text-xs text-slate-500 mt-3">
            <span>30 (Very Strict)</span>
            <span>70 (Balanced)</span>
            <span>95 (Very Relaxed)</span>
          </div>
        </div>
      )}

      {/* Stats Grid */}
      <div className="grid grid-cols-1 sm:grid-cols-2 gap-4 sm:gap-5 mb-6 sm:mb-8">
        <div className="bg-slate-900/50 border border-slate-800 rounded-lg p-4 sm:p-6 shadow-sm">
          <div className="flex items-center justify-between mb-3 sm:mb-4">
            <h3 className="text-xs sm:text-sm font-semibold text-slate-400 uppercase tracking-wide">Transactions Analyzed</h3>
            <svg className="w-4 h-4 sm:w-5 sm:h-5 text-slate-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z" />
            </svg>
          </div>
          <div className="text-3xl sm:text-4xl font-semibold text-slate-50 mb-3 sm:mb-4">{formatNumber(stats.total_requests)}</div>
          <div className="grid grid-cols-3 gap-2 sm:gap-3 pt-3 sm:pt-4 border-t border-slate-800">
            <div>
              <div className="text-[10px] sm:text-xs text-slate-500 mb-1">Allowed</div>
              <div className="text-xs sm:text-sm font-semibold text-emerald-400">{formatNumber(Math.max(0, stats.total_requests - stats.total_blocked - stats.total_warnings))}</div>
            </div>
            <div>
              <div className="text-[10px] sm:text-xs text-slate-500 mb-1">Warned</div>
              <div className="text-xs sm:text-sm font-semibold text-amber-400">{formatNumber(stats.total_warnings)}</div>
            </div>
            <div>
              <div className="text-[10px] sm:text-xs text-slate-500 mb-1">Blocked</div>
              <div className="text-xs sm:text-sm font-semibold text-red-400">{formatNumber(stats.total_blocked)}</div>
            </div>
          </div>
        </div>

        <div className="bg-slate-900/50 border border-slate-800 rounded-lg p-4 sm:p-6 shadow-sm">
          <div className="flex items-center justify-between mb-3 sm:mb-4">
            <h3 className="text-xs sm:text-sm font-semibold text-slate-400 uppercase tracking-wide">Credits Available</h3>
            <svg className="w-4 h-4 sm:w-5 sm:h-5 text-slate-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 8c-1.657 0-3 .895-3 2s1.343 2 3 2 3 .895 3 2-1.343 2-3 2m0-8c1.11 0 2.08.402 2.599 1M12 8V7m0 1v8m0 0v1m0-1c-1.11 0-2.08-.402-2.599-1M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
            </svg>
          </div>
          <div className="text-3xl sm:text-4xl font-semibold text-slate-50 mb-3 sm:mb-4">{formatNumber(stats.credits_balance - stats.credits_used_this_month)}</div>
          <div className="pt-3 sm:pt-4 border-t border-slate-800">
            <div className="flex items-center justify-between text-xs sm:text-sm">
              <span className="text-slate-400">Used this month</span>
              <span className="font-semibold text-slate-300">{formatNumber(stats.credits_used_this_month)}</span>
            </div>
          </div>
        </div>
      </div>

      {/* Active Security Rules - Enhanced with Details */}
      {rules && showRules && (
        <div className="bg-slate-900/50 border border-slate-800 rounded-lg p-6 mb-8 shadow-sm">
          <div className="flex items-center justify-between mb-5">
            <div>
              <h3 className="text-lg font-semibold text-slate-50 mb-1">Active Security Rules</h3>
              <div className="text-sm text-slate-400">{rules.rules_source} • {rules.active_rules} active rules</div>
            </div>
            <button
              onClick={() => setShowRules(false)}
              className="px-4 py-2 bg-slate-800 hover:bg-slate-700 border border-slate-700 rounded-md text-sm text-slate-300 hover:text-slate-100 transition-colors font-medium"
            >
              Hide
            </button>
          </div>
          <div className="space-y-3">
            {rules.rules.map((rule) => (
              <div 
                key={rule.id} 
                className="bg-slate-950/30 border border-slate-800 rounded-lg p-5 hover:bg-slate-950/50 hover:border-slate-700 transition-colors"
              >
                <div className="flex items-start justify-between gap-4 mb-3">
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-2 mb-2 flex-wrap">
                      <span className="text-sm font-semibold text-slate-100">{rule.name}</span>
                      {rule.action === 'block' && (
                        <span className="px-2 py-0.5 rounded-md text-[10px] font-bold bg-red-950/50 text-red-400 border border-red-800/50 uppercase tracking-wide">BLOCK</span>
                      )}
                      {rule.action === 'alert' && (
                        <span className="px-2 py-0.5 rounded-md text-[10px] font-bold bg-amber-950/50 text-amber-400 border border-amber-800/50 uppercase tracking-wide">ALERT</span>
                      )}
                      <span className={`px-2 py-0.5 rounded-md text-[10px] font-semibold border uppercase tracking-wide ${
                        rule.severity === 'critical' ? 'bg-red-950/50 text-red-400 border-red-800/50' :
                        rule.severity === 'high' ? 'bg-orange-950/50 text-orange-400 border-orange-800/50' :
                        rule.severity === 'medium' ? 'bg-amber-950/50 text-amber-400 border-amber-800/50' :
                        'bg-blue-950/50 text-blue-400 border-blue-800/50'
                      }`}>
                        {rule.severity}
                      </span>
                    </div>
                    <div className="text-xs text-slate-500 font-mono mb-2">{rule.id}</div>
                    {rule.description && (
                      <div className="text-sm text-slate-400 leading-relaxed">{rule.description}</div>
                    )}
                  </div>
                  <div className="text-right shrink-0">
                    {rule.hit_count > 0 && (
                      <div className="text-xs">
                        <div className="text-blue-400 font-semibold text-lg">{formatNumber(rule.hit_count)}</div>
                        <div className="text-slate-500 text-xs uppercase tracking-wide">detections</div>
                      </div>
                    )}
                  </div>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* API Key Regeneration Flow - Only shown when active */}
      {(newApiKey || regenerateApiKey) && (
        <div className="bg-slate-900/50 border border-slate-800 rounded-lg p-5 mb-8 shadow-sm">
          {newApiKey ? (
            <>
              <div className="flex items-center gap-3 mb-4">
                <svg className="w-5 h-5 text-amber-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
                </svg>
                <span className="text-amber-400 text-sm font-semibold">New API Key Generated</span>
              </div>
              <p className="text-slate-400 text-sm mb-4">Copy your API key now. For security reasons, it won't be displayed again.</p>
              <div className="flex items-center gap-3">
                <div className="bg-slate-950/50 rounded-md p-3 flex items-center justify-between gap-3 flex-1 border border-slate-800">
                  <code className="text-sm text-emerald-400 font-mono break-all flex-1">{newApiKey}</code>
                  <button
                    onClick={copyApiKey}
                    className="text-slate-400 hover:text-slate-200 transition-colors flex-shrink-0"
                    title="Copy API Key"
                  >
                    <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z" />
                    </svg>
                  </button>
                </div>
                <button
                  onClick={() => setNewApiKey(null)}
                  className="px-4 py-2 bg-slate-800 hover:bg-slate-700 rounded-md text-sm font-medium transition-colors whitespace-nowrap border border-slate-700"
                >
                  Done
                </button>
              </div>
            </>
          ) : (
            <>
              <div className="flex items-center gap-3 mb-4">
                <svg className="w-5 h-5 text-red-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
                </svg>
                <span className="text-red-400 text-sm font-semibold">Confirm API Key Regeneration</span>
              </div>
              <p className="text-slate-400 text-sm mb-4">This action will immediately invalidate your current API key. Any applications using the old key will lose access.</p>
              <div className="flex gap-3">
                <button
                  onClick={handleRegenerateApiKey}
                  className="px-4 py-2 bg-red-600 hover:bg-red-700 rounded-md text-sm font-medium transition-colors"
                >
                  Confirm Regeneration
                </button>
                <button
                  onClick={() => setRegenerateApiKey(false)}
                  className="px-4 py-2 bg-slate-800 hover:bg-slate-700 rounded-md text-sm font-medium transition-colors border border-slate-700"
                >
                  Cancel
                </button>
              </div>
            </>
          )}
        </div>
      )}

      {/* RPC & Key Actions */}
      <div className="flex flex-col gap-3 mb-8">
        <div className="bg-slate-900/50 border border-slate-800 rounded-lg p-4 shadow-sm">
          <div className="flex items-center justify-between gap-3 mb-3">
            <div className="flex items-center gap-2">
              <svg className="w-4 h-4 text-slate-500 flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13 10V3L4 14h7v7l9-11h-7z" />
              </svg>
              <span className="text-xs text-slate-500 font-semibold uppercase tracking-wide">RPC Endpoint</span>
            </div>
            <button
              onClick={() => navigator.clipboard.writeText(rpcUrl)}
              className="text-slate-400 hover:text-slate-200 transition-colors text-sm flex items-center gap-2 font-medium flex-shrink-0"
            >
              <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z" />
              </svg>
              <span className="hidden sm:inline">Copy</span>
            </button>
          </div>
          <code className="text-sm text-emerald-400 font-mono break-all">{rpcUrl}</code>
        </div>
        <button
          onClick={() => setRegenerateApiKey(true)}
          className="w-full bg-slate-900/50 border border-slate-800 hover:bg-slate-800 rounded-lg px-5 py-3 text-sm text-slate-300 hover:text-slate-100 transition-colors flex items-center justify-center gap-2 font-medium shadow-sm"
        >
          <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
          </svg>
          Regenerate API Key
        </button>
      </div>

      {/* Security Events */}
      <div className="bg-slate-900/50 border border-slate-800 rounded-lg p-6 sm:p-8 mb-8 shadow-sm">
        <div className="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-4 mb-6">
          <div className="flex items-center gap-4">
            <h2 className="text-xl sm:text-2xl font-semibold text-slate-50">Security Events</h2>
            {rules && (
              <button
                onClick={() => setShowRules(!showRules)}
                className="text-sm text-slate-400 hover:text-slate-200 transition-colors flex items-center gap-1.5 font-medium"
              >
                {rules.active_rules} rules
                <svg className={`w-3.5 h-3.5 transition-transform ${showRules ? 'rotate-180' : ''}`} fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 9l-7 7-7-7" />
                </svg>
              </button>
            )}
          </div>
          {events.length > 0 && (
            <button
              onClick={exportToCSV}
              className="inline-flex items-center justify-center gap-2 px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white font-medium rounded-md transition-colors text-sm shadow-sm"
              title="Export all events to CSV"
            >
              <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 10v6m0 0l-3-3m3 3l3-3m2 8H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
              </svg>
              <span className="hidden xs:inline">Export CSV</span>
              <span className="xs:hidden">Export</span>
            </button>
          )}
        </div>
        {events.length > 0 ? (
          <>
            <div className="space-y-4">
              {(() => {
                const startIndex = (currentPage - 1) * eventsPerPage;
                const endIndex = startIndex + eventsPerPage;
                const paginatedEvents = events.slice(startIndex, endIndex);
                return paginatedEvents.map((event) => (
                  <div
                    key={event.id}
                    className={`relative group rounded-lg border transition-all duration-200 overflow-hidden ${
                      event.event_type === 'blocked'
                        ? 'bg-slate-900/40 border-red-900/40 hover:border-red-800/60 hover:shadow-lg hover:shadow-red-900/10'
                        : event.event_type === 'warned'
                        ? 'bg-slate-900/40 border-amber-900/40 hover:border-amber-800/60 hover:shadow-lg hover:shadow-amber-900/10'
                        : 'bg-slate-900/40 border-emerald-900/40 hover:border-emerald-800/60 hover:shadow-lg hover:shadow-emerald-900/10'
                    }`}
                  >
                    {/* Status indicator bar */}
                    <div className={`absolute left-0 top-0 bottom-0 w-1 ${
                      event.event_type === 'blocked' ? 'bg-red-500' :
                      event.event_type === 'warned' ? 'bg-amber-500' : 'bg-emerald-500'
                    }`} />
                    
                    <div className="p-4 sm:p-5">
                      <div className="flex flex-col gap-4">
                        <div className="flex-1 min-w-0">
                          {/* Header with badges */}
                          <div className="flex flex-wrap items-center gap-2 mb-3">
                            <div className={`inline-flex items-center gap-1.5 px-3 py-1.5 rounded-md font-semibold text-xs ${
                              event.event_type === 'blocked'
                                ? 'bg-red-950/50 text-red-400 border border-red-800/50'
                                : event.event_type === 'warned'
                                ? 'bg-amber-950/50 text-amber-400 border border-amber-800/50'
                                : 'bg-emerald-950/50 text-emerald-400 border border-emerald-800/50'
                            }`}>
                              {event.event_type === 'blocked' && (
                                <svg className="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M18.364 18.364A9 9 0 005.636 5.636m12.728 12.728A9 9 0 015.636 5.636m12.728 12.728L5.636 5.636" />
                                </svg>
                              )}
                              {event.event_type === 'warned' && (
                                <svg className="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
                                </svg>
                              )}
                              {event.event_type === 'allowed' && (
                                <svg className="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
                                </svg>
                              )}
                              {event.event_type.toUpperCase()}
                            </div>
                            <span className="px-2.5 py-1 text-xs rounded-md bg-slate-800/50 text-slate-300 font-medium border border-slate-700/50">
                              {event.severity}
                            </span>
                            {event.threat_category && (
                              <span className="px-2.5 py-1 text-xs rounded-md bg-violet-950/50 text-violet-300 font-medium border border-violet-800/50">
                                {event.threat_category}
                              </span>
                            )}
                          </div>
                          
                          {/* Description */}
                          {event.summary && (
                            <p className="text-sm text-slate-100 mb-3 font-medium leading-relaxed">{event.summary}</p>
                          )}
                          {!event.summary && event.description && (
                            <p className="text-sm text-slate-200 mb-3 leading-relaxed">{event.description}</p>
                          )}
                          {event.event_type === 'allowed' && !event.summary && !event.description && (
                            <p className="text-sm text-slate-300 mb-3 flex items-center gap-2">
                              <svg className="w-4 h-4 text-emerald-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
                              </svg>
                              Transaction sent successfully
                            </p>
                          )}
                          
                          {/* Transaction metadata grid */}
                          <div className="grid grid-cols-1 sm:grid-cols-2 gap-x-6 gap-y-2.5 text-xs">
                            {event.method && (
                              <div className="flex items-center gap-2">
                                <span className="text-slate-500 font-medium">Method:</span>
                                <span className="font-mono bg-slate-800/50 px-2 py-1 rounded text-slate-200 border border-slate-700/50">{event.method}</span>
                              </div>
                            )}
                            {event.wallet && (
                              <div className="flex items-center gap-2">
                                <span className="text-slate-500 font-medium">Wallet:</span>
                                <span className="font-mono text-slate-300 text-[11px]">{event.wallet.slice(0, 8)}...{event.wallet.slice(-8)}</span>
                              </div>
                            )}
                            {event.amount && (
                              <div className="flex items-center gap-2">
                                <span className="text-slate-500 font-medium">Amount:</span>
                                <span className="text-slate-200">{event.amount}</span>
                              </div>
                            )}
                            {event.programs && event.programs.length > 0 && (
                              <div className="flex items-center gap-2">
                                <span className="text-slate-500 font-medium">Programs:</span>
                                <span className="text-slate-200">{event.programs.join(', ')}</span>
                              </div>
                            )}
                            {event.risk_score !== undefined && (
                              <div className="flex items-center gap-2">
                                <span className="text-slate-500 font-medium">Risk Weight:</span>
                                <div className="flex items-center gap-2">
                                  <div className="relative w-24 h-2 bg-slate-800/50 rounded-full overflow-hidden border border-slate-700/50">
                                    <div 
                                      className={`absolute left-0 top-0 bottom-0 rounded-full transition-all ${
                                        event.risk_score > 75 ? 'bg-red-500' :
                                        event.risk_score > 50 ? 'bg-amber-500' :
                                        event.risk_score > 25 ? 'bg-yellow-500' : 'bg-emerald-500'
                                      }`}
                                      style={{ width: `${event.risk_score}%` }}
                                    />
                                  </div>
                                  <span className={`font-semibold ${
                                    event.risk_score > 50 ? 'text-red-400' : 
                                    event.risk_score > 25 ? 'text-amber-400' : 'text-emerald-400'
                                  }`}>
                                    {event.risk_score}/100
                                  </span>
                                </div>
                              </div>
                            )}
                            {event.rule_matches !== undefined && event.rule_matches > 0 && (
                              <div className="flex items-center gap-2">
                                <span className="text-slate-500 font-medium">Rule Matches:</span>
                                <span className="px-2 py-0.5 rounded bg-amber-950/50 text-amber-300 font-semibold border border-amber-800/50">{event.rule_matches}</span>
                              </div>
                            )}
                          </div>
                        </div>
                        
                        {/* Timestamp and links - Mobile optimized */}
                        <div className="flex flex-col gap-3 mt-4 pt-4 border-t border-slate-800/50">
                          <div className="flex items-center justify-between">
                            <span className="text-xs text-slate-500 font-medium">
                              {new Date(event.created_at).toLocaleString('en-US', {
                                month: 'short',
                                day: 'numeric',
                                hour: '2-digit',
                                minute: '2-digit'
                              })}
                            </span>
                          </div>
                          {event.signature ? (
                            <div className="flex flex-col gap-2">
                              <div className="flex items-center gap-2 bg-slate-800/50 rounded-md px-3 py-2 border border-slate-700/50 overflow-hidden">
                                <span className="text-[10px] text-slate-500 font-medium uppercase flex-shrink-0">TX:</span>
                                <code className="text-[10px] font-mono text-blue-400 truncate flex-1 min-w-0">
                                  {event.signature}
                                </code>
                                <button
                                  onClick={() => navigator.clipboard.writeText(event.signature || '')}
                                  className="text-slate-400 hover:text-slate-200 transition-colors shrink-0"
                                  title="Copy signature"
                                >
                                  <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z" />
                                  </svg>
                                </button>
                              </div>
                              <div className="flex flex-wrap gap-2">
                                <a 
                                  href={`https://solscan.io/tx/${event.signature}${solanaNetwork === 'devnet' ? '?cluster=devnet' : ''}`}
                                  target="_blank"
                                  rel="noopener noreferrer"
                                  className="inline-flex items-center justify-center gap-1.5 px-3 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-md text-xs font-medium transition-all flex-1 sm:flex-initial"
                                >
                                  <svg className="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14" />
                                  </svg>
                                  Solscan
                                </a>
                                <a 
                                  href={`https://explorer.solana.com/tx/${event.signature}${solanaNetwork === 'devnet' ? '?cluster=devnet' : ''}`}
                                  target="_blank"
                                  rel="noopener noreferrer"
                                  className="inline-flex items-center justify-center gap-1.5 px-3 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-md text-xs font-medium transition-all flex-1 sm:flex-initial"
                                >
                                  <svg className="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14" />
                                  </svg>
                                  Explorer
                                </a>
                              </div>
                            </div>
                          ) : (
                            <div className="text-slate-400 italic text-xs px-3 py-2 bg-slate-800/50 rounded border border-slate-700/50 text-center">
                              {event.event_type === 'blocked' ? 'Transaction Blocked' : 'No signature'}
                            </div>
                          )}
                        </div>
                      </div>

                      
                      {/* Matched Rule IDs - Compact display */}
                      {event.matched_rule_ids && event.matched_rule_ids.length > 0 && (
                        <div className="mt-4 pt-4 border-t border-slate-800/50">
                          <div className="flex items-start gap-3">
                            <div className="flex items-center gap-2 shrink-0">
                              <svg className="w-4 h-4 text-amber-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
                              </svg>
                              <span className="text-sm text-slate-300 font-medium">Triggered Rules:</span>
                            </div>
                            <div className="flex flex-wrap gap-2">
                              {event.matched_rule_ids.map((ruleId, idx) => (
                                <button
                                  key={idx}
                                  onClick={() => {
                                    setShowRules(true);
                                  }}
                                  className="px-2.5 py-1 bg-amber-950/50 hover:bg-amber-950/70 border border-amber-800/50 hover:border-amber-700/60 text-amber-300 rounded-md text-xs font-mono font-semibold transition-all cursor-pointer"
                                  title="Click to view rule details"
                                >
                                  {ruleId}
                                </button>
                              ))}
                            </div>
                          </div>
                        </div>
                      )}
                    </div>
                  </div>
                ));
              })()}
            </div>
            {events.length > 0 && (
              <div className="mt-8 space-y-4">
                {events.length > eventsPerPage && (
                  <div className="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4">
                    <div className="text-sm text-slate-400">
                      Showing {((currentPage - 1) * eventsPerPage) + 1} to {Math.min(currentPage * eventsPerPage, events.length)} of {events.length} events
                    </div>
                    <div className="flex items-center gap-2 overflow-x-auto w-full sm:w-auto">
                      <button
                        onClick={() => setCurrentPage(p => Math.max(1, p - 1))}
                        disabled={currentPage === 1}
                        className="px-3 sm:px-4 py-2 bg-slate-800 hover:bg-slate-700 disabled:opacity-50 disabled:cursor-not-allowed disabled:bg-slate-800/50 rounded-md transition-colors text-sm font-medium border border-slate-700 whitespace-nowrap"
                      >
                        <span className="hidden sm:inline">Previous</span>
                        <span className="sm:hidden">Prev</span>
                      </button>
                      <div className="flex items-center gap-1 overflow-x-auto">
                        {(() => {
                          const totalPages = Math.ceil(events.length / eventsPerPage);
                          const pages = [];
                          
                          // Show first page
                          if (currentPage > 2) {
                            pages.push(
                              <button
                                key={1}
                                onClick={() => setCurrentPage(1)}
                                className="px-3 py-2 bg-slate-800 hover:bg-slate-700 rounded-md transition-colors text-sm font-medium border border-slate-700"
                              >
                                1
                              </button>
                            );
                            if (currentPage > 3) {
                              pages.push(<span key="ellipsis1" className="px-2 text-slate-500">...</span>);
                            }
                          }
                          
                          // Show current page and neighbors
                          for (let i = Math.max(1, currentPage - 1); i <= Math.min(totalPages, currentPage + 1); i++) {
                            pages.push(
                              <button
                                key={i}
                                onClick={() => setCurrentPage(i)}
                                className={`px-3 py-2 rounded-md transition-colors text-sm font-medium border ${
                                  i === currentPage
                                    ? 'bg-blue-600 text-white border-blue-600'
                                    : 'bg-slate-800 hover:bg-slate-700 border-slate-700'
                                }`}
                              >
                                {i}
                              </button>
                            );
                          }
                          
                          // Show last page
                          if (currentPage < totalPages - 1) {
                            if (currentPage < totalPages - 2) {
                              pages.push(<span key="ellipsis2" className="px-2 text-slate-500">...</span>);
                            }
                            pages.push(
                              <button
                                key={totalPages}
                                onClick={() => setCurrentPage(totalPages)}
                                className="px-3 py-2 bg-slate-800 hover:bg-slate-700 rounded-md transition-colors text-sm font-medium border border-slate-700"
                              >
                                {totalPages}
                              </button>
                            );
                          }
                          
                          return pages;
                        })()}
                      </div>
                      <button
                        onClick={() => setCurrentPage(p => Math.min(Math.ceil(events.length / eventsPerPage), p + 1))}
                        disabled={currentPage >= Math.ceil(events.length / eventsPerPage)}
                        className="px-3 sm:px-4 py-2 bg-slate-800 hover:bg-slate-700 disabled:opacity-50 disabled:cursor-not-allowed disabled:bg-slate-800/50 rounded-md transition-colors text-sm font-medium border border-slate-700 whitespace-nowrap"
                      >
                        Next
                      </button>
                    </div>
                  </div>
                )}
                
                {/* Load More Button */}
                <div className="flex justify-center pt-4">
                  <button
                    onClick={loadMoreEvents}
                    disabled={loadingMore}
                    className="inline-flex items-center gap-2 px-6 py-2.5 bg-slate-800 hover:bg-slate-700 disabled:opacity-50 disabled:cursor-not-allowed rounded-md transition-colors text-sm font-medium border border-slate-700"
                  >
                    {loadingMore ? (
                      <>
                        <svg className="animate-spin h-4 w-4" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
                          <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4"></circle>
                          <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                        </svg>
                        Loading...
                      </>
                    ) : (
                      <>
                        <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 9l-7 7-7-7" />
                        </svg>
                        Load More Events
                      </>
                    )}
                  </button>
                </div>
              </div>
            )}
          </>
        ) : (
          <div className="flex flex-col items-center justify-center py-16 text-center">
            <div className="bg-slate-800/50 rounded-full p-8 mb-6 border border-slate-700/50">
              <svg className="w-14 h-14 text-slate-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944a11.955 11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016z" />
              </svg>
            </div>
            <h3 className="text-xl font-semibold text-slate-200 mb-3">No Security Events</h3>
            <p className="text-slate-400 text-sm max-w-md mb-6">
              Your transactions will be analyzed in real-time. Any threats detected or warnings will appear here automatically.
            </p>
            <div className="text-sm text-slate-500">
              Start sending transactions through your RPC endpoint to see security analysis
            </div>
          </div>
        )}
      </div>

      {/* Getting Started - RPC Usage */}
      <div className="mt-8 bg-slate-900/50 border border-slate-800 rounded-lg p-4 sm:p-6 lg:p-8 shadow-sm">
        <h2 className="text-xl sm:text-2xl font-semibold text-slate-50 mb-2 sm:mb-3">RPC Integration</h2>
        <p className="text-slate-400 text-sm mb-6 sm:mb-8">
          Configure your Solana application to use SecureCheck's protected RPC endpoint with API key authentication.
        </p>

        {/* HTTP Header Authentication */}
        <div className="mb-8">
          <div className="flex items-center gap-3 mb-5">
            <h3 className="text-lg font-semibold text-slate-200">Authorization Header</h3>
          </div>
          
          <div className="bg-slate-950/50 rounded-lg p-4 sm:p-5 mb-4 relative group border border-slate-800">
            <div className="flex items-center justify-between mb-3">
              <div className="text-xs text-slate-500 font-semibold uppercase tracking-wide">JavaScript / TypeScript</div>
              <button
                onClick={() => {
                  const code = `import { Connection } from '@solana/web3.js';

const connection = new Connection(
  '${rpcUrl}',
  {
    httpHeaders: {
      'Authorization': 'Bearer ${currentApiKey || 'your_api_key_here'}'
    }
  }
);`;
                  navigator.clipboard.writeText(code);
                }}
                className="px-3 py-1.5 bg-slate-800 hover:bg-slate-700 text-slate-300 hover:text-slate-100 text-xs rounded-md transition-all border border-slate-700 font-medium sm:opacity-0 sm:group-hover:opacity-100"
              >
                Copy
              </button>
            </div>
            <pre className="text-xs text-slate-300 overflow-x-auto font-mono -mx-4 sm:mx-0 px-4 sm:px-0">
{`import { Connection } from '@solana/web3.js';

const connection = new Connection(
  '${rpcUrl}',
  {
    httpHeaders: {
      'Authorization': 'Bearer ${currentApiKey ? `${currentApiKey.substring(0, 4)}...${currentApiKey.substring(currentApiKey.length - 4)}` : 'your_api_key_here'}'
    }
  }
);`}
            </pre>
          </div>

          <div className="bg-slate-950/50 rounded-lg p-4 sm:p-5 mb-4 relative group border border-slate-800">
            <div className="flex items-center justify-between mb-3">
              <div className="text-xs text-slate-500 font-semibold uppercase tracking-wide">cURL with Authorization</div>
              <button
                onClick={() => {
                  const code = `curl -X POST ${rpcUrl} \\
  -H "Authorization: Bearer ${currentApiKey || 'your_api_key_here'}" \\
  -H "Content-Type: application/json" \\
  -d '{"jsonrpc":"2.0","id":1,"method":"getVersion"}'`;
                  navigator.clipboard.writeText(code);
                }}
                className="px-3 py-1.5 bg-slate-800 hover:bg-slate-700 text-slate-300 hover:text-slate-100 text-xs rounded-md transition-all border border-slate-700 font-medium sm:opacity-0 sm:group-hover:opacity-100"
              >
                Copy
              </button>
            </div>
            <pre className="text-xs text-slate-300 overflow-x-auto font-mono -mx-4 sm:mx-0 px-4 sm:px-0">
{`curl -X POST ${rpcUrl} \\
  -H "Authorization: Bearer ${currentApiKey ? `${currentApiKey.substring(0, 4)}...${currentApiKey.substring(currentApiKey.length - 4)}` : 'your_api_key_here'}" \\
  -H "Content-Type: application/json" \\
  -d '{"jsonrpc":"2.0","id":1,"method":"getVersion"}'`}
            </pre>
          </div>

          <div className="bg-slate-950/50 rounded-lg p-4 sm:p-5 relative group border border-slate-800">
            <div className="flex items-center justify-between mb-3">
              <div className="text-xs text-slate-500 font-semibold uppercase tracking-wide">cURL with Query Param</div>
              <button
                onClick={() => {
                  const code = `curl -X POST '${rpcUrl}?api-key=${currentApiKey || 'your_api_key_here'}' \\
  -H "Content-Type: application/json" \\
  -d '{"jsonrpc":"2.0","id":1,"method":"getVersion"}'`;
                  navigator.clipboard.writeText(code);
                }}
                className="px-3 py-1.5 bg-slate-800 hover:bg-slate-700 text-slate-300 hover:text-slate-100 text-xs rounded-md transition-all border border-slate-700 font-medium sm:opacity-0 sm:group-hover:opacity-100"
              >
                Copy
              </button>
            </div>
            <pre className="text-xs text-slate-300 overflow-x-auto font-mono -mx-4 sm:mx-0 px-4 sm:px-0">
{`curl -X POST '${rpcUrl}?api-key=${currentApiKey ? `${currentApiKey.substring(0, 4)}...${currentApiKey.substring(currentApiKey.length - 4)}` : 'your_api_key_here'}' \\
  -H "Content-Type: application/json" \\
  -d '{"jsonrpc":"2.0","id":1,"method":"getVersion"}'`}
            </pre>
          </div>
        </div>

        {/* Resources */}
        <div className="pt-6 sm:pt-8 border-t border-slate-800">
          <div className="flex flex-col sm:flex-row gap-3">
            <a
              href="https://github.com/securecheckio/sol-shield"
              target="_blank"
              rel="noopener noreferrer"
              className="inline-flex items-center justify-center gap-2 px-5 py-2.5 bg-slate-800 hover:bg-slate-700 font-medium rounded-md transition-colors text-sm border border-slate-700"
            >
              <svg className="w-5 h-5" fill="currentColor" viewBox="0 0 24 24">
                <path fillRule="evenodd" d="M12 2C6.477 2 2 6.484 2 12.017c0 4.425 2.865 8.18 6.839 9.504.5.092.682-.217.682-.483 0-.237-.008-.868-.013-1.703-2.782.605-3.369-1.343-3.369-1.343-.454-1.158-1.11-1.466-1.11-1.466-.908-.62.069-.608.069-.608 1.003.07 1.531 1.032 1.531 1.032.892 1.53 2.341 1.088 2.91.832.092-.647.35-1.088.636-1.338-2.22-.253-4.555-1.113-4.555-4.951 0-1.093.39-1.988 1.029-2.688-.103-.253-.446-1.272.098-2.65 0 0 .84-.27 2.75 1.026A9.564 9.564 0 0112 6.844c.85.004 1.705.115 2.504.337 1.909-1.296 2.747-1.027 2.747-1.027.546 1.379.202 2.398.1 2.651.64.7 1.028 1.595 1.028 2.688 0 3.848-2.339 4.695-4.566 4.943.359.309.678.92.678 1.855 0 1.338-.012 2.419-.012 2.747 0 .268.18.58.688.482A10.019 10.019 0 0022 12.017C22 6.484 17.522 2 12 2z" clipRule="evenodd" />
              </svg>
              Documentation
            </a>
            <a
              href="https://forms.gle/LLavLazLf3aasRZc6"
              target="_blank"
              rel="noopener noreferrer"
              className="inline-flex items-center justify-center gap-2 px-5 py-2.5 bg-blue-600 hover:bg-blue-700 font-medium rounded-md transition-colors text-sm"
            >
              <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M7 8h10M7 12h4m1 8l-4-4H5a2 2 0 01-2-2V6a2 2 0 012-2h14a2 2 0 012 2v8a2 2 0 01-2 2h-3l-4 4z" />
              </svg>
              Give Feedback
            </a>
          </div>
        </div>
      </div>
    </div>
  );
};

export default Dashboard;
