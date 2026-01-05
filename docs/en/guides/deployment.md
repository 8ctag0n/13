# Deployment Guide

Deploy ZyberLink to production environments.

## Overview

This guide covers deploying ZyberLink components to production:
- Solana program deployment
- Prover node setup
- Client SDK integration

## Prerequisites

**Production environment requirements:**
- Linux server (Ubuntu 22.04+ recommended)
- Rust 1.75+
- Solana CLI 2.1+
- 8GB+ RAM
- 4+ CPU cores

**For provers specifically:**
- 16GB+ RAM (recommended)
- 8+ CPU cores (recommended)
- Stable internet connection
- 24/7 uptime capability

## Part 1: Deploy Solana Program

### 1.1 Build the Program

Replace `<ORG>` with your GitHub organization or mirror.

```bash
# Clone repository
git clone https://github.com/<ORG>/zyb-chain.git
cd zyb-chain/solana

# Build Solana program
cargo build-sbf --manifest-path=programs/zyberlink/Cargo.toml
```

### 1.2 Configure Solana CLI

```bash
# Set to mainnet-beta (or devnet for testing)
solana config set --url https://api.mainnet-beta.solana.com

# Set keypair (your deployment wallet)
solana config set --keypair /path/to/your/keypair.json

# Verify configuration
solana config get
```

### 1.3 Fund Deployment Wallet

```bash
# Check balance
solana balance

# You'll need ~5-10 SOL for deployment + rent
```

### 1.4 Deploy Program

```bash
# Deploy to Solana
solana program deploy target/deploy/zyberlink.so

# Save the Program ID that is printed
# Example output:
# Program Id: 7xYz...abc123
```

### 1.5 Verify Deployment

```bash
# Check program exists
solana program show <PROGRAM_ID>

# Expected output shows program details, balance, authority
```

## Part 2: Run a Prover Node

### 2.1 Server Setup

```bash
# Update system
sudo apt update && sudo apt upgrade -y

# Install dependencies
sudo apt install -y build-essential pkg-config libssl-dev

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

### 2.2 Clone and Build

Replace `<ORG>` with your GitHub organization or mirror.

```bash
# Clone repository
git clone https://github.com/<ORG>/zyb-compute.git
cd zyb-compute

# Build prover node (optimized)
cargo build --release --bin zyberlink-prover

# Binary will be at: target/release/zyberlink-prover
```

### 2.3 Configure Prover

Create configuration file:

```bash
mkdir -p ~/.zyberlink
nano ~/.zyberlink/config.toml
```

Add configuration:

```toml
[network]
rpc_url = "https://api.mainnet-beta.solana.com"
program_id = "YOUR_PROGRAM_ID_HERE"

[prover]
wallet_path = "/path/to/prover/keypair.json"
auto_claim = true
max_concurrent_jobs = 3

[fhe]
enabled = true
max_computation_time_seconds = 60
```

### 2.4 Run Interactive Setup

```bash
cd zyb-compute
cargo run --release -- wizard
```

This wizard will:
- Generate or import a wallet
- Configure RPC endpoint
- Set program ID
- Register prover on-chain
- Fund wallet if needed

### 2.5 Start Prover Node

```bash
# Run in foreground (for testing)
./target/release/zyberlink-prover

# Run in background (production)
nohup ./target/release/zyberlink-prover > prover.log 2>&1 &

# Or use systemd (recommended for production)
```

### 2.6 Create Systemd Service (Recommended)

Create service file:

```bash
sudo nano /etc/systemd/system/zyberlink-prover.service
```

Add:

```ini
[Unit]
Description=ZyberLink Prover Node
After=network.target

[Service]
Type=simple
User=youruser
WorkingDirectory=/home/youruser/zyb-compute
ExecStart=/home/youruser/zyb-compute/target/release/zyberlink-prover
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
```

Enable and start:

```bash
sudo systemctl daemon-reload
sudo systemctl enable zyberlink-prover
sudo systemctl start zyberlink-prover

# Check status
sudo systemctl status zyberlink-prover

# View logs
sudo journalctl -u zyberlink-prover -f
```

## Part 3: Client SDK Integration

### 3.1 Add SDK Dependency

In your project's `Cargo.toml`:

```toml
[dependencies]
zyberlink-sdk = "0.1"
solana-client = "1.18"
solana-sdk = "1.18"
tokio = { version = "1", features = ["full"] }
```

### 3.2 Basic Client Setup

```rust
use zyberlink_sdk::MarketplaceClient;
use solana_sdk::signature::{Keypair, read_keypair_file};
use solana_client::rpc_client::RpcClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize RPC client
    let rpc_url = "https://api.mainnet-beta.solana.com";
    let rpc_client = RpcClient::new(rpc_url.to_string());

    // Load wallet
    let payer = read_keypair_file("/path/to/keypair.json")?;

    // Initialize marketplace client
    let program_id = "YOUR_PROGRAM_ID".parse()?;
    let client = MarketplaceClient::new(
        rpc_client,
        payer,
        program_id,
    );

    Ok(())
}
```

### 3.3 Create FHE Job

```rust
use zyberlink_sdk::fhe::{FheOperation, FheJobBuilder};
use tfhe::prelude::*;

