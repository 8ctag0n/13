# System Overview

ZyberLink is a decentralized marketplace for privacy-preserving computations on Solana.

## Core Concept

Applications can offload compute-intensive FHE (Fully Homomorphic Encryption) and ZK (Zero-Knowledge) operations to a network of independent provers. On-chain consensus ensures correctness, and payments are automated.

**Key Innovation:** Multi-prover consensus eliminates single points of failure while maintaining cryptographic security guarantees.

## High-Level Architecture

```
┌──────────────────────────────────────────────────────────┐
│                    Client Application                    │
│  (Mobile app, web app, backend service, etc.)            │
└────────────────────┬─────────────────────────────────────┘
                     │
                     │ 1. Create encrypted job
                     │
                     ▼
┌──────────────────────────────────────────────────────────┐
│              Solana Marketplace Program                  │
│  - Job queue management                                  │
│  - Prover registry                                       │
│  - Consensus verification                                │
│  - Payment distribution                                  │
└────────────────────┬─────────────────────────────────────┘
                     │
                     │ 2. Job broadcast
                     │
        ┌────────────┼────────────┐
        │            │            │
        ▼            ▼            ▼
┌──────────┐  ┌──────────┐  ┌──────────┐
│ Prover A │  │ Prover B │  │ Prover C │
│  (FHE)   │  │  (FHE)   │  │  (FHE)   │
└────┬─────┘  └────┬─────┘  └────┬─────┘
     │             │             │
     │ 3. Execute independently   │
     │             │             │
     ▼             ▼             ▼
  result_A     result_B     result_C
     │             │             │
     └─────────────┼─────────────┘
                   │
                   ▼
┌──────────────────────────────────────────────────────────┐
│            Consensus Verification                        │
│  If 2+ provers agree → Accept & Pay                     │
│  If mismatch → Penalize dishonest prover                │
└──────────────────────────────────────────────────────────┘
```

## System Components

### 1. Client SDK

**Purpose:** Simplified interface for applications to submit jobs and retrieve results.

**Key Features:**
- Type-safe job creation
- Automatic encryption of sensitive data
- Result verification
- Payment handling

**Supported Languages:**
- Rust (primary)
- JavaScript/TypeScript (planned)
- Python (planned)

### 2. Solana Marketplace Program

**Purpose:** Coordinate job distribution, consensus, and payments.

**Core Functions:**
- **Job Queue:** Manages pending computations
- **Prover Registry:** Tracks available provers and their capabilities
- **Consensus Engine:** Verifies multi-prover agreement
- **Escrow System:** Holds payments until job completion

**Written in:** Pure Rust (no Anchor framework) for maximum performance.

### 3. Prover Nodes

**Purpose:** Execute computations and submit results.

**Components:**
- **Job Monitor:** Watches for new jobs on-chain
- **FHE Engine:** TFHE-rs library for encrypted computations
- **ZK Module:** Halo2 for zero-knowledge proofs (future)
- **TUI Dashboard:** Real-time monitoring interface

**Hardware Requirements:**
- **Minimum:** 4 CPU cores, 8GB RAM
- **Recommended:** 8+ CPU cores, 16GB+ RAM
- **Future:** GPU/FPGA acceleration support

### 4. Consensus Mechanism

**How it works:**

1. **Job Posted:** Client encrypts data and posts job to marketplace
2. **Multiple Claims:** 3+ provers claim the same job
3. **Independent Execution:** Each prover computes result separately
4. **Result Submission:** Provers submit result hashes on-chain
5. **Consensus Check:** Program verifies 2-of-3 (or 3-of-5) agreement
6. **Payment Distribution:** Matching provers are paid; dishonest ones penalized

**Why Consensus?**
- **No trust required:** Don't need to trust any single prover
- **Censorship resistant:** No central authority can block jobs
- **Byzantine fault tolerant:** Works even with some dishonest provers

## Data Flow

### Example: FHE Addition (Encrypted 5 + 7)

**Step 1:** Client encrypts inputs
```
plaintext: 5 + 7
encrypted: enc(5) + enc(7)
```

**Step 2:** Create job on Solana
```
Job {
  id: "abc123",
  operation: FHE_ADD,
  encrypted_inputs: [enc(5), enc(7)],
  reward: 0.002 SOL,
  consensus_threshold: 2-of-3
}
```

**Step 3:** Provers claim and execute
```
Prover A: computes enc(5) + enc(7) = enc(12)
          submits hash(enc(12))

Prover B: computes enc(5) + enc(7) = enc(12)
          submits hash(enc(12))

Prover C: computes enc(5) + enc(7) = enc(12)
          submits hash(enc(12))
```

