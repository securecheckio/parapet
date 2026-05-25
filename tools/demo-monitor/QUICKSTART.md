# Drift Attack Demo - Quick Start

**2-minute interactive demo** showing how Parapet could have prevented the $285M Drift attack.

## Get Running in 60 Seconds

## 1. Build & Run

```bash
cd parapet/tools/demo-monitor
./start-demo.sh
```

Or manually:
```bash
cargo run --release
```

## 2. Open Demo

Visit: `http://localhost:3030`

## 3. Click Through Stages

1. **Stage 1**: March pre-signing → Ledger shows "Unknown" - Parapet decodes admin transfer
2. **Stage 2**: April execution → Same tx hits chain (too late to block)
3. **Stage 3**: April +1 second → Second tx completes attack

## Key Points

- **Impact Banner**: $285M lost vs $285M protected
- **Two Columns**: Baseline (left) vs Parapet (right)
- **Console**: Real-time alerts streaming
- **Talking Points**: Scroll down in each result

## The Message

**Don't Blind Sign - Decode First**

Hardware wallets show instruction types ("AdvanceNonceAccount + Unknown").  
Parapet decodes what transactions actually DO ("Admin transfer to unknown address").

Stage 1 was the prevention moment. With Parapet, the attack stops there.

## Demo Script

- **2-minute version**: `PRESENTER_GUIDE_2MIN.md` ⭐ (Use this!)
- **Full version**: `PRESENTER_GUIDE.md` (15-20 min workshop)
- **Technical docs**: `README.md`

---

That's it! Ready to present. 🛡️
