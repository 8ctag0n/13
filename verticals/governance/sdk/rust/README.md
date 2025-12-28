# Futarchy SDK

Rust SDK for interacting with the ZyberLink Futarchy prediction markets Solana program.

## Overview

The Futarchy SDK provides a high-level interface to:

- Create and manage prediction markets
- Place privacy-preserving bets using ZK proofs
- Settle markets and claim payouts
- Query market state from the blockchain
- Work with FHE-encrypted pool aggregation
- Implement governance-driven decision making

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
futarchy-sdk = { path = "../futarchy-sdk" }
```

## Quick Start

### Create a Client

```rust
use futarchy_sdk::*;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

let program_id = Pubkey::from_str("AQUUuRSwDhB1eeC2Caa8GPVGV4YzZkJ1YiSvZd3BBPij")?;
let client = FutarchyClient::new("https://api.devnet.solana.com", program_id);
```

### Create a Market

```rust
use solana_sdk::signature::{Keypair, Signer};

let authority = Keypair::new();
let oracle = Keypair::new().pubkey();

let market_id = 1;
let question_hash = [0u8; 32]; // Hash of market question
let end_time = 1735689600; // Unix timestamp
let max_bet = 1_000_000_000; // 1 SOL in lamports

let ix = client.create_market(
    &authority.pubkey(),
    market_id,
    question_hash,
    &oracle,
    end_time,
    max_bet,
)?;
```

### Place a Bet

```rust
let bettor = Keypair::new();
let zk_generator = Pubkey::from_str("ZK_PROGRAM_ID")?;

let bet_commitment = [1u8; 32]; // Poseidon(amount, position, blinding)
let proof = vec![0u8; 256]; // ZK proof bytes
let public_inputs = vec![1u8; 80]; // Public inputs for proof
let amount = 500_000_000; // 0.5 SOL
let circuit_type = 30; // MarketBet circuit

let ix = client.place_bet(
    &bettor.pubkey(),
    market_id,
    bet_commitment,
    proof,
    public_inputs,
    amount,
    circuit_type,
    None, // No FHE encryption
    None, // No side
    &zk_generator,
    None, // No FHE accounts
)?;
```

### Place an Encrypted Bet (FHE)

```rust
let encrypted_amount = vec![3u8; 128]; // FHE ciphertext
let side = Some(true); // true = YES, false = NO

let fhe_accounts = FheAccounts {
    fhe_job: fhe_job_pubkey,
    fhe_consensus: fhe_consensus_pubkey,
    fhe_escrow: fhe_escrow_pubkey,
    fhe_generator_program: fhe_program_id,
};

let ix = client.place_bet(
    &bettor.pubkey(),
    market_id,
    bet_commitment,
    proof,
    public_inputs,
    amount,
    31, // MarketBetWithPoI circuit
    Some(encrypted_amount),
    side,
    &zk_generator,
    Some(fhe_accounts),
)?;
```

### Settle a Market

```rust
let oracle = Keypair::new();
let outcome = true; // true = YES won, false = NO won

let ix = client.settle_market(&oracle.pubkey(), market_id, outcome)?;
```

### Claim Payout

```rust
let user = Keypair::new();
let zk_generator = Pubkey::from_str("ZK_PROGRAM_ID")?;

let claim_nullifier = [2u8; 32]; // Prevents double claims
let proof = vec![0u8; 256]; // MarketClaim proof
let public_inputs = vec![1u8; 105];
let payout_amount = 1_500_000_000; // 1.5 SOL

let ix = client.claim_payout(
    &user.pubkey(),
    market_id,
    claim_nullifier,
    proof,
    public_inputs,
    payout_amount,
    &zk_generator,
    true, // Include position account
)?;
```

If you want to generate the proof locally via snarkjs:

```rust
use futarchy_sdk::{
    generate_market_claim_proof, MarketClaimProofPaths, MarketClaimWitness, SnarkjsCommand
};

let proof_paths = MarketClaimProofPaths {
    wasm_path: "circuits/market/market_claim_js/market_claim.wasm".into(),
    zkey_path: "circuits/market/market_claim_final.zkey".into(),
};

