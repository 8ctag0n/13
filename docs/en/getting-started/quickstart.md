# Quickstart

Get ZyberLink running in 5 minutes.

## Prerequisites

Before you begin, ensure you have:

- **Rust 1.75+** - [Install Rust](https://rustup.rs/)
- **Solana CLI 2.1+** - [Install Solana](https://docs.solana.com/cli/install-solana-cli-tools)
- **Unix-like environment** - Linux, macOS, or WSL2

## Quick Setup

### 1. Clone the Repository

```bash
git clone https://github.com/yourusername/zyberlink.git
cd zyberlink
```

### 2. Build the Project

```bash
cargo build --release
```

This will compile all components: the Solana program, SDK, prover node, and utilities.

**Expected time:** 3-5 minutes on first build.

### 3. Start Local Validator

Open a new terminal and start a local Solana validator:

```bash
solana-test-validator --reset
```

Leave this running in the background.

### 4. Deploy the Program

In your main terminal, deploy the ZyberLink marketplace program:

```bash
# Build the Solana program
cargo build-sbf

# Deploy to local validator
solana program deploy target/deploy/zyberlink.so
```

Save the **Program ID** that appears after deployment - you'll need it later.

### 5. Run the Interactive Demo

The fastest way to see ZyberLink in action:

```bash
cd demo
./run-demo.sh
```

This script will:
1. Configure the local environment
2. Start a prover node with the TUI interface
3. Generate sample FHE computation jobs
4. Show real-time job processing

**What you'll see:**

- Terminal UI showing prover status
- Jobs being claimed and processed
- FHE computations completing
- Earnings accumulating
- Live system statistics

### 6. Stop the Demo

Press `q` in the TUI to gracefully shut down the prover node.

## What's Next?

Now that you have ZyberLink running:

- **[Demo Guide](demo.md)** - Explore the interactive demo in detail
- **[Architecture](../architecture/overview.md)** - Understand how ZyberLink works
- **[Guides](../guides/)** - Deploy to production

## Troubleshooting

### Build fails with linking errors

Make sure you have the latest Rust toolchain:
```bash
rustup update
```

### Solana CLI not found

Add Solana to your PATH:
```bash
export PATH="$HOME/.local/share/solana/install/active_release/bin:$PATH"
```

### Program deployment fails

Check that the local validator is running:
```bash
solana cluster-version
```

Should show version info if the validator is accessible.

### Need help?

- Check the [GitHub Issues](https://github.com/yourusername/zyberlink/issues)
- Join our community discussions

## Advanced Setup

### Running Your Own Prover Node

Instead of using the demo script, you can run a prover node manually:

```bash
cd prover-node
cargo run --release -- wizard
```

The interactive wizard will guide you through:
- Wallet setup
- Solana RPC configuration
- Program ID configuration
- Prover registration

### Running Tests

Verify everything works correctly:

```bash
# Unit tests
cargo test

# Integration tests
cargo test --test integration_tests

# E2E tests (requires running validator + prover)
./scripts/test-e2e.sh
```

## What You Built

You now have a complete ZyberLink development environment with:

- ✅ Local Solana validator
- ✅ ZyberLink marketplace program deployed
- ✅ Prover node ready to process jobs
- ✅ Demo infrastructure for testing

Ready to dive deeper? Check out the [Demo Guide](demo.md) to explore all features.
