# CypherLink / ZyberLink

**Decentralized ZK Compute Marketplace on Solana**

> "The fastest Zcash mobile wallet, powered by decentralized proving"

## Overview

CypherLink enables mobile devices to offload intensive ZK proof generation to powerful prover nodes, coordinated via Solana with ZK Compression. By decentralizing compute-intensive operations, we make privacy-preserving technologies practical for everyday mobile use.

**Demo Application:** CypherWallet - A Zcash mobile wallet that generates shielded transaction proofs in ~15 seconds (versus 2+ minutes with native proving), demonstrating the power of decentralized ZK compute.

## Key Features

- **10x Faster** - Proof generation in 15s vs 150s on-device
- **90% Less Battery** - 0.3% vs 3% battery consumption per transaction
- **Privacy Preserved** - Witness data encrypted end-to-end with post-quantum crypto
- **Decentralized** - No AWS dependency, permissionless prover network
- **Earn Passive Income** - Monetize idle desktop compute power by running a prover node

## Architecture

```
┌─────────────────┐
│  Mobile Client  │ (Flutter + Rust FFI)
│  (CypherWallet) │
└────────┬────────┘
         │ 1. Generate encrypted witness
         │ 2. Create proving job
         ▼
┌─────────────────┐
│ Solana Program  │ (ZK Compression via Light Protocol)
│  (Marketplace)  │
└────────┬────────┘
         │ 3. Job discovery
         │ 4. Claim job
         ▼
┌─────────────────┐
│  Prover Node    │ (Rust + Halo2)
│   (Desktop)     │
└────────┬────────┘
         │ 5. Generate ZK proof (~15s)
         │ 6. Submit proof
         ▼
┌─────────────────┐
│   Verification  │ (On-chain)
│   + Payment     │
└─────────────────┘
         │ 7. Verify + broadcast transaction
         ▼
    Zcash Network
```

## Technology Stack

- **Smart Contracts:** Solana (bare metal, no Anchor) + Light Protocol (ZK Compression)
- **Prover Node:** Rust + Halo2 (Orchard circuit)
- **Mobile Client:** Flutter + Rust FFI
- **Reputation:** Solana Attestation Service (SAS)
- **Encryption:** Post-quantum key exchange

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
├── prover-node/           # Desktop prover daemon
├── cypherlink-wallet/     # Flutter mobile wallet
├── shared/                # Shared types and utilities
├── tests/                 # Integration tests
├── scripts/               # Deployment and utility scripts
└── docs/                  # Documentation
```

## Status

**In Development** - Zypherpunk Hackathon (Nov 10 - Dec 1, 2025)

Current phase: Week 1 - Core Infrastructure

See [ROADMAP.md](docs/ROADMAP.md) for detailed timeline and milestones.

## Getting Started

### Prerequisites

- Rust 1.75+
- Solana CLI 1.18+
- Flutter 3.16+
- Node.js 18+ (for testing)

### Quick Start

```bash
# Clone the repository
git clone https://github.com/yourusername/zyberlink.git
cd zyberlink

# Install dependencies
./scripts/setup.sh

# Build all components
cargo build --release

# Run tests
cargo test --all

# Start local Solana validator
solana-test-validator --reset

# Deploy program (in new terminal)
./scripts/deploy-local.sh

# Start prover node
cd prover-node && cargo run --release

# Run mobile wallet (in new terminal)
cd cypherlink-wallet && flutter run
```

See individual component READMEs for detailed setup instructions.

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

# Build mobile app
cd cypherlink-wallet
flutter build apk --release  # Android
flutter build ios --release  # iOS
```

## Use Cases

### Current: Zcash Mobile Wallet
- Shielded transactions 10x faster
- Practical privacy for mobile users
- Battery-efficient proving

### Planned: Anonymous Voting
- DAO governance with privacy
- Verifiable but anonymous votes
- Mobile participation enabled

### Future: Private Credentials
- KYC without data exposure
- Portable identity proofs
- Compliance-friendly privacy

## Business Model

- **10% platform fee** on all proving jobs
- User pays ~$0.02 per proof
- Prover earns ~$0.018 per proof
- Platform earns ~$0.002 per proof

**Example:** 10,000 proofs/day = $200 daily volume = $20 platform revenue

## Contributing

This project is currently in hackathon development mode. After December 1, 2025, we will open for community contributions.

For now, if you're interested in:
- Running a prover node (beta testing)
- Testing the wallet (mobile beta)
- Partnership discussions

Please reach out via [contact method].

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

- **Zcash Foundation** - For the Orchard circuit and ZK research
- **Solana Foundation** - For the high-performance blockchain layer
- **Light Protocol** - For ZK Compression technology
- **Zypherpunk Hackathon** - For the catalyst to build this

---

Built with privacy, powered by decentralization.