let witness = MarketClaimWitness {
    market_id,
    nullifier: claim_nullifier,
    payout_amount,
    resolution: 1,
    total_pool: 2_000_000_000,
    winning_pool: 1_000_000_000,
    bet_commitment: [1u8; 32],
    timestamp: chrono::Utc::now().timestamp(),
    secret: [9u8; 32],
    bet_amount: 1_000_000_000,
    bet_side: 1,
    blinding: [7u8; 32],
};

let proof = generate_market_claim_proof(
    &SnarkjsCommand::npx(),
    &proof_paths,
    &witness,
)?;
```

### Query Market State

```rust
// Get a specific market
let market = client.get_market(market_id).await?;
println!("Market status: {:?}", market.status);
println!("Total pool: {} lamports", market.total_pool());
println!("YES percentage: {}%", market.yes_vote_percentage());

// Get all markets
let all_markets = client.get_all_markets().await?;
for (pubkey, market) in all_markets {
    println!("Market {}: {}", market.market_id, pubkey);
}

// Get active markets only
let active_markets = client.get_active_markets().await?;

// Get settled markets only
let settled_markets = client.get_settled_markets().await?;
```

### Find Pending FHE Jobs (For Provers)

```rust
let pending_jobs = client.get_pending_fhe_jobs().await?;

for job in pending_jobs {
    println!("Market {}: FHE job on {} side", job.market_id, if job.side { "YES" } else { "NO" });
    println!("Encrypted pool size: {} bytes", job.encrypted_pool.len());

    // Process FHE computation...
    // let result = process_fhe(&job.encrypted_pool)?;

    // Submit result
    // let ix = client.update_pool(...)?;
}
```

### Update Encrypted Pool (Prover Submission)

```rust
let updater = Keypair::new();
let fhe_job_id = 42;
let encrypted_result = vec![4u8; 128]; // New encrypted pool value
let side = true; // Update YES pool

let ix = client.update_pool(
    &updater.pubkey(),
    market_id,
    fhe_job_id,
    encrypted_result,
    side,
    &fhe_job_account,
    &fhe_consensus_account,
)?;
```

### Work with PDAs

```rust
// Derive PDAs without querying blockchain
let market_pda = client.find_market_pda(market_id);
println!("Market address: {}", market_pda.address);
println!("Market bump: {}", market_pda.bump);

let position_pda = client.find_position_pda(market_id, &user.pubkey());
println!("Position address: {}", position_pda.address);

let escrow_pda = client.find_escrow_pda(market_id);
println!("Escrow address: {}", escrow_pda.address);
```

## Architecture

### Modules

- `accounts` - PDA derivation functions
- `client` - High-level `FutarchyClient` wrapper
- `error` - Error types
- `instruction` - Instruction enum definitions
- `instructions` - Instruction builder functions
- `queries` - RPC query functions
- `state` - State account structures

### Key Types

#### Market State

```rust
pub struct Market {
    pub authority: Pubkey,
    pub market_id: u64,
    pub oracle: Pubkey,
    pub status: MarketStatus,
    pub total_yes_bets: u64,
    pub total_no_bets: u64,
    pub encrypted_pool_yes: Vec<u8>,
    pub encrypted_pool_no: Vec<u8>,
    pub resolution: Option<bool>,
    // ... governance fields
}
```

#### Market Status

```rust
pub enum MarketStatus {
    Active = 0,
    Paused = 1,
    Settled = 2,
    Cancelled = 3,
}
```

#### Executable Actions (Governance)

```rust
pub enum ExecutableAction {
    None,
    TransferLamports { recipient: Pubkey, amount: u64 },
    TransferSplToken { token_mint: Pubkey, recipient: Pubkey, amount: u64 },
    UpdateProgramData { program_id: Pubkey, new_authority: Option<Pubkey> },
    ExecuteCustom { target_program: Pubkey, instruction_data: Vec<u8> },
}
```

## Examples

See the `examples/` directory for complete working examples:

- `create_market.rs` - Create a new prediction market
- `place_bet.rs` - Place bets (both transparent and FHE-encrypted)

Run examples:

```bash
cargo run --example create_market
cargo run --example place_bet
```

## Testing

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_instruction_serialization

# Run with output
cargo test -- --nocapture
```

## Program ID

**Devnet:** `AQUUuRSwDhB1eeC2Caa8GPVGV4YzZkJ1YiSvZd3BBPij`

## API Reference

Full API documentation:

```bash
cargo doc --no-deps --open
```

## License

MIT OR Apache-2.0
