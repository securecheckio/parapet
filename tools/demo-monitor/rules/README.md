# Demo Monitor Rules

This directory contains rule files for the Parapet RPC Proxy demo configuration.

## Active Rules

### demo-combined-rules.json (8 rules)

**Used by**: `docker-compose.yml` (default configuration)

This combined ruleset includes both Samui Wallet testing and Drift Attack protection:

#### Samui Wallet Testing (1 rule)
1. **dev-alert-sol-transfer-gt-0.01**
   - Alert on SOL transfers > 0.01 SOL
   - Action: Alert
   - Use case: Wallet integration testing

#### Drift Attack Detection (7 rules)
2. **block-presigned-authority-fingerprint**
   - Block pre-signed authority changes (fingerprint detection)
   - Action: Block
   - Weight: 100 (Critical)

3. **block-nonce-custom-program-outflow**
   - Block pre-signed transactions with fund outflow to custom programs
   - Action: Block
   - Weight: 100 (Critical)

4. **block-log-admin-change**
   - Block pre-signed transactions with admin/authority changes in logs
   - Action: Block
   - Weight: 100 (Critical)

5. **critical-timelock-removal**
   - Alert when governance timelock set to zero
   - Action: Alert
   - Weight: 90 (Critical)

6. **critical-presigned-squads-proposal**
   - Alert on pre-signed Squads V4 proposals
   - Action: Alert
   - Weight: 95 (Critical)

7. **alert-squads-vault-execution**
   - Alert on Squads V4 vault transaction executions
   - Action: Alert
   - Weight: 50 (Medium)

8. **alert-nonce-custom-program**
   - Alert on pre-signed transactions to custom programs
   - Action: Alert
   - Weight: 60 (High)

## Individual Rule Files

### samui-wallet-0.01-sol.json (1 rule)
Original Samui Wallet testing rule (kept for reference).

## Usage

The Docker deployment automatically loads `demo-combined-rules.json`:

```bash
./start-with-docker.sh
```

## Customization

To use different rules, update `RULES_PATH` in `docker-compose.yml`:

```yaml
environment:
  - RULES_PATH=/app/rules/your-custom-rules.json
```

## Testing

View loaded rules:
```bash
docker logs demo-parapet-proxy 2>&1 | grep "Loaded rule"
```

Verify rule count:
```bash
docker logs demo-parapet-proxy 2>&1 | grep "Rule engine initialized"
```

Expected output:
```
✅ Rule engine initialized with 8 rules from /app/rules/demo-combined-rules.json
```

## Rule Actions

- **Alert**: Transaction processed but alert logged (non-blocking)
- **Block**: Transaction rejected before signing/submission
- **Allow**: Transaction passes (no action)

## References

- Full Drift attack analysis: `../../docs/DRIFT_ATTACK.md`
- Rule format specification: `../../../docs/RULES_FORMAT.md`
- Original Drift rules: `../../../rules/examples/drift-attack-detection.json`
