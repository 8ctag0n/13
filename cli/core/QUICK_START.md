# Quick Start Guide - Zyb Wizard

## Prerequisites

1. **Build the CLI**:
```bash
cd /home/deploy/2025q4/13-area3-cli
cargo build -p zyb-cli --release
```

2. **Binary location**:
```bash
./target/release/zyb
```

## Running the Wizard

### Option 1: From project root
```bash
./target/release/zyb wizard
```

### Option 2: From zyb-cli directory
```bash
cd src/zyb-cli
../../target/release/zyb wizard
```

## Testing Without Real Server

The wizard will work up until the payment/server connection step. You can test:

1. **Menu navigation** - Arrow keys and Enter
2. **Circuit selection** - Choose from 8 circuits
3. **File input** - Use `./example_witness.json`
4. **Path expansion** - Enter `~/.config/solana/id.json`
5. **Configuration** - Set server/RPC URLs

The wizard will fail gracefully at the server connection step if no server is running.

## Full End-to-End Test

To test the complete flow, you need:

1. **Running server** at `http://localhost:3000`
2. **Funded Solana keypair** at `~/.config/solana/id.json`
3. **Valid witness file** (use `example_witness.json` as template)

### Complete Workflow
```bash
# Start from zyb-cli directory
cd src/zyb-cli

# Run wizard
../../target/release/zyb wizard

# Follow prompts:
# 1. Select "Create a ZK proof job"
# 2. Choose circuit (e.g., ProofOfInnocence)
# 3. Enter witness path: ./example_witness.json
# 4. Enter keypair path: ~/.config/solana/id.json
# 5. Server URL: http://localhost:3000
# 6. RPC URL: https://api.devnet.solana.com
# 7. Confirm payment
# 8. View job details
```

## Keyboard Controls

- **Arrow Keys** - Navigate menus
- **Enter** - Select option
- **Type text** - For file paths and URLs
- **Ctrl+C** - Exit wizard

## Example Session

```
╭─────────────────────────────────────╮
│     Welcome to Zyberlink CLI        │
│     Privacy-Preserving Compute      │
╰─────────────────────────────────────╯

? What would you like to do?
  ❯ Create a ZK proof job
    Create an FHE computation job
    Check job status
    List available circuits
    Exit

[Arrow down to select an option, Enter to confirm]
```

## Troubleshooting

### "Witness file not found"
- Ensure you're in the correct directory
- Use absolute paths or `./` prefix for current directory
- Check file exists: `ls -la example_witness.json`

### "Keypair file not found"
- Default path: `~/.config/solana/id.json`
- Generate keypair: `solana-keygen new`
- Or specify custom path

### "Failed to connect to server"
- Check server is running: `curl http://localhost:3000/health`
- Verify server URL is correct
- Check firewall/network settings

### "Payment failed"
- Ensure keypair has SOL: `solana balance`
- Check RPC URL is reachable
- Verify you're on correct network (devnet/mainnet)

## Development Testing

### Quick compile and run
```bash
# Development build (faster)
cargo build -p zyb-cli
./target/debug/zyb wizard

# Release build (optimized)
cargo build -p zyb-cli --release
./target/release/zyb wizard
```

### Check compilation
```bash
cargo check -p zyb-cli
```

### Fix warnings
```bash
cargo fix --lib -p zyb-cli
```

## Features Implemented

- [x] Interactive menu system
- [x] Circuit selection (8 circuits)
- [x] Witness file loading and validation
- [x] Keypair path expansion
- [x] Server configuration
- [x] Payment flow integration
- [x] Job creation
- [x] Status checking
- [x] Colored output and progress indicators
- [x] Error handling
- [x] Next action menu

## Features Coming Soon

- [ ] FHE job wizard
- [ ] Configuration profiles
- [ ] Batch job creation
- [ ] Job monitoring/polling
- [ ] Witness templates

## Commands Quick Reference

### Wizard
```bash
zyb wizard
```

### ZK Commands (manual)
```bash
# List circuits
zyb zk circuits

# Check status
zyb zk status <JOB_ID>

# Create job (manual)
zyb zk create --circuit-type 10 --witness ./witness.json --creator <PUBKEY>
```

### FHE Commands
```bash
# Encrypt value
zyb fhe encrypt --value 42

# Decrypt result
zyb fhe decrypt --path ./fhe-output
```

## Help Commands

```bash
# General help
zyb --help

# Wizard help
zyb wizard --help

# ZK help
zyb zk --help

# FHE help
zyb fhe --help
```

## Binary Locations

- **Debug**: `./target/debug/zyb`
- **Release**: `./target/release/zyb`
- **Size**: ~8.8 MB (release)

## Installation (Optional)

```bash
# Install to system (requires cargo)
cargo install --path src/zyb-cli

# Now available as 'zyb' command globally
zyb wizard
```

## Configuration Files

The wizard uses these files:
- **Witness**: User-provided JSON (e.g., `example_witness.json`)
- **Keypair**: Solana keypair (~/.config/solana/id.json)
- **No config file**: All settings are entered interactively

## Next Steps

After successfully creating a job:
1. Note the Job ID
2. Check status: `zyb zk status <JOB_ID>`
3. Wait for prover to complete
4. Retrieve proof when status is "completed"

## Support

- **Documentation**: See `WIZARD_README.md`
- **Implementation**: See `IMPLEMENTATION_SUMMARY.md`
- **Source**: `src/ui/wizard.rs`
