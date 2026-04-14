#!/bin/bash

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "🚀 STARTING COMPLETE PARAPET STACK"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Check Redis
echo "1️⃣  Checking Redis..."
redis-cli ping > /dev/null 2>&1 || { echo "❌ Redis not running. Start with: docker run -d -p 6379:6379 redis:7-alpine"; exit 1; }
echo "   ✅ Redis running"

# Start API
echo "2️⃣  Starting API Server..."
cd api-core
../target/release/parapet-api-core > /tmp/parapet-api.log 2>&1 &
API_PID=$!
echo "   Started API (PID: $API_PID)"
cd ..
sleep 2

# Start Proxy
echo "3️⃣  Starting Proxy..."
cd proxy
ESCALATION_APPROVER_WALLET=vines1vzrYbzLMRdu58ou5XTby4qAqVRLmqo36NKPTg ../target/release/parapet-proxy > /tmp/parapet-proxy.log 2>&1 &
PROXY_PID=$!
echo "   Started Proxy (PID: $PROXY_PID)"
cd ..
sleep 3

# Start Dashboard
echo "4️⃣  Starting Dashboard..."
cd dashboard
python3 -m http.server 8080 > /tmp/parapet-dashboard.log 2>&1 &
DASHBOARD_PID=$!
echo "   Started Dashboard (PID: $DASHBOARD_PID)"
cd ..
sleep 2

# Test services
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "🧪 TESTING SERVICES"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

curl -s http://localhost:3001/health > /dev/null 2>&1 && echo "✅ API:       http://localhost:3001" || echo "❌ API failed"
curl -s http://localhost:8899/health > /dev/null 2>&1 && echo "✅ Proxy:     http://localhost:8899" || echo "❌ Proxy failed"
curl -s http://localhost:8080 > /dev/null 2>&1 && echo "✅ Dashboard: http://localhost:8080" || echo "❌ Dashboard failed"

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "✅ STACK READY!"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "🌐 Open in browser: http://localhost:8080"
echo ""
echo "📊 Test escalation flow:"
echo "   ./ESCALATION_DEMO.sh"
echo ""
echo "🔍 View logs:"
echo "   tail -f /tmp/parapet-api.log"
echo "   tail -f /tmp/parapet-proxy.log"
echo "   tail -f /tmp/parapet-dashboard.log"
echo ""
echo "🛑 Stop all services:"
echo "   pkill -f parapet-api-core"
echo "   pkill -f parapet-proxy"
echo "   pkill -f 'http.server 8080'"
echo ""
