# API Reference

Complete reference documentation for the ZyberLink SDK.

## Table of Contents

- [MarketplaceClient](#marketplaceclient)
- [InstructionBuilder](#instructionbuilder)
- [Data Types](#data-types)
- [Instructions](#instructions)
- [Helper Functions](#helper-functions)
- [Error Handling](#error-handling)

## MarketplaceClient

The main client for interacting with the ZyberLink marketplace program.

### Constructor

#### `new`

Creates a new marketplace client with default commitment level (confirmed).

```rust
pub fn new(rpc_url: String, program_id: Pubkey) -> Self
```

**Parameters:**
- `rpc_url`: Solana RPC endpoint URL
- `program_id`: Deployed marketplace program ID

**Returns:** `MarketplaceClient` instance

**Example:**
```rust
let client = MarketplaceClient::new(
    "http://localhost:8899".to_string(),
    program_id,
);
```

#### `new_with_commitment`

Creates a client with custom commitment level.

```rust
pub fn new_with_commitment(
    rpc_url: String,
    program_id: Pubkey,
    commitment: CommitmentConfig,
) -> Self
```

**Parameters:**
- `rpc_url`: Solana RPC endpoint URL
- `program_id`: Deployed marketplace program ID
- `commitment`: Commitment level (processed, confirmed, finalized)

**Returns:** `MarketplaceClient` instance

**Example:**
```rust
use solana_sdk::commitment_config::CommitmentConfig;

let client = MarketplaceClient::new_with_commitment(
    "http://localhost:8899".to_string(),
    program_id,
    CommitmentConfig::finalized(),
);
```

### Instruction Builders

#### `initialize_instruction`

Builds an instruction to initialize the marketplace configuration.

```rust
pub fn initialize_instruction(
    &self,
    authority: &Pubkey,
    fee_basis_points: u16,
    min_stake_amount: u64,
    min_reputation_score: u32,
    default_job_timeout_seconds: i64,
) -> Result<Instruction>
```

**Parameters:**
- `authority`: Marketplace authority public key (must sign)
- `fee_basis_points`: Protocol fee in basis points (e.g., 1000 = 10%)
- `min_stake_amount`: Minimum stake required for provers (lamports)
- `min_reputation_score`: Minimum reputation score to claim jobs
- `default_job_timeout_seconds`: Default timeout for jobs

**Returns:** `Result<Instruction>`

**Example:**
```rust
let ix = client.initialize_instruction(
    &authority.pubkey(),
    1000,           // 10% fee
    1_000_000_000,  // 1 SOL min stake
    500,            // Min reputation
    600,            // 10 minute timeout
)?;
```

#### `register_prover_instruction`

Builds an instruction to register a new prover.

```rust
pub fn register_prover_instruction(
    &self,
    prover_authority: &Pubkey,
    stake_amount: u64,
    encryption_pubkey: [u8; 32],
) -> Result<Instruction>
```

**Parameters:**
- `prover_authority`: Prover's authority public key (must sign)
- `stake_amount`: Amount to stake (lamports)
- `encryption_pubkey`: Public key for witness encryption (32 bytes)

**Returns:** `Result<Instruction>`

**Example:**
```rust
let encryption_key = [0u8; 32]; // Generate proper key in production

let ix = client.register_prover_instruction(
    &prover.pubkey(),
    5_000_000_000, // 5 SOL stake
    encryption_key,
)?;
```

#### `create_job_instruction`

Builds an instruction to create a new computation job.

```rust
pub fn create_job_instruction(
    &self,
    job_creator: &Pubkey,
    job_id: u64,
    circuit_type: CircuitType,
    witness_commitment: [u8; 32],
    witness_size: u32,
    price_lamports: u64,
    timeout_seconds: i64,
    fhe_config: Option<FheConsensusConfig>,
) -> Result<Instruction>
```

**Parameters:**
- `job_creator`: Job creator public key (must sign and pay)
- `job_id`: Unique job identifier
- `circuit_type`: Type of computation (FHE or ZK)
- `witness_commitment`: Hash commitment of encrypted witness
- `witness_size`: Size of witness data in bytes
- `price_lamports`: Payment for job completion
- `timeout_seconds`: Job timeout in seconds
- `fhe_config`: Optional FHE consensus configuration

**Returns:** `Result<Instruction>`

**Example:**
```rust
use cypherlink_types::{CircuitType, FheConsensusConfig};

let ix = client.create_job_instruction(
    &creator.pubkey(),
    1,                      // Job ID
    CircuitType::FheAdd,    // FHE addition
    witness_commitment,
    1024,                   // 1KB witness
    1_000_000_000,          // 1 SOL payment
    600,                    // 10 minute timeout
    Some(FheConsensusConfig {
        required_provers: 3,
        consensus_threshold: 2, // 2-of-3
    }),
)?;
```

#### `claim_job_instruction`

Builds an instruction for a prover to claim a pending job.

```rust
pub fn claim_job_instruction(
    &self,
    prover_authority: &Pubkey,
    job_pda: &Pubkey,
) -> Result<Instruction>
```

**Parameters:**
- `prover_authority`: Prover's authority public key (must sign)
- `job_pda`: Job account PDA

**Returns:** `Result<Instruction>`

**Example:**
```rust
let (job_pda, _) = client.get_job_pda(&creator, job_id);

let ix = client.claim_job_instruction(
    &prover.pubkey(),
    &job_pda,
)?;
```

#### `submit_proof_instruction`

Builds an instruction to submit a ZK proof.

```rust
pub fn submit_proof_instruction(
    &self,
    prover_authority: &Pubkey,
    job_pda: &Pubkey,
    job_creator: &Pubkey,
    proof_commitment: [u8; 32],
    proof_size: u32,
) -> Result<Instruction>
```

**Parameters:**
- `prover_authority`: Prover's authority public key (must sign)
- `job_pda`: Job account PDA
- `job_creator`: Original job creator public key
- `proof_commitment`: Hash commitment of the proof
- `proof_size`: Size of proof in bytes

**Returns:** `Result<Instruction>`

**Note:** This method fetches the protocol fee recipient from on-chain config.

**Example:**
```rust
let ix = client.submit_proof_instruction(
    &prover.pubkey(),
    &job_pda,
    &creator,
    proof_commitment,
    2048, // 2KB proof
)?;
```

#### `submit_fhe_result_instruction`

Builds an instruction to submit an FHE computation result.

```rust
pub fn submit_fhe_result_instruction(
    &self,
    prover_authority: &Pubkey,
    job_pda: &Pubkey,
    result_hash: [u8; 32],
) -> Result<Instruction>
```

**Parameters:**
- `prover_authority`: Prover's authority public key (must sign)
- `job_pda`: Job account PDA
- `result_hash`: Hash of the FHE computation result

**Returns:** `Result<Instruction>`

**Example:**
```rust
use sha3::{Digest, Sha3_256};

let result_hash = Sha3_256::digest(&fhe_result).into();

let ix = client.submit_fhe_result_instruction(
    &prover.pubkey(),
    &job_pda,
    result_hash,
)?;
```

#### `finalize_fhe_job_instruction`

Builds an instruction to finalize an FHE job after consensus.

```rust
pub fn finalize_fhe_job_instruction(
    &self,
    finalizer: &Pubkey,
    job_pda: &Pubkey,
    job_creator: &Pubkey,
    prover_accounts: &[Pubkey],
) -> Result<Instruction>
```

**Parameters:**
- `finalizer`: Anyone can finalize (must sign for tx fee)
- `job_pda`: Job account PDA
- `job_creator`: Original job creator
- `prover_accounts`: Array of matching prover public keys

**Returns:** `Result<Instruction>`

**Note:** This method fetches the protocol fee recipient from on-chain config.

**Example:**
```rust
let matching_provers = vec![prover_a.pubkey(), prover_b.pubkey()];

let ix = client.finalize_fhe_job_instruction(
    &finalizer.pubkey(),
    &job_pda,
    &creator,
    &matching_provers,
)?;
```

#### `cancel_job_instruction`

Builds an instruction to cancel a pending job.

```rust
pub fn cancel_job_instruction(
    &self,
    job_creator: &Pubkey,
    job_pda: &Pubkey,
) -> Result<Instruction>
```

**Parameters:**
- `job_creator`: Job creator public key (must sign)
- `job_pda`: Job account PDA

**Returns:** `Result<Instruction>`

**Example:**
```rust
let ix = client.cancel_job_instruction(
    &creator.pubkey(),
    &job_pda,
)?;
```

### Transaction Helpers

#### `send_and_confirm_transaction`

Sends and confirms a transaction with the provided instructions.

```rust
pub fn send_and_confirm_transaction(
    &self,
    instructions: &[Instruction],
    signers: &[&Keypair],
) -> Result<Signature>
```

**Parameters:**
- `instructions`: Array of instructions to execute
- `signers`: Array of keypairs to sign the transaction

**Returns:** `Result<Signature>` - Transaction signature

**Example:**
```rust
let signature = client.send_and_confirm_transaction(
    &[initialize_ix, register_ix],
    &[&authority, &prover],
)?;
```

### High-Level FHE Methods

#### `submit_fhe_result`

High-level method to submit an FHE result.

```rust
pub fn submit_fhe_result(
    &self,
    prover: &Keypair,
    job_pda: &Pubkey,
    result_hash: [u8; 32],
) -> Result<Signature>
```

**Parameters:**
- `prover`: Prover keypair (for signing)
- `job_pda`: Job account PDA
- `result_hash`: Hash of computation result

**Returns:** `Result<Signature>`

**Example:**
```rust
let signature = client.submit_fhe_result(
    &prover,
    &job_pda,
    result_hash,
)?;
```

#### `finalize_fhe_job`

High-level method to finalize an FHE job.

```rust
pub fn finalize_fhe_job(
    &self,
    finalizer: &Keypair,
    job_pda: &Pubkey,
    job_creator: &Pubkey,
    matching_prover_pubkeys: &[Pubkey],
) -> Result<Signature>
```

**Parameters:**
- `finalizer`: Finalizer keypair (anyone can finalize)
- `job_pda`: Job account PDA
- `job_creator`: Original job creator
- `matching_prover_pubkeys`: Array of matching provers

**Returns:** `Result<Signature>`

**Example:**
```rust
let signature = client.finalize_fhe_job(
    &finalizer,
    &job_pda,
    &creator.pubkey(),
    &[prover_a.pubkey(), prover_b.pubkey()],
)?;
```

### PDA Helpers

#### `get_config_pda`

Derives the marketplace configuration PDA.

```rust
pub fn get_config_pda(&self) -> (Pubkey, u8)
```

**Returns:** `(Pubkey, u8)` - PDA and bump seed

**Example:**
```rust
let (config_pda, bump) = client.get_config_pda();
```

#### `get_prover_pda`

Derives a prover account PDA.

```rust
pub fn get_prover_pda(&self, prover_authority: &Pubkey) -> (Pubkey, u8)
```

**Parameters:**
- `prover_authority`: Prover's authority public key

**Returns:** `(Pubkey, u8)` - PDA and bump seed

**Example:**
```rust
let (prover_pda, bump) = client.get_prover_pda(&prover.pubkey());
```

#### `get_job_pda`

Derives a job account PDA.

```rust
pub fn get_job_pda(&self, job_creator: &Pubkey, job_id: u64) -> (Pubkey, u8)
```

**Parameters:**
- `job_creator`: Job creator public key
- `job_id`: Unique job identifier

**Returns:** `(Pubkey, u8)` - PDA and bump seed

**Example:**
```rust
let (job_pda, bump) = client.get_job_pda(&creator, 1);
```

#### `get_escrow_pda`

Derives an escrow account PDA for a job.

```rust
pub fn get_escrow_pda(&self, job_pda: &Pubkey) -> (Pubkey, u8)
```

**Parameters:**
- `job_pda`: Job account PDA

**Returns:** `(Pubkey, u8)` - PDA and bump seed

**Example:**
```rust
let (escrow_pda, bump) = client.get_escrow_pda(&job_pda);
```

## InstructionBuilder

Pure instruction builder for wallet integration (no RPC dependency).

### Constructor

#### `new`

Creates a new instruction builder.

```rust
pub fn new(program_id: Pubkey) -> Self
```

**Parameters:**
- `program_id`: Marketplace program ID

**Returns:** `InstructionBuilder` instance

**Example:**
```rust
use cypherlink_sdk::instructions::InstructionBuilder;

let builder = InstructionBuilder::new(program_id);
```

### PDA Helpers

#### `config_pda`

```rust
pub fn config_pda(&self) -> (Pubkey, u8)
```

#### `prover_pda`

```rust
pub fn prover_pda(&self, authority: &Pubkey) -> (Pubkey, u8)
```

#### `job_pda`

```rust
pub fn job_pda(&self, creator: &Pubkey, job_id: u64) -> (Pubkey, u8)
```

#### `escrow_pda`

```rust
pub fn escrow_pda(&self, job_pda: &Pubkey) -> (Pubkey, u8)
```

### Instruction Methods

All instruction methods match `MarketplaceClient` but accept `Pubkey` instead of `&Pubkey` for wallet compatibility.

#### `initialize`

```rust
pub fn initialize(
    &self,
    authority: Pubkey,
    fee_basis_points: u16,
    min_stake_amount: u64,
    min_reputation_score: u32,
    default_job_timeout_seconds: i64,
) -> Result<Instruction>
```

#### `register_prover`

```rust
pub fn register_prover(
    &self,
    prover_authority: Pubkey,
    stake_amount: u64,
    encryption_pubkey: [u8; 32],
) -> Result<Instruction>
```

#### `create_job`

```rust
pub fn create_job(
    &self,
    creator: Pubkey,
    job_id: u64,
    circuit_type: CircuitType,
    witness_commitment: [u8; 32],
    witness_size: u32,
    price_lamports: u64,
    timeout_seconds: i64,
    fhe_config: Option<FheConsensusConfig>,
) -> Result<Instruction>
```

#### `claim_job`

```rust
pub fn claim_job(&self, prover: Pubkey, job_pda: Pubkey) -> Result<Instruction>
```

#### `submit_proof`

```rust
pub fn submit_proof(
    &self,
    prover: Pubkey,
    job_pda: Pubkey,
    job_creator: Pubkey,
    protocol_fee_recipient: Pubkey,
    proof_commitment: [u8; 32],
    proof_size: u32,
) -> Result<Instruction>
```

#### `submit_fhe_result`

```rust
pub fn submit_fhe_result(
    &self,
    prover: Pubkey,
    job_pda: Pubkey,
    result_hash: [u8; 32],
) -> Result<Instruction>
```

#### `finalize_fhe_job`

```rust
pub fn finalize_fhe_job(
    &self,
    finalizer: Pubkey,
    job_pda: Pubkey,
    job_creator: Pubkey,
    protocol_fee_recipient: Pubkey,
    matching_provers: &[Pubkey],
) -> Result<Instruction>
```

#### `cancel_job`

```rust
pub fn cancel_job(&self, creator: Pubkey, job_pda: Pubkey) -> Result<Instruction>
```

## Data Types

### CircuitType

Enum representing the type of computation.

```rust
pub enum CircuitType {
    ZcashOrchard,
    AnonymousVote,
    Credential,
    FheAdd,
    FheMultiply,
    FheSubtract,
    Custom(String),
}
```

**Variants:**
- `ZcashOrchard`: Zcash Orchard shielded transaction
- `AnonymousVote`: Anonymous voting circuit
- `Credential`: Credential verification circuit
- `FheAdd`: FHE addition operation
- `FheMultiply`: FHE multiplication operation
- `FheSubtract`: FHE subtraction operation
- `Custom(String)`: Custom circuit type

**Example:**
```rust
let circuit = CircuitType::FheAdd;
```

### FheConsensusConfig

Configuration for FHE multi-prover consensus.

```rust
pub struct FheConsensusConfig {
    pub required_provers: u8,
    pub consensus_threshold: u8,
}
```

**Fields:**
- `required_provers`: Number of provers required to claim the job
- `consensus_threshold`: Number of matching results needed for consensus

**Example:**
```rust
let config = FheConsensusConfig {
    required_provers: 3,
    consensus_threshold: 2, // 2-of-3 consensus
};
```

### JobStatus

Enum representing the current status of a job.

```rust
pub enum JobStatus {
    Pending,
    Claimed,
    Computing,
    Completed,
    Failed,
    Cancelled,
}
```

**Variants:**
- `Pending`: Job created, waiting for provers
- `Claimed`: Prover(s) have claimed the job
- `Computing`: Computation in progress
- `Completed`: Job successfully completed
- `Failed`: Job failed (timeout or consensus failure)
- `Cancelled`: Job cancelled by creator

### Job

On-chain job account structure.

```rust
pub struct Job {
    pub creator: Pubkey,
    pub circuit_type: CircuitType,
    pub witness_commitment: [u8; 32],
    pub witness_size: u32,
    pub price_lamports: u64,
    pub status: JobStatus,
    pub created_at: i64,
    pub timeout_at: i64,
    pub claimed_provers: Vec<Pubkey>,
    pub fhe_config: Option<FheConsensusConfig>,
    pub fhe_results: Vec<FheResult>,
    pub proof_commitment: Option<[u8; 32]>,
    pub proof_size: Option<u32>,
}
```

**Example:**
```rust
use borsh::BorshDeserialize;

let account = rpc_client.get_account(&job_pda)?;
let job = Job::try_from_slice(&account.data)?;

println!("Job status: {:?}", job.status);
```

### FheResult

FHE computation result submitted by a prover.

```rust
pub struct FheResult {
    pub prover: Pubkey,
    pub result_hash: [u8; 32],
    pub submitted_at: i64,
}
```

**Fields:**
- `prover`: Prover who submitted this result
- `result_hash`: Hash of the computation result
- `submitted_at`: Unix timestamp of submission

### Prover

On-chain prover account structure.

```rust
pub struct Prover {
    pub authority: Pubkey,
    pub stake_amount: u64,
    pub reputation_score: u32,
    pub total_jobs_completed: u64,
    pub total_jobs_failed: u64,
    pub is_active: bool,
    pub encryption_pubkey: [u8; 32],
    pub registered_at: i64,
}
```

### MarketplaceConfig

Marketplace configuration account.

```rust
pub struct MarketplaceConfig {
    pub authority: Pubkey,
    pub fee_basis_points: u16,
    pub min_stake_amount: u64,
    pub min_reputation_score: u32,
    pub default_job_timeout_seconds: i64,
    pub protocol_fee_recipient: Pubkey,
    pub next_job_id: u64,
}
```

## Instructions

### MarketplaceInstruction

Enum of all program instructions.

```rust
pub enum MarketplaceInstruction {
    Initialize {
        fee_basis_points: u16,
        min_stake_amount: u64,
        min_reputation_score: u32,
        default_job_timeout_seconds: i64,
    },
    RegisterProver {
        stake_amount: u64,
        encryption_pubkey: [u8; 32],
    },
    CreateJob {
        circuit_type: CircuitType,
        witness_commitment: [u8; 32],
        witness_size: u32,
        price_lamports: u64,
        timeout_seconds: i64,
        fhe_config: Option<FheConsensusConfig>,
    },
    ClaimJob,
    SubmitProof {
        proof_commitment: [u8; 32],
        proof_size: u32,
    },
    CancelJob,
    SlashProver {
        slash_amount: u64,
    },
    SubmitFheResult {
        result_hash: [u8; 32],
    },
    FinalizeFheJob,
}
```

### Serialization

```rust
impl MarketplaceInstruction {
    pub fn pack(&self) -> Result<Vec<u8>, std::io::Error>
    pub fn unpack(input: &[u8]) -> Result<Self, std::io::Error>
}
```

**Example:**
```rust
let instruction = MarketplaceInstruction::ClaimJob;
let serialized = instruction.pack()?;
let deserialized = MarketplaceInstruction::unpack(&serialized)?;
```

## Helper Functions

### Account Deserialization

```rust
use borsh::BorshDeserialize;

// Deserialize Job account
let job = Job::try_from_slice(&account_data)?;

// Deserialize Prover account
let prover = Prover::try_from_slice(&account_data)?;

// Deserialize Config account
let config = MarketplaceConfig::try_from_slice(&account_data)?;
```

### Hash Computation

```rust
use sha3::{Digest, Sha3_256};

fn compute_commitment(data: &[u8]) -> [u8; 32] {
    Sha3_256::digest(data).into()
}
```

## Error Handling

The SDK uses `anyhow::Result` for flexible error handling.

### Common Errors

**RPC Errors:**
```rust
// Connection failed
Err: RpcError: Failed to get account

// Account not found
Err: RpcError: AccountNotFound
```

**Instruction Errors:**
```rust
// Insufficient funds
Err: InstructionError: InsufficientFunds

// Invalid account
Err: InstructionError: InvalidAccountData
```

**Program Errors:**
```rust
// Custom program errors (from on-chain program)
Err: ProgramError: Custom(1001) // Insufficient stake
Err: ProgramError: Custom(1002) // Job not pending
```

### Error Handling Pattern

```rust
use anyhow::{Context, Result};

fn create_job() -> Result<()> {
    let signature = client
        .send_and_confirm_transaction(&[ix], &[&creator])
        .context("Failed to create job")?;

    Ok(())
}

// Usage
match create_job() {
    Ok(_) => println!("Success!"),
    Err(e) => {
        eprintln!("Error: {:?}", e);
        for cause in e.chain() {
            eprintln!("  Caused by: {}", cause);
        }
    }
}
```

## Type Conversions

### Pubkey Conversions

```rust
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

// From string
let pubkey = Pubkey::from_str("11111111111111111111111111111111")?;

// To string
let string = pubkey.to_string();

// From bytes
let pubkey = Pubkey::new_from_array(bytes);

// To bytes
let bytes = pubkey.to_bytes();
```

### Lamports Conversions

```rust
const LAMPORTS_PER_SOL: u64 = 1_000_000_000;

// SOL to lamports
let lamports = 5.0 * LAMPORTS_PER_SOL as f64;

// Lamports to SOL
let sol = lamports as f64 / LAMPORTS_PER_SOL as f64;
```

## Version Information

**Current SDK Version:** 0.2.0

**Solana SDK Version:** 1.18+

**Minimum Rust Version:** 1.75+

## Next Steps

- **[SDK Integration Guide](sdk-integration.md)** - Integration patterns and best practices
- **[Examples Guide](examples.md)** - Real-world code examples
- **[Architecture Overview](../architecture/overview.md)** - System architecture

## Support

For API questions:
- GitHub Issues: [Report issues](https://github.com/yourusername/zyberlink/issues)
- Documentation: [Full docs](https://docs.zyberlink.io)
