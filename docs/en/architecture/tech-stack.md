# Technology Stack

ZyberLink uses a carefully selected stack optimized for performance, security, and decentralization.

## Core Technologies

### Blockchain: Solana

**Why Solana:**
- **Fast:** Sub-second finality (400ms blocks)
- **Cheap:** ~$0.00025 per transaction
- **Scalable:** 3,000+ TPS sustained, 65k+ TPS theoretical
- **Parallel:** Sealevel runtime enables concurrent execution

**Alternative considered:** Ethereum L1/L2, but too slow and expensive for high-frequency computation marketplace.

### Smart Contracts: Bare Metal Rust

**Not using Anchor framework** - Direct solana_program for:
- Maximum performance
- Smaller binary size
- Better optimization control
- Easier integration with advanced features (Light Protocol)

**Dependencies:**
```toml
solana-program = "1.18"
borsh = "1.5"  # Serialization
```

## Cryptography

### FHE Engine: TFHE-rs (by Zama)

**Purpose:** Fully Homomorphic Encryption computations

**Why TFHE-rs:**
- ✅ Production-ready library
- ✅ Maintained by Zama (FHE experts)
- ✅ Comprehensive operation support
- ✅ Good performance characteristics
- ✅ Active development

**Operations supported:**
- Integer arithmetic (add, multiply, subtract)
- Boolean logic (AND, OR, NOT)
- Comparisons (greater than, less than, equal)
- Future: Advanced circuits (division, modulo, etc.)

**Version:** 0.6+

**Alternative considered:** Concrete, SEAL, HElib - TFHE-rs offers best Rust integration.

### ZK Circuits: Halo2 (Future)

**Purpose:** Zero-knowledge proof generation

**Why Halo2:**
- No trusted setup required
- Recursion support
- Efficient proof generation
- Used by Zcash (battle-tested)

**Status:** Planned for Phase 2

### Post-Quantum Encryption: ML-KEM

**Purpose:** Key exchange for witness encryption

**Why ML-KEM (Kyber):**
- NIST-selected post-quantum standard
- Future-proof against quantum attacks
- Good performance on mobile

**Library:** `pqcrypto-kyber`

## Prover Node Stack

### Language: Rust

**Why Rust:**
- Memory safety without garbage collection
- Zero-cost abstractions
- Excellent async support (Tokio)
- Direct FFI with cryptography libraries
- Cargo ecosystem

**Version:** Rust 1.75+

### Async Runtime: Tokio

**Purpose:** Handle concurrent job monitoring, RPC calls, computation

```toml
tokio = { version = "1", features = ["full"] }
```

### UI Framework: Ratatui

**Purpose:** Terminal User Interface for prover monitoring

**Features:**
- Real-time dashboard
- Job status updates
- Earnings tracking
- Network statistics

**Why TUI over GUI:**
- Lightweight (runs on servers)
- SSH-friendly (remote management)
- Resource efficient
- Developer-friendly

```toml
ratatui = "0.24"
crossterm = "0.27"
```

### RPC Client: solana-client

**Purpose:** Interact with Solana blockchain

```toml
solana-client = "1.18"
solana-sdk = "1.18"
```

## SDK Stack

### Primary Language: Rust

**Client SDK structure:**
```
sdk/
├── Cargo.toml
└── src/
    ├── client.rs         # MarketplaceClient
    ├── instructions.rs   # Instruction builders
    ├── state.rs          # Account deserializers
    └── crypto.rs         # Encryption utilities
```

**Future SDKs:**
- JavaScript/TypeScript (web/mobile)
- Python (data science, ML)
- Go (backend services)

## Development Tools

### Build System: Cargo

**Workspace structure:**
```toml
[workspace]
members = [
    "programs/zyberlink",
    "sdk",
    "prover-node",
    "shared/types",
    "shared/crypto",
    "e2e-tests",
]
```

### Testing: Rust + solana-program-test

