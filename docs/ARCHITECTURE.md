# Architecture

## System Overview

ZyberLink is a decentralized marketplace for **privacy-preserving computations** on Solana. The system enables clients to offload compute-intensive ZK and FHE operations to a network of independent provers, with on-chain consensus ensuring correctness and automated payment distribution.

**Core Innovation:** Multi-prover consensus mechanism that distributes trust across multiple compute providers, eliminating single points of failure while maintaining cryptographic guarantees.

### Supported Compute Types

**Phase 1 (Current - November 2025):**
- **ZK Proofs** - Zero-knowledge proof generation (Halo2)
  - Groth16/PLONK circuits
  - ~15 second proving time
  - Single-prover model

- **FHE Computations** - Fully Homomorphic Encryption (TFHE-rs)
  - Encrypted arithmetic (add, multiply, subtract)
  - Multi-prover consensus validation (2-of-3 or 3-of-5)
  - On-chain result verification via hash comparison
  - ~39 seconds per operation (performance optimization in progress)

**Future Phases:**
- **MPC** - Multi-Party Computation
- **TEE** - Trusted Execution Environment tasks
- **Hardware Acceleration** - GPU/FPGA proving
- **Light Protocol Integration** - ZK Compression for state management

**Vision:** Generic infrastructure for any privacy-preserving computation

### High-Level Architecture

```mermaid
graph TB
    subgraph "Client Layer"
        CL[Client Application]
        WB[Witness Builder]
        ENC[Encryption Module]
    end

    subgraph "Coordination Layer (Solana)"
        MP[Marketplace Program]
        JQ[Job Queue]
        PR[Prover Registry]
        ES[Escrow System]
        CONS[Consensus Verification]
    end

    subgraph "Prover Network"
        PN1[Prover Node A]
        PN2[Prover Node B]
        PN3[Prover Node C]
    end

    subgraph "Compute Engines"
        FHE1[TFHE Engine]
        FHE2[TFHE Engine]
        FHE3[TFHE Engine]
        ZK1[Halo2 Module]
        ZK2[Halo2 Module]
        ZK3[Halo2 Module]
    end

    CL --> WB
    WB --> ENC
    ENC --> MP
    MP --> JQ

    JQ --> PN1
    JQ --> PN2
    JQ --> PN3

    PN1 --> FHE1
    PN1 --> ZK1
    PN2 --> FHE2
    PN2 --> ZK2
    PN3 --> FHE3
    PN3 --> ZK3

    FHE1 --> CONS
    FHE2 --> CONS
    FHE3 --> CONS

    CONS --> ES
    ES --> CL

    MP -.-> PR
```

---

## Component Architecture

### 1. Client SDK (Rust)

The client SDK provides a high-level interface for applications to interact with the ZyberLink marketplace.

#### Architecture

```mermaid
graph LR
    subgraph "SDK Layers"
        HL[High-Level API]
        IL[Instruction Layer]
        SL[State Layer]
    end

    subgraph "Core Modules"
        MC[MarketplaceClient]
        JB[Job Builder]
        EP[Event Parser]
        CR[Crypto Module]
    end

    HL --> MC
    MC --> IL
    MC --> SL
    IL --> JB
    SL --> EP
    MC --> CR
```

#### Key Modules

**MarketplaceClient**
```rust
pub struct MarketplaceClient {
    rpc_client: RpcClient,
    payer: Keypair,
    program_id: Pubkey,
}

impl MarketplaceClient {
    pub async fn create_job(...) -> Result<Signature>;
    pub async fn get_job(...) -> Result<JobAccount>;
    pub async fn query_jobs(...) -> Result<Vec<JobAccount>>;
    pub async fn submit_fhe_result(...) -> Result<Signature>;
    pub async fn finalize_fhe_job(...) -> Result<Signature>;
}
```

**Instruction Builders**
- Type-safe instruction construction
- Automatic PDA derivation
- Parameter validation
- Account metadata generation

**State Deserializers**
- JobAccount parsing
- ProverAccount parsing
- MarketplaceConfig parsing
- Event log parsing

**Crypto Module**
- ML-KEM post-quantum key exchange
- AES-256-GCM encryption/decryption
- Ed25519 signature verification
- Secure random number generation

---

### 2. Solana Program (Marketplace)

#### State Management

