# CypherLink / ZyberLink

**Decentralized ZK Compute Marketplace on Solana**

> "Multi-prover consensus network for privacy-preserving computations"

## Overview

ZyberLink is a decentralized marketplace for ZK and FHE computations on Solana. Multiple independent provers compete to execute cryptographic computations, with on-chain consensus ensuring correctness. By distributing trust across a prover network, we enable censorship-resistant, privacy-preserving infrastructure for any application.

**Current Focus:** Multi-prover consensus for FHE (Fully Homomorphic Encryption) computations with on-chain verification and automated payment distribution to honest provers.

## Key Features

- **Multi-Prover Consensus** - 2-of-3 or 3-of-5 consensus ensures computation correctness
- **FHE Computations** - Fully homomorphic encryption support via TFHE-rs
- **Privacy Preserved** - Encrypted witness data, provers never see plaintext
- **Decentralized** - Permissionless prover network, no single point of failure
- **Automated Payments** - On-chain verification with trustless payment distribution
- **Censorship Resistant** - No central authority can block jobs or provers

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
│  │ (TFHE)   │  │ (TFHE)   │  │ (TFHE)   │ │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘ │
└───────│─────────────│─────────────│────────┘
        │ 3. Claim    │             │
        │ 4. Compute  │ Compute     │ Compute
        │    (async)  │ (async)     │ (async)
        ▼             ▼             ▼
┌────────────────────────────────────────────┐
│   Result Submission + Consensus            │
│   Prover A: hash_abc...                    │
│   Prover B: hash_abc... ✓ (match)          │
│   Prover C: hash_def... ✗ (mismatch)       │
└────────────────┬───────────────────────────┘
                 │ 5. 2/3 consensus reached
                 ▼
┌────────────────────────────────────────────┐
│   On-Chain Verification & Payment          │
│   - Pay Prover A & B (matching results)    │
│   - Penalize Prover C (dishonest)          │
│   - Return result to client                │
└────────────────────────────────────────────┘
```

## Technology Stack

- **Smart Contracts:** Solana (bare metal, no Anchor)
- **Prover Node:** Rust + TFHE-rs (FHE engine) + Halo2 (ZK circuits)
- **SDK:** Rust client library with 3-layer architecture
- **Consensus:** On-chain hash-based result verification
- **Encryption:** Post-quantum key exchange (ML-KEM)
- **UI:** Terminal User Interface (TUI) with ratatui

## Quick Links

- [Strategy](docs/STRATEGY.md) - Hackathon strategy and go-to-market plan
- [Approach](docs/APPROACH.md) - Why we chose wallet as demo
- [Roadmap](docs/ROADMAP.md) - 21-day development timeline
- [Pitch](docs/PITCH.md) - Hackathon pitch deck
- [Demo Script](docs/DEMO_SCRIPT.md) - 3-minute demonstration flow
- [Architecture](docs/ARCHITECTURE.md) - Technical deep dive
- [Tech Stack](docs/TECH_STACK.md) - Technologies and versions
- [Decision Log](docs/DECISION_LOG.md) - Key architectural decisions

## Repository Structure

```
zyberlink/
├── programs/              # Solana smart contracts
│   └── cypherlink/        # Main marketplace program
├── sdk/                   # Client SDK (Rust)
├── prover-node/           # Desktop prover daemon with TUI
├── blink-server/          # Solana Actions/Blinks server
├── shared/                # Shared types and utilities
│   ├── types/             # State definitions
│   └── crypto/            # Cryptographic utilities
├── e2e-tests/             # End-to-end integration tests
├── witness-storage/       # Encrypted witness storage server
├── demo/                  # Demo scripts and orchestration
├── scripts/               # Deployment and utility scripts
└── docs/                  # Documentation
```

## Status

**In Development** - Zypherpunk Hackathon (Nov 10 - Dec 1, 2025)

**Current Features:**
- [DONE]Multi-prover marketplace on Solana
- [DONE]FHE computation engine (TFHE-rs)
- [DONE]On-chain consensus algorithm
- [DONE]Terminal UI for prover monitoring
- [DONE]Interactive setup wizard
- [DONE]FHE E2E tests (14/14 self-executing tests passing)

See [ROADMAP.md](ROADMAP.md) for planned features and timeline.

## Getting Started

### Prerequisites

- Rust 1.75+
- Solana CLI 2.1+
- cargo (included with Rust)

### Quick Start

```bash
# Clone the repository
git clone https://github.com/yourusername/zyberlink.git
cd zyberlink

