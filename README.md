# ZyberLink

**Decentralized Privacy-Preserving Compute Marketplace on Solana**

> First FHE marketplace with multi-prover consensus - compute on encrypted data without trusting anyone

[![Status](https://img.shields.io/badge/status-v0.0.2--zyb-blue)]()
[![Tests](https://img.shields.io/badge/tests-55%2F56%20passing-green)]()
[![Season](https://img.shields.io/badge/season-S0%20Solana%20SZN-purple)]()

## Devnet Programs

| Program | Address | Explorer |
|---------|---------|----------|
| zyberlink | `GVCw9MYL6YywDPwsQkqC3xsvw5ETRH7KGkgxCDpeYfeu` | [View](https://explorer.solana.com/address/GVCw9MYL6YywDPwsQkqC3xsvw5ETRH7KGkgxCDpeYfeu?cluster=devnet) |
| bedrock | `Di2Tu6aNpJpPyxbMAasoLQU2yLqYMFWvUoV7cq7sXfvx` | [View](https://explorer.solana.com/address/Di2Tu6aNpJpPyxbMAasoLQU2yLqYMFWvUoV7cq7sXfvx?cluster=devnet) |
| futarchy_markets | `AQUUuRSwDhB1eeC2Caa8GPVGV4YzZkJ1YiSvZd3BBPij` | [View](https://explorer.solana.com/address/AQUUuRSwDhB1eeC2Caa8GPVGV4YzZkJ1YiSvZd3BBPij?cluster=devnet) |
| fhe_generator | `C8PpHFCKZ4F2Szbir2EMS4S4H3mwQqHNWUXK1N21nfAB` | [View](https://explorer.solana.com/address/C8PpHFCKZ4F2Szbir2EMS4S4H3mwQqHNWUXK1N21nfAB?cluster=devnet) |
| zk_generator | `Dzvy1pzCBgtMw5Fte2GybpeN2PsLPW8t7zDvLfKpxnSS` | [View](https://explorer.solana.com/address/Dzvy1pzCBgtMw5Fte2GybpeN2PsLPW8t7zDvLfKpxnSS?cluster=devnet) |
| threshold | `Apstq5JFS67Gzb5Yc2cMDuwx8JQoMJ9h2KvDYyCZA9Wh` | [View](https://explorer.solana.com/address/Apstq5JFS67Gzb5Yc2cMDuwx8JQoMJ9h2KvDYyCZA9Wh?cluster=devnet) |

## Live Demo

**https://demo.zyberlink.fun**

## Documentation

| Resource | Path | Description |
|----------|------|-------------|
| GitBook Docs | [/docs/book](docs/book) | Full documentation (EN/ES) |
| Zyb CLI (FHE) | [/src/zyb-cli](src/zyb-cli) | Client-side encryption tool |
| Rust SDK | [/sdks/rust](sdks/rust) | Integration library for Rust apps |
| API Reference | [/docs/book/en/guides/api-reference.md](docs/book/en/guides/api-reference.md) | Backend API endpoints |

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
zyb dev-job plan   # Dry-run (uses dev-job.toml)

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

### dev-job.toml (Config)

Create `dev-job.toml` in the repo root to set defaults for `zyb dev-job`:

```toml
[common]
profile = "local"
rpc_url = "http://localhost:8899"
program_id = "HnRTpCx7Xs3f1BKkVhkeZwcqRfPmSDpVQ6QxgN7Vm8Rt"
backend_url = "http://localhost:3000"
keypair = "/tmp/job-creator-keypair.json"
no_airdrop = false
json = true

[run]
cases = ["mix"]          # or types = ["add","sum","average"]
interval_secs = 10
once = false
shuffle = false

[verify]
types = ["sum", "count-if"]
all = false

[webapp]
verify = false

[profiles.local]
rpc_url = "http://localhost:8899"
backend_url = "http://localhost:3000"

[profiles.devnet]
rpc_url = "https://api.devnet.solana.com"
backend_url = "https://demo.zyberlink.fun"
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
## Zyb CLI (FHE)

Encrypt data locally before sending to the marketplace. Full docs: [/src/zyb-cli](src/zyb-cli)

```bash
# Build
cargo build --release -p zyb-cli

# Encrypt values
./target/release/zyb fhe encrypt --values 100,200,300,400,500

# Output files in ./fhe-output:
# - client_key.bin (SECRET - keep local for decryption)
# - witness.bin (upload this to marketplace)

# Decrypt result after job completes
./target/release/zyb fhe decrypt -p ./fhe-output -r "BASE64_RESULT"
```

## SDK Integration

Rust library for programmatic integration. Path: [/sdks/rust](sdks/rust)

```rust
use zyberlink_sdk::ZyberClient;

// Initialize client
let client = ZyberClient::new(rpc_url, api_url);

// Create FHE job
let job_id = client.create_job(
    encrypted_data,
    server_key,
    Operation::Sum,
    price_lamports,
).await?;

// Poll for result
let result = client.wait_for_result(job_id).await?;
```

See [SDK Integration Guide](docs/book/en/guides/sdk-integration.md) for full documentation.

## Dev Job Runner (Testing)

Create and verify jobs programmatically.

```bash
# Build (zyb-cli)
cargo build --release -p zyb-cli

# Run PoI test (CountIf >= 18 on [15,20,25,17] → expects 2)
./target/release/zyb dev-job verify --types count-if

# Run Sum test (Sum [10,20,30] → expects 60)
./target/release/zyb dev-job verify --types sum
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
├── zyb-cli/         # Unified CLI (FHE/ZK/dev-job)
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
