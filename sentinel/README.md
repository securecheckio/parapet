# Sentinel - Your Guardian Against Phishing

A guardian agent that throws itself on the grenade for you. Sentinel visits suspicious sites with a throwaway wallet, triggers malicious transactions, and analyzes them so you don't have to risk your real funds.

## Features

- **Throwaway wallet generation** - Fresh keypair per run, zero risk
- **Two-phase navigation** - Opportunistic pattern matching + LLM fallback
- **Transaction interception** - Captures unsigned transactions before signing
- **Parapet integration** - Automatic rule engine and simulation analysis
- **JSON output** - Structured reports ready for DB ingestion

## Usage

### Docker (Recommended)

```bash
# Build the image
docker build -t securecheck/sentinel .

# Run against a URL (opportunistic mode, no LLM)
docker run --rm \
  -e SOL_SHIELD_RPC_URL=http://host.docker.internal:8899 \
  securecheck/sentinel https://example.com/airdrop

# Run with LLM fallback
docker run --rm \
  -e SOL_SHIELD_RPC_URL=http://host.docker.internal:8899 \
  -e LLM_API_KEY=sk-... \
  -e LLM_MODEL=gpt-4o \
  securecheck/sentinel https://example.com/airdrop | jq .
```

### Docker Compose

```bash
# Set environment variables
export SOL_SHIELD_RPC_URL=http://host.docker.internal:8899
export LLM_API_KEY=sk-...

# Run
docker-compose run --rm sentinel https://example.com/airdrop
```

### Local Development

```bash
npm install
npm run build

# Run
SOL_SHIELD_RPC_URL=http://localhost:8899 node dist/index.js https://example.com/airdrop
```

## Environment Variables

| Variable | Required | Description |
|----------|----------|-------------|
| `SOL_SHIELD_RPC_URL` | Yes | Parapet proxy RPC endpoint |
| `LLM_API_KEY` | No | API key for LLM (OpenAI, Groq, etc.) |
| `LLM_BASE_URL` | No | Custom LLM endpoint (for Ollama, Groq, etc.) |
| `LLM_MODEL` | No | Model name (default: gpt-4o) |
| `NAVIGATION_TIMEOUT` | No | Max time per navigation step in ms (default: 30000) |
| `MAX_STEPS` | No | Max navigation steps (default: 10) |

## Output Format

```json
{
  "url": "https://example.com/airdrop",
  "scanned_at": "2026-04-06T20:00:00Z",
  "throwaway_wallet": "Ek9xR...",
  "transaction_captured": true,
  "programs_invoked": [
    {
      "address": "11111111111111111111111111111111",
      "known": true,
      "name": "System Program"
    }
  ],
  "rules_matched": [
    {
      "id": "drainer-set-authority",
      "action": "block",
      "message": "..."
    }
  ],
  "risk_level": "critical",
  "verdict": "MALICIOUS"
}
```

## How Sentinel Protects You

1. **Sacrifices itself** - Generates a throwaway wallet (zero balance, discarded after)
2. **Takes the hit** - Visits the suspicious site in a sandboxed browser
3. **Triggers the trap** - Navigates through the site to trigger malicious transactions
4. **Captures the attack** - Intercepts the unsigned transaction before it can do harm
5. **Analyzes the threat** - Sends it through Parapet's rule engine
6. **Reports back** - Tells you exactly what the site tried to do

**You stay safe. Sentinel takes the risk.**

## License

MIT
