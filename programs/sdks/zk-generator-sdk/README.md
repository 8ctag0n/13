# ZK Generator SDK

Client SDK for interacting with the ZyberLink `zk-generator` Solana program.

## Overview

The `zk-generator-sdk` provides a Rust-based interface for creating and managing zero-knowledge proof jobs on Solana. This SDK handles:

- PDA derivation for job and escrow accounts
- Instruction building for all zk-generator operations
- Account metadata helpers for complex CPI calls
- Type re-exports from the zk-generator program

## Features

- **Type-safe** instruction builders
- **Automatic PDA derivation** for job and escrow accounts
- **Comprehensive documentation** with examples
- **Full test coverage** including doctests
- **Support for all circuit types** (legacy and v2.0)

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
zk-generator-sdk = { path = "path/to/zk-generator-sdk" }
solana-sdk = "2.3"
```

## Usage

### 1. Create a ZK Proof Job

```rust
use zk_generator_sdk::{derive_job_pda, derive_escrow_pda, instructions};
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};

// Setup
let program_id = Pubkey::from_str("ZKGen...").unwrap();
let creator = Keypair::new();
let job_id = 12345u64;

// Derive PDAs
let (job_pda, _) = derive_job_pda(&program_id, &creator.pubkey(), job_id);
let (escrow_pda, _) = derive_escrow_pda(&program_id, &job_pda);

// Create instruction
let ix = instructions::create_job(
    &program_id,
    &creator.pubkey(),
    &job_pda,
    &escrow_pda,
    10, // CircuitType::ProofOfInnocence
    [0u8; 32], // witness_hash
    1024, // witness_size
    100_000_000, // 0.1 SOL
    3600, // 1 hour timeout
);

// Send transaction
let tx = Transaction::new_signed_with_payer(
    &[ix],
    Some(&creator.pubkey()),
    &[&creator],
    recent_blockhash,
);
```

### 2. Claim a Job (Prover)

```rust
use zk_generator_sdk::instructions;
use solana_sdk::pubkey::Pubkey;

let program_id = Pubkey::from_str("ZKGen...").unwrap();
let bedrock_program_id = Pubkey::from_str("Bedrock...").unwrap();
let prover = Keypair::new();
let job_pda = Pubkey::from_str("Job...").unwrap();

// Derive prover PDA from bedrock program
let (prover_pda, _) = Pubkey::find_program_address(
    &[b"prover", prover.pubkey().as_ref()],
    &bedrock_program_id
);

// Create claim instruction with CPI verification
let ix = instructions::claim_job(
    &program_id,
    &prover.pubkey(),
    &job_pda,
    Some(&prover_pda),
    Some(&bedrock_program_id),
);
```

### 3. Submit Proof

```rust
use zk_generator_sdk::instructions;
use solana_sdk::pubkey::Pubkey;

let program_id = Pubkey::from_str("ZKGen...").unwrap();
let bedrock_program_id = Pubkey::from_str("Bedrock...").unwrap();
let prover = Keypair::new();
let job_pda = Pubkey::from_str("Job...").unwrap();
let escrow_pda = Pubkey::from_str("Escrow...").unwrap();
let fee_recipient = Pubkey::from_str("Fee...").unwrap();

// Derive bedrock PDAs
let (prover_pda, _) = Pubkey::find_program_address(
    &[b"prover", prover.pubkey().as_ref()],
    &bedrock_program_id
);
let (config_pda, _) = Pubkey::find_program_address(
    &[b"config"],
    &bedrock_program_id
);

// Compute proof hash (off-chain)
let proof_hash = [0u8; 32]; // Hash of the proof

let ix = instructions::submit_proof(
    &program_id,
    &prover.pubkey(),
    &job_pda,
    &escrow_pda,
    &fee_recipient,
    &bedrock_program_id,
    &prover_pda,
    &config_pda,
    proof_hash,
);
```

### 4. Dispute Proof

```rust
use zk_generator_sdk::instructions;
use solana_sdk::pubkey::Pubkey;

let program_id = Pubkey::from_str("ZKGen...").unwrap();
let bedrock_program_id = Pubkey::from_str("Bedrock...").unwrap();
let disputor = Keypair::new();
let job_pda = Pubkey::from_str("Job...").unwrap();
let prover = Pubkey::from_str("Prover...").unwrap();
let treasury = Pubkey::from_str("Treasury...").unwrap();

// Derive bedrock PDAs
let (prover_pda, _) = Pubkey::find_program_address(
    &[b"prover", prover.as_ref()],
    &bedrock_program_id
);
let (config_pda, _) = Pubkey::find_program_address(
    &[b"config"],
    &bedrock_program_id
);

