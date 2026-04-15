#!/bin/bash

USER_WALLET="DfaQ3MBiL9ighEyVpu9zt9efjWiBRq8sdJQCnawQpR7N"

# Kill everything
pkill -9 -f parapet-api-core 2>/dev/null || true
pkill -9 -f parapet-proxy 2>/dev/null || true
sleep 2

# Clear Redis
redis-cli FLUSHDB > /dev/null 2>&1

# Start API
cd api-core
../target/release/parapet-api-core > /tmp/api.log 2>&1 &
cd ..
sleep 3

# Check API started
curl -s http://localhost:3001/health > /dev/null 2>&1
if [ $? -eq 0 ]; then
  echo "✅ API running on 3001"
else
  echo "❌ API failed to start"
  echo "Check: tail -20 /tmp/api.log"
  exit 1
fi

# Start Proxy
cd proxy
ESCALATION_APPROVER_WALLET=$USER_WALLET ../target/release/parapet-proxy > /tmp/proxy.log 2>&1 &
cd ..
sleep 3

# Check Proxy started
curl -s http://localhost:8899/health > /dev/null 2>&1
if [ $? -eq 0 ]; then
  echo "✅ Proxy running on 8899"
else
  echo "❌ Proxy failed to start"
  echo "Check: tail -20 /tmp/proxy.log"
  exit 1
fi

# Create test escalation
ESC_ID="esc_demo_$(date +%s)"
redis-cli SET "escalation:pending:$ESC_ID" "{
  \"escalation_id\": \"$ESC_ID\",
  \"canonical_hash\": \"TestHash123\",
  \"requester_wallet\": \"TestUser\",
  \"approver_wallet\": \"$USER_WALLET\",
  \"risk_score\": 85,
  \"warnings\": [\"High risk detected\"],
  \"decoded_instructions\": [],
  \"suggested_rules\": [],
  \"status\": \"pending\",
  \"created_at\": $(date +%s),
  \"expires_at\": $(($(date +%s) + 600))
}" EX 600 > /dev/null

redis-cli SADD "escalation:pending:approver:$USER_WALLET" "$ESC_ID" > /dev/null

echo "✅ Created escalation: $ESC_ID"
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "🌐 Open: http://localhost:8080"
echo "🔐 Connect wallet: $USER_WALLET"
echo ""
echo "If wallet connection fails, check browser console (F12)"
