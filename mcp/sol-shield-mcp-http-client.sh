#!/bin/bash
# Sol-Shield MCP HTTP Client Wrapper
# Makes the config cleaner by hiding the npx complexity

set -e

# Configuration
MCP_SERVER_URL="${MCP_SERVER_URL:-https://api.solshield.com/mcp}"
API_KEY="${SOL_SHIELD_API_KEY}"

if [ -z "$API_KEY" ]; then
    echo "Error: SOL_SHIELD_API_KEY environment variable not set" >&2
    exit 1
fi

# Run the MCP HTTP adapter with Authorization header
export AUTHORIZATION="Bearer $API_KEY"

exec npx -y @modelcontextprotocol/create-server http "$MCP_SERVER_URL"
