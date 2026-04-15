#!/bin/bash
set -e

USER_WALLET="DfaQ3MBiL9ighEyVpu9zt9efjWiBRq8sdJQCnawQpR7N"

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "🎯 ESCALATION TEST WITH YOUR WALLET"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Wallet: $USER_WALLET"
echo ""

# Stop old services
pkill -f parapet-api-core 2>/dev/null || true
pkill -f parapet-proxy 2>/dev/null || true
pkill -f "http.server 8080" 2>/dev/null || true
sleep 1

# Start API
echo "1️⃣  Starting API server..."
cd api-core
../target/release/parapet-api-core > /tmp/api-user.log 2>&1 &
API_PID=$!
cd ..
sleep 2
echo "   API PID: $API_PID"

# Start Proxy with YOUR wallet
echo "2️⃣  Starting Proxy with YOUR wallet as approver..."
cd proxy
ESCALATION_APPROVER_WALLET=$USER_WALLET ../target/release/parapet-proxy > /tmp/proxy-user.log 2>&1 &
PROXY_PID=$!
cd ..
sleep 3
echo "   Proxy PID: $PROXY_PID"

# Start Dashboard
echo "3️⃣  Starting Dashboard..."
cd dashboard
python3 -m http.server 8080 > /tmp/dash-user.log 2>&1 &
DASH_PID=$!
cd ..
sleep 2
echo "   Dashboard PID: $DASH_PID"

# Clear Redis
echo ""
echo "4️⃣  Clearing old escalations..."
redis-cli FLUSHDB > /dev/null 2>&1
echo "   ✅ Redis cleared"

# Create test escalation FOR YOUR WALLET
echo ""
echo "5️⃣  Creating test escalation..."
ESC_ID="esc_test_$(date +%s)"
redis-cli SET "escalation:pending:$ESC_ID" '{
  "escalation_id": "'$ESC_ID'",
  "canonical_hash": "DemoTxHash123abc",
  "requester_wallet": "TestWallet...xyz",
  "approver_wallet": "'$USER_WALLET'",
  "risk_score": 85,
  "warnings": ["⚠️ High risk token delegation", "⚠️ Interacting with unverified program"],
  "decoded_instructions": [
    {"program": "Token Program", "instruction": "Approve", "accounts": ["delegate"]}
  ],
  "suggested_rules": [],
  "status": "pending",
  "created_at": '$(date +%s)',
  "expires_at": '$(($(date +%s) + 600))'
}' EX 600 > /dev/null

# Add to your wallet's pending set
redis-cli SADD "escalation:pending:approver:$USER_WALLET" "$ESC_ID" > /dev/null
redis-cli EXPIRE "escalation:pending:approver:$USER_WALLET" 600 > /dev/null

echo "   ✅ Created: $ESC_ID"
echo ""

# Verify
echo "6️⃣  Verifying escalation exists..."
echo ""
redis-cli GET "escalation:pending:$ESC_ID" | jq '.'
echo ""

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "✅ ALL SET! NOW TEST THE UI"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "📍 Services Running:"
echo "   API:       http://localhost:3001"
echo "   Proxy:     http://localhost:8899"
echo "   Dashboard: http://localhost:8080"
echo ""
echo "🎯 STEPS TO TEST:"
echo ""
echo "1. Open browser: http://localhost:8080"
echo "2. Click 'Connect Wallet'"
echo "3. Connect with: $USER_WALLET"
echo "4. You should see the escalation appear!"
echo "5. Click 'Approve' or 'Deny'"
echo ""
echo "📊 Your escalation:"
echo "   ID: $ESC_ID"
echo "   Risk Score: 85"
echo "   Status: Pending"
echo ""
echo "🔍 Debug:"
echo "   Check Redis: redis-cli GET \"escalation:pending:$ESC_ID\""
echo "   API logs:    tail -f /tmp/api-user.log"
echo "   Proxy logs:  tail -f /tmp/proxy-user.log"
echo ""