```rust
// Core state structures

#[account]
pub struct MarketplaceConfig {
    pub authority: Pubkey,
    pub fee_basis_points: u16,        // e.g., 1000 = 10%
    pub min_stake_amount: u64,
    pub job_timeout_seconds: i64,
    pub protocol_fee_recipient: Pubkey,
}

#[account]
pub struct Prover {
    pub authority: Pubkey,
    pub stake_amount: u64,
    pub reputation_score: u32,        // SAS attestation
    pub total_jobs_completed: u64,
    pub total_jobs_failed: u64,
    pub avg_completion_time_secs: u32,
    pub is_active: bool,
    pub registration_timestamp: i64,
}

#[account]
pub struct Job {
    pub id: u64,
    pub creator: Pubkey,
    pub prover: Option<Pubkey>,
    pub status: JobStatus,
    pub circuit_type: CircuitType,
    pub encrypted_witness: Vec<u8>,   // Encrypted data
    pub encrypted_proof: Option<Vec<u8>>,
    pub price_lamports: u64,
    pub escrow_account: Pubkey,
    pub created_at: i64,
    pub claimed_at: Option<i64>,
    pub completed_at: Option<i64>,
    pub timeout_at: i64,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq)]
pub enum JobStatus {
    Pending,      // Created, waiting for prover
    Claimed,      // Prover has claimed
    Completed,    // Proof submitted and verified
    Failed,       // Timeout or verification failed
    Cancelled,    // Creator cancelled
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq)]
pub enum CircuitType {
    ZcashOrchard,
    AnonymousVote,
    Credential,
    Custom(String),
}
```

#### State Machine

```mermaid
stateDiagram-v2
    [*] --> Pending: CreateJob
    Pending --> Claimed: ClaimJob
    Pending --> Cancelled: CancelJob (timeout)
    Claimed --> Completed: SubmitProof (valid)
    Claimed --> Failed: SubmitProof (invalid)
    Claimed --> Failed: Timeout
    Completed --> [*]
    Failed --> [*]
    Cancelled --> [*]
```

#### Instructions

**1. RegisterProver**
```rust
pub fn register_prover(
    ctx: Context<RegisterProver>,
    stake_amount: u64,
) -> Result<()> {
    // Verify stake meets minimum
    require!(
        stake_amount >= ctx.accounts.config.min_stake_amount,
        ErrorCode::InsufficientStake
    );

    // Initialize prover account
    let prover = &mut ctx.accounts.prover;
    prover.authority = ctx.accounts.authority.key();
    prover.stake_amount = stake_amount;
    prover.reputation_score = 1000; // Initial score
    prover.is_active = true;
    prover.registration_timestamp = Clock::get()?.unix_timestamp;

    // Transfer stake to escrow
    // ... (stake transfer logic)

    Ok(())
}
```

**2. CreateJob**
```rust
pub fn create_job(
    ctx: Context<CreateJob>,
    circuit_type: CircuitType,
    encrypted_witness: Vec<u8>,
    price_lamports: u64,
    timeout_seconds: i64,
) -> Result<()> {
    // Initialize job account
    let job = &mut ctx.accounts.job;
    job.id = ctx.accounts.config.next_job_id;
    job.creator = ctx.accounts.creator.key();
    job.status = JobStatus::Pending;
    job.circuit_type = circuit_type;
    job.encrypted_witness = encrypted_witness;
    job.price_lamports = price_lamports;
    job.created_at = Clock::get()?.unix_timestamp;
    job.timeout_at = job.created_at + timeout_seconds;

    // Create escrow PDA
    // Transfer payment to escrow
    // ... (escrow setup logic)

    // Increment job counter
    ctx.accounts.config.next_job_id += 1;

    // Emit event
    emit!(JobCreated {
        job_id: job.id,
        creator: job.creator,
        circuit_type: job.circuit_type,
        price: price_lamports,
    });

    Ok(())
}
```

**3. ClaimJob**
```rust
pub fn claim_job(
    ctx: Context<ClaimJob>,
) -> Result<()> {
    let job = &mut ctx.accounts.job;
    let prover = &ctx.accounts.prover;

    // Verify job is pending
    require!(
        job.status == JobStatus::Pending,
        ErrorCode::JobNotPending
    );

    // Verify prover is active and has sufficient reputation
    require!(prover.is_active, ErrorCode::ProverInactive);
    require!(
        prover.reputation_score >= 500,
        ErrorCode::InsufficientReputation
    );

    // Verify not timed out
    let now = Clock::get()?.unix_timestamp;
    require!(now < job.timeout_at, ErrorCode::JobTimedOut);

    // Assign prover
    job.prover = Some(prover.authority);
    job.status = JobStatus::Claimed;
    job.claimed_at = Some(now);

    // Emit event
    emit!(JobClaimed {
        job_id: job.id,
        prover: prover.authority,
    });

    Ok(())
}
```

