# Parapet Examples

Example code demonstrating how to integrate with Parapet RPC proxy.

## Agent Integration Examples

### Python Example

**File:** `agent-integration-example.py`

Demonstrates:
- Health checks
- RPC method calls
- Transaction simulation with Parapet analysis
- Safe transaction sending with risk assessment

**Requirements:**
```bash
pip install requests base58
```

**Usage:**
```bash
# Basic test
python agent-integration-example.py --rpc http://localhost:8899

# With API key
python agent-integration-example.py --rpc http://localhost:8899 --api-key sk_test_your_key

# Custom risk threshold
python agent-integration-example.py --rpc http://localhost:8899 --max-risk 70
```

### TypeScript Example

**File:** `agent-integration-example.ts`

Demonstrates:
- Type-safe integration with Parapet
- Complete agent workflow
- Error handling
- Security logging

**Requirements:**
```bash
npm install @solana/web3.js
npm install -g ts-node typescript
```

**Usage:**
```bash
# Run the demo
SOL_SHIELD_RPC_URL=http://localhost:8899 ts-node agent-integration-example.ts
```

### Shell Script

**File:** `../proxy/test-agent-integration.sh`

Quick integration tests using cURL. Tests:
- Health endpoint
- Pass-through RPC methods
- JSON-RPC compatibility

**Usage:**
```bash
cd ../proxy
./test-agent-integration.sh
```

## Complete Integration Guide

See `AI_AGENT_INTEGRATION_GUIDE.md` in the parent directory for comprehensive documentation including:

- Parapet overview and architecture
- Complete RPC method reference
- Risk scoring and decision logic
- Authentication options
- Error codes and handling
- Best practices for agents
- Example workflows
- Testing checklist
