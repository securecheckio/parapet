# Drift Attack Demo Monitor

Interactive demonstration showing how Parapet Security could have prevented the $285M Drift Protocol attack through pre-signing transaction analysis.

## Overview

This tool provides a visual, stage-by-stage replay of the Drift Protocol attack that occurred on April 1, 2026. It demonstrates the critical difference between **blind signing** (industry standard) and **pre-signing analysis** (Parapet's approach).

### Key Message: Don't Blind Sign - Decode First

The demo showcases three critical stages showing the difference between hardware wallet blind signing and Parapet's transaction decoding:

1. **March 23-30: Pre-Signing Detection** - RPC proxy decodes "Unknown Instruction" → reveals hidden admin transfer. **PREVENTION OPPORTUNITY**
2. **April 1, 16:05:18: On-Chain Execution** - Same transaction from Stage 1, now broadcast. Monitoring detects it but too late to block (already signed)
3. **April 1, 16:05:19: Coordinated Attack** - Second pre-signed transaction 1 second later. Monitoring confirms coordinated exploit ($285M drained)

## Quick Start

### 1. Start the Demo Monitor

```bash
cd parapet/tools/demo-monitor
cargo run
```

The server will start on `http://localhost:3030`

### 2. (Optional) Start Parapet RPC Proxy

For live RPC integration:

```bash
# In another terminal
cd parapet/rpc-proxy
RULES_PATH=../rules/examples/drift-attack-detection.json cargo run
```

### 3. Open the Demo

Visit `http://localhost:3030` in your browser.

## Architecture

```
┌─────────────┐
│  Frontend   │  User clicks stage buttons
│   (React)   │
└──────┬──────┘
       │ POST /api/stage/fire
       ▼
┌─────────────┐
│  Demo API   │  Orchestrates simulation
│   (Axum)    │
└──────┬──────┘
       │
       ├──> Baseline Result (mocked - no protection)
       │
       └──> Parapet Analysis
            ├──> RPC Proxy (simulateTransaction)
            └──> Rule Engine (drift-attack-detection.json)
```

## Demo Flow

### Stage 1: Pre-Signing Detection (March 23-30)

- **Baseline**: Security Council member signs blindly on Ledger
  - Ledger shows: `AdvanceNonceAccount` + `Unknown Instruction`
  - Told: "Air-gapped signing for security" + "Routine multisig upgrade"
  - Signs without seeing the full transaction
- **Parapet**: CRITICAL RPC Alert during simulateTransaction
  - Decodes "Unknown Instruction" → Admin transfer to H7PiGqq...
  - **CRITICAL**: H7PiGqq... has ZERO prior interactions with Drift
    - Not a current multisig member
    - Not a historical governance address
    - Completely unknown address
  - Shows: "Pre-signed admin transfer to UNKNOWN ADDRESS"
  - Risk Score 95/100
- **Impact**: **PREVENTION** - Member can reject before signing (stops attack entirely)
- **Mode**: RPC Proxy intercepts simulateTransaction call
- **Talking Points**:
  - Hardware wallets only show instruction types, not decoded content
  - Social engineering: "Air-gapped signing" sounds like security best practice
  - **The critical insight**: Expected "routine upgrade" but target is unknown
  - Parapet reveals: "You're transferring admin to an address with no protocol history"
  - This is where the attack could have been stopped

### Stage 2: On-Chain Execution (April 1, 16:05:18)

- **Context**: SAME transaction from Stage 1 - signed in March, broadcast in April
- **Baseline**: Transaction executes on-chain undetected
- **Parapet**: Monitoring Alert - "Pre-signed admin transfer executing"
- **Impact**: Too late to prevent, but enables incident response
- **Mode**: On-chain monitoring (Geyser/Helius)
- **Talking Points**:
  - Already signed weeks ago (can't block)
  - Monitoring detects durable nonce + admin transfer pattern
  - Risk Score 95/100 - Unknown address receiving admin rights
  - Triggers: Emergency pause, security team alert, incident response

### Stage 3: Coordinated Attack (April 1, 16:05:19)

- **Baseline**: Second pre-signed transaction executes 1 second later
- **Parapet**: CRITICAL Alert - "Coordinated attack: Sequential admin actions"
- **Impact**: $285M total, full exploit confirmed
- **Mode**: On-chain monitoring (Geyser/Helius)
- **Talking Points**:
  - Only 1 second between transactions (pre-coordinated)
  - Multiple admin actions = confirmed exploit
  - Risk Score 100/100 - Full protocol compromise
  - Detection enables emergency response (but damage already done)

## Visual Impact

The demo prominently displays:

```
WITHOUT PARAPET                    |  WITH PARAPET
Ledger: "Unknown Instruction"      |  "Admin transfer to unknown address"
Blindly signed ✓                   |  ALERT - Reject transaction
$285,000,000 LOST                  |  $285,000,000 PROTECTED
```

**The Critical Difference:** Hardware wallets show instruction types. Parapet decodes what they actually DO.

## Configuration

### Environment Variables

- `PARAPET_RPC_URL` - URL of Parapet RPC Proxy (default: `http://localhost:8899`)
- `RUST_LOG` - Log level (default: `info`)

### Files

- `src/main.rs` - HTTP server and API endpoints
- `src/stages.rs` - Stage definitions with real transaction signatures
- `src/rpc_client.rs` - RPC proxy integration
- `src/monitor.rs` - Baseline and alert generation
- `frontend/` - HTML/CSS/JS demo interface

## API Endpoints

### GET /api/stages

Returns all demo stages

**Response:**

```json
[
  {
    "id": 1,
    "name": "March 2026: Nonce Account Creation",
    "date": "March 24, 2026",
    "description": "...",
    "tx_signature": "...",
    "expected_result": "alert",
    "parapet_rules": ["alert-durable-nonce-creation"],
    "talking_points": [...]
  }
]
```

### POST /api/stage/fire

Fire a demo stage event

**Request:**

```json
{
  "stage_id": 1
}
```

**Response:**

```json
{
  "stage": { ... },
  "baseline_result": {
    "status": "Signed ✓",
    "loss_amount": 0
  },
  "parapet_result": {
    "action": "alert",
    "risk_score": 45,
    "rules_triggered": ["alert-durable-nonce-creation"],
    "message": "⚠️ ALERT: You are creating a durable nonce account...",
    "analysis_time_ms": 12.5
  }
}
```

### GET /api/alerts

Get all monitor alerts

**Response:**

```json
[
  {
    "stage_id": 1,
    "alert_type": "nonce_creation_detected",
    "message": "...",
    "risk_score": 45,
    "timestamp": 1714579200
  }
]
```

## Real Transaction Signatures

The demo uses actual Drift attack transactions:

- **TX A (Admin Transfer)**: `2HvMSgDEfKhNryYZKhjowrBY55rUx5MWtcWkG9hqxZCFBaTiahPwfynP1dxBSRk9s5UTVc8LFeS4Btvkm9pc2C4H`
- **TX B (Execution)**: `4BKBmAJn6TdsENij7CsVbyMVLJU1tX27nfrMM1zgKv1bs2KJy6Am2NqdA3nJm4g9C6eC64UAf5sNs974ygB9RsN1`
- **Nonce Account**: `7s7s6saC5LHZoLyBXLM3pCjpWaA7meyQdP8NiH9ktAeC`

## Development

### Build

```bash
cargo build --release
```

### Run Tests

```bash
cargo test
```

### Frontend Development

The frontend is static HTML/CSS/JS served from the `frontend/` directory. Edit files and refresh browser to see changes.

## Future Enhancements

- Add WebSocket real-time alerts
- Integrate with actual Solana RPC for transaction fetching
- Add more attack stages (fund drainage, etc.)
- Export demo results as PDF report
- Add presenter mode with auto-advance

## License

MIT - Part of the Parapet Security project