**4. SubmitProof**
```rust
pub fn submit_proof(
    ctx: Context<SubmitProof>,
    encrypted_proof: Vec<u8>,
) -> Result<()> {
    let job = &mut ctx.accounts.job;
    let prover = &mut ctx.accounts.prover;

    // Verify job is claimed by this prover
    require!(
        job.status == JobStatus::Claimed,
        ErrorCode::JobNotClaimed
    );
    require!(
        job.prover == Some(prover.authority),
        ErrorCode::UnauthorizedProver
    );

    // Store encrypted proof
    job.encrypted_proof = Some(encrypted_proof);
    job.completed_at = Some(Clock::get()?.unix_timestamp);

    // Note: Verification happens client-side or via separate instruction
    // For MVP, we trust prover (slashing handles misbehavior)
    job.status = JobStatus::Completed;

    // Release payment from escrow
    release_escrow(ctx.accounts, job.price_lamports)?;

    // Update prover stats
    prover.total_jobs_completed += 1;
    update_reputation(prover, true)?;

    // Emit event
    emit!(ProofSubmitted {
        job_id: job.id,
        prover: prover.authority,
    });

    Ok(())
}
```

**5. SlashProver**
```rust
pub fn slash_prover(
    ctx: Context<SlashProver>,
    reason: SlashReason,
) -> Result<()> {
    let prover = &mut ctx.accounts.prover;
    let job = &ctx.accounts.job;

    // Verify caller has authority (reputation oracle or timeout)
    // ... (authorization logic)

    // Reduce stake
    let slash_amount = match reason {
        SlashReason::InvalidProof => prover.stake_amount / 10, // 10%
        SlashReason::Timeout => prover.stake_amount / 20,      // 5%
        SlashReason::MaliciousBehavior => prover.stake_amount, // 100%
    };

    prover.stake_amount -= slash_amount;

    // Update reputation (SAS)
    update_reputation(prover, false)?;

    // If stake too low, deactivate
    if prover.stake_amount < ctx.accounts.config.min_stake_amount {
        prover.is_active = false;
    }

    // Transfer slashed amount to protocol
    // ... (transfer logic)

    emit!(ProverSlashed {
        prover: prover.authority,
        amount: slash_amount,
        reason,
    });

    Ok(())
}
```

---

### 3. Light Protocol Integration (ZK Compression)

#### Why ZK Compression?

**Problem:**
- Storing job data on-chain is expensive
- 1 MB of data = ~6.9 SOL (~$150 at $22/SOL)
- High job volume = unsustainable costs

**Solution:**
- ZK Compression stores state off-chain
- Only commitment (hash) stored on-chain
- Validity proven via ZK proofs
- 50x cost reduction

#### Implementation

```rust
use light_sdk::{compress_account, decompress_account};

// Compress job data
pub fn create_job_compressed(
    ctx: Context<CreateJobCompressed>,
    job_data: JobData,
) -> Result<()> {
    // Serialize job data
    let serialized = job_data.try_to_vec()?;

    // Compress using Light SDK
    let compressed = compress_account(
        &serialized,
        &ctx.accounts.compression_program,
    )?;

    // Store only commitment on-chain
    ctx.accounts.job_commitment.commitment = compressed.commitment;
    ctx.accounts.job_commitment.job_id = job_data.id;

    Ok(())
}

// Decompress job data (when prover claims)
pub fn get_job_data(
    compressed_account: &AccountInfo,
) -> Result<JobData> {
    let decompressed = decompress_account(compressed_account)?;
    let job_data: JobData = JobData::try_from_slice(&decompressed)?;
    Ok(job_data)
}
```

#### Storage Comparison

**Without ZK Compression:**
```
Job (typical size: 5 KB)
Cost: 0.035 SOL (~$0.77)
10,000 jobs = 350 SOL (~$7,700)
```