# Build all components
cargo build --release

# Run tests
cargo test --all

# Start local Solana validator (Terminal 1)
solana-test-validator --reset

# Deploy program (Terminal 2)
solana program deploy target/deploy/cypherlink.so

# Run interactive prover setup wizard (Terminal 3)
cd prover-node
cargo run --release -- wizard

# Or run demo with multiple provers (Terminal 3)
cd demo
./run-demo.sh
```

See [demo/QUICKSTART.md](demo/QUICKSTART.md) for detailed demo instructions.

## Development

### Running Tests

```bash
# Unit tests
cargo test

# Integration tests
cargo test --test integration_tests

# E2E tests (requires running validator + prover)
./scripts/test-e2e.sh
```

### Building for Production

```bash
# Build optimized binaries
cargo build --release --all

# Build specific components
cargo build --release -p cypherlink-sdk
cargo build --release -p prover-node
cargo build --release -p blink-server
```

## Use Cases

### FHE-Powered Privacy Applications
- **Private DeFi:** Encrypted balance swaps without revealing amounts
- **Confidential DAOs:** Private voting with verifiable results
- **Privacy-Preserving Analytics:** Compute on encrypted datasets
- **Secure Multi-Party Computation:** Distributed computation without trust

### ZK Proof Infrastructure
- **Wallet Proving:** Offload mobile ZK proof generation to network
- **Proof Aggregation:** Batch multiple proofs efficiently
- **Circuit Marketplaces:** Provers advertise specialized circuit support
- **Cross-Chain Privacy:** Bridge ZK proofs across L1s/L2s

## Business Model

- **10% platform fee** on all proving jobs
- User pays ~$0.02 per proof
- Prover earns ~$0.018 per proof
- Platform earns ~$0.002 per proof

**Example:** 10,000 proofs/day = $200 daily volume = $20 platform revenue

## Roadmap

### Phase 1: Multi-Prover Consensus (Current - Nov 2025)
- [DONE]Solana marketplace program
- [DONE]FHE computation engine (TFHE-rs)
- [DONE]Terminal UI for prover monitoring
- [DONE]Complete FHE E2E integration
- [DONE]On-chain consensus finalization

### Phase 2: Light Protocol Integration (Post-Hackathon)
- 🔜 ZK Compression for state management
- 🔜 Reduced on-chain storage costs
- 🔜 Scalable job history

### Phase 3: Advanced Features (Q1 2026)
- 🔮 Hardware acceleration (GPU/FPGA support)
- 🔮 Mobile wallet SDK integration
- 🔮 Reputation-weighted prover selection (SAS)
- 🔮 Dynamic pricing mechanisms
- 🔮 Circuit marketplace

### Phase 4: Production Deployment (Q2 2026)
- 🔮 Security audit
- 🔮 Mainnet deployment
- 🔮 Developer grants program
- 🔮 Enterprise API tier

## Contributing

This project is currently in hackathon development mode. After December 1, 2025, we will open for community contributions.

For now, if you're interested in:
- Running a prover node (beta testing)
- Integration partnerships
- Contributing to the codebase

Please reach out via GitHub Discussions or open an issue.

## Related Repositories

- [zypherbunk_hack](../zypherbunk_hack/) - Research and planning documentation
- Additional repos will be listed as project expands

## License

MIT OR Apache-2.0

Licensed under either of:
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option.

## Acknowledgments

- **Solana Foundation** - For the high-performance blockchain layer
- **ZAMA (TFHE-rs)** - For the FHE library powering encrypted computations
- **Zcash Foundation** - For ZK proving research and Halo2 circuits
- **Zypherpunk Hackathon** - For the catalyst to build this infrastructure

---

Built with privacy, powered by decentralization.
