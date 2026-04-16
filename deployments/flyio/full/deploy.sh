#!/bin/bash
# Deploy complete Parapet stack to Fly.io
set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}=== Parapet Full Stack Fly.io Deployment ===${NC}"
echo ""

# Change to deployment directory
cd "$(dirname "$0")"
DEPLOY_DIR=$(pwd)

# Check if fly CLI is installed
if ! command -v fly &> /dev/null; then
    echo -e "${RED}Error: fly CLI not found${NC}"
    echo "Install it from: https://fly.io/install"
    exit 1
fi

# Check if logged in
if ! fly auth whoami &> /dev/null; then
    echo -e "${RED}Error: Not logged into Fly.io${NC}"
    echo "Run: fly auth login"
    exit 1
fi

# Load configuration if .env exists
if [ -f "$DEPLOY_DIR/.env" ]; then
    echo -e "${GREEN}Loading configuration from .env${NC}"
    source "$DEPLOY_DIR/.env"
else
    echo -e "${YELLOW}Warning: .env not found${NC}"
    echo "Using default configuration. Copy .env.example to .env to customize."
fi

# Set default app names
FLY_PROXY_APP=${FLY_PROXY_APP:-parapet-proxy}
FLY_API_APP=${FLY_API_APP:-parapet-api}
FLY_DASHBOARD_APP=${FLY_DASHBOARD_APP:-parapet-dashboard}

echo ""
echo "Deploying full stack:"
echo "  - Proxy:     $FLY_PROXY_APP"
echo "  - API:       $FLY_API_APP"
echo "  - Dashboard: $FLY_DASHBOARD_APP"
echo "  - Redis:     parapet-redis"
echo ""

# Change to parapet root
cd "$DEPLOY_DIR/../../../.."
PARAPET_ROOT=$(pwd)

# ===========================================
# Step 1: Create Redis
# ===========================================
echo -e "${GREEN}Step 1: Setting up Redis${NC}"

if fly redis list | grep -q "parapet-redis"; then
    echo "Redis 'parapet-redis' already exists"
else
    echo "Creating Redis..."
    fly redis create --name parapet-redis --region iad
fi

echo ""

# ===========================================
# Step 2: Deploy API
# ===========================================
echo -e "${GREEN}Step 2: Deploying API${NC}"

cd "$PARAPET_ROOT"

if fly apps list | grep -q "$FLY_API_APP"; then
    echo "API app exists, deploying update..."
    fly deploy --config "$DEPLOY_DIR/fly.api.toml" \
               --dockerfile "$DEPLOY_DIR/Dockerfile.api" \
               --app "$FLY_API_APP"
else
    echo "Creating API app..."
    fly launch --config "$DEPLOY_DIR/fly.api.toml" \
               --dockerfile "$DEPLOY_DIR/Dockerfile.api" \
               --no-deploy \
               --name "$FLY_API_APP"
    
    fly redis connect parapet-redis -a "$FLY_API_APP"
    fly deploy --config "$DEPLOY_DIR/fly.api.toml" \
               --dockerfile "$DEPLOY_DIR/Dockerfile.api" \
               --app "$FLY_API_APP"
fi

# Configure API secrets
if [ -n "$AUTHORIZED_WALLETS" ]; then
    echo "Setting AUTHORIZED_WALLETS..."
    fly secrets set AUTHORIZED_WALLETS="$AUTHORIZED_WALLETS" -a "$FLY_API_APP"
fi

if [ -n "$MCP_API_KEYS" ]; then
    echo "Setting MCP_API_KEYS..."
    fly secrets set MCP_API_KEYS="$MCP_API_KEYS" -a "$FLY_API_APP"
fi

if [ -n "$HELIUS_API_KEY" ]; then
    fly secrets set HELIUS_API_KEY="$HELIUS_API_KEY" -a "$FLY_API_APP"
fi

