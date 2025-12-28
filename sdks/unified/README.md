# ZyberLink Unified SDK

A unified Rust SDK for all ZyberLink programs: Bedrock, ZK-Generator, FHE-Generator, and Futarchy.

## Current Status: PHASE 1 - Core Types

This SDK is currently in **PHASE 1** of implementation, providing the foundational `UnifiedJob` abstraction.

### Implemented (Phase 1)

- `UnifiedJob` enum - Represents jobs from any generator (ZK, FHE, Legacy)
- `ZkJobData`, `FheJobData`, `LegacyJobData` - Type-specific job data
- `FheConsensusData` - Multi-prover consensus state for FHE jobs
- `JobStatus` - Job state management with extension trait
- `CircuitType` - Circuit type constants and helpers
- Error types (`UnifiedError`, `Result`)

### Coming Soon

- **Phase 1.2**: Job queries (`JobQuery`) - Cross-program job search
- **Phase 2**: Program-specific SDKs (Bedrock, ZK, FHE, Futarchy)
- **Phase 3**: Unified client (`ZyberUnified`)
- **Phase 4**: Prover utilities (`JobListener`, `JobClaimer`, `JobSubmitter`)
- **Phase 5**: Utils and crypto helpers

## Core Abstraction: UnifiedJob

The `UnifiedJob` enum provides a single API for working with jobs from any generator:

```rust
use zyberlink_unified_sdk::prelude::*;

fn process_job(job: UnifiedJob) {
    // Universal access to common fields
    println!("Job ID: {}", job.id());
    println!("Price: {} lamports", job.price());
    println!("Status: {:?}", job.status());
    println!("Circuit: {}", circuit_name(job.circuit_type()));
    
    // Type-specific handling
    match job {
        UnifiedJob::Zk(data) => {
            println!("ZK job with circuit {}", data.circuit_type);
        }
        UnifiedJob::Fhe(data) => {
            println!("FHE job requiring {} provers", 
                data.consensus.as_ref().map(|c| c.required_provers).unwrap_or(1)
            );
        }
        UnifiedJob::Legacy(data) => {
            println!("Legacy job (deprecated)");
        }
    }
    
    // Helper methods
    if job.can_claim() {
        println!("Job is claimable!");
    }
    
    if job.is_fhe() && job.has_fhe_consensus() {
        println!("FHE consensus reached");
    }
}
```

## Job Types

### ZK Jobs (ZK-Generator)

Single-prover jobs for zero-knowledge proofs:

```rust
let zk_job = UnifiedJob::Zk(ZkJobData {
    pubkey: job_address,
    common: job_common,
    circuit_type: 30, // MarketBet
});

assert!(zk_job.is_zk());
assert!(!zk_job.is_fhe());
```

### FHE Jobs (FHE-Generator)

Multi-prover jobs with consensus:

```rust
let fhe_job = UnifiedJob::Fhe(FheJobData {
    pubkey: job_address,
    common: job_common,
    circuit_type: 4, // FHE_ADD
    fhe_consensus_bump: 250,
    consensus: Some(consensus_data),
});

assert!(fhe_job.is_fhe());
if let Some(consensus) = fhe_job.fhe_consensus() {
    println!("Required provers: {}", consensus.required_provers);
    println!("Claimed: {}", consensus.claimed_count);
}
```

### Legacy Jobs (Deprecated)

Backwards compatibility with the monolithic zyberlink program:

```rust
let legacy_job = UnifiedJob::Legacy(LegacyJobData::from_account(
    job_address,
    legacy_account,
    fhe_consensus_opt,
));

assert!(legacy_job.is_legacy());
```

## Circuit Types

The SDK provides constants and helpers for all circuit types:

### ZK Circuits

- **Legacy (0-3)**: `CIRCUIT_ZCASH_ORCHARD`, `CIRCUIT_ZCASH_SAPLING`, etc.
- **v2.0 Core (10-19)**: `CIRCUIT_PROOF_OF_INNOCENCE`
- **v2.0 Voting (20-29)**: `CIRCUIT_PRIVATE_VOTE`, `CIRCUIT_PRIVATE_VOTE_POI`
- **v2.0 Market (30-39)**: `CIRCUIT_MARKET_BET`, `CIRCUIT_MARKET_BET_POI`, `CIRCUIT_MARKET_CLAIM`
- **v2.0 Portfolio (40-49)**: `CIRCUIT_PORTFOLIO_COMPLIANCE`, `CIRCUIT_PORTFOLIO_NET_WORTH`

### FHE Circuits

- **Operations (4-11)**: `CIRCUIT_FHE_ADD`, `CIRCUIT_FHE_MULTIPLY`, `CIRCUIT_FHE_SUM`, etc.
- **Futarchy (12)**: `CIRCUIT_FHE_FUTARCHY_POOL_UPDATE`

### Circuit Helpers

```rust
use zyberlink_unified_sdk::prelude::*;

// Check circuit type
assert!(is_zk_circuit(0));  // ZCASH_ORCHARD
assert!(is_fhe_circuit(4)); // FHE_ADD
assert!(is_v2_circuit(30)); // MARKET_BET

// Get human-readable name
println!("{}", circuit_name(30)); // "Market Bet"

// Check PoI requirement
assert!(requires_poi(21)); // PRIVATE_VOTE_POI
```

## Error Handling

```rust
use zyberlink_unified_sdk::prelude::*;

fn example() -> Result<()> {
    // Operations that may fail
    let job = deserialize_job(&data)
        .map_err(|e| UnifiedError::DeserializationError(e.to_string()))?;
    
    if !job.can_claim() {
        return Err(UnifiedError::InvalidJobState(
            "Job is not claimable".to_string()
        ));
    }
    
    Ok(())
}
```

## Architecture

```text
zyberlink-unified-sdk/
├── src/
│   ├── core/
│   │   ├── job.rs       - UnifiedJob, ZkJobData, FheJobData, LegacyJobData
│   │   ├── status.rs    - JobStatus + extension trait
│   │   ├── circuits.rs  - Circuit constants and helpers
│   │   ├── error.rs     - UnifiedError + Result
│   │   └── mod.rs       - Re-exports
│   └── lib.rs           - Public API + prelude
```

## Dependencies

- `zyberlink-jobs` - Shared `JobCommon` structure
- `zyberlink-types` - Shared types (`JobStatus`, `CircuitType`)
- `solana-sdk` - Solana primitives
- `borsh` - Serialization
- `thiserror` - Error handling

## Design Decisions

### Why Enum vs Trait?

We use an `enum` for `UnifiedJob` (vs a trait-based approach) because:

1. **Exhaustive matching**: Compiler ensures all cases are handled
2. **Zero-cost**: No vtable overhead or dynamic dispatch
3. **Simple serialization**: Direct borsh (de)serialization
4. **Limited variants**: Only 3 job types (Zk, Fhe, Legacy)
5. **Pattern matching ergonomics**: `if let UnifiedJob::Fhe { .. }`

See `private/sdk-technical-decisions.md` for full rationale.

## Testing

Run tests:

```bash
cargo test -p zyberlink-unified-sdk
```

## Contributing

This SDK is part of the larger wt-infra monorepo. See the main README for contribution guidelines.

## License

MIT OR Apache-2.0
