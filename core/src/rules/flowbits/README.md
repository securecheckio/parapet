# Flowbits

Stateful cross-transaction tracking system for detecting multi-stage attacks.

## Overview

Flowbits enable rules to remember context across multiple transactions from the same wallet. Inspired by Snort IDS flowbits, this allows detection of attack patterns that span multiple transactions.

**Example:** Track when a user approves unlimited token delegation (Transaction 1), then alert if tokens are later transferred (Transaction 2).

## How It Works

Rules can set boolean flags, increment counters, or record timestamps that persist across transactions for a given wallet. These values expire after a configurable TTL (default: 5 minutes).

```
TX 1: Unlimited approval → Set flowbit "delegation_approved"
TX 2: Token transfer → Check if "delegation_approved" → ALERT
```

## Storage

- **In-memory** - HashMap-based, fast lookups (<10μs)
- **Ephemeral** - State lost on restart
- **TTL-based** - Automatic expiration prevents memory bloat
- **Per-wallet** - Isolated state for each wallet address

## Configuration

```bash
PARAPET_FLOWBITS_DEFAULT_TTL=300  # 5 minutes default
PARAPET_FLOWBITS_MAX_WALLETS=10000  # Memory limit
```

## Use Cases

- Multi-stage attack detection
- Behavioral analysis (first transaction patterns)
- Rate limiting per wallet
- Session tracking

For persistent state across proxy restarts, use Redis-backed storage in the escalations module.