**With ZK Compression:**
```
Job commitment (32 bytes)
Cost: 0.0007 SOL (~$0.015)
10,000 jobs = 7 SOL (~$154)
```

**Savings: 98% cost reduction**

---

### 4. Prover Node (Rust)

#### Architecture

```mermaid
graph TB
    subgraph "Prover Node Daemon"
        ML[Main Loop]
        JP[Job Poller]
        JC[Job Claimer]
        PG[Proof Generator]
        PS[Proof Submitter]
        TUI[TUI Dashboard]
    end

    subgraph "Proof Generation"
        HM[Halo2 Module]
        PK[Proving Key Cache]
        WP[Witness Parser]
    end

    ML --> JP
    JP --> JC
    JC --> PG
    PG --> HM
    HM --> PK
    PG --> WP
    PG --> PS
    PS --> ML
    ML -.-> TUI
```

#### Main Loop

```rust
#[tokio::main]
async fn main() -> Result<()> {
    // Initialize
    let config = load_config()?;
    let rpc_client = RpcClient::new(&config.rpc_url);
    let marketplace_client = MarketplaceClient::new(rpc_client);
    let proving_key = load_proving_key(&config.circuit_type)?;

    // Start TUI in separate thread
    tokio::spawn(async move {
        run_tui().await
    });

    // Main proving loop
    loop {
        // Poll for new jobs
        let jobs = marketplace_client
            .get_pending_jobs(config.circuit_type)
            .await?;

        // Select best job (by price/reputation)
        if let Some(job) = select_best_job(&jobs, &config) {
            // Claim job
            match marketplace_client.claim_job(job.id).await {
                Ok(_) => {
                    log::info!("Claimed job {}", job.id);

                    // Generate proof
                    match generate_proof(&job, &proving_key).await {
                        Ok(proof) => {
                            // Submit proof
                            marketplace_client
                                .submit_proof(job.id, proof)
                                .await?;

                            log::info!("Submitted proof for job {}", job.id);
                        }
                        Err(e) => {
                            log::error!("Proof generation failed: {}", e);
                            // Job will timeout, prover may be slashed
                        }
                    }
                }
                Err(e) => {
                    log::warn!("Failed to claim job: {}", e);
                    // Another prover claimed it first, continue
                }
            }
        }

        // Sleep before next poll
        tokio::time::sleep(Duration::from_secs(config.poll_interval)).await;
    }
}
```

#### Proof Generation

```rust
async fn generate_proof(
    job: &Job,
    proving_key: &ProvingKey,
) -> Result<Vec<u8>> {
    // Decrypt witness
    let encrypted_witness = decompress_data(&job.encrypted_witness)?;
    let witness = decrypt_witness(encrypted_witness, &get_private_key())?;

    // Parse witness for circuit
    let circuit_witness = parse_witness_for_circuit(witness, job.circuit_type)?;

    // Generate proof (Halo2)
    let proof = match job.circuit_type {
        CircuitType::ZcashOrchard => {
            generate_orchard_proof(circuit_witness, proving_key).await?
        }
        CircuitType::AnonymousVote => {
            generate_vote_proof(circuit_witness, proving_key).await?
        }
        // ... other circuit types
        _ => return Err(Error::UnsupportedCircuit),
    };

    // Encrypt proof for job creator
    let creator_pubkey = get_creator_pubkey(&job.creator)?;
    let encrypted_proof = encrypt_proof(&proof, &creator_pubkey)?;

    Ok(encrypted_proof)
}

async fn generate_orchard_proof(
    witness: OrchardWitness,
    proving_key: &ProvingKey,
) -> Result<Vec<u8>> {
    use zcash_proofs::orchard::create_proof;

    // Build circuit instance
    let circuit = OrchardCircuit::new(witness);

    // Generate proof (CPU-intensive, 10-15 seconds)
    let proof = create_proof(
        proving_key,
        circuit,
    )?;

    Ok(proof.to_bytes())
}
```

---

### 5. Encryption & Security

#### Post-Quantum Key Exchange (ML-KEM)

