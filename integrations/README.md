# Parapet Integrations

Integration tools and examples for connecting Parapet to external systems.

## Available Integrations

### [agent-kit/](./agent-kit/)
Tools for AI agents and autonomous systems:
- MCP (Model Context Protocol) integration
- Programmatic API access
- Async transaction monitoring
- Risk assessment helpers

## Planned Integrations

### Wallet Providers
- Phantom SDK integration
- Backpack API examples
- Solflare adapter

### RPC Providers
- Helius enriched transactions
- QuickNode integration
- Triton examples

### Security Services
- OtterSec integration
- Rugcheck API adapter
- Security.txt discovery

### Monitoring & Alerting
- Telegram bot examples
- Slack notifications
- Email alerts

## Integration Patterns

### 1. Client-Side (Wallet/DApp)
Integrate Parapet directly into your application:

```rust
use parapet_core::RuleEngine;

let engine = RuleEngine::new()?;
let result = engine.evaluate(&transaction)?;

if result.risk_score > threshold {
    warn_user(&result);
}
```

### 2. Proxy Mode (RPC Layer)
Route RPC traffic through Parapet proxy:

```typescript
// Change RPC endpoint
const connection = new Connection(
  'http://localhost:8899',  // Parapet proxy
  'confirmed'
);
```

### 3. API Mode (Backend Service)
Query Parapet API from your backend:

```bash
curl -X POST http://localhost:3001/api/v1/transactions/analyze \
  -H "X-API-Key: your-key" \
  -d '{"transaction": "base64..."}'
```

## Creating New Integrations

1. **Identify Use Case**: What are you connecting to Parapet?
2. **Choose Pattern**: Client-side, proxy, or API?
3. **Review Examples**: See existing integrations in this directory
4. **Build & Test**: Create your integration with tests
5. **Document**: Add README with usage examples
6. **Share**: Submit PR to contribute back

## Integration Requirements

- **Authentication**: Use API keys or wallet signatures
- **Error Handling**: Handle network failures gracefully
- **Rate Limiting**: Respect API rate limits
- **Caching**: Cache results when possible
- **Monitoring**: Log integration health metrics

## Support

For integration help:
- Check [docs/](../docs/) for API documentation
- Open an issue for integration requests
- Join our community for support

## Contributing

We welcome new integrations! See [../CONTRIBUTING.md](../CONTRIBUTING.md) for guidelines.
