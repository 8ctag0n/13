# Prover Node Setup Guide

Run a ZyberLink prover node to process privacy-preserving computations and earn SOL.

## Overview

A ZyberLink prover node is a service that executes FHE computations on encrypted data. The current implementation is a foundational prototype focused on demonstrating the core privacy-preserving compute marketplace concept.

**What provers do:**
- Poll the on-chain marketplace for pending jobs
- Claim jobs based on simple profitability checks
- Download encrypted witness data
- Execute FHE operations on ciphertext
- Submit results for consensus verification

**Current Status:** Early prototype. Basic functionality working, advanced features in development.

## Prerequisites

### Hardware Requirements

**Minimum:**
- 8 GB RAM
- 4 CPU cores
- 20 GB storage
- Internet connection

**Recommended:**
- 16 GB RAM
- 8 CPU cores
- 50 GB storage

**Note:** FHE operations are CPU-intensive. More cores = better performance.

### Software Requirements

- **Rust** 1.75+
- **Solana CLI** 1.18+
- **Git**
- Basic Linux/Unix knowledge

### Initial Funding

- ~0.1-0.2 SOL for registration and transaction fees

## Quick Start

### 1. Install Dependencies

```bash
# Ubuntu/Debian
sudo apt update
sudo apt install -y build-essential pkg-config libssl-dev curl git

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Install Solana CLI
sh -c "$(curl -sSfL https://release.solana.com/stable/install)"
export PATH="$HOME/.local/share/solana/install/active_release/bin:$PATH"
```

### 2. Clone and Build

```bash
# Clone repository
git clone https://github.com/8ctag0n/13.git zyberlink
cd zyberlink

# Build prover (takes 5-10 minutes)
cargo build --release -p zyberlink-prover

# Binary location: target/release/zyberlink-prover
```

### 3. Create Keypair

```bash
# Generate prover keypair
solana-keygen new --outfile ~/.config/solana/prover.json

# Backup this file - it's your prover identity!
cp ~/.config/solana/prover.json ~/prover-backup.json
```

### 4. Fund Wallet

**For Devnet (testing):**
```bash
solana config set --url https://api.devnet.solana.com
solana airdrop 2 ~/.config/solana/prover.json
```

**For Mainnet:**
```bash
solana config set --url https://api.mainnet-beta.solana.com
# Transfer SOL from your main wallet
solana transfer <PROVER_PUBKEY> 0.2 --from ~/.config/solana/id.json
```

### 5. Configure Environment

```bash
# Set required environment variables
export SOLANA_RPC_URL=https://api.devnet.solana.com
export ZYBERLINK_PROGRAM_ID=<PROGRAM_ID>  # Get from team/docs
export WITNESS_BACKEND_URL=http://localhost:8080
export RUST_LOG=info
```

### 6. Register Prover

Register your prover on-chain:

```bash
./target/release/zyberlink-prover register \
  --program-id $ZYBERLINK_PROGRAM_ID \
  --stake-amount 100000000  # 0.1 SOL
```

**Note:** This is a one-time registration. Your stake is locked until you unregister.

### 7. Run Prover

Start the prover node:

```bash
./target/release/zyberlink-prover \
  --program-id $ZYBERLINK_PROGRAM_ID \
  --witness-backend-url $WITNESS_BACKEND_URL
```

**Expected output:**
```
[INFO] Starting ZyberLink Prover Node
[INFO] Prover Authority: 7vX8h9...
[INFO] Program ID: ZyberLink...
[INFO] Polling for jobs every 5 seconds...
[INFO] Found 0 pending jobs
[INFO] Found 2 pending jobs
[INFO] Processing job 1...
[INFO] [Job 1] Claimed successfully
[INFO] [Job 1] Downloading witness...
[INFO] [Job 1] Executing FHE operation...
[INFO] [Job 1] Completed! ✅
```

## Configuration Options

Command-line arguments:

| Argument | Description | Default |
|----------|-------------|---------|
| `--rpc-url` | Solana RPC endpoint | http://localhost:8899 |
| `--program-id` | ZyberLink program ID | Required |
| `--keypair` | Path to prover keypair | ~/.config/solana/id.json |
| `--witness-backend-url` | Witness server URL | http://localhost:8080 |
| `--poll-interval` | Job polling interval (seconds) | 5 |
| `--min-roi` | Minimum ROI to accept jobs (%) | 20.0 |
| `--max-concurrent-jobs` | Max parallel jobs | 3 |
| `--tui-mode` | Enable terminal UI | false |

**Example with custom settings:**
```bash
./target/release/zyberlink-prover \
  --rpc-url https://api.devnet.solana.com \
  --program-id $ZYBERLINK_PROGRAM_ID \
  --keypair ~/.config/solana/prover.json \
  --witness-backend-url http://localhost:8080 \
  --poll-interval 5 \
  --min-roi 15.0 \
  --max-concurrent-jobs 2
```

## Running as a Service

For continuous operation, run prover as a systemd service:

### 1. Create Service File

```bash
sudo nano /etc/systemd/system/zyberlink-prover.service
```

### 2. Add Configuration

```ini
[Unit]
Description=ZyberLink Prover Node
After=network.target

[Service]
Type=simple
User=YOUR_USERNAME
WorkingDirectory=/home/YOUR_USERNAME/zyberlink
Environment="RUST_LOG=info"
Environment="SOLANA_RPC_URL=https://api.devnet.solana.com"
ExecStart=/home/YOUR_USERNAME/zyberlink/target/release/zyberlink-prover \
  --program-id YOUR_PROGRAM_ID \
  --witness-backend-url http://localhost:8080
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
```