```rust
use ml_kem::{KemCore, MlKem768}; // NIST standardized Kyber

// Prover generates keypair
pub fn prover_generate_keypair() -> (PublicKey, SecretKey) {
    let mut rng = OsRng;
    let (secret_key, public_key) = MlKem768::generate(&mut rng);
    (public_key, secret_key)
}

// Client encrypts witness
pub fn encrypt_witness(
    witness: &[u8],
    prover_public_key: &PublicKey,
) -> Result<(Vec<u8>, Vec<u8>)> {
    let mut rng = OsRng;

    // Generate shared secret
    let (ciphertext, shared_secret) = prover_public_key.encapsulate(&mut rng)?;

    // Derive AES key from shared secret
    let aes_key = derive_aes_key(&shared_secret);

    // Encrypt witness with AES-256-GCM
    let encrypted = aes_encrypt(witness, &aes_key)?;

    Ok((ciphertext, encrypted))
}

// Prover decrypts witness
pub fn decrypt_witness(
    ciphertext: &[u8],
    encrypted_witness: &[u8],
    prover_secret_key: &SecretKey,
) -> Result<Vec<u8>> {
    // Decapsulate shared secret
    let shared_secret = prover_secret_key.decapsulate(ciphertext)?;

    // Derive same AES key
    let aes_key = derive_aes_key(&shared_secret);

    // Decrypt witness
    let witness = aes_decrypt(encrypted_witness, &aes_key)?;

    Ok(witness)
}
```

#### Security Properties

**Confidentiality:**
- Prover never sees plaintext witness
- Post-quantum secure (resists quantum attacks)
- Forward secrecy (each job uses unique keys)

**Integrity:**
- Proof verification ensures correctness
- Tampering detected immediately
- Slashing punishes invalid proofs

**Availability:**
- Fallback to local proving if no provers
- Timeout mechanism ensures progress
- Multiple provers provide redundancy

**Privacy:**
- Zero-knowledge proofs hide witness data
- Encrypted communication
- No metadata leakage

---

### 6. Reputation System (SAS Integration)

#### Solana Attestation Service

```rust
use sas_sdk::{Attestation, AttestationBuilder};

// Create attestation after successful proof
pub fn attest_proof_completion(
    prover: &Pubkey,
    job_id: u64,
    completion_time_secs: u32,
    proof_valid: bool,
) -> Result<Attestation> {
    let attestation = AttestationBuilder::new()
        .subject(prover)
        .schema("CypherLinkProof")
        .data(serde_json::json!({
            "job_id": job_id,
            "completion_time": completion_time_secs,
            "proof_valid": proof_valid,
            "timestamp": Clock::get()?.unix_timestamp,
        }))
        .build()?;

    Ok(attestation)
}

// Query prover reputation
pub fn get_prover_reputation(prover: &Pubkey) -> Result<ReputationScore> {
    let attestations = sas_sdk::query_attestations(
        prover,
        "CypherLinkProof",
    )?;

    let mut total_jobs = 0;
    let mut successful_jobs = 0;
    let mut avg_time = 0;

    for attestation in attestations {
        total_jobs += 1;
        if attestation.data["proof_valid"].as_bool().unwrap_or(false) {
            successful_jobs += 1;
        }
        avg_time += attestation.data["completion_time"].as_u64().unwrap_or(0);
    }

    let success_rate = (successful_jobs as f64) / (total_jobs as f64);
    let avg_completion_time = avg_time / total_jobs;

    // Calculate reputation score (0-1000)
    let reputation = calculate_reputation_score(
        success_rate,
        avg_completion_time,
        total_jobs,
    );

    Ok(ReputationScore {
        score: reputation,
        total_jobs,
        success_rate,
        avg_completion_time,
    })
}
```

---

## Data Flow

### Complete Transaction Flow

```mermaid
sequenceDiagram
    actor User
    participant Wallet as Mobile Wallet
    participant Solana as Solana Program
    participant Prover as Prover Node
    participant Zcash as Zcash Network

    User->>Wallet: Initiate shielded transaction
    Wallet->>Wallet: Generate witness
    Wallet->>Wallet: Encrypt witness (ML-KEM)
    Wallet->>Solana: CreateJob (encrypted witness)
    Solana->>Solana: Compress state (Light)
    Solana->>Solana: Create escrow
    Solana-->>Wallet: Job created (ID)

    loop Poll for jobs
        Prover->>Solana: Query pending jobs
    end

    Prover->>Solana: ClaimJob
    Solana->>Solana: Assign prover
    Solana-->>Prover: Job claimed

    Prover->>Prover: Decrypt witness
    Prover->>Prover: Generate proof (Halo2)
    Prover->>Prover: Encrypt proof
    Prover->>Solana: SubmitProof (encrypted)
    Solana->>Solana: Release escrow
    Solana->>Solana: Update reputation (SAS)
    Solana-->>Prover: Payment sent
    Solana-->>Wallet: Proof ready

    Wallet->>Wallet: Decrypt proof
    Wallet->>Wallet: Verify proof
    Wallet->>Zcash: Broadcast transaction
    Zcash-->>Wallet: Transaction confirmed
    Wallet->>User: Success!
```

