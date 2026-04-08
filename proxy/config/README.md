# Parapet Configuration Files

This directory contains configuration files for Parapet's security analysis.

## Known-Safe Programs

### `known-safe-programs.json`

The default list of verified safe Solana programs. These programs are excluded from risk calculations in deep scanning (inner instruction analysis).

**Categories:**
- **System**: Core Solana programs (System, Token, Stake, etc.)
- **Token**: SPL Token and Token-2022 programs
- **NFT**: Metaplex and NFT-related programs
- **Utility**: Memo and other utility programs

**Usage:**
The scanner automatically loads this file. No configuration needed.

### Creating a Custom Safe Programs List

You can create your own list of trusted programs to merge with the defaults:

1. Copy `custom-safe-programs-example.json` to create your list:
   ```bash
   cp custom-safe-programs-example.json my-safe-programs.json
   ```

2. Edit the file to add your trusted programs:
   ```json
   {
     "version": "1.0",
     "name": "My Custom Safe Programs",
     "programs": [
       {
         "address": "YourProgramAddress111111111111111111111",
         "name": "Your Program Name",
         "category": "defi",
         "description": "What this program does"
       }
     ]
   }
   ```

3. Use it with the wallet scanner:
   ```bash
   ./scan-wallet.sh WALLET_ADDRESS --safe-programs-file my-safe-programs.json
   ```

**Important Notes:**
- Your custom list is **merged** with the default list (not replaced)
- Only add programs you trust and have verified
- Check program addresses on Solana explorers (solscan.io, solana.fm) before adding
- Programs in these lists are excluded from "unknown program" risk calculations

## Program Categories

When adding programs to your custom list, use these categories:

| Category | Description | Examples |
|----------|-------------|----------|
| `system` | Core Solana system programs | System Program, Token Program |
| `token` | Token and fungible asset programs | SPL Token, Token-2022 |
| `nft` | NFT and digital collectibles | Metaplex, Candy Machine |
| `defi` | Decentralized finance protocols | Jupiter, Orca, Raydium |
| `utility` | Common utility programs | Memo, Name Service |

## Command-Line Usage

### View Default Safe Programs
The scanner loads the default list automatically.

### Use Custom Safe Programs
```bash
# Merge your custom list with defaults
./scan-wallet.sh WALLET --safe-programs-file path/to/custom-safe-programs.json

# Or with the Rust binary directly
cargo run --release --bin wallet-scanner -- WALLET \
  --safe-programs-file path/to/custom-safe-programs.json
```

### Verify What's Loaded
Enable debug logging to see which programs are loaded:
```bash
RUST_LOG=info ./scan-wallet.sh WALLET --safe-programs-file my-list.json
```

Look for log lines:
```
[INFO] ✅ Loaded known-safe programs from: proxy/config/known-safe-programs.json
[INFO] 📋 Loaded 18 known-safe programs from: my-list.json
[INFO] ✅ Merged custom safe programs: 18 original + 5 new = 23 total
```

## File Format Specification

```json
{
  "version": "1.0",
  "name": "List Name",
  "description": "Optional description",
  "last_updated": "YYYY-MM-DD",
  "programs": [
    {
      "address": "PROGRAM_ADDRESS_BASE58",     // Required
      "name": "Human-readable name",           // Required
      "category": "system|token|nft|defi|utility",  // Optional
      "description": "What it does"            // Optional
    }
  ],
  "categories": {                               // Optional metadata
    "category_name": "Description"
  },
  "notes": [                                    // Optional
    "Additional information"
  ]
}
```

## Security Considerations

### What This Affects
- **Inner Instruction Analysis**: Programs in the safe list are excluded from "unknown program" risk scores
- **CPI Detection**: Deep CPIs involving only safe programs have lower risk scores
- **Alert Generation**: Unknown programs trigger alerts, safe programs do not

### What This Does NOT Affect
- **Rule Engine**: Rules still evaluate all transactions
- **Threat Detection**: Other security checks are still performed
- **Program Verification**: OtterSec and Helius checks still run

### Best Practices

1. **Verify Before Adding**: Always verify program addresses on multiple explorers
2. **Check Audits**: Only add programs that have been audited or are widely trusted
3. **Start Small**: Begin with well-known protocols (Jupiter, Orca, Raydium)
4. **Regular Updates**: Review and update your list as you discover new trusted programs
5. **Be Conservative**: When in doubt, don't add it - false positives are better than false negatives

### Finding Program Addresses

1. **Solscan**: https://solscan.io - Search for protocol name
2. **Solana.fm**: https://solana.fm - Verify program details
3. **Protocol Docs**: Check official documentation for canonical addresses
4. **GitHub**: Most protocols publish addresses in their repos

## Examples

### Example: Adding Jupiter to Safe List
```json
{
  "version": "1.0",
  "name": "My DeFi Safe Programs",
  "programs": [
    {
      "address": "JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN",
      "name": "Jupiter Aggregator v6",
      "category": "defi",
      "description": "Jupiter swap aggregator - audited by OtterSec"
    }
  ]
}
```

### Example: Adding Multiple DEX Programs
See `custom-safe-programs-example.json` for a complete example with multiple DeFi protocols.

## Troubleshooting

### "Could not load known-safe-programs.json"
The scanner will use a minimal fallback list. Check:
- Are you running from the correct directory?
- Does `proxy/config/known-safe-programs.json` exist?

### "Failed to load custom safe programs"
Check:
- Is the file path correct?
- Is the JSON valid? (Use a JSON validator)
- Do all programs have required fields (address, name)?

### Programs Still Flagged as Unknown
- Verify the program address matches exactly (case-sensitive, no typos)
- Check that your custom file is being loaded (enable INFO logging)
- Ensure the custom list is being merged (check log output)

## Related Documentation

- **Deep Scanning Guide**: `../scanner/DEEP_SCANNING.md`
- **Rule Engine**: `../proxy/rules/README.md`
- **CPI Detection Rules**: `../proxy/rules/presets/deep-cpi-scan.json`
