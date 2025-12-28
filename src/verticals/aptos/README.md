# ZyberLink Aptos - Privacy-Preserving Compute Infrastructure

Port of ZyberLink infrastructure to Aptos blockchain, enabling privacy-preserving computations with FHE and ZK proofs.

## Architecture Overview

```
┌─────────────────────────────────────────────────────┐
│  SHARED OFF-CHAIN INFRASTRUCTURE                    │
│  ├── Workers (Rust) - Monitor Aptos, Solana, Starknet│
│  ├── Proof Server (snarkjs)                         │
│  ├── Witness Generator                              │
│  └── FHE Engine (tfhe-rs)                           │
└──────┬──────────────────────────────────────────────┘
       │
       ↓ Monitors Aptos events
       │
┌──────▼──────────────────────────────────────────────┐
│  APTOS ON-CHAIN (Move Modules)                      │
│  ├── jobs.move - Job orchestration ✅                │
│  ├── zk_verifier.move - Groth16 verification        │
│  ├── payments.move - Escrow & rewards               │
│  ├── bft_consensus.move - Multi-worker consensus    │
│  └── privacy_infra.move - Private credit (Phase 2)  │
└──────────────────────────────────────────────────────┘
```

## Components

### On-Chain (Move)

#### 1. `jobs.move` ✅ IMPLEMENTED
- **Job lifecycle**: Submit → Claim → Compute → Verify → Payment
- **Escrow system**: Locks payment until job completion
- **Events**: `JobCreated`, `JobClaimed`, `JobCompleted`
- **Status**: Pending → Claimed → Completed/Failed

**Functions**:
- `submit_job()` - User submits encrypted task data + payment
- `claim_job()` - Worker claims pending job
- `submit_result()` - Worker submits encrypted result + ZK proof
- `get_job()` - View function to query job details

#### 2. `zk_verifier.move` - TODO
- Uses Aptos native BN254 support (confirmed available ✅)
- Verifies Groth16 proofs on-chain
- Module: `aptos_std::crypto_algebra::pairing`
- Gas cost: ~140M units per verification (within 200M limit)

#### 3. `payments.move` - TODO
- APT/USDC escrow management
- Worker rewards distribution
- Slashing for invalid proofs

#### 4. `bft_consensus.move` - TODO
- Multi-worker result consensus
- Byzantine fault tolerance (2-of-3, 3-of-5)
- Slash dissenters, reward majority

#### 5. `privacy_infra.move` - TODO (Phase 2)
- Private lending pools
- Encrypted loan data
- NFT collateral integration

### Off-Chain (Rust Workers)

**Shared with Solana/Starknet**:
- Monitor job events from Aptos
- Fetch encrypted data (IPFS/Arweave)
- Perform FHE computations (tfhe-rs)
- Generate ZK proofs (snarkjs)
- Submit results to Aptos

**Adapters needed**:
- Aptos client integration
- Event listener for `JobCreatedEvent`
- Transaction builder for `submit_result()`

## Development Setup

