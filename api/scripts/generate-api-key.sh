#!/bin/bash
# Generate a secure API key for MCP access

set -e

# Generate a secure random API key (32 bytes = 64 hex characters)
API_KEY=$(openssl rand -hex 32)

echo "===================================="
echo "   Parapet MCP API Key"
echo "===================================="
echo ""
echo "Generated API Key:"
echo "$API_KEY"
echo ""
echo "Add this to your environment variables:"
echo ""
echo "# For Docker Compose:"
echo "MCP_API_KEYS=$API_KEY"
echo ""
echo "# For Terraform (add to variables):"
echo "mcp_api_keys = \"$API_KEY\""
echo ""
echo "# For local development:"
echo "export MCP_API_KEYS=\"$API_KEY\""
echo ""
echo "Add to your MCP client config:"
echo ""
echo '{
  "mcpServers": {
    "parapet": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/create-server", "http", "https://your-api-url.com/mcp"],
      "env": {
        "MCP_API_KEY": "'$API_KEY'"
      }
    }
  }
}'
echo ""
echo "⚠️  Keep this key secret! Anyone with this key can use your MCP server."
echo "===================================="
