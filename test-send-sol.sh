#!/bin/bash
set -e

RECIPIENT="CyJSpqonriELcXeSQXnZ17AQsb77ZsHWFdttMmBstq8s"
AMOUNT="0.01"
FROM_WALLET="DfaQ3MBiL9ighEyVpu9zt9efjWiBRq8sdJQCnawQpR7N"

echo "🚀 Sending $AMOUNT SOL to $RECIPIENT through Parapet Proxy..."
echo ""
echo "Proxy: http://localhost:8899"
echo "Upstream: Devnet"
echo "From: $FROM_WALLET"
echo ""

# Send transaction via proxy
solana transfer $RECIPIENT $AMOUNT \
  --from $FROM_WALLET \
  --url http://localhost:8899 \
  --commitment confirmed

echo ""
echo "✅ Transaction sent!"
echo "Check the dashboard for escalation: http://localhost:8080"
