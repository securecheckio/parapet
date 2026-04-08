# Parapet MCP Server

A Model Context Protocol (MCP) server that exposes Parapet's wallet scanning and program analysis capabilities to AI assistants like Claude, Cursor, and other MCP clients.

## Features

- **Wallet Security Scanning**: Comprehensive security analysis of Solana wallets
  - Active threat detection (unlimited delegations, compromised authorities)
  - Historical transaction analysis with deep CPI scanning
  - Risk scoring and threat classification
  - Integration with Helius, OtterSec, and Jupiter for enhanced analysis

- **Program Analysis**: Security and verification checks for Solana programs
  - On-chain data verification
  - OtterSec verification status
  - Helius identity and reputation checks
  - Explorer links for manual review

## Installation

### Build from Source

```bash
cd parapet/mcp-server
cargo build --release
```

The binary will be available at `target/release/parapet-mcp`.

### Add to MCP Client

#### Cursor

Add to your Cursor settings (`~/.cursor/config.json`):

```json
{
  "mcpServers": {
    "parapet": {
      "command": "/path/to/parapet-mcp",
      "env": {
        "SOLANA_RPC_URL": "https://api.mainnet-beta.solana.com",
        "HELIUS_API_KEY": "your-helius-api-key",
        "RUST_LOG": "info"
      }
    }
  }
}
```

#### Claude Desktop

Add to `~/Library/Application Support/Claude/claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "parapet": {
      "command": "/path/to/parapet-mcp",
      "env": {
        "SOLANA_RPC_URL": "https://api.mainnet-beta.solana.com",
        "HELIUS_API_KEY": "your-helius-api-key"
      }
    }
  }
}
```

## Environment Variables

- `SOLANA_RPC_URL`: Solana RPC endpoint (default: `https://api.mainnet-beta.solana.com`)
- `HELIUS_API_KEY`: Optional, enables Helius identity checks
- `RULES_PATH`: Optional, path to custom rules file
- `RUST_LOG`: Log level (default: `info`)

## Available Tools

### 1. scan_wallet

Scan a Solana wallet for security threats and suspicious activity.

**Parameters:**
- `wallet_address` (required): Solana wallet address to scan
- `rpc_url` (optional): Custom RPC URL
- `max_transactions` (optional): Maximum transactions to analyze (default: 100)
- `time_window_days` (optional): Days to scan back (default: 30)
- `format` (optional): Output format - `summary`, `detailed`, or `json` (default: `summary`)

**Example:**
```
scan_wallet({
  "wallet_address": "7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU",
  "max_transactions": 50,
  "time_window_days": 7,
  "format": "summary"
})
```

**Output:**
Returns a formatted security report including:
- Security score (0-100)
- Risk level assessment
- Detected threats with severity levels
- Suspicious programs encountered
- Actionable recommendations

### 2. analyze_program

Analyze a Solana program for security and verification status.

**Parameters:**
- `program_id` (required): Solana program ID to analyze
- `rpc_url` (optional): Custom RPC URL
- `network` (optional): Network name - `mainnet-beta`, `devnet`, or `testnet` (default: `mainnet-beta`)

**Example:**
```
analyze_program({
  "program_id": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",
  "network": "mainnet-beta"
})
```

**Output:**
Returns analysis including:
- On-chain account data
- OtterSec verification status
- Helius identity information (if API key provided)
- Explorer links for manual review

## Usage Examples

### In Cursor

Once configured, you can ask Cursor:

```
"Scan my Solana wallet 7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU for security issues"

"Analyze the security of Solana program TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
```

### Programmatic Usage

The MCP server communicates via JSON-RPC over stdio:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "scan_wallet",
    "arguments": {
      "wallet_address": "7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU",
      "format": "json"
    }
  }
}
```

## Architecture

The MCP server is built directly on top of the `parapet-scanner` library, providing:

- **Native Rust Performance**: No subprocess spawning or serialization overhead
- **Direct Library Integration**: Uses the same analyzers and rule engine as the CLI tools
- **Comprehensive Analysis**: Full access to all Parapet features including:
  - Core security analyzers (basic, token, system program, complexity)
  - Deep CPI (Cross-Program Invocation) scanning
  - Third-party integrations (Helius, OtterSec, Jupiter)
  - Custom rule engine with preset rulesets

## Security Considerations

- **RPC Endpoints**: Use trusted RPC endpoints. Public endpoints may be rate-limited.
- **API Keys**: Keep your Helius API key secure. Only set it in trusted environments.
- **Wallet Privacy**: Wallet addresses and transaction data are sent to the configured RPC endpoint.
- **Third-Party Services**: The server may contact OtterSec and Helius APIs for verification and identity checks.

## Troubleshooting

### Server not starting

Check logs (written to stderr):
```bash
RUST_LOG=debug parapet-mcp
```

### Rate limiting

If you encounter rate limiting with public RPC endpoints:
1. Use a private RPC provider (Helius, QuickNode, etc.)
2. Reduce `max_transactions` parameter
3. Increase delays by setting `rpc_delay_ms` in the code

### Missing API features

Ensure all required features are enabled in `Cargo.toml`:
```toml
parapet-core = { 
    path = "../core", 
    features = ["helius", "ottersec", "jupiter", "program-analysis"] 
}
```

## Development

### Running Tests

```bash
cargo test
```

### Building for Release

```bash
cargo build --release --bin parapet-mcp
```

### Debugging MCP Protocol

Set `RUST_LOG=debug` to see all JSON-RPC messages:

```bash
RUST_LOG=debug parapet-mcp
```

## License

Apache-2.0

## Related Projects

- [Parapet Core](../core) - Rule engine and analyzers
- [Parapet Scanner](../scanner) - Wallet and program scanning library
- [Parapet Proxy](../proxy) - RPC proxy with transaction filtering
- [Parapet API](../api) - REST API and dashboard