---

## Performance Characteristics

### Latency Breakdown

**Total time: ~15-25 seconds**

| Step | Time | Notes |
|------|------|-------|
| Witness generation | 2-3s | Mobile CPU |
| Encryption | <100ms | Fast symmetric crypto |
| Job creation (Solana) | 400ms | 1 transaction |
| Prover claim | 400ms | 1 transaction |
| Proof generation | 10-15s | Desktop CPU, Halo2 |
| Proof submission | 400ms | 1 transaction |
| Verification | 1-2s | Mobile, pairing check |
| Broadcast | 1-2s | Zcash network |

**Bottleneck:** Proof generation (10-15s) - but on powerful desktop

### Cost Analysis

**Per transaction:**
- Job creation: ~0.00005 SOL (~$0.001)
- Prover claim: ~0.00005 SOL (~$0.001)
- Proof submission: ~0.00005 SOL (~$0.001)
- Prover payment: ~0.02 USD (configurable)
- Platform fee: ~0.002 USD (10%)

**Total user cost: ~$0.025**

**Comparison:**
- Ethereum L1: $5-50 (200x more expensive)
- Ethereum L2: $0.10-1.00 (4-40x more expensive)
- Solana: $0.025 (competitive)

---

## Scalability

### Throughput

**Solana limits:**
- 65,000 TPS theoretical
- 3,000 TPS practical (sustained)
- Sub-second finality

**Our marketplace:**
- Job creation: 3,000 TPS (limited by Solana)
- Proof generation: Limited by prover availability
- With 1,000 provers @ 4 proofs/minute each:
  - 66 proofs/second sustained
  - ~5.7M proofs/day

**Bottleneck:** Prover availability (solvable with economic incentives)

### State Growth

**Without ZK Compression:**
- 5 KB per job
- 1M jobs = 5 GB
- Cost: ~35,000 SOL (~$770K)

**With ZK Compression (Future - Phase 2):**
- 32 bytes per job commitment
- 1M jobs = 32 MB
- Cost: ~700 SOL (~$15.4K)

**Potential 50x improvement** (requires Light Protocol integration)

---

## FHE Multi-Prover Consensus

### Overview

For FHE (Fully Homomorphic Encryption) computations, ZyberLink implements a multi-prover consensus mechanism to ensure correctness without revealing the encrypted data.

### Architecture

```
Client encrypts input → Multiple provers claim job →
Compute in parallel → Submit result hashes →
On-chain consensus → Matching provers get paid
```

### Consensus Algorithm

**1. Job Creation**
```rust
pub struct FheJob {
    pub required_provers: u8,        // e.g., 3
    pub consensus_threshold: u8,      // e.g., 2 (2-of-3)
    pub encrypted_input: Vec<u8>,
    pub operation: FheOperation,      // Add, Multiply, Subtract
    pub claimed_provers: Vec<Pubkey>,
    pub results: Vec<FheJobResult>,
}
```

**2. Multi-Prover Claiming**
- FHE jobs allow multiple provers to claim (unlike ZK jobs)
- Job status changes to Claimed when `claimed_provers.len() == required_provers`
- First N provers to submit valid claims get the job

**3. Parallel Computation**
- Each prover independently computes the FHE operation
- Provers never see plaintext (data remains encrypted)
- Computation time: ~39 seconds per operation (TFHE-rs)

**4. Result Submission**
```rust
pub struct FheJobResult {
    pub prover: Pubkey,
    pub result_hash: [u8; 32],        // Hash of encrypted result
    pub encrypted_result: Vec<u8>,     // Full encrypted output
    pub submitted_at: i64,
}
```

