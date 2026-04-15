#!/bin/bash
echo "🧪 Testing Parapet Escalation Flow"
echo "===================================="
echo ""
echo "Prerequisites:"
echo "✓ Redis running on localhost:6379"
echo "✓ API server on port 3001"
echo "✓ Proxy on port 8899"
echo ""

# Test 1: Check Redis
echo "1️⃣  Testing Redis connection..."
redis-cli ping > /dev/null 2>&1 && echo "   ✅ Redis is responding" || { echo "   ❌ Redis not running"; exit 1; }

# Test 2: Check API health
echo "2️⃣  Testing API health..."
curl -s http://localhost:3001/health > /dev/null 2>&1 && echo "   ✅ API is responding" || echo "   ⚠️  API not responding (start it first)"

# Test 3: Check Proxy health  
echo "3️⃣  Testing Proxy health..."
curl -s http://localhost:8899/health > /dev/null 2>&1 && echo "   ✅ Proxy is responding" || echo "   ⚠️  Proxy not responding (start it first)"

# Test 4: Send a transaction that should trigger escalation
echo "4️⃣  Sending test transaction (should trigger escalation)..."
curl -s -X POST http://localhost:8899 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "getLatestBlockhash"
  }' | jq '.'

echo ""
echo "5️⃣  Checking for escalations in Redis..."
redis-cli KEYS "escalation:pending:*" | head -5

echo ""
echo "✅ Test complete!"
echo ""
echo "To approve an escalation:"
echo "  curl -X POST http://localhost:3001/api/v1/escalations/{ID}/approve"
echo ""
echo "To view escalations:"
echo "  curl http://localhost:3001/api/v1/escalations"
