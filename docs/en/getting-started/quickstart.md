# Quickstart

Get ZyberLink running in 5 minutes.

## Prerequisites

Before you begin, ensure you have:

- **Rust 1.75+** - [Install Rust](https://rustup.rs/)
- **Solana CLI 2.1+** - [Install Solana](https://docs.solana.com/cli/install-solana-cli-tools)
- **Docker + Docker Compose** - Required for the local stack
- **Unix-like environment** - Linux, macOS, or WSL2

## Quick Setup

### 1. Clone the Platform Repository

```bash
git clone https://github.com/8ctagon/zyb-platform.git
cd zyb-platform
```

### 2. Configure Local Environment

```bash
cp zyb.example.toml zyb.toml
cp .env.example .env
```

### 3. Start Local Stack

Use Docker Compose:

```bash
docker-compose up -d
```

Or use the Makefile (recommended):

```bash
make start
```

### 4. Run the Interactive Demo

The fastest way to see ZyberLink in action:

```bash
./demo.sh
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

### 5. Stop the Demo

Press `q` in the TUI to gracefully shut down the prover node, then stop the stack:

```bash
make stop
# or: docker-compose down
```

## What's Next?

Now that you have ZyberLink running:

- **[Demo Guide](demo.md)** - Explore the interactive demo in detail
- **[Architecture](../architecture/overview.md)** - Understand how ZyberLink works
- **[Guides](../guides/)** - Deploy to production

## Troubleshooting

### Docker Compose fails to start

Check Docker status and logs:
```bash
docker-compose ps
docker-compose logs -f
```

### Solana CLI not found

Add Solana to your PATH:
```bash
export PATH="$HOME/.local/share/solana/install/active_release/bin:$PATH"
```

### Need help?

- See the [Repository Map](repositories.md) for the correct issue tracker
- Join our community discussions

## Advanced Setup

### Running Your Own Prover Node

Instead of using the demo script, run a prover node manually from `zyb-compute`.
Follow the [Prover Setup Guide](../guides/prover-setup.md).

### Running Tests

Tests are organized per repository. Start with `zyb-platform/tests` and each repo's README.

## What You Built

You now have a complete ZyberLink development environment with:

- ✅ Local Solana validator
- ✅ ZyberLink marketplace program deployed
- ✅ Prover node ready to process jobs
- ✅ Demo infrastructure for testing

Ready to dive deeper? Check out the [Demo Guide](demo.md) to explore all features.