**Test types:**
- Unit tests (per component)
- Integration tests (SDK + program)
- E2E tests (full workflow simulation)

```bash
# Run all tests
cargo test --all

# E2E tests with local validator
./scripts/test-e2e.sh
```

### CI/CD: GitHub Actions

**Automated checks:**
- Build verification
- Test execution
- Code formatting (rustfmt)
- Linting (clippy)

## Dependencies Overview

### Production Dependencies

**Blockchain:**
- `solana-program` 1.18
- `solana-client` 1.18
- `solana-sdk` 1.18

**Cryptography:**
- `tfhe` 0.6
- `pqcrypto-kyber` 0.7
- `sha2` 0.10

**Serialization:**
- `borsh` 1.5
- `serde` 1.0
- `bincode` 1.3

**Async/Networking:**
- `tokio` 1.35
- `reqwest` 0.11

**UI:**
- `ratatui` 0.24
- `crossterm` 0.27

### Development Dependencies

- `solana-program-test` (testing)
- `proptest` (property testing)
- `criterion` (benchmarking)

## Performance Characteristics

### Binary Sizes

| Component | Size (optimized) |
|-----------|------------------|
| Solana Program | ~250 KB |
| Prover Node | ~15 MB |
| SDK Library | ~5 MB |

### Resource Usage

**Prover Node (per instance):**
- **CPU:** 1-4 cores active during computation
- **RAM:** 2-4 GB typical, 8 GB max
- **Disk:** <100 MB for binaries, variable for job cache
- **Network:** ~1-5 MB/hour (job monitoring + submission)

### Compilation Times

**First build:**
- Full workspace: 5-10 minutes
- Program only: 1-2 minutes
- SDK only: 30-60 seconds

**Incremental builds:**
- ~10-30 seconds (typical code changes)

## Future Stack Additions

### Phase 2: State Compression

**Light Protocol SDK:**
```toml
light-sdk = "0.9"
```

**Benefits:**
- 50x cheaper state storage
- Scalable job history
- Efficient prover registry

### Phase 3: Hardware Acceleration

**GPU Support:**
- CUDA for NVIDIA GPUs
- ROCm for AMD GPUs
- ~10-100x FHE performance boost

**FPGA Support:**
- Custom FHE accelerators
- Verilog/VHDL integration

### Phase 4: Advanced Features

**MPC Integration:**
- `mpc-coalition` library
- Multi-party computation protocols

**TEE Support:**
- Intel SGX
- AMD SEV
- ARM TrustZone

## Version Requirements

**Minimum versions:**
- Rust: 1.75
- Solana CLI: 2.1
- Cargo: 1.75 (comes with Rust)

**Recommended:**
- Rust: Latest stable
- Solana CLI: Latest stable
- OS: Linux (Ubuntu 22.04+) or macOS 13+

## Build Configuration

### Release Profile

```toml
[profile.release]
opt-level = 3
lto = true
codegen-units = 1
strip = true
```

Optimized for:
- Maximum performance
- Minimum binary size
- Production deployment

### Development Profile

```toml
[profile.dev]
opt-level = 0
debug = true
```

Fast compilation for rapid iteration.

## Security Considerations

**Dependencies:**
- Regularly updated (Dependabot)
- Vetted for security (cargo-audit)
- Minimal supply chain attack surface

**Cryptography:**
- Use only audited libraries (TFHE-rs, Kyber)
- No custom crypto implementations
- Follow NIST/industry standards

**Smart Contracts:**
- Bare metal (no complex frameworks)
- Auditable code paths
- Defensive programming

## Next Steps

- **[FHE Design](fhe-design.md)** - Deep dive into encryption implementation
- **[System Overview](overview.md)** - Complete architecture
- **[Quickstart](../getting-started/quickstart.md)** - Build and run ZyberLink

---

**Stack Philosophy:** Battle-tested, performant, secure.