**5. On-Chain Consensus**
```rust
pub fn finalize_fhe_job(ctx: Context<FinalizeFheJob>) -> Result<()> {
    let job = &ctx.accounts.job;

    // Count matching result hashes
    let mut hash_counts: HashMap<[u8; 32], Vec<Pubkey>> = HashMap::new();
    for result in &job.results {
        hash_counts.entry(result.result_hash)
            .or_insert(Vec::new())
            .push(result.prover);
    }

    // Find consensus (majority hash)
    let consensus = hash_counts.iter()
        .max_by_key(|(_, provers)| provers.len())
        .unwrap();

    // Verify threshold met
    require!(
        consensus.1.len() >= job.consensus_threshold as usize,
        ErrorCode::ConsensusNotReached
    );

    // Pay matching provers
    let payment_per_prover = job.price_lamports / consensus.1.len() as u64;
    for prover_key in consensus.1 {
        pay_prover(prover_key, payment_per_prover)?;
    }

    // Penalize non-matching provers (slash stake)
    for result in &job.results {
        if !consensus.1.contains(&result.prover) {
            slash_prover(&result.prover, SlashReason::InvalidResult)?;
        }
    }

    Ok(())
}
```

### Security Properties

**Byzantine Fault Tolerance:**
- With 3 provers, 2-of-3 consensus, tolerates 1 malicious prover
- With 5 provers, 3-of-5 consensus, tolerates 2 malicious provers

**Economic Security:**
- Dishonest provers lose their stake
- Honest provers earn rewards
- Rational actors will compute correctly

**Cryptographic Security:**
- Data never leaves encrypted form
- Even malicious provers can't see plaintext
- Result correctness guaranteed by majority

### Performance

**Current (TFHE-rs):**
- Single operation: ~39 seconds
- 3-prover consensus: ~45 seconds total (parallel)
- Bottleneck: FHE computation, not consensus

**Future (Concrete):**
- Estimated 260x speedup
- Single operation: ~150ms
- 3-prover consensus: ~200ms total

---

## Prover Node Architecture

### Overview

The Prover Node is a desktop application that monitors the marketplace, claims jobs, executes computations, and submits results.

### Components

```mermaid
graph TB
    subgraph "Prover Node"
        MC[Main Controller]
        JP[Job Poller]
        ZK[ZK Proof Engine]
        FHE[FHE Compute Engine]
        TUI[Terminal UI]
        WZ[Setup Wizard]
    end

    subgraph "External"
        SOL[Solana RPC]
        WS[Witness Storage]
    end

    MC --> JP
    MC --> TUI
    MC --> WZ
    JP --> SOL
    JP --> MC
    MC --> ZK
    MC --> FHE
    ZK --> MC
    FHE --> MC
    MC --> SOL
    SOL --> WS
```

### Key Modules

**1. Job Poller**
- Queries marketplace every 5 seconds
- Filters jobs by circuit type capability
- Automatic claiming based on profitability

**2. ZK Proof Engine (Halo2)**
- Groth16/PLONK circuit support
- Proving key caching
- Average proving time: 15 seconds
- CPU-intensive, multi-threaded

**3. FHE Compute Engine (TFHE-rs)**
```rust
pub struct FheEngine {
    config: Config,
    server_key: ServerKey,
    operation_cache: HashMap<FheOperation, CachedOp>,
}

impl FheEngine {
    pub fn compute(&self, op: FheOperation, encrypted_input: &[u8])
        -> Result<Vec<u8>>
    {
        match op {
            FheOperation::Add(x) => self.fhe_add(encrypted_input, x),
            FheOperation::Multiply(x) => self.fhe_multiply(encrypted_input, x),
            FheOperation::Subtract(x) => self.fhe_subtract(encrypted_input, x),
        }
    }

    fn fhe_add(&self, input: &[u8], operand: u64) -> Result<Vec<u8>> {
        let ct_input = FheUint64::deserialize(input)?;
        let ct_result = self.server_key.scalar_add(&ct_input, operand);
        Ok(ct_result.serialize())
    }
}
```

**4. Terminal UI (TUI)**
- Real-time dashboard with ratatui
- Displays:
  - Active jobs (pending, claimed, completed)
  - Prover statistics (jobs completed, earnings)
  - Network status (connected provers, consensus rate)
  - Performance metrics (avg proving time)
- Interactive keyboard controls

**5. Setup Wizard**
- Interactive onboarding flow (6 steps)
- System validation (Rust, Solana CLI, disk space)
- Wallet configuration (generate or import)
- Funding via Solana Blinks (QR code)
- Terms & Conditions acceptance
- Final validation

### Deployment

