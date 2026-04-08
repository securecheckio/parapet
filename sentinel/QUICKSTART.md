# Sentinel - Quick Start Guide

**Your guardian against phishing. Sentinel throws itself on the grenade so you don't have to.**

## Prerequisites

- Docker installed
- Parapet proxy running (or access to one)
- Optional: OpenAI API key or compatible LLM endpoint

## Build the Docker Image

```bash
cd parapet/sentinel
docker build -t securecheck/sentinel .
```

## Deploy Sentinel Against a Suspicious Site

### Basic Usage (Opportunistic Mode - No LLM)

```bash
docker run --rm \
  -e SOL_SHIELD_RPC_URL=http://host.docker.internal:8899 \
  securecheck/sentinel https://bulktrade.me/coin
```

This will:
1. Generate a throwaway Solana keypair
2. Launch a headless browser
3. Try common wallet connection patterns
4. Capture any transaction the site creates
5. Analyze it via Parapet proxy
6. Output JSON report to stdout

### With LLM Fallback

For sites with unusual UI that don't match common patterns:

```bash
docker run --rm \
  -e SOL_SHIELD_RPC_URL=http://host.docker.internal:8899 \
  -e LLM_API_KEY=sk-... \
  -e LLM_MODEL=gpt-4o \
  securecheck/sentinel https://example.com/airdrop
```

### Using Groq (Fast & Free)

```bash
docker run --rm \
  -e SOL_SHIELD_RPC_URL=http://host.docker.internal:8899 \
  -e LLM_BASE_URL=https://api.groq.com/openai/v1 \
  -e LLM_API_KEY=gsk_... \
  -e LLM_MODEL=llama-3.3-70b-versatile \
  securecheck/phishing-agent https://example.com/airdrop
```

### Using Local Ollama

```bash
docker run --rm \
  -e SOL_SHIELD_RPC_URL=http://host.docker.internal:8899 \
  -e LLM_BASE_URL=http://host.docker.internal:11434/v1 \
  -e LLM_API_KEY=ollama \
  -e LLM_MODEL=llama3.1 \
  securecheck/phishing-agent https://example.com/airdrop
```

## Output Format

### JSON (default)

```json
{
  "url": "https://bulktrade.me/coin",
  "scanned_at": "2026-04-06T20:00:00Z",
  "throwaway_wallet": "Ek9xR...",
  "transaction_captured": true,
  "programs_invoked": [
    {
      "address": "11111111111111111111111111111111",
      "known": true,
      "name": "System Program"
    },
    {
      "address": "Fg6Pa...LnS",
      "known": false
    }
  ],
  "rules_matched": [
    {
      "id": "drainer-set-authority",
      "action": "block",
      "message": "Unknown program requests SetAuthority"
    }
  ],
  "risk_level": "critical",
  "verdict": "MALICIOUS"
}
```

### Human-Readable

```bash
docker run --rm \
  -e SOL_SHIELD_RPC_URL=http://host.docker.internal:8899 \
  securecheck/sentinel https://bulktrade.me/coin --format human
```

## Exit Codes

- `0` - Safe or unknown (no transaction captured)
- `1` - Suspicious
- `2` - Malicious
- `3` - Error during analysis

## Piping to jq

```bash
docker run --rm \
  -e SOL_SHIELD_RPC_URL=http://host.docker.internal:8899 \
  securecheck/sentinel https://bulktrade.me/coin | jq '.verdict'
```

## Using via MCP

If you have the Parapet MCP server running, you can call the tool directly:

```json
{
  "method": "tools/call",
  "params": {
    "name": "analyze_phishing_site",
    "arguments": {
      "url": "https://bulktrade.me/coin"
    }
  }
}
```

## Troubleshooting

### Docker can't reach host.docker.internal

On Linux, use `--add-host=host.docker.internal:host-gateway`:

```bash
docker run --rm \
  --add-host=host.docker.internal:host-gateway \
  -e SOL_SHIELD_RPC_URL=http://host.docker.internal:8899 \
  securecheck/sentinel https://example.com
```

### Browser crashes or hangs

Increase timeout:

```bash
docker run --rm \
  -e SOL_SHIELD_RPC_URL=http://host.docker.internal:8899 \
  -e NAVIGATION_TIMEOUT=60000 \
  securecheck/sentinel https://example.com
```

### No transaction captured

The site might:
- Require specific wallet types (Phantom, Solflare, etc.)
- Use custom wallet adapters
- Detect automation
- Not actually be malicious

Try with LLM fallback enabled for better navigation.
