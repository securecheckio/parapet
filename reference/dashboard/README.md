# Parapet Reference Dashboard

Full-featured marketing and learning platform demonstrating Parapet capabilities.

## Purpose

This is a **reference implementation** showing:
- Transaction simulation UI
- Educational content about Solana security
- Multi-wallet integration patterns
- Rules visualization
- Risk scoring explanations

**For AI agent activity monitoring**, use `../../dashboard/` instead.

## Features

- 🎓 **Learning Mode**: Interactive tutorials on Solana security
- 🔬 **Simulation**: Test transactions against security rules
- 📊 **Visualization**: See how rules evaluate transactions
- 🎨 **Marketing**: Landing pages and product information
- 🔗 **Integration Examples**: Multiple wallet providers

## Architecture

Multi-page React application with:
- React Router for navigation
- Context API for state management
- Wallet adapter integration
- Parapet API client

## Setup

```bash
# Install dependencies
npm install

# Configure API endpoint
cp .env.example .env
# Edit .env with your API URL

# Start dev server
npm run dev
```

## Structure

```
src/
├── pages/          # Marketing and feature pages
│   ├── Home.tsx
│   ├── Learn.tsx
│   ├── Simulate.tsx
│   └── Dashboard.tsx
├── components/     # Reusable UI components
├── contexts/       # React contexts
└── lib/            # Utilities and API client
```

## Comparison: Reference vs Production

| Feature | Reference Dashboard | Production Dashboard |
|---------|---------------------|---------------------|
| Purpose | Learning & marketing | AI agent monitoring |
| Pages | Multi-page (React Router) | Single-page |
| Focus | Education & simulation | Real-time activity |
| Target | End users & developers | AI agents & systems |
| Complexity | High (many features) | Low (focused) |

## Use Cases

### This Reference Dashboard
- Marketing site for Parapet
- Educational platform for learning Solana security
- Demonstration of Parapet capabilities
- Testing and simulation tool

### Production Dashboard (`../../dashboard/`)
- Real-time transaction monitoring for AI agents
- Operational security feed
- Minimal, focused interface

## Development

```bash
# Run development server
npm run dev

# Build for production
npm run build

# Preview production build
npm run preview
```

## Deployment

This is reference code. For production deployment:

1. Customize branding and content
2. Configure API endpoints
3. Add analytics and monitoring
4. Set up CDN and caching
5. Implement proper error handling

Or use `../../dashboard/` for a production-ready activity monitoring solution.

## Contributing

Improvements welcome! This serves as a learning resource for the community.
