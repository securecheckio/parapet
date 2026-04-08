#!/bin/bash
# Example: Using Emerging Threat Detection with Parapet

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "=========================================="
echo "Parapet Emerging Threat Detection Demo"
echo "=========================================="
echo ""

# Check for HELIUS_API_KEY
if [ -z "$HELIUS_API_KEY" ]; then
    echo -e "${RED}ERROR: HELIUS_API_KEY not set${NC}"
    echo "Get a free API key at: https://dashboard.helius.dev"
    echo ""
    echo "Usage:"
    echo "  export HELIUS_API_KEY=your_key_here"
    echo "  $0 <wallet_address>"
    exit 1
fi

# Check for wallet address argument
if [ -z "$1" ]; then
    echo -e "${YELLOW}Usage: $0 <wallet_address>${NC}"
    echo ""
    echo "Example:"
    echo "  $0 9xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVFin"
    exit 1
fi

WALLET=$1

echo -e "${GREEN}✓${NC} Helius API key configured"
echo -e "${GREEN}✓${NC} Scanning wallet: $WALLET"
echo ""

# Build wallet-scanner if needed
if [ ! -f "../target/release/wallet-scanner" ]; then
    echo "Building wallet-scanner..."
    cd ../scanner
    cargo build --release --bin wallet-scanner
    cd ../examples
fi

# Run scan with emerging threat detection
echo "Running scan with emerging threat detection..."
echo ""

# Set rules to use emerging threats preset
export RULES_PATH="../proxy/rules/presets/emerging-threats.json"

../target/release/wallet-scanner "$WALLET" \
    --max-transactions 50 \
    --time-window-days 7 \
    --format pretty

echo ""
echo "=========================================="
echo "Scan Complete"
echo "=========================================="
echo ""
echo "What was checked:"
echo "  ✓ Active unlimited delegations"
echo "  ✓ Transfer velocity (>10 tx/hour = drain)"
echo "  ✓ Counterparty concentration (>80% = phishing)"
echo "  ✓ Funding source (sybil/bot detection)"
echo "  ✓ Known scammer interactions"
echo ""
echo "Rules used: emerging-threats.json"
echo "  - active-drain-velocity (BLOCK)"
echo "  - phishing-victim-concentration (ALERT)"
echo "  - sybil-wallet (ALERT)"
echo "  - compromised-agent (BLOCK)"
echo ""