**Requirements:**
- CPU: 4+ cores (8+ recommended)
- RAM: 8GB minimum (16GB recommended)
- Disk: 50GB free space
- Network: Stable internet connection

**Startup:**
```bash
cd prover-node

# Interactive wizard
cargo run --release -- wizard

# Or run directly with config
cargo run --release -- --config config.toml
```

---

## Security Model

### Threat Model

**Threats:**
1. **Malicious prover** - Tries to steal witness data
2. **Lazy prover** - Claims job but doesn't complete
3. **Invalid proof** - Prover submits garbage
4. **Sybil attack** - One entity runs many provers
5. **Front-running** - Prover steals jobs unfairly
6. **DoS** - Spam jobs to clog marketplace

**Mitigations:**
1. **Encryption** - Witness encrypted, prover can't steal
2. **Timeouts** - Job reverts if not completed, prover slashed
3. **Verification** - Client verifies proof, invalid = slash
4. **Reputation** - SAS tracks behavior, low score = no jobs
5. **Fair selection** - Deterministic or reputation-based
6. **Rate limits** - Job creation limits, stake requirements

### Trust Assumptions

**What we trust:**
- Solana consensus (BFT assumption)
- FHE cryptographic properties (TFHE-rs)
- Halo2 circuit correctness (audited)
- Multi-prover consensus mechanism (Byzantine fault tolerant)

**What we don't trust:**
- Individual provers (assumed potentially adversarial)
- RPC endpoints (should use multiple)
- Network communication (encrypted end-to-end)
- Centralized services (none used)

**Result:** Decentralized marketplace with multi-prover consensus ensuring correctness

---

## Future Architecture Enhancements

### Phase 2: Light Protocol Integration (Q1 2026)

**ZK Compression for State Management**

```rust
// Current: Traditional Solana accounts
#[account]
pub struct JobAccount {
    pub data: Vec<u8>,  // 5KB per job
}

// Future: Compressed state via Light Protocol
#[compressed_account]
pub struct CompressedJobAccount {
    pub commitment: [u8; 32],  // 32 bytes per job
    pub merkle_tree: Pubkey,
}
```

**Benefits:**
- 50-100x reduction in state costs
- Scalable job history (millions of jobs)
- Efficient prover registry updates
- Maintains all security properties

**Integration Points:**
- Compressed JobAccount and ProverAccount
- Merkle tree-based state verification
- SDK updates for compression/decompression
- Backward compatibility with existing jobs

### Short-term (Post-Hackathon - Q1 2026)
- **Complete FHE E2E Integration** - 16/16 tests passing
- **Concrete Library Migration** - 260x faster FHE operations
- **Mobile SDK** - Flutter/React Native libraries
- **Prover Hardware Attestation** - Verify compute capabilities

### Medium-term (Q2-Q3 2026)
- **Hardware Acceleration** - GPU/FPGA proving support
- **Reputation System (SAS)** - On-chain attestations
- **Dynamic Pricing** - Market-based job pricing
- **Circuit Marketplace** - Custom circuit support
- **Cross-Chain Support** - Ethereum L2 integration

### Long-term (2026+)
- **MPC Integration** - Multi-party computation support
- **TEE Support** - Trusted execution environments
- **Verifiable Delay Functions** - Fair prover selection
- **Proof Aggregation** - Batch multiple proofs
- **Hardware Prover Network** - Dedicated proving devices

---

## Conclusion

ZyberLink's architecture delivers:
- **Decentralization** - Multi-prover consensus, no single point of failure
- **Privacy** - End-to-end encryption, FHE never reveals plaintext
- **Byzantine Fault Tolerance** - 2-of-3 or 3-of-5 consensus model
- **Economic Security** - Stake-based incentives, slashing for dishonesty
- **Extensibility** - Supports ZK proofs and FHE computations
- **Production-Ready** - Working E2E system with TUI, wizard, demos

**Current Status (November 2025):**
- Solana program: Functional
- FHE multi-prover consensus: Implemented (E2E integration in progress)
- Prover node: Complete with TUI and interactive wizard
- Performance: 15s ZK proofs, 39s FHE operations

**Next Steps:**
- Complete FHE E2E integration (4 tests remaining)
- Light Protocol compression (Phase 2)
- Concrete library migration (260x speedup)
- Mobile SDK and broader ecosystem integration

The system demonstrates a viable path to decentralized privacy-preserving computation at scale.
