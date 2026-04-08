#!/bin/bash
set -e

echo "🔐 Setting up local HTTPS for RPC proxy..."
echo ""

# Create directory for certificates
mkdir -p /tmp/rpc-ssl

# Generate self-signed certificate
echo "📜 Generating self-signed certificate..."
openssl req -x509 -newkey rsa:4096 -keyout /tmp/rpc-ssl/key.pem -out /tmp/rpc-ssl/cert.pem -days 365 -nodes -subj "/CN=192.168.86.37"

echo ""
echo "✅ Certificate generated!"
echo ""

# Check if nginx is installed
if ! command -v nginx &> /dev/null; then
    echo "📦 Installing nginx..."
    sudo apt-get update -qq
    sudo apt-get install -y nginx
fi

# Create nginx config
echo "⚙️  Creating nginx HTTPS proxy configuration..."
sudo tee /etc/nginx/sites-available/rpc-proxy-https > /dev/null << 'EOF'
server {
    listen 8443 ssl;
    server_name 192.168.86.37;

    ssl_certificate /tmp/rpc-ssl/cert.pem;
    ssl_certificate_key /tmp/rpc-ssl/key.pem;

    location / {
        proxy_pass http://localhost:8899;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection 'upgrade';
        proxy_set_header Host $host;
        proxy_cache_bypass $http_upgrade;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
EOF

# Enable the site
sudo ln -sf /etc/nginx/sites-available/rpc-proxy-https /etc/nginx/sites-enabled/

# Test nginx config
echo ""
echo "🧪 Testing nginx configuration..."
sudo nginx -t

# Restart nginx
echo ""
echo "🔄 Starting nginx..."
sudo systemctl restart nginx

echo ""
echo "✅ HTTPS proxy is ready!"
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "📱 Use this URL in Backpack mobile:"
echo "   https://192.168.86.37:8443"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "⚠️  Note: Your phone will show a certificate warning"
echo "   This is normal for self-signed certificates."
echo "   Accept/proceed anyway to continue."
echo ""
