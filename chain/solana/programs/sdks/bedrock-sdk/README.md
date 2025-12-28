# Bedrock SDK

Client SDK for interacting with the ZyberLink Bedrock program.

## Overview

The Bedrock SDK provides a convenient Rust interface for building transactions that interact with the Bedrock on-chain program. Bedrock manages the core registry of provers, validators, and global configuration for the ZyberLink network.

## Features

- **PDA Derivation**: Helper functions to derive Program Derived Addresses
- **Instruction Builders**: Type-safe instruction constructors for all Bedrock operations
- **Account Helpers**: Structs to organize account metadata
- **Type Re-exports**: All Bedrock types available from a single import

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
bedrock-sdk = { path = "../../programs/sdks/bedrock-sdk" }
```

## Usage

### Initialize Bedrock

```rust
use bedrock_sdk::{derive_config_pda, instructions};
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer};

let bedrock_program_id = Pubkey::from_str("BeDrockXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX").unwrap();
let admin = Keypair::new();
let zk_generator_program = Pubkey::from_str("ZkGenXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX").unwrap();
let fhe_generator_program = Pubkey::from_str("FheGenXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX").unwrap();

let (config_pda, _) = derive_config_pda(&bedrock_program_id);

let ix = instructions::initialize(
    &bedrock_program_id,
    &admin.pubkey(),
    &config_pda,
    &zk_generator_program,
    &fhe_generator_program,
);

// Send transaction with `ix`
```

### Register a Prover

```rust
use bedrock_sdk::{derive_prover_pda, instructions};
use solana_sdk::{signature::Keypair, signer::Signer};

let prover = Keypair::new();
let (prover_pda, _) = derive_prover_pda(&bedrock_program_id, &prover.pubkey());

let ix = instructions::register_prover(
    &bedrock_program_id,
    &prover.pubkey(),
    &prover_pda,
    100_000_000, // 0.1 SOL stake
);

// Send transaction with `ix`, signed by `prover`
```

### Update Prover Stats (from Generator)

This is typically called via CPI from generator programs:

```rust
use bedrock_sdk::{derive_prover_pda, instructions};

let prover_wallet = Pubkey::from_str("ProverXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX").unwrap();
let (prover_pda, _) = derive_prover_pda(&bedrock_program_id, &prover_wallet);

let ix = instructions::update_prover_stats(
    &bedrock_program_id,
    &generator_program_id,
    &prover_pda,
    true,  // job_completed
    false, // job_failed
);

// Issue as CPI from generator program
```

### Register a Validator

```rust
use bedrock_sdk::{derive_validator_pda, instructions, ValidatorRegion};
use solana_sdk::{signature::Keypair, signer::Signer};

let validator = Keypair::new();
let (validator_pda, _) = derive_validator_pda(&bedrock_program_id, &validator.pubkey());

let ix = instructions::register_validator(
    &bedrock_program_id,
    &validator.pubkey(),
    &validator_pda,
    "https://validator.example.com".to_string(),
    ValidatorRegion::NorthAmerica,
    100_000_000, // 0.1 SOL stake
);

// Send transaction with `ix`, signed by `validator`
```

### Slash a Prover (from Generator)

```rust
use bedrock_sdk::{instructions, SlashReason};

let ix = instructions::slash_prover(
    &bedrock_program_id,
    &generator_program_id,
    &prover_pda,
    10_000_000, // slash amount in lamports
    SlashReason::Timeout,
);

// Issue as CPI from generator program
```

## PDA Derivation

The SDK provides helper functions to derive all Program Derived Addresses:

```rust
use bedrock_sdk::{derive_config_pda, derive_prover_pda, derive_validator_pda};

// Config PDA (singleton)
let (config_pda, config_bump) = derive_config_pda(&program_id);

// Prover PDA (per wallet)
let (prover_pda, prover_bump) = derive_prover_pda(&program_id, &prover_wallet);

// Validator PDA (per wallet)
let (validator_pda, validator_bump) = derive_validator_pda(&program_id, &validator_wallet);
```

## Account Structures

All account structures from the Bedrock program are re-exported:

- `BedrockConfig` - Global configuration
- `ProverAccount` - Prover registration and stats
- `ValidatorAccount` - Validator registration and stats

## Enums

- `SlashReason` - Reasons for slashing (Timeout, InvalidResult, ConsensusMismatch, InvalidKeyShare)
- `ValidatorRegion` - Geographic regions for validators (NorthAmerica, Europe, Asia, LatinAmerica, Africa)

## Testing

Run the SDK tests:

```bash
cd /home/deploy/2025q4/13-area2-provers/src/programs
cargo test -p bedrock-sdk
```

## Architecture

The Bedrock SDK is designed to work with the Bedrock on-chain program, which serves as the core registry for:

- **Global Configuration**: Admin authority, generator program IDs, threshold encryption keys
- **Prover Registry**: Registered provers with stake, reputation, and activity tracking
- **Validator Registry**: Validators for threshold encryption with geographic distribution

## License

MIT OR Apache-2.0
