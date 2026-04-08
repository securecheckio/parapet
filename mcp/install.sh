#!/bin/bash
# Install Parapet MCP Server

set -e

echo "Building Parapet MCP Server..."
cargo build --release --bin parapet-mcp

BINARY_PATH="$(pwd)/target/release/parapet-mcp"

echo ""
echo "✅ Build complete!"
echo ""
echo "Binary location: $BINARY_PATH"
echo ""
echo "To use with Cursor, add this to your settings:"
echo ""
echo '{
  "mcpServers": {
    "parapet": {
      "command": "'$BINARY_PATH'",
      "env": {
        "SOLANA_RPC_URL": "https://api.mainnet-beta.solana.com",
        "HELIUS_API_KEY": "your-api-key-here",
        "RUST_LOG": "info"
      }
    }
  }
}'
echo ""
echo "For Claude Desktop, add the above to:"
echo "  macOS: ~/Library/Application Support/Claude/claude_desktop_config.json"
echo "  Linux: ~/.config/Claude/claude_desktop_config.json"
echo ""
