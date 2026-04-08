#!/bin/bash
#
# Sol-Shield Program Analyzer Wrapper
# Analyzes Solana programs for security and verification status
#

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BINARY_PATH="../target/release/program-analyzer"

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Load Helius API key from common locations if not already set
if [ -z "$HELIUS_API_KEY" ]; then
    for env_file in "../proxy/.env" "../../proxy/.env" ".env"; do
        if [ -f "$env_file" ] && grep -q "HELIUS_API_KEY" "$env_file"; then
            export HELIUS_API_KEY=$(grep HELIUS_API_KEY "$env_file" | cut -d'=' -f2 | tr -d ' "')
            echo -e "${GREEN}✅ Loaded Helius API key from $env_file${NC}"
            break
        fi
    done
fi

# Configure RPC URL based on Helius key availability
if [ -n "$HELIUS_API_KEY" ]; then
    export SOLANA_RPC_URL="https://mainnet.helius-rpc.com/?api-key=$HELIUS_API_KEY"
    echo -e "${GREEN}✅ Using Helius RPC (faster, higher limits)${NC}"
else
    export SOLANA_RPC_URL="https://api.mainnet-beta.solana.com"
    echo -e "${YELLOW}⚠️  Using public RPC (rate limited, set HELIUS_API_KEY for better performance)${NC}"
fi

# Run the analyzer
"$BINARY_PATH" "$@"
