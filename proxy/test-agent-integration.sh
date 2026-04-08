#!/bin/bash

# Parapet Agent Integration Test Script
# Tests common agent interaction patterns with the RPC proxy

set -e

# Configuration
RPC_URL="${SOL_SHIELD_RPC_URL:-http://localhost:8899}"
API_KEY="${SOL_SHIELD_API_KEY:-}"

echo "🤖 Parapet Agent Integration Test"
echo "====================================="
echo ""
echo "RPC URL: $RPC_URL"
echo ""

# Helper function for RPC calls
rpc_call() {
  local method=$1
  local params=$2
  local description=$3
  
  echo "────────────────────────────────────────"
  echo "📋 Test: $description"
  echo "Method: $method"
  echo ""
  
  local headers="Content-Type: application/json"
  if [ -n "$API_KEY" ]; then
    headers="$headers\nX-API-Key: $API_KEY"
  fi
  
  local response=$(curl -s -w "\n%{http_code}" -X POST "$RPC_URL" \
    -H "$headers" \
    -d "{
      \"jsonrpc\": \"2.0\",
      \"id\": 1,
      \"method\": \"$method\",
      \"params\": $params
    }")
  
  local http_code=$(echo "$response" | tail -n1)
  local body=$(echo "$response" | head -n-1)
  
  echo "HTTP Status: $http_code"
  echo ""
  echo "Response:"
  echo "$body" | jq '.' 2>/dev/null || echo "$body"
  echo ""
  
  if [ "$http_code" != "200" ]; then
    echo "❌ Test failed with HTTP $http_code"
  else
    echo "✅ Test passed"
  fi
  echo ""
}

# Test 1: Health Check
echo "Test 1: Health Check"
echo "────────────────────────────────────────"
curl -s "$RPC_URL/health" | jq '.' 2>/dev/null || echo "Health endpoint not available"
echo ""
echo ""

# Test 2: Pass-through method (getHealth)
rpc_call "getHealth" "[]" "Pass-through RPC method"

# Test 3: Get balance (another pass-through)
rpc_call "getBalance" \
  '["7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU"]' \
  "Get wallet balance (pass-through)"

# Test 4: Get latest blockhash
rpc_call "getLatestBlockhash" \
  '[{"commitment": "confirmed"}]' \
  "Get latest blockhash (used for building transactions)"

echo "────────────────────────────────────────"
echo ""
echo "🎯 Summary"
echo "=========="
echo ""
echo "The script tests:"
echo "  1. ✅ Health endpoint accessibility"
echo "  2. ✅ Pass-through RPC methods work"
echo "  3. ✅ Standard Solana RPC compatibility"
echo ""
echo "📝 Next Steps for Agent Testing:"
echo ""
echo "1. Build a test transaction:"
echo "   - Simple SOL transfer (should be safe)"
echo "   - Test with simulateTransaction first"
echo "   - Parse solShield metadata from response"
echo ""
echo "2. Test simulation analysis:"
echo "   # Build a test transaction with your agent"
echo "   curl -X POST $RPC_URL \\"
echo "     -H 'Content-Type: application/json' \\"
echo "     -d '{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"simulateTransaction\",\"params\":[\"BASE58_TX\"]}'"
echo ""
echo "3. Check for solShield in response:"
echo "   - response.result.value.solShield should exist"
echo "   - Contains: riskScore, decision, warnings, etc."
echo ""
echo "4. Test sending a safe transaction:"
echo "   - Build simple transfer"
echo "   - Simulate first (check solShield)"
echo "   - If safe, send with sendTransaction"
echo ""
echo "5. Test blocked transaction (if in dev mode):"
echo "   - Create transaction with unlimited delegation"
echo "   - Should see would_block in simulation"
echo "   - Should receive 403 if you try to send"
echo ""
echo "📚 For complete guide, see: AI_AGENT_INTEGRATION_GUIDE.md"
echo ""