**Step 4:** Consensus verification
```
hash_A == hash_B == hash_C  →  Consensus reached (3/3)
```

**Step 5:** Payment distribution
```
Prover A: +0.00066 SOL
Prover B: +0.00066 SOL
Prover C: +0.00066 SOL
Platform: +0.0002 SOL (10% fee)
```

**Step 6:** Client retrieves result
```
encrypted result: enc(12)
client decrypts: 12 ✓
```

## Security Model

### Threat Model

**What we protect against:**
- ✅ Dishonest provers (consensus detects mismatches)
- ✅ Data leakage (all data encrypted end-to-end)
- ✅ Censorship (permissionless network)
- ✅ Single point of failure (distributed provers)

**What we don't protect against:**
- ❌ Client-side compromise (if client is hacked, keys are exposed)
- ❌ All provers colluding (requires Byzantine majority)
- ❌ Solana blockchain failure (inherent blockchain dependency)

### Encryption Guarantees

**Witness Encryption:**
- Client encrypts all inputs before posting job
- Uses post-quantum ML-KEM key exchange
- Provers never see plaintext

**Result Encryption:**
- FHE results remain encrypted throughout
- Only client can decrypt final result
- Even Solana program doesn't see plaintext

**Privacy Level:**
- **High:** Data never leaves encrypted form
- **Comparable to:** Local computation
- **Better than:** Centralized servers (no single point of trust)

## Performance Characteristics

### Latency

| Operation | Local | ZyberLink | Improvement |
|-----------|-------|-----------|-------------|
| FHE Addition | N/A (mobile can't run) | 5-10s | ∞ (enables mobile) |
| FHE Multiplication | N/A | 10-15s | ∞ (enables mobile) |
| ZK Proof (Halo2) | 120-180s | 15-25s | 6-10x faster |

### Throughput

- **Current:** ~100 jobs/hour per prover
- **Optimized:** ~1000 jobs/hour (with batching)
- **Network:** Scales linearly with provers

### Cost

- **Client pays:** ~$0.02 per FHE operation
- **Prover earns:** ~$0.018 per operation
- **Platform fee:** 10% (~$0.002)

## Scalability

### Current Limitations

- **FHE Performance:** ~39s per operation (optimization in progress)
- **On-Chain Storage:** Job state costs ~0.002 SOL rent
- **Network Size:** Limited by Solana TPS (~65k TPS theoretical)

### Future Improvements

**Phase 2: Light Protocol Integration**
- ZK Compression for state management
- Reduced on-chain storage costs (10-100x)
- Scalable job history

**Phase 3: Hardware Acceleration**
- GPU proving support
- FPGA co-processors
- 10-100x FHE performance boost

**Phase 4: Advanced Consensus**
- Reputation-weighted selection
- SLAs for enterprise clients
- Dynamic pricing mechanisms

## Technology Stack

**Blockchain:**
- Solana (high TPS, low fees)
- Custom program (no Anchor for max performance)

**Cryptography:**
- TFHE-rs (FHE engine by Zama)
- Halo2 (ZK circuits)
- ML-KEM (post-quantum encryption)

**Prover Node:**
- Rust (systems programming)
- Ratatui (TUI framework)
- Tokio (async runtime)

**Development:**
- Cargo workspaces (mono-repo)
- Bare metal Solana programs
- Comprehensive E2E tests

## Use Cases

### Immediate Applications

**1. Privacy-Preserving DeFi**
- Encrypted balance swaps
- Private order books
- Confidential auctions

**2. DAO Governance**
- Private voting with verifiable results
- Encrypted proposal scores
- Anonymous delegation

**3. Secure Analytics**
- Compute on encrypted datasets
- Private aggregate statistics
- Confidential business intelligence

### Future Applications

**4. Mobile ZK Wallets**
- Offload proof generation from phones
- Battery-friendly privacy
- Instant transactions

**5. Multi-Party Computation**
- Collaborative computation without trust
- Distributed data analysis
- Secure multi-party auctions

**6. Cross-Chain Privacy**
- Bridge ZK proofs between L1s/L2s
- Encrypted cross-chain messaging
- Private atomic swaps

## Next Steps

- **[FHE Design](fhe-design.md)** - Deep dive into encryption
- **[Tech Stack](tech-stack.md)** - Complete technology overview
- **[Quickstart](../getting-started/quickstart.md)** - Try it yourself

---

**ZyberLink Architecture:** Decentralized privacy infrastructure for Web3.