if [ -n "$JUPITER_API_KEY" ]; then
    fly secrets set JUPITER_API_KEY="$JUPITER_API_KEY" -a "$FLY_API_APP"
fi

if [ -n "$OTTERSEC_API_KEY" ]; then
    fly secrets set OTTERSEC_API_KEY="$OTTERSEC_API_KEY" -a "$FLY_API_APP"
fi

echo ""

# ===========================================
# Step 3: Deploy Proxy
# ===========================================
echo -e "${GREEN}Step 3: Deploying Proxy${NC}"

if fly apps list | grep -q "$FLY_PROXY_APP"; then
    echo "Proxy app exists, deploying update..."
    fly deploy --config "$DEPLOY_DIR/fly.proxy.toml" \
               --dockerfile "$DEPLOY_DIR/Dockerfile.proxy" \
               --app "$FLY_PROXY_APP"
else
    echo "Creating Proxy app..."
    fly launch --config "$DEPLOY_DIR/fly.proxy.toml" \
               --dockerfile "$DEPLOY_DIR/Dockerfile.proxy" \
               --no-deploy \
               --name "$FLY_PROXY_APP"
    
    fly redis connect parapet-redis -a "$FLY_PROXY_APP"
    fly deploy --config "$DEPLOY_DIR/fly.proxy.toml" \
               --dockerfile "$DEPLOY_DIR/Dockerfile.proxy" \
               --app "$FLY_PROXY_APP"
fi

# Configure proxy secrets
if [ -n "$HELIUS_API_KEY" ]; then
    fly secrets set HELIUS_API_KEY="$HELIUS_API_KEY" -a "$FLY_PROXY_APP"
fi

if [ -n "$JUPITER_API_KEY" ]; then
    fly secrets set JUPITER_API_KEY="$JUPITER_API_KEY" -a "$FLY_PROXY_APP"
fi

if [ -n "$OTTERSEC_API_KEY" ]; then
    fly secrets set OTTERSEC_API_KEY="$OTTERSEC_API_KEY" -a "$FLY_PROXY_APP"
fi

echo ""



# ===========================================
# Step 5: Verify deployment
# ===========================================
echo -e "${GREEN}Step 5: Verifying deployment${NC}"

echo "Waiting for services to start..."
sleep 10

PROXY_URL=$(fly info -a "$FLY_PROXY_APP" -j | jq -r '.Hostname')
API_URL=$(fly info -a "$FLY_API_APP" -j | jq -r '.Hostname')
DASHBOARD_URL=$(fly info -a "$FLY_DASHBOARD_APP" -j | jq -r '.Hostname')

echo -n "Checking proxy... "
if curl -sf "https://$PROXY_URL/health" > /dev/null; then
    echo -e "${GREEN}OK${NC}"
else
    echo -e "${RED}FAILED${NC}"
fi

echo -n "Checking API... "
if curl -sf "https://$API_URL/health" > /dev/null; then
    echo -e "${GREEN}OK${NC}"
else
    echo -e "${RED}FAILED${NC}"
fi

echo -n "Checking dashboard... "
if curl -sf "https://$DASHBOARD_URL/" > /dev/null; then
    echo -e "${GREEN}OK${NC}"
else
    echo -e "${RED}FAILED${NC}"
fi

echo ""
echo -e "${GREEN}=== Full Stack Deployed ===${NC}"
echo ""
echo "  Proxy:     https://$PROXY_URL"
echo "  API:       https://$API_URL"
echo "  Dashboard: https://$DASHBOARD_URL"
echo ""
echo "Next steps:"
echo "  1. Open dashboard: https://$DASHBOARD_URL"
echo "  2. Test proxy: curl https://$PROXY_URL/health"
echo "  3. View logs: fly logs -a $FLY_PROXY_APP"
echo "  4. Use proxy: Point your Solana client to https://$PROXY_URL"
echo ""
echo "Documentation: $DEPLOY_DIR/README.md"