// Generate FHE keys
let config = ConfigBuilder::default().build();
let (client_key, server_key) = generate_keys(config);

// Encrypt inputs
let encrypted_a = client_key.encrypt(42u64);
let encrypted_b = client_key.encrypt(58u64);

// Create job
let job_id = client.create_fhe_job(
    FheOperation::Add,
    vec![encrypted_a, encrypted_b],
    server_key,
    2_000_000, // reward in lamports
).await?;

println!("Job created: {}", job_id);
```

### 3.4 Monitor and Retrieve Result

```rust
// Wait for job completion
let result = client.wait_for_job_completion(job_id).await?;

// Decrypt result
let plaintext = client_key.decrypt(&result.encrypted_output);
println!("Result: {}", plaintext); // 100
```

## Monitoring and Maintenance

### Prover Node Monitoring

**Check node status:**
```bash
# Via systemd
sudo systemctl status zyberlink-prover

# Via logs
sudo journalctl -u zyberlink-prover --since "1 hour ago"

# Via RPC
curl http://localhost:8080/status
```

**Monitor earnings:**
```bash
# Check prover wallet balance
solana balance /path/to/prover/keypair.json

# View job history
./target/release/zyberlink-prover stats
```

### Health Checks

Create monitoring script:

```bash
#!/bin/bash
# health-check.sh

# Check if prover is running
if ! systemctl is-active --quiet zyberlink-prover; then
    echo "ERROR: Prover node is down"
    # Send alert (email, Telegram, etc.)
    exit 1
fi

# Check if node is responsive
if ! curl -sf http://localhost:8080/health > /dev/null; then
    echo "ERROR: Prover node not responding"
    exit 1
fi

echo "OK: Prover node healthy"
```

Run via cron every 5 minutes:
```bash
*/5 * * * * /path/to/health-check.sh
```

## Security Best Practices

### Wallet Security

1. **Never commit private keys to git**
2. **Use hardware wallets for high-value operations**
3. **Rotate keys periodically**
4. **Keep backup of keypairs in secure location**

### Server Security

```bash
# Enable firewall
sudo ufw enable
sudo ufw allow 22/tcp  # SSH
sudo ufw allow 8080/tcp  # Prover API (if exposed)

# Keep system updated
sudo apt update && sudo apt upgrade -y

# Disable root SSH
sudo nano /etc/ssh/sshd_config
# Set: PermitRootLogin no
sudo systemctl restart sshd
```

### RPC Security

- Use rate-limited RPC endpoints
- Consider running your own Solana validator
- Implement retry logic with exponential backoff

## Troubleshooting

### Program deployment fails

```bash
# Check SOL balance
solana balance

# Increase compute budget
solana program deploy --max-len 500000 target/deploy/zyberlink.so
```

### Prover node crashes

```bash
# Check logs
sudo journalctl -u zyberlink-prover -n 100

# Common issues:
# - Insufficient RAM: Increase swap or upgrade server
# - RPC timeouts: Use dedicated RPC endpoint
# - Wallet funds depleted: Fund prover wallet
```

### Jobs not being claimed

```bash
# Verify prover is registered
solana account <PROVER_PUBKEY>

# Check prover status in TUI
./target/release/zyberlink-prover

# Verify network connectivity
curl https://api.mainnet-beta.solana.com -v
```

## Scaling

### Multiple Prover Instances

Run multiple provers on same server:

```bash
# Copy configuration for each instance
cp ~/.zyberlink/config.toml ~/.zyberlink/config-prover2.toml

# Run with different config
./target/release/zyberlink-prover --config ~/.zyberlink/config-prover2.toml
```

### Load Balancing

For high-volume applications, use multiple RPC endpoints:

```toml
[network]
rpc_urls = [
    "https://api.mainnet-beta.solana.com",
    "https://solana-api.projectserum.com",
    "https://your-dedicated-rpc.com"
]
load_balance_strategy = "round_robin"
```

## Production Checklist

Before going live:

- [ ] Solana program deployed and verified
- [ ] Prover node(s) running with systemd
- [ ] Monitoring and alerting configured
- [ ] Backups of all keypairs created
- [ ] Firewall rules configured
- [ ] Health check scripts scheduled
- [ ] Client SDK integrated and tested
- [ ] Documentation updated with production configs

## Support

For production deployment assistance:
- GitHub Issues: https://github.com/YOUR_ORG/zyb-platform/issues (docs) or see the [Repository Map](../getting-started/repositories.md)
- Community Discord: [link]
- Enterprise support: [contact email]

---

**Production Deployment:** Secure, monitored, scalable.
