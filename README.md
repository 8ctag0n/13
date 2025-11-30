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

---

## Demo Features (Hackathon)

### Private Analytics (Zcash-ready)
Compute aggregations over encrypted transaction data without revealing amounts.

**Supported Operations:**
- `Sum` - Total of encrypted values
- `Average` - Mean of encrypted values
- `CountIf` - Count values matching a predicate

**Flow:**
1. User encrypts transaction amounts locally with `fhe-cli encrypt`
2. Uploads `witness.bin` to webapp
3. Selects operation (Sum/Average)
4. Multi-prover network computes on encrypted data
5. User decrypts final result locally

### Proof of Innocence (FHE)
Verify wallet has no interactions with sanctioned addresses - without revealing transaction history.

**How it works:**
1. User encrypts their transaction counterparty addresses
2. Backend runs `count_if` with `EqualTo` predicate against OFAC list
3. If result = 0, wallet is verified innocent
4. User never reveals their actual transaction data

### Verification Panel
Visual consensus verification for FHE jobs:

- **Witness Hash** - Blake2s256 hash of input data (proves all provers worked on same input)
- **Result Hash** - Blake2s256 hash of encrypted output (deterministic commitment)
- **Prover Table** - Shows each prover's commitment and match status
- **Consensus Badge** - N-of-M consensus indicator

---

## FHE CLI Tool

The `fhe-cli` is the user's offline encryption/decryption tool. **No WASM in browser** - all FHE operations happen locally.

### Installation

```bash
# Build from source
cd src/fhe-cli
cargo build --release

# Binary at: target/release/fhe-cli
```

### Commands

#### Encrypt Single Value
```bash
# Interactive
fhe-cli encrypt

# With value
fhe-cli encrypt -v 42

# Custom output path
fhe-cli encrypt -v 42 -p ./my-job
```

#### Encrypt Multiple Values (for Sum/Average)
```bash
# Comma-separated values
fhe-cli encrypt --values 10,20,30,40,50

# For analytics demo
fhe-cli encrypt --values 100,250,75,300,125
```

#### Decrypt Result
```bash
# Interactive (prompts for base64 result)
fhe-cli decrypt -p ./fhe-output

# With result
fhe-cli decrypt -p ./fhe-output -r "base64_encrypted_result..."
```

### Generated Files

| File | Size | Purpose |
|------|------|---------|
| `client_key.bin` | ~2.5 MB | **SECRET** - Keep locally for decryption |
| `server_key.bin` | ~18 MB | Public - Sent to provers |
| `encrypted_data.bin` | ~200 bytes | Encrypted values |
| `witness.bin` | ~18 MB | **Upload this** - Combined package for job creation |
| `metadata.json` | ~200 bytes | Job metadata (original values for reference) |

### Witness Format

The `witness.bin` file combines server key and encrypted data:

```
┌─────────────────────────────────────────────────────┐
│ encrypted_data_len (4 bytes, u32 LE)                │
├─────────────────────────────────────────────────────┤
│ encrypted_data (variable length)                    │
├─────────────────────────────────────────────────────┤
│ server_key (bincode serialized, ~18 MB)             │
└─────────────────────────────────────────────────────┘
```

---

## API Endpoints

### Job Management

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/api/jobs/create` | Create new FHE job |
| `GET` | `/api/jobs/{id}` | Get job details |
| `GET` | `/api/jobs/{id}/status` | Get job status |
| `GET` | `/api/jobs/{id}/result` | Get job result (when completed) |
| `GET` | `/api/jobs/list` | List all jobs |
| `GET` | `/api/jobs/by-wallet/{pubkey}` | Jobs by wallet |

### Job Creation Request

```json
{
  "operation": "sum",
  "predicate": null,
  "server_key": "base64...",
  "encrypted_data": "base64...",
  "signature": "base64...",
  "user_pubkey": "solana_pubkey",
  "payment_method": "sol",
  "price_lamports": 1000000,
  "required_provers": 3,
  "consensus_threshold": 2
}
```

### Operations

| Operation | Predicate | Description |
|-----------|-----------|-------------|
| `sum` | - | Sum all encrypted values |
| `average` | - | Average of encrypted values |
| `count_if` | `EqualTo(n)` | Count values equal to n |
| `count_if` | `GreaterThan(n)` | Count values > n |
| `count_if` | `LessThan(n)` | Count values < n |

---

## Webapp Pages

| Route | Page | Description |
|-------|------|-------------|
| `/` | Landing | Hero + features |
| `/#dashboard` | Dashboard | Quick actions + recent jobs |
| `/#analytics` | Analytics | Private sum/average computations |
| `/#proof-of-innocence` | PoI | Sanctions verification |
| `/#create-job` | CreateJob | Full custom job creation |
| `/#my-jobs` | MyJobs | User's job history |
| `/#job-details/{id}` | JobDetails | Single job details |

---

## Demo Quickstart

```bash
# 1. Start local environment
make localnet-setup && make localnet-init && make localnet-run

# 2. Generate test data (in another terminal)
cd src/fhe-cli
cargo run --release -- encrypt --values 100,200,150,300,250

# 3. Open webapp
open http://localhost:5173

# 4. Go to Analytics page
#    - Upload witness.bin from ./fhe-output
#    - Select "Sum" operation
#    - Connect wallet and submit

# 5. After job completes, decrypt
cargo run --release -- decrypt -p ./fhe-output
```

## License

MIT OR Apache-2.0

## Acknowledgments

- **Solana Foundation** - High-performance blockchain
- **ZAMA (TFHE-rs)** - FHE library
- **Zcash Foundation** - Halo2 circuits
- **Zypherpunk Hackathon** - Catalyst for this project
