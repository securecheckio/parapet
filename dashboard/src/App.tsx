import { useState, useEffect } from 'react'
import { Shield, AlertTriangle, Activity, CheckCircle2, XCircle } from 'lucide-react'
import bs58 from 'bs58'

// Auto-detect API URL based on current location
const getApiUrl = () => {
  const hostname = window.location.hostname
  if (hostname !== 'localhost' && hostname !== '127.0.0.1') {
    return `https://${hostname}:9444`
  }
  return 'http://localhost:3001'
}

const getWsUrl = () => {
  const hostname = window.location.hostname
  if (hostname !== 'localhost' && hostname !== '127.0.0.1') {
    return `wss://${hostname}:9444/ws/activity`
  }
  return 'ws://localhost:3001/ws/activity'
}

const API_URL = getApiUrl()
const WS_URL = getWsUrl()

interface ActivityEvent {
  activity_id: string
  wallet: string
  risk_score: number
  rule_id: string
  rule_name: string
  message: string
  canonical_hash: string
  signature?: string
  network?: string
  timestamp: number
  action: 'allowed' | 'blocked' | 'flagged'
}

interface WalletOption {
  name: string
  provider: any
  icon: string
}

function App() {
  const [wallet, setWallet] = useState<string | null>(null)
  const [walletProvider, setWalletProvider] = useState<any>(null)
  const [activities, setActivities] = useState<ActivityEvent[]>([])
  const [wsConnected, setWsConnected] = useState(false)
  const [connecting, setConnecting] = useState(false)
  const [notification, setNotification] = useState<{type: 'success' | 'error', message: string} | null>(null)
  const [showWalletSelector, setShowWalletSelector] = useState(false)
  const [availableWallets, setAvailableWallets] = useState<WalletOption[]>([])

  // Detect all available wallets
  const detectAllWallets = (): WalletOption[] => {
    if (typeof window === 'undefined') return []
    
    const wallets: WalletOption[] = []
    
    if ((window as any).backpack?.isBackpack) {
      wallets.push({
        name: 'Backpack',
        provider: (window as any).backpack,
        icon: '🎒'
      })
    }
    
    if ((window as any).phantom?.solana?.isPhantom) {
      wallets.push({
        name: 'Phantom',
        provider: (window as any).phantom.solana,
        icon: '👻'
      })
    }
    
    if ((window as any).solflare?.isSolflare) {
      wallets.push({
        name: 'Solflare',
        provider: (window as any).solflare,
        icon: '☀️'
      })
    }
    
    // Generic fallback (only if no specific wallet detected)
    if (wallets.length === 0 && (window as any).solana) {
      wallets.push({
        name: 'Solana Wallet',
        provider: (window as any).solana,
        icon: '🟣'
      })
    }
    
    return wallets
  }

  // Detect installed wallet (legacy - for single wallet case)
  const detectWallet = () => {
    const wallets = detectAllWallets()
    return wallets.length > 0 ? wallets[0].provider : null
  }

  // Update available wallets on mount and periodically
  useEffect(() => {
    const updateWallets = () => {
      const wallets = detectAllWallets()
      setAvailableWallets(wallets)
    }
    
    updateWallets()
    const interval = setInterval(updateWallets, 1000)
    
    return () => clearInterval(interval)
  }, [])

  // Base58 encoding helper
  const encodeBase58 = (buffer: Uint8Array): string => {
    return bs58.encode(buffer)
  }

  // Connect wallet with specific provider
  const connectWalletWithProvider = async (provider: any, walletName: string) => {
    try {
      setConnecting(true)
      setShowWalletSelector(false)

      const response = await provider.connect({ onlyIfTrusted: false })
      const publicKey = response?.publicKey?.toString() || provider.publicKey?.toString()

      if (!publicKey) {
        throw new Error('Failed to get wallet public key')
      }

      setWallet(publicKey)
      setWalletProvider(provider)
      localStorage.setItem('parapet_wallet', publicKey)
      localStorage.setItem('parapet_wallet_provider', walletName)

      // Load recent activity
      await loadRecentActivity(publicKey, provider)

      // Connect to activity WebSocket
      connectWebSocket(publicKey, provider)
      
      setNotification({ type: 'success', message: `Connected to ${walletName}` })
      setTimeout(() => setNotification(null), 3000)
    } catch (error) {
      console.error('Failed to connect wallet:', error)
      setNotification({ type: 'error', message: 'Failed to connect wallet: ' + (error as Error).message })
      setTimeout(() => setNotification(null), 5000)
    } finally {
      setConnecting(false)
    }
  }

  // Connect wallet (shows selector if multiple wallets available)
  const connectWallet = async () => {
    const wallets = detectAllWallets()
    
    if (wallets.length === 0) {
      setNotification({ type: 'error', message: 'No Solana wallet found. Please install Backpack, Phantom, or Solflare.' })
      setTimeout(() => setNotification(null), 5000)
      return
    }
    
    if (wallets.length === 1) {
      // Only one wallet, connect directly
      await connectWalletWithProvider(wallets[0].provider, wallets[0].name)
    } else {
      // Multiple wallets, show selector
      setShowWalletSelector(true)
    }
  }

  // Disconnect wallet
  const disconnectWallet = () => {
    setWallet(null)
    setWalletProvider(null)
    setActivities([])
    setWsConnected(false)
    localStorage.removeItem('parapet_wallet')
    localStorage.removeItem('parapet_wallet_provider')
    setNotification({ type: 'success', message: 'Wallet disconnected' })
    setTimeout(() => setNotification(null), 3000)
  }

  // Load recent activity
  const loadRecentActivity = async (walletAddress: string, provider: any) => {
    try {
      const timestamp = Math.floor(Date.now() / 1000)
      const message = `parapet:activity:${walletAddress}:${timestamp}`

      const encodedMessage = new TextEncoder().encode(message)
      const signatureResponse = await provider.signMessage(encodedMessage, 'utf8')
      const signature = encodeBase58(signatureResponse.signature)

      const url = `${API_URL}/api/v1/activity/recent?wallet=${encodeURIComponent(walletAddress)}&message=${encodeURIComponent(message)}&signature=${encodeURIComponent(signature)}&timestamp=${timestamp}&limit=50`

      const response = await fetch(url, {
        method: 'GET',
        headers: {
          'Content-Type': 'application/json',
        },
      })

      if (response.ok) {
        const data = await response.json()
        console.log('📊 Loaded recent activity:', data)
        setActivities(data)
      } else {
        console.error('Failed to load activity:', response.status)
      }
    } catch (error) {
      console.error('Failed to load activity:', error)
    }
  }

  // Connect to WebSocket for real-time updates
  const connectWebSocket = async (walletAddress: string, provider: any) => {
    try {
      const ws = new WebSocket(WS_URL)

      ws.onopen = async () => {
        console.log('📡 WebSocket connected, sending subscription...')

        const timestamp = Math.floor(Date.now() / 1000)
        const message = `parapet:ws:activity:subscribe:${walletAddress}:${timestamp}`
        const encodedMessage = new TextEncoder().encode(message)
        const signatureResponse = await provider.signMessage(encodedMessage, 'utf8')
        const signature = encodeBase58(signatureResponse.signature)

        const subscribeMsg = {
          wallet: walletAddress,
          message,
          signature,
          timestamp,
        }

        ws.send(JSON.stringify(subscribeMsg))
        setWsConnected(true)
      }

      ws.onmessage = (event) => {
        try {
          const data = JSON.parse(event.data)
          console.log('📨 Activity update:', data)

          // Add new activity to top of list
          setActivities(prev => [data, ...prev].slice(0, 50))
        } catch (error) {
          console.error('Failed to parse WebSocket message:', error)
        }
      }

      ws.onerror = (error) => {
        console.error('WebSocket error:', error)
        setWsConnected(false)
      }

      ws.onclose = () => {
        console.log('📴 WebSocket disconnected')
        setWsConnected(false)
        
        // Reconnect after 5 seconds
        setTimeout(() => {
          if (wallet && walletProvider) {
            connectWebSocket(wallet, walletProvider)
          }
        }, 5000)
      }
    } catch (error) {
      console.error('Failed to connect WebSocket:', error)
    }
  }

  // Auto-connect on page load
  useEffect(() => {
    const savedWallet = localStorage.getItem('parapet_wallet')
    if (savedWallet) {
      console.log('🔄 Attempting auto-reconnect for:', savedWallet)
      const provider = detectWallet()
      
      if (provider) {
        provider.connect({ onlyIfTrusted: true })
          .then((response: any) => {
            const publicKey = response?.publicKey?.toString() || provider.publicKey?.toString()
            if (publicKey && publicKey === savedWallet) {
              console.log('✅ Auto-reconnected:', publicKey)
              setWallet(publicKey)
              setWalletProvider(provider)
              loadRecentActivity(publicKey, provider)
              connectWebSocket(publicKey, provider)
            }
          })
          .catch((error: any) => {
            console.log('Auto-reconnect failed (user may need to manually connect):', error)
          })
      }
    }
  }, [])

  // Format timestamp
  const formatTime = (timestamp: number) => {
    const date = new Date(timestamp * 1000)
    const now = new Date()
    const diff = now.getTime() - date.getTime()
    const seconds = Math.floor(diff / 1000)
    const minutes = Math.floor(seconds / 60)
    const hours = Math.floor(minutes / 60)
    const days = Math.floor(hours / 24)

    if (days > 0) return `${days}d ago`
    if (hours > 0) return `${hours}h ago`
    if (minutes > 0) return `${minutes}m ago`
    return `${seconds}s ago`
  }

  // Risk color
  const getRiskColor = (score: number) => {
    if (score >= 70) return 'text-red-400'
    if (score >= 40) return 'text-yellow-400'
    return 'text-green-400'
  }

  // Action icon
  const getActionIcon = (action: string) => {
    switch (action) {
      case 'allowed': return <CheckCircle2 className="w-5 h-5 text-green-400" />
      case 'blocked': return <XCircle className="w-5 h-5 text-red-400" />
      case 'flagged': return <AlertTriangle className="w-5 h-5 text-yellow-400" />
      default: return <Activity className="w-5 h-5 text-gray-400" />
    }
  }

  // Generate Solscan URL
  const getSolscanUrl = (signature: string | undefined, network: string | undefined) => {
    if (!signature) return null
    
    const baseUrl = 'https://solscan.io/tx'
    const cluster = network === 'mainnet-beta' ? '' : `?cluster=${network || 'devnet'}`
    
    return `${baseUrl}/${signature}${cluster}`
  }

  return (
    <div className="min-h-screen bg-slate-950 pb-20">
      {/* Notification Toast */}
      {notification && (
        <div className={`fixed top-4 left-1/2 -translate-x-1/2 z-[100] px-6 py-4 rounded-lg shadow-2xl border-2 max-w-md w-full mx-4 ${
          notification.type === 'success' 
            ? 'bg-green-500/90 border-green-400 text-white' 
            : 'bg-red-500/90 border-red-400 text-white'
        }`}>
          <div className="flex items-center gap-3">
            {notification.type === 'success' ? (
              <CheckCircle2 className="w-5 h-5 flex-shrink-0" />
            ) : (
              <XCircle className="w-5 h-5 flex-shrink-0" />
            )}
            <p className="font-semibold text-sm">{notification.message}</p>
            <button 
              onClick={() => setNotification(null)}
              className="ml-auto text-white/80 hover:text-white"
            >
              ✕
            </button>
          </div>
        </div>
      )}

      {/* Wallet Selector Modal */}
      {showWalletSelector && (
        <div className="fixed inset-0 z-[100] flex items-center justify-center bg-black/70 backdrop-blur-sm">
          <div className="bg-slate-900 border border-slate-700 rounded-2xl p-6 max-w-md w-full mx-4 shadow-2xl">
            <div className="flex items-center justify-between mb-4">
              <h2 className="text-xl font-bold text-slate-100">Select Wallet</h2>
              <button
                onClick={() => setShowWalletSelector(false)}
                className="text-slate-400 hover:text-slate-200 text-xl"
              >
                ✕
              </button>
            </div>
            
            <p className="text-slate-400 text-sm mb-6">
              Multiple wallets detected. Choose one to connect:
            </p>

            <div className="space-y-3">
              {availableWallets.map((walletOption) => (
                <button
                  key={walletOption.name}
                  onClick={() => connectWalletWithProvider(walletOption.provider, walletOption.name)}
                  disabled={connecting}
                  className="w-full flex items-center gap-4 p-4 bg-slate-800 hover:bg-slate-700 border border-slate-700 hover:border-blue-500 rounded-xl transition-all disabled:opacity-50"
                >
                  <span className="text-3xl">{walletOption.icon}</span>
                  <div className="flex-1 text-left">
                    <div className="font-semibold text-slate-100">{walletOption.name}</div>
                    <div className="text-xs text-slate-400">
                      {walletOption.name === 'Backpack' && 'Multi-chain wallet'}
                      {walletOption.name === 'Phantom' && 'Popular Solana wallet'}
                      {walletOption.name === 'Solflare' && 'Feature-rich wallet'}
                      {walletOption.name === 'Solana Wallet' && 'Generic wallet provider'}
                    </div>
                  </div>
                  <div className="text-blue-400">→</div>
                </button>
              ))}
            </div>

            {availableWallets.length === 0 && (
              <div className="text-center py-8 text-slate-400">
                <p className="mb-2">No wallets detected</p>
                <p className="text-sm">Please install Backpack, Phantom, or Solflare</p>
              </div>
            )}
          </div>
        </div>
      )}

      {/* Header */}
      <div className="sticky top-0 z-50 bg-slate-950/80 backdrop-blur-xl border-b border-white/10">
        <div className="max-w-4xl mx-auto px-4 py-4 flex items-center justify-between">
          <div className="flex items-center gap-3">
            <Shield className="w-8 h-8 text-blue-400" />
            <div>
              <h1 className="text-xl font-bold bg-gradient-to-r from-blue-400 to-cyan-400 text-transparent bg-clip-text">
                Parapet
              </h1>
              <p className="text-xs text-slate-400">Activity Feed</p>
            </div>
          </div>

          {wallet ? (
            <div className="flex items-center gap-3">
              <div className="text-right">
                <div className="text-xs font-mono text-slate-400">
                  {wallet.slice(0, 4)}...{wallet.slice(-4)}
                </div>
                <div className={`text-xs ${wsConnected ? 'text-green-400' : 'text-red-400'}`}>
                  {wsConnected ? '● Connected' : '● Disconnected'}
                </div>
              </div>
              <button
                onClick={disconnectWallet}
                className="px-4 py-2 bg-slate-800 hover:bg-slate-700 rounded-lg text-sm font-medium transition-colors"
              >
                Disconnect
              </button>
            </div>
          ) : (
            <button
              onClick={connectWallet}
              disabled={connecting}
              className="px-6 py-2 bg-gradient-to-r from-blue-500 to-cyan-500 hover:from-blue-600 hover:to-cyan-600 rounded-lg font-semibold text-white transition-all disabled:opacity-50"
            >
              {connecting ? 'Connecting...' : 'Connect Wallet'}
            </button>
          )}
        </div>
      </div>

      {/* Activity Feed */}
      <div className="max-w-4xl mx-auto px-4 py-6">
        {!wallet ? (
          <div className="text-center py-20">
            <Shield className="w-16 h-16 mx-auto text-slate-600 mb-4" />
            <h2 className="text-xl font-semibold text-slate-300 mb-2">Connect Your Wallet</h2>
            <p className="text-slate-500">Monitor your transaction activity in real-time</p>
          </div>
        ) : activities.length === 0 ? (
          <div className="text-center py-20">
            <Activity className="w-16 h-16 mx-auto text-slate-600 mb-4" />
            <h2 className="text-xl font-semibold text-slate-300 mb-2">No Recent Activity</h2>
            <p className="text-slate-500">Send a transaction to see it appear here</p>
          </div>
        ) : (
          <div className="space-y-3">
            {activities.map((activity) => (
              <div
                key={activity.activity_id}
                className="bg-slate-900/50 backdrop-blur border border-white/5 rounded-xl p-4 hover:bg-slate-900/70 transition-all"
              >
                <div className="flex items-start justify-between mb-3">
                  <div className="flex items-center gap-3">
                    {getActionIcon(activity.action)}
                    <div>
                      <h3 className="font-semibold text-slate-200">{activity.rule_name}</h3>
                      <p className="text-xs text-slate-500">{formatTime(activity.timestamp)}</p>
                    </div>
                  </div>
                  <div className="text-right">
                    <div className={`text-sm font-bold ${getRiskColor(activity.risk_score)}`}>
                      Risk: {activity.risk_score}
                    </div>
                    <div className="text-xs text-slate-500 capitalize">{activity.action}</div>
                  </div>
                </div>

                <p className="text-sm text-slate-400 mb-2">{activity.message}</p>

                <div className="flex items-center justify-between">
                  <div className="text-xs font-mono text-slate-600 truncate flex-1">
                    {activity.signature ? `Sig: ${activity.signature.slice(0, 8)}...${activity.signature.slice(-8)}` : activity.canonical_hash}
                  </div>
                  {activity.signature && (
                    <a
                      href={getSolscanUrl(activity.signature, activity.network) || '#'}
                      target="_blank"
                      rel="noopener noreferrer"
                      className="ml-3 px-3 py-1 bg-blue-500/10 hover:bg-blue-500/20 border border-blue-500/30 rounded text-xs text-blue-400 hover:text-blue-300 transition-colors flex items-center gap-1"
                    >
                      <span>View on Solscan</span>
                      <span>↗</span>
                    </a>
                  )}
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  )
}

export default App
