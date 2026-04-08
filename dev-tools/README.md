# Development Tools

Utility scripts for local development and testing.

## Local HTTPS Testing

Two options for wrapping the HTTP RPC proxy with HTTPS for mobile wallet testing:

### Option 1: https-proxy.py (Lightweight)

Python-based HTTPS proxy - no system dependencies.

**Usage:**
```bash
# Start local RPC proxy first
cd ../parapet-rpc-proxy
cargo run

# In another terminal, start HTTPS proxy
cd ../dev-tools
./https-proxy.py

# Configure mobile wallet to use:
# https://YOUR_LOCAL_IP:9443
```

**Requirements:**
- Python 3.6+
- OpenSSL (for certificate generation)
- Local RPC proxy running on port 8899

**Pros:**
- No system installation required
- Easy to start/stop
- Request logging built-in

### Option 2: setup-local-https.sh (Production-like)

Nginx-based HTTPS proxy - more robust, production-like setup.

**Usage:**
```bash
# One-time setup (installs nginx, configures SSL)
cd dev-tools
./setup-local-https.sh

# Configure mobile wallet to use:
# https://YOUR_LOCAL_IP:8443

# To stop:
sudo systemctl stop nginx
```

**Requirements:**
- Linux system with systemd
- sudo access
- Local RPC proxy running on port 8899

**Pros:**
- Production-like environment
- Better performance
- Runs as system service

## Which to Use?

- **Quick testing**: Use `https-proxy.py` (faster setup, easier cleanup)
- **Extended development**: Use `setup-local-https.sh` (more stable, runs in background)