### Prerequisites
- Aptos CLI (install: https://aptos.dev/tools/install-cli/)
- Move compiler
- Rust (for workers)

### Installation

```bash
# Install Aptos CLI
curl -fsSL "https://aptos.dev/scripts/install_cli.py" | python3

# Initialize Aptos account
aptos init --network devnet

# Build contracts
cd aptos/contracts
aptos move compile

# Run tests
aptos move test
```

### Deploy to Devnet

```bash
# Deploy contracts
aptos move publish --profile devnet

# Initialize jobs registry
aptos move run \
  --function-id 'default::jobs::initialize' \
  --profile devnet
```

## Job Lifecycle Example

### 1. User submits job

```typescript
import { AptosClient, AptosAccount } from "aptos";

const client = new AptosClient("https://fullnode.devnet.aptoslabs.com");
const user = new AptosAccount();

const payload = {
  type: "entry_function_payload",
  function: "0x123::jobs::submit_job",
  type_arguments: [],
  arguments: [
    "0x123",                        // registry_addr
    Array.from(encryptedData),      // task_data
    "1000000",                      // payment (0.01 APT)
    "10",                           // circuit_type
  ],
};

const txn = await client.generateTransaction(user.address(), payload);
const signedTxn = await client.signTransaction(user, txn);
await client.submitTransaction(signedTxn);
```

### 2. Worker monitors events

```rust
// Worker Rust code (to be implemented)
let events = aptos_client
    .get_events_by_event_handle(
        registry_addr,
        "JobsRegistry",
        "job_created_events",
        None,
        None,
    )
    .await?;

for event in events {
    let job_id = event.data["job_id"];
    // Claim job
    claim_job(worker, registry_addr, job_id).await?;
}
```

### 3. Worker computes + generates proof

```rust
// Off-chain computation (shared infrastructure)
let witness = fetch_witness_from_ipfs(&job.task_data).await?;
let fhe_result = fhe_engine.compute(&witness)?;
let proof = zk_prover.generate_proof(circuit_type, &witness, &fhe_result)?;
```

### 4. Worker submits result

```rust
// Submit back to Aptos
submit_result(
    worker,
    registry_addr,
    job_id,
    fhe_result,
    proof.to_bytes(),
    public_inputs,
).await?;
```

## BN254 Support Confirmation ✅

**Research completed** (Dec 15, 2025):
- Aptos has **native BN254** support via `aptos_std::crypto_algebra`
- Generic Groth16 verifier available in Move framework
- Production validation: Aptos Keyless Wallets use Groth16/BN254
- Gas costs: ~140M units per verification (practical for real-world use)

**Decision**: Use native on-chain verification (Option A)
**Timeline**: 5-9 weeks for complete ZK integration

See: `/private/research/BN254_AVAILABILITY.md` for full analysis

## Roadmap

### Phase 0: Foundation (Week 1-4)
- [x] Setup project structure
- [x] Implement `jobs.move` (base version)
- [ ] Deploy to devnet
- [ ] Basic testing

### Phase 1: ZK Verification (Week 3-8)
- [ ] Implement `zk_verifier.move` using native BN254
- [ ] Register verification keys for circuits
- [ ] Integrate verification into `submit_result()`
- [ ] E2E test with real proofs

### Phase 2: Multi-Worker BFT (Week 7-12)
- [ ] Implement `bft_consensus.move`
- [ ] Worker registry + staking
- [ ] Slashing for invalid proofs
- [ ] 2-of-3 consensus testing

### Phase 3: Private Credit (Week 9-14)
- [ ] Implement `privacy_infra.move`
- [ ] Encrypted lending pools
- [ ] NFT collateral integration
- [ ] FHE-based liquidation checks

### Phase 4: Production (Week 13-16)
- [ ] Security audit
- [ ] Testnet stress testing
- [ ] Mainnet deployment
- [ ] Grant applications

## Testing

```bash
# Run all tests
aptos move test

# Run specific test
aptos move test --filter test_full_job_lifecycle

# Gas profiling
aptos move test --gas-profile
```

## Key Differences from Solana

| Aspect | Solana | Aptos |
|--------|--------|-------|
| Language | Rust | Move |
| Accounts | Account model | Resource model |
| Concurrency | Parallel execution | STM (optimistic) |
| ZK Support | Limited | Native BN254 ✅ |
| Gas Model | Compute units | Gas units (higher for ZK) |
| Events | Program logs | Typed events ✅ |

## Resources

- [Aptos Docs](https://aptos.dev/)
- [Move Book](https://move-language.github.io/move/)
- [Aptos Cryptography](https://aptos.dev/build/smart-contracts/cryptography)
- [BN254 Research](../private/research/BN254_AVAILABILITY.md)

## Next Steps

1. **Implement `zk_verifier.move`** - Use native BN254 pairing
2. **Adapt workers** - Add Aptos client to existing Rust workers
3. **Testing** - E2E flow with real proofs from Solana circuits
4. **Private Credit** - Build encrypted lending on top of jobs module

---

**Status**: Phase 0 - Foundation in progress
**Last Updated**: 2025-12-16
