#!/bin/bash
# Start Caddy with local self-signed certs for testing

set -e

# Get local IP
LOCAL_IP=$(hostname -I | awk '{print $1}')
echo "🔍 Detected local IP: $LOCAL_IP"

# Update Caddyfile with actual IP
sed "s/10\.0\.0\.84/$LOCAL_IP/g" Caddyfile.local > Caddyfile.generated

echo "📋 Generated Caddyfile for $LOCAL_IP"
echo ""
echo "🚀 Starting Caddy with self-signed certs..."
echo ""

# Install Caddy if not present
if ! command -v caddy &> /dev/null; then
    echo "❌ Caddy not found. Installing..."
    echo ""
    echo "Run one of these commands:"
    echo ""
    echo "  Ubuntu/Debian:"
    echo "    sudo apt install -y debian-keyring debian-archive-keyring apt-transport-https curl"
    echo "    curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/gpg.key' | sudo gpg --dearmor -o /usr/share/keyrings/caddy-stable-archive-keyring.gpg"
    echo "    curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/debian.deb.txt' | sudo tee /etc/apt/sources.list.d/caddy-stable.list"
    echo "    sudo apt update && sudo apt install caddy"
    echo ""
    echo "  Docker:"
    echo "    docker-compose up -d"
    echo ""
    exit 1
fi

# Create logs directory
mkdir -p logs

# Start Caddy
sudo caddy run --config Caddyfile.generated --adapter caddyfile

echo ""
echo "✅ Caddy started with HTTPS!"
echo ""
echo "📱 Use these URLs in Backpack mobile:"
echo "   RPC:       https://$LOCAL_IP:8443"
echo "   API:       https://$LOCAL_IP:3443"
echo "   Dashboard: https://$LOCAL_IP:8443/dashboard"
echo ""
echo "⚠️  Your browser will warn about self-signed cert - this is expected!"
echo "    On mobile: Trust the certificate when prompted"
echo ""
