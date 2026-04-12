#!/bin/bash

# Wallet scanner wrapper with Helius RPC support

set -e

# Try to load Helius API key from .env files if not already set
if [ -z "$HELIUS_API_KEY" ]; then
    # Check common locations for .env files
    ENV_LOCATIONS=(
        "../../proxy/.env"
        "../proxy/.env"
        "../../saas/.env.local"
        "../saas/.env.local"
        ".env"
    )
    
    for env_file in "${ENV_LOCATIONS[@]}"; do
        if [ -f "$env_file" ]; then
            FOUND_KEY=$(grep "^HELIUS_API_KEY=" "$env_file" 2>/dev/null | cut -d= -f2 | tr -d '"' | tr -d "'" | head -1)
            if [ -n "$FOUND_KEY" ]; then
                export HELIUS_API_KEY="$FOUND_KEY"
                echo "✅ Loaded Helius API key from $env_file"
                break
            fi
        fi
    done
fi

# Check for Helius API key
if [ -z "$HELIUS_API_KEY" ]; then
    echo "⚠️  HELIUS_API_KEY not set. Using public RPC (slower, rate limited)"
    echo "    Set it with: export HELIUS_API_KEY=your_key"
    RPC_URL="https://api.mainnet-beta.solana.com"
    DEFAULT_DELAY=0  # Let scanner auto-calculate from analyzers
else
    echo "✅ Using Helius RPC (with dynamic analyzer coordination)"
    RPC_URL="https://mainnet.helius-rpc.com/?api-key=${HELIUS_API_KEY}"
    DEFAULT_DELAY=0  # Let scanner auto-calculate from analyzers
fi

# Parse arguments
WALLET=""
MAX_TX=100
DAYS=30
DELAY=${DEFAULT_DELAY}
FORMAT="pretty"
NETWORK="mainnet-beta"
SAFE_PROGRAMS_FILE=""

while [[ $# -gt 0 ]]; do
    case $1 in
        -t|--max-transactions)
            MAX_TX="$2"
            shift 2
            ;;
        -d|--days)
            DAYS="$2"
            shift 2
            ;;
        --rpc-delay-ms)
            DELAY="$2"
            shift 2
            ;;
        --safe-programs-file)
            SAFE_PROGRAMS_FILE="$2"
            shift 2
            ;;
        -f|--format)
            FORMAT="$2"
            shift 2
            ;;
        -n|--network)
            NETWORK="$2"
            shift 2
            ;;
        --devnet)
            NETWORK="devnet"
            if [ -n "$HELIUS_API_KEY" ]; then
                RPC_URL="https://devnet.helius-rpc.com/?api-key=${HELIUS_API_KEY}"
            else
                RPC_URL="https://api.devnet.solana.com"
            fi
            shift
            ;;
        -h|--help)
            echo "Usage: $0 <wallet_address> [options]"
            echo ""
            echo "Options:"
            echo "  -t, --max-transactions NUM   Max transactions (default: 100)"
            echo "  -d, --days NUM              Days to look back (default: 30)"
            echo "      --rpc-delay-ms MS       RPC throttle delay (default: auto)"
            echo "  -f, --format FORMAT         Output: pretty, json, brief"
            echo "  -n, --network NET           mainnet-beta, devnet, testnet"
            echo "      --devnet                Shortcut for devnet"
            echo ""
            echo "Environment:"
            echo "  HELIUS_API_KEY              Your Helius API key (recommended)"
            echo ""
            echo "Examples:"
            echo "  # With Helius (faster)"
            echo "  export HELIUS_API_KEY=your_key"
            echo "  $0 WALLET_ADDRESS"
            echo ""
            echo "  # Without Helius (public RPC)"
            echo "  $0 WALLET_ADDRESS -t 50 -d 7"
            exit 0
            ;;
        *)
            if [ -z "$WALLET" ]; then
                WALLET="$1"
            else
                echo "Error: Unknown argument '$1'"
                exit 1
            fi
            shift
            ;;
    esac
done

if [ -z "$WALLET" ]; then
    echo "Error: Wallet address required"
    echo "Run '$0 --help' for usage"
    exit 1
fi

# Build the binary if needed (it goes to sol-shield/target, not scanner/target)
BINARY_PATH="../target/release/wallet-scanner"
if [ ! -f "$BINARY_PATH" ]; then
    echo "Building wallet scanner..."
    cargo build --release --bin wallet-scanner
fi

# Build safe-programs-file argument if specified
SAFE_PROGRAMS_ARG=""
if [ -n "$SAFE_PROGRAMS_FILE" ]; then
    SAFE_PROGRAMS_ARG="--safe-programs-file $SAFE_PROGRAMS_FILE"
fi

# Run the scanner
exec "$BINARY_PATH" "$WALLET" \
    --rpc-url "$RPC_URL" \
    -t "$MAX_TX" \
    -d "$DAYS" \
    --rpc-delay-ms "$DELAY" \
    -f "$FORMAT" \
    -n "$NETWORK" \
    $SAFE_PROGRAMS_ARG
