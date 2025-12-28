# FHE Generator SDK

Client SDK for interacting with the ZyberLink FHE Generator program.

## Features

- **PDA Derivation**: Helper functions to derive Program Derived Addresses
- **Instruction Builders**: Type-safe instruction builders for all FHE operations
- **Type Re-exports**: Access to core types from the fhe-generator program

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
fhe-generator-sdk = { path = "../path/to/fhe-generator-sdk" }
```

## Usage

### Derive PDAs

```rust
use fhe_generator_sdk::{derive_job_pda, derive_consensus_pda, derive_escrow_pda};
use solana_sdk::pubkey::Pubkey;

let program_id = Pubkey::new_unique();
let creator = Pubkey::new_unique();
let job_id = 12345u64;

// Derive job PDA
let (job_pda, job_bump) = derive_job_pda(&program_id, &creator, job_id);

// Derive consensus PDA
let (consensus_pda, consensus_bump) = derive_consensus_pda(&program_id, job_id);

// Derive escrow PDA
let (escrow_pda, escrow_bump) = derive_escrow_pda(&program_id, &job_pda);
```

### Create a Job

```rust
use fhe_generator_sdk::{instructions::create_job, CIRCUIT_FHE_ADD};

let instruction = create_job(
    &program_id,
    &creator,
    job_id,
    CIRCUIT_FHE_ADD,          // circuit_type
    witness_hash,              // [u8; 32]
    witness_size,              // u32
    price_lamports,            // u64
    timeout_seconds,           // i64
    3,                         // required_provers
    2,                         // consensus_threshold (2 out of 3)
    0,                         // operation_param1
    0,                         // operation_param2
    0,                         // operation_param3
);
```

### Claim a Job

```rust
use fhe_generator_sdk::instructions::claim_job;
use bedrock_sdk::derive_prover_pda;

let prover = Pubkey::new_unique();
let (prover_pda, _) = derive_prover_pda(&bedrock_program_id, &prover);

let instruction = claim_job(
    &program_id,
    &prover,
    &job_pda,
    &consensus_pda,
    &prover_pda,
    &bedrock_program_id,
);
```

### Submit Result

```rust
use fhe_generator_sdk::instructions::submit_result;

let instruction = submit_result(
    &program_id,
    &prover,
    &job_pda,
    &consensus_pda,
    result_hash,  // [u8; 32]
);
```

### Finalize Job

```rust
use fhe_generator_sdk::instructions::finalize_job;

let instruction = finalize_job(
    &program_id,
    &finalizer,
    &job_pda,
    &consensus_pda,
    &escrow_pda,
    &creator,
    &fee_recipient,
    &bedrock_program_id,
    &bedrock_config,
    &prover_wallets,  // &[Pubkey]
    &prover_pdas,     // &[Pubkey]
);
```

### Cancel Job

```rust
use fhe_generator_sdk::instructions::cancel_job;

let instruction = cancel_job(
    &program_id,
    &creator,
    &job_pda,
    &consensus_pda,
    &escrow_pda,
);
```

## FHE Circuit Types

Available circuit types (re-exported from fhe-generator):

- `CIRCUIT_FHE_ADD` (4) - Encrypted addition
- `CIRCUIT_FHE_MULTIPLY` (5) - Encrypted multiplication
- `CIRCUIT_FHE_SUM` (6) - Sum of encrypted values
- `CIRCUIT_FHE_THRESHOLD` (7) - Threshold check
- `CIRCUIT_FHE_RANGE_CHECK` (8) - Range verification
- `CIRCUIT_FHE_AVERAGE` (9) - Average computation
- `CIRCUIT_FHE_COUNT_IF` (10) - Conditional counting
- `CIRCUIT_FHE_HISTOGRAM` (11) - Distribution histogram

## License

MIT OR Apache-2.0
