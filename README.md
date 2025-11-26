# ZyberLink

**Decentralized ZK & FHE Compute Marketplace on Solana**

> Multi-prover consensus network for privacy-preserving computations

## Overview

ZyberLink is a decentralized marketplace for ZK and FHE computations on Solana. Multiple independent provers compete to execute cryptographic computations, with on-chain consensus ensuring correctness.

**Current Status:** Zypherpunk Hackathon (Nov 10 - Dec 1, 2025)

## Key Features

- **Multi-Prover Consensus** - 2-of-3 or 3-of-5 consensus ensures computation correctness
- **FHE Computations** - Fully homomorphic encryption support via TFHE-rs
- **Privacy Preserved** - Encrypted witness data, provers never see plaintext
- **Dynamic Pricing** - Market-based job pricing
- **Automated Payments** - On-chain verification with trustless payment distribution

## Architecture

```
┌──────────────┐
│    Client    │ (Any application needing ZK/FHE compute)
└──────┬───────┘
       │ 1. Encrypt witness, create job
       ▼
┌─────────────────────────┐
│   Solana Program        │
│   (Marketplace)         │
└────┬────────────────────┘
     │ 2. Job broadcast
     ▼
┌────────────────────────────────────────────┐
│           Prover Network                   │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐ │
│  │ Prover A │  │ Prover B │  │ Prover C │ │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘ │
└───────│─────────────│─────────────│────────┘
        │ 3. Claim    │             │
        │ 4. Compute  │ Compute     │ Compute
        ▼             ▼             ▼
┌────────────────────────────────────────────┐
│   Result Submission + Consensus            │
│   2/3 matching results = verified          │
└────────────────┬───────────────────────────┘
                 │ 5. Payment distributed
                 ▼
┌────────────────────────────────────────────┐
│   On-Chain Verification & Payment          │
│   - Pay matching provers                   │
│   - Penalize dishonest provers             │
└────────────────────────────────────────────┘
```

## Quick Start

```bash
# Full setup from scratch (recommended)
make localnet-setup    # or: make l1 - Setup validator + db + deploy
make localnet-init     # or: make l2 - Initialize marketplace
make localnet-run      # or: make l3 - Start backend + provers + frontend

# Optional: auto-generate test jobs
make localnet-jobs     # or: make l4

# Stop everything
make localnet-stop     # or: make l0

# Check status
make health
make status
```

### Prerequisites

- Rust 1.75+
- Solana CLI 2.1+
- Node.js 18+ (for frontend)
- Podman or Docker (for PostgreSQL)

## Project Structure

```
zyberlink/
├── src/                    # All source code
│   ├── programs/           # Solana smart contracts
│   ├── sdk/                # Client SDK (Rust)
│   ├── prover-node/        # Prover daemon
│   ├── blink-server/       # Backend API
│   ├── webapp/             # Frontend (Svelte)
│   ├── shared/             # Shared types & crypto
│   ├── witness-storage/    # Encrypted witness storage
│   └── fhe-cli/            # FHE command line tools
├── tests/                  # E2E tests
├── docs/                   # Documentation
│   ├── book/               # GitBook (public)
│   └── private/            # Internal docs
├── infra/                  # Infrastructure
│   ├── docker/             # Docker compose files
│   ├── demo/               # Demo scripts
│   └── dev/                # Development tools
├── scripts/                # Automation scripts
├── Makefile                # Build & run commands
└── Cargo.toml              # Workspace config
```

## Development

```bash
# Build all
cargo build --release

# Run tests
cargo test --all
make test-e2e

# Check service health
make health

# View logs
make logs              # All logs
make logs-backend      # Backend only
make logs-provers      # Provers only
```

### Make Commands

| Command | Alias | Description |
|---------|-------|-------------|
| `make localnet-setup` | `l1` | Setup validator + db + deploy program |
| `make localnet-init` | `l2` | Initialize marketplace on-chain |
| `make localnet-run` | `l3` | Start backend + provers + frontend |
| `make localnet-jobs` | `l4` | Auto-generate test jobs |
| `make localnet-stop` | `l0` | Stop all services |
| `make health` | - | Check service status |
| `make logs` | - | Tail all logs |

Run `make help` for full list of commands.

## Technology Stack

- **Smart Contracts:** Solana (bare metal, no Anchor)
- **Backend:** Rust + Axum + PostgreSQL
- **Frontend:** Svelte + Vite
- **Prover:** Rust + TFHE-rs (FHE) + Halo2 (ZK)
- **SDK:** Rust client library
- **Encryption:** Post-quantum key exchange (ML-KEM)

## Status

- E2E Tests: 55/56 passing (98%)
- Backend API: 100% coverage
- Security Audit: 6/6 checks passing

See [ROADMAP.md](ROADMAP.md) for full status and planned features.

## Use Cases

- **Private DeFi:** Encrypted swaps without revealing amounts
- **Confidential DAOs:** Private voting with verifiable results
- **Wallet Proving:** Offload mobile ZK proof generation
- **Privacy Analytics:** Compute on encrypted datasets

## License

MIT OR Apache-2.0

## Acknowledgments

- **Solana Foundation** - High-performance blockchain
- **ZAMA (TFHE-rs)** - FHE library
- **Zcash Foundation** - Halo2 circuits
- **Zypherpunk Hackathon** - Catalyst for this project
