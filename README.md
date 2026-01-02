# ZyberLink Root

**Status:** Active Development
**License:** Apache 2.0 (Open Source)

ZyberLink is a decentralized privacy-preserving compute marketplace on Solana, enabling Fully Homomorphic Encryption (FHE) and Zero-Knowledge (ZK) computations.

## Repository Structure

This project is a monorepo containing the following workspaces:

### Core Infrastructure
- **[zyb-chain](zyb-chain/)** - Blockchain abstraction layer and smart contracts (Solana, Starknet).
- **[zyb-kernel](zyb-kernel/)** - Core primitives, FHE engine wrapper, and shared types.
- **[zyb-compute](zyb-compute/)** - The **Prover Node** software responsible for executing FHE/ZK jobs.
- **[zyb-circuits](zyb-circuits/)** - ZK circuits (Circom) for Market, Vote, and Proof of Innocence.

### Services & Backend
- **[zyb-services](zyb-services/)** - Backend microservices including `blink-server` and `public-api`.
- **[zyb-cli](zyb-cli/)** - Unified CLI tool (`zyb`) for encryption, key management, and job debugging.

### Frontend & Apps
- **[zyb-apps](zyb-apps/)** - User-facing applications (Webapp, Wallet Extension, Design System).
- **[zyb-sdks](zyb-sdks/)** - Client SDKs for Governance, Identity, and DeFi integration.

### Platform
- **[zyb-platform](zyb-platform/)** - Deployment configurations, Docker setups, and integration tests.

## Documentation

- **[Private Docs](private/)** - Internal strategic plans and gap analysis.
- **[Prover Node](zyb-compute/README.md)** - How to run a node.
- **[Webapp](zyb-apps/webapp/README.md)** - Frontend development.

## Quick Start

### 1. Prerequisites
- Rust 1.75+
- Node.js 18+ & pnpm
- Docker & Docker Compose
- Solana CLI

### 2. Run Local Environment
See `zyb-services/README.md` for detailed instructions on starting the local stack.

```bash
# Example (check specific makefiles in zyb-platform)
make c1  # Start containers
```

## Contributing

Please check the `CONTRIBUTING.md` file (if available) or refer to specific sub-directories for development guidelines.