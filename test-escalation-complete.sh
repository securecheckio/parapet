#!/bin/bash
set -e

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "🧪 PARAPET ESCALATION FLOW TEST"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Step 1: Check all services
echo "1️⃣  Checking services..."
redis-cli ping > /dev/null 2>&1 && echo "   ✅ Redis" || { echo "   ❌ Redis not running"; exit 1; }
curl -s http://localhost:3001/health > /dev/null 2>&1 && echo "   ✅ API" || { echo "   ❌ API not running"; exit 1; }
curl -s http://localhost:8899/health > /dev/null 2>&1 && echo "   ✅ Proxy" || { echo "   ❌ Proxy not running"; exit 1; }
echo ""

# Step 2: Clear old escalations
echo "2️⃣  Clearing old escalations..."
redis-cli DEL $(redis-cli KEYS "escalation:*" 2>/dev/null) > /dev/null 2>&1 || true
echo "   ✅ Redis cleared"
echo ""

# Step 3: Send a normal transaction (should pass)
echo "3️⃣  Testing PASS: Normal getHealth request..."
RESPONSE=$(curl -s -X POST http://localhost:8899 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}')
echo "   Response: $RESPONSE"
if echo "$RESPONSE" | grep -q "result"; then
  echo "   ✅ Transaction passed (as expected)"
else
  echo "   ⚠️  Unexpected response"
fi
echo ""

# Step 4: Check for escalations (should be none yet)
echo "4️⃣  Checking for escalations..."
ESCALATIONS=$(redis-cli KEYS "escalation:pending:*" 2>/dev/null | wc -l)
echo "   Found $ESCALATIONS pending escalations"
echo ""

# Step 5: Get escalations via API
echo "5️⃣  Listing escalations via API..."
curl -s http://localhost:3001/api/v1/escalations | jq '.' || echo "   No escalations or API endpoint different"
echo ""

# Step 6: Summary
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "📊 TEST SUMMARY"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "Services Status:"
echo "  ✅ All services running and responding"
echo ""
echo "Escalation Status:"
echo "  Escalations in Redis: $ESCALATIONS"
echo ""
echo "📝 NOTE: To trigger an escalation, you need to:"
echo "   1. Send a transaction that violates a rule (risk > 30)"
echo "   2. The proxy will create an escalation in Redis"
echo "   3. Check: redis-cli KEYS 'escalation:*'"
echo "   4. Approve via: curl -X POST http://localhost:3001/api/v1/escalations/{ID}/approve"
echo ""
echo "View proxy logs: tail -f /tmp/proxy.log"
echo "View API logs: tail -f /tmp/api.log"
echo ""