// Full proof and public inputs for on-chain verification
let proof = vec![0u8; 256]; // Groth16 proof
let public_inputs = vec![0u8; 96]; // Circuit-specific

let ix = instructions::dispute_proof(
    &program_id,
    &disputor.pubkey(),
    &job_pda,
    &prover,
    &treasury,
    &bedrock_program_id,
    &prover_pda,
    &config_pda,
    proof,
    public_inputs,
);
```

### 5. Cancel Job (Creator)

```rust
use zk_generator_sdk::instructions;
use solana_sdk::pubkey::Pubkey;

let program_id = Pubkey::from_str("ZKGen...").unwrap();
let creator = Keypair::new();
let job_pda = Pubkey::from_str("Job...").unwrap();
let escrow_pda = Pubkey::from_str("Escrow...").unwrap();

let ix = instructions::cancel_job(
    &program_id,
    &creator.pubkey(),
    &job_pda,
    &escrow_pda,
);
```

## Supported Circuit Types

The SDK supports all circuit types defined in the zk-generator program:

### Legacy Circuits (v1.x)
- `ZcashOrchard` (0)
- `ZcashSapling` (1)
- `AnonymousVote` (2)
- `Credential` (3)

### Core Primitives (v2.0)
- `ProofOfInnocence` (10) - Merkle non-membership proof

### Voting Circuits (v2.0)
- `PrivateVote` (20) - Anonymous DAO voting
- `PrivateVoteWithPoI` (21) - Voting with conflict-of-interest check

### Market Circuits (v2.0)
- `MarketBet` (30) - Private prediction market bet
- `MarketBetWithPoI` (31) - Bet with insider trading check
- `MarketClaim` (32) - Claim market winnings

### Portfolio Circuits (v2.0)
- `PortfolioCompliance` (40) - Regulatory compliance proof
- `PortfolioNetWorth` (41) - Net worth threshold proof

## PDA Derivation

### Job PDA
```rust
use zk_generator_sdk::derive_job_pda;

let (job_pda, bump) = derive_job_pda(&program_id, &creator, job_id);
// Seeds: ["zk_job", creator, job_id.to_le_bytes()]
```

### Escrow PDA
```rust
use zk_generator_sdk::derive_escrow_pda;

let (escrow_pda, bump) = derive_escrow_pda(&program_id, &job_pda);
// Seeds: ["zk_escrow", job_pda]
```

## Account Helpers

The SDK provides helper functions to build AccountMeta arrays for complex instructions:

```rust
use zk_generator_sdk::accounts;

// Create job accounts
let accounts = accounts::create_job_accounts(&creator, &job_pda, &escrow_pda);

// Claim job with CPI
let accounts = accounts::claim_job_accounts_with_cpi(
    &prover,
    &job_pda,
    &prover_pda,
    &bedrock_program,
);

// Submit proof accounts
let accounts = accounts::submit_proof_accounts(
    &prover,
    &job_pda,
    &escrow_pda,
    &fee_recipient,
    &bedrock_program,
    &prover_pda,
    &config_pda,
);
```

## Testing

Run the SDK tests:

```bash
cargo test -p zk-generator-sdk
```

## Architecture

The SDK is organized into three main modules:

- **`pda`** - PDA derivation functions
- **`instructions`** - Instruction builders for all operations
- **`accounts`** - AccountMeta helpers for complex instructions

## Type Re-exports

The SDK re-exports key types from the zk-generator program:

```rust
use zk_generator_sdk::{
    ZkGeneratorInstruction,  // Instruction enum
    ZkJob,                   // Job account structure
    CircuitType,             // Circuit type enum
    ZK_JOB_SEED,            // Job PDA seed constant
    ESCROW_SEED,            // Escrow PDA seed constant
};
```

## Error Handling

All instruction builders use `expect()` for serialization errors, as these should never fail with valid inputs. For production code, you may want to add additional validation before calling these functions.

## Integration with Bedrock

The zk-generator program integrates with the Bedrock program for prover verification and reputation management. When using CPI-enabled instructions (`claim_job`, `submit_proof`, `dispute_proof`), you'll need to derive Bedrock PDAs:

```rust
// Prover PDA from Bedrock
let (prover_pda, _) = Pubkey::find_program_address(
    &[b"prover", prover.as_ref()],
    &bedrock_program_id
);

// Config PDA from Bedrock
let (config_pda, _) = Pubkey::find_program_address(
    &[b"config"],
    &bedrock_program_id
);
```

## License

MIT or Apache-2.0

## Contributing

Contributions are welcome! Please ensure all tests pass before submitting PRs.
