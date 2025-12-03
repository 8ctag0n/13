# ZyberLink

**Decentralized Privacy-Preserving Compute Marketplace on Solana**

> First FHE marketplace with multi-prover consensus - compute on encrypted data without trusting anyone

[![Status](https://img.shields.io/badge/status-v0.0.1--zypherpunk-blue)]()
[![Tests](https://img.shields.io/badge/tests-55%2F56%20passing-green)]()
[![Season](https://img.shields.io/badge/season-S0%20Solana%20SZN-purple)]()

## Live Demo

**https://demo.zyberlink.fun**

## Overview

ZyberLink enables computations on encrypted data using Fully Homomorphic Encryption (FHE). Multiple independent provers compute on ciphertext, with on-chain consensus ensuring correctness. Users' data is **never** exposed - not even to the compute nodes.

**Repository:** https://github.com/8ctag0n/13/

## Key Features

- **Multi-Prover Consensus** - 2-of-3 or 3-of-5 verification eliminates single point of trust
- **FHE Operations** - Sum, Average, CountIf, Threshold, Histogram via TFHE-rs
- **Privacy Preserved** - Data encrypted client-side, provers never see plaintext
- **On-Chain Settlement** - Solana smart contracts handle payments and verification
- **Smart Cleanup** - Automatic witness data deletion after job completion

## Quick Start

### Localnet (Development)
```bash
make c1   # Start containers (validator + postgres + backend + nginx)
make c2   # Initialize marketplace + register provers
make c3   # Start provers

# Run tests
make e2e-poi   # Proof of Innocence test
make e2e-sum   # Census Sum test

make c0   # Stop all
```

### Devnet (Production-like)
```bash
# Setup (one time)
./scripts/setup-devnet.sh --funder ./keypair-with-sol.json

make d1   # Start containers (no validator, uses devnet)
make d2   # Initialize marketplace
make d3   # Start provers

make d0   # Stop all
```

## Architecture

```
┌─────────────┐     ┌──────────────┐     ┌─────────────┐
│   Client    │────▶│   Backend    │────▶│   Provers   │
│  (SDK/CLI)  │     │  (Rust API)  │     │  (FHE/ZK)   │
└─────────────┘     └──────────────┘     └─────────────┘
       │                   │                    │
       │            ┌──────┴──────┐            │
       │            │  PostgreSQL │            │
       │            │  (Witnesses)│            │
       │            └─────────────┘            │
       │                                       │
       └──────────────┬────────────────────────┘
                      ▼
              ┌───────────────┐
              │    Solana     │
              │  (Jobs/Pay)   │
              └───────────────┘
```

## Use Cases

### Proof of Innocence
Verify wallet has no sanctioned interactions without revealing transaction history:
```
CountIf(sanctions_list, == user_id) → 0 means innocent
```

### Private Analytics
Aggregate encrypted data without seeing individual values:
```
Sum([encrypted_values]) → total without exposure
```

### Age Verification
Prove age >= 18 without revealing exact age:
```
Threshold(encrypted_age, >= 18) → true/false
```

## Tech Stack

| Component | Technology |
|-----------|------------|
| FHE Engine | TFHE-rs 0.10 |
| ZK Proofs | Halo2 |
| Blockchain | Solana |
| Backend | Rust + Axum |
| Frontend | Svelte |
| Database | PostgreSQL |

## Project Structure

```
src/
├── programs/        # Solana smart contracts
├── sdk/             # Rust client library
├── prover-node/     # Prover daemon
├── blink-server/    # Backend API
├── webapp/          # Frontend (Svelte)
├── fhe-cli/         # CLI encryption tool
└── shared/          # Shared types
```

## Roadmap

| Season | Focus | Status |
|--------|-------|--------|
| **S0** | Solana Foundation | Active |
| S0.5 | Operational Improvements | Planned |
| S1 | Multi-chain Interoperability | Planned |
| S1.5 | Third-party Adoption | Future |
| S2 | Consumer Applications | Vision |

See [ROADMAP.md](ROADMAP.md) for details.

## Status

- **Tests:** 55/56 passing (98%)
- **E2E PoI CountIf:** Passing
- **E2E Census Sum:** Passing

## Commands Reference

| Command | Description |
|---------|-------------|
| `make c0` | Stop localnet |
| `make c1` | Start localnet containers |
| `make c2` | Init marketplace + register provers |
| `make c3` | Start provers |
| `make d0-d3` | Same for devnet |
| `make e2e-poi` | Run PoI test |
| `make e2e-sum` | Run Sum test |
| `make help` | Full command list |

## License

MIT

---

**Zypherpunk Hackathon** - December 2025