### 3. Enable and Start

```bash
# Reload systemd
sudo systemctl daemon-reload

# Start service
sudo systemctl start zyberlink-prover

# Enable on boot
sudo systemctl enable zyberlink-prover

# Check status
sudo systemctl status zyberlink-prover

# View logs
sudo journalctl -u zyberlink-prover -f
```

## Monitoring

### Check Prover Status

```bash
# Via systemd
sudo systemctl status zyberlink-prover

# View recent logs
sudo journalctl -u zyberlink-prover -n 50

# Check wallet balance
solana balance ~/.config/solana/prover.json
```

### TUI Mode (Terminal UI)

For real-time monitoring with a visual interface:

```bash
./target/release/zyberlink-prover --tui-mode \
  --program-id $ZYBERLINK_PROGRAM_ID \
  --witness-backend-url $WITNESS_BACKEND_URL
```

The TUI displays:
- Jobs claimed, completed, failed
- Earnings (SOL)
- Recent job history
- System uptime

Press `q` to quit.

## Troubleshooting

### Prover Not Starting

**Check logs for errors:**
```bash
./target/release/zyberlink-prover --program-id $ZYBERLINK_PROGRAM_ID
# Look for error messages
```

**Common issues:**
- Missing `ZYBERLINK_PROGRAM_ID`
- Invalid keypair path
- Insufficient SOL balance
- Cannot connect to RPC

### No Jobs Being Claimed

**Possible reasons:**
1. No jobs available on network
2. Other provers claiming jobs first
3. Your ROI threshold too high
4. Not registered as prover

**Solutions:**
```bash
# Lower ROI threshold
--min-roi 10.0

# Check registration
solana account <PROVER_PDA>

# Verify you have SOL for transaction fees
solana balance ~/.config/solana/prover.json
```

### Witness Download Fails

**Error:** "Failed to download witness from backend"

**Check witness backend:**
```bash
# Test backend connectivity
curl $WITNESS_BACKEND_URL/health

# Verify URL is correct
echo $WITNESS_BACKEND_URL
```

### FHE Computation Errors

**Error:** "Failed to deserialize server key" or "FHE computation failed"

**Possible causes:**
- Corrupted witness data
- Insufficient memory (need 8GB+)
- TFHE version mismatch

**Try:**
```bash
# Check available memory
free -h

# Reduce concurrent jobs
--max-concurrent-jobs 1

# Restart prover
sudo systemctl restart zyberlink-prover
```

## Current Limitations

The current prover implementation is an early prototype with the following limitations:

**Known limitations:**
- Basic job selection (no advanced profitability optimization)
- Simple consensus mechanism (2-of-3 or 3-of-5)
- Limited error recovery
- No automatic restart on failures
- Basic logging and monitoring

**In Development:**
- Advanced ROI calculation
- GPU acceleration for FHE operations
- Prover reputation system
- Automatic failover and recovery
- Enhanced monitoring dashboard

**Recommended for:** Testing, development, and early adopters willing to run experimental software.

## Best Practices

### Security

```bash
# Secure your keypair
chmod 600 ~/.config/solana/prover.json

# Backup keypair offline
cp ~/.config/solana/prover.json /media/usb/backup/

# Don't expose RPC ports publicly
# Use firewall to restrict access
```

### Reliability

```bash
# Run as systemd service (auto-restart)
sudo systemctl enable zyberlink-prover

# Monitor logs regularly
sudo journalctl -u zyberlink-prover --since "1 hour ago"

# Keep SOL balance above 0.05
# (for transaction fees)
```

### Performance

```bash
# Adjust concurrent jobs based on RAM
# 8GB RAM = 1-2 jobs
# 16GB RAM = 2-3 jobs

# Use local RPC if possible (lower latency)
# Use SSD storage (faster witness downloads)
```

## For Development

### Local Testing Setup

```bash
# Start local validator
make c1

# Initialize marketplace
make c2

# Start prover in development mode
RUST_LOG=debug ./target/release/zyberlink-prover \
  --rpc-url http://localhost:8899 \
  --program-id <LOCAL_PROGRAM_ID> \
  --witness-backend-url http://localhost:8080
```

### Running E2E Tests

```bash
# Test Proof of Innocence flow
make e2e-poi

# Test Census Sum flow
make e2e-sum
```

## Next Steps

- **[Complete Prover Documentation](/src/prover-node/README.md)** - Full technical details
- **[FHE Operations](/docs/book/en/concepts/fhe-operations.md)** - Understand what you're computing
- **[Analytics Guide](analytics-guide.md)** - See what jobs look like
- **[Proof of Innocence Guide](proof-of-innocence-guide.md)** - Another use case

## Getting Help

For prover setup questions:

- **GitHub Issues:** https://github.com/8ctag0n/13/issues
- **Documentation:** https://docs.zyberlink.fun
- **Discord:** [Join community]

## Related Documentation

- [Prover Node README](/src/prover-node/README.md) - Complete technical documentation
- [ROI Calculator](/src/prover-node/src/roi_calculator.rs) - Profitability logic
- [FHE Engine](/src/prover-node/src/fhe_engine.rs) - Computation implementation

---

**Run a Prover:** Process encrypted computations. Earn SOL. Support privacy-preserving compute.
