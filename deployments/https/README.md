# HTTPS Reverse Proxy for Parapet

Add HTTPS support to Parapet deployments using Caddy or nginx reverse proxy.

**Why needed:** Wallet extensions (Phantom, Solflare) require HTTPS for RPC connections.

## Quick Start with Caddy (Recommended)

### 1. Edit Caddyfile

Replace `YOUR_DOMAIN` with your actual domain:

```bash
cp Caddyfile Caddyfile.local
nano Caddyfile.local
```

```caddyfile
# Change:
YOUR_DOMAIN {
    reverse_proxy localhost:8899
}

# To:
rpc.yourdomain.com {
    reverse_proxy localhost:8899
}
```

### 2. Start Services

```bash
# Start main Parapet stack first
cd ../proxy-only/docker
docker-compose up -d

# Start Caddy reverse proxy
cd ../../https
docker-compose -f docker-compose.caddy.yml up -d
```

### 3. Access via HTTPS

```
https://rpc.yourdomain.com    → Proxy (port 8899)
https://api.yourdomain.com    → API (port 3001)
https://dashboard.yourdomain.com → Dashboard (port 8080)
```

Caddy automatically handles:
- ✅ Let's Encrypt SSL certificates
- ✅ Automatic HTTPS redirects
- ✅ Certificate renewal

## Alternative: nginx

If you prefer nginx:

```bash
# Edit nginx.conf with your domain
nano nginx.conf

# Start nginx reverse proxy
docker-compose -f docker-compose.nginx.yml up -d
```

**Note:** nginx requires manual SSL certificate setup (use certbot).

## Local Development HTTPS

For local testing without a domain:

```bash
# Use the Python HTTPS proxy (self-signed cert)
./https-proxy.py

# Or generate self-signed cert and configure Caddy
./setup-local-https.sh
```

## Configuration

### Caddyfile Features

- **CORS enabled** for Web3 wallets
- **OPTIONS handling** for preflight requests
- **JSON logging** for monitoring
- **Automatic HTTPS** with Let's Encrypt

### Production Checklist

- [ ] Domain DNS points to your server IP
- [ ] Port 80 and 443 open in firewall
- [ ] Email configured in Caddyfile (for Let's Encrypt)
- [ ] Test HTTPS: `curl https://rpc.yourdomain.com/health`
- [ ] Test wallet connection with Phantom/Solflare

## Troubleshooting

**Let's Encrypt fails:**
- Verify DNS points to server
- Check port 80 is accessible (needed for ACME challenge)
- Check Caddy logs: `docker logs parapet-caddy`

**Wallet can't connect:**
- Verify HTTPS works: `curl https://rpc.yourdomain.com`
- Check CORS headers: `curl -H "Origin: chrome-extension://..." https://rpc.yourdomain.com`
- Test with browser console in wallet extension
