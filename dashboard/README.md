# Parapet Activity Feed Dashboard

Lightweight activity monitoring dashboard for AI agents and automated systems. Built with React + Vite + Tailwind CSS.

**Purpose**: Real-time transaction monitoring for AI agents, showing security analysis results as transactions are processed through the Parapet proxy.

## Production Deployment

### Quick Deploy (Static Files)
```bash
# Build production bundle
npm run build

# Serve with any static file server
cd dist && python3 -m http.server 8080
```

### Docker Deployment
```bash
# Build and run with Docker
docker build -t parapet-dashboard .
docker run -p 8080:80 parapet-dashboard

# Or use Docker Compose
cd deployments/dashboard
docker-compose up -d
```

### Deploy Script
```bash
./deploy.sh
```

## Development

```bash
# Install dependencies
npm install --legacy-peer-deps

# Start dev server
npm run dev

# Build for production
npm run build
```

## Configuration

The dashboard connects to the Parapet API at `http://localhost:3001` by default.

API URLs are auto-detected based on hostname:
- Local: `http://localhost:3001`
- Production: `https://{hostname}:9444`

## Production Bundle

- **Size**: ~33KB (single HTML file with inline assets)
- **No runtime dependencies**: Fully static, works anywhere
- **Mobile-first**: Optimized for touch interfaces
- **Real-time**: WebSocket updates for new escalations

## Features

- ✅ Multi-wallet support (Backpack, Phantom, Solflare)
- ✅ Real-time activity feed via WebSocket
- ✅ Transaction risk scoring (0-100)
- ✅ Solscan transaction links (network-aware)
- ✅ Wallet selector for multiple installed wallets
- ✅ Authenticated API with wallet signatures
- ✅ Mobile-optimized UI

**Note**: Escalation management (approve/deny) is a separate feature and will be added later.

## Tech Stack

- **React 18** - UI framework
- **Vite** - Build tool
- **Tailwind CSS** - Styling
- **TypeScript** - Type safety
- **bs58** - Solana signature encoding
- **lucide-react** - Icons
