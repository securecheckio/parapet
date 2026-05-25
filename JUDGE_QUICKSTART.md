# Quick Demo for Judges (2 Minutes)

## Start Parapet (30 seconds)

```bash
docker run -d -p 8899:8899 \
  -e UPSTREAM_RPC_URL=https://api.mainnet-beta.solana.com \
  -e RULES_FEED_URLS=https://parapet-rules.securecheck.io/community/demo-test.json \
  ghcr.io/securecheckio/parapet:latest
```

Wait 5 seconds, then verify it's running:

```bash
curl -X POST http://localhost:8899 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}'
```

Should return: `{"jsonrpc":"2.0","result":"ok","id":1}`

---

## Test It (1 minute)

### Option 1: With Backpack Wallet

1. Open Backpack wallet
2. **Settings → Developer Settings → RPC URL**
3. Enter: `http://localhost:8899`
4. Try to send **0.01 SOL** → ✅ **Passes** (below threshold)
5. Try to send **0.2 SOL** → 🛑 **BLOCKED** with message: "Transaction transfers more than 0.1 SOL"

### Option 2: Command Line (Easiest)

First, create a test transaction (this won't actually execute):

```bash
# Use Solana CLI to generate a test transfer
solana transfer --allow-unfunded-recipient \
  --url http://localhost:8899 \
  --simulate-tx-only \
  Gv9v1Q8pZzLXxcvvhq6YxqQxQ7QxqQxQ7QxqQxQ7Qxq1 \
  0.05
# Should pass (below 0.1 SOL threshold)

solana transfer --allow-unfunded-recipient \
  --url http://localhost:8899 \
  --simulate-tx-only \
  Gv9v1Q8pZzLXxcvvhq6YxqQxQ7QxqQxQ7QxqQxQ7Qxq1 \
  0.5
# Should be BLOCKED (above 0.1 SOL threshold)
```

### Option 3: Watch Docker Logs

```bash
docker logs -f $(docker ps | grep parapet | awk '{print $1}')
```

You'll see:
- ✅ Rules loaded successfully
- 📊 Transaction analysis in real-time
- 🛑 Blocks when rules trigger
- ⚠️ Alerts on detections

---

## What Just Happened?

**Parapet acts as a security layer between your wallet and Solana:**

```
Wallet → Parapet (localhost:8899) → Solana Mainnet
           ↓
       Analyzes before signing
       Blocks/Alerts based on rules
```

**Demo rules active:**
- ✅ **Blocks** SOL transfers >0.1 SOL
- ✅ **Alerts** on any SOL transfer (shows detection)

**Production rules available:**
- Unlimited token approvals (common drain attack)
- Freeze + Approval combos
- 64 known phishing accounts ($1.1M losses)
- Research-backed patterns from academic papers
- And more: https://parapet-rules.securecheck.io/

---

## Stop Demo

```bash
docker stop $(docker ps | grep parapet | awk '{print $1}')
```

---

## Key Points for Judging

1. **Drop-in protection** - No code changes, just point wallet to localhost:8899
2. **Real-time analysis** - Catches attacks before they happen
3. **Composable rules** - Mix multiple rule feeds (we just used demo rules)
4. **Open & auditable** - All rules are JSON, fully visible
5. **Research-backed** - SolPhish rules based on 8,058 analyzed attacks

**The 0.1 SOL threshold is just for demo safety. Production uses sophisticated patterns:**
- Token delegation analysis
- Authority transfer detection
- Multi-signature pattern matching
- Known malicious address blocking
- CPI depth and instruction padding checks

---

## Need More?

- **Full rule catalog**: https://parapet-rules.securecheck.io/
- **GitHub**: https://github.com/securecheckio/parapet
- **Research paper**: https://arxiv.org/abs/2505.04094 (SolPhish detection)

**Total time**: 2 minutes to see transaction protection in action! 🛡️
