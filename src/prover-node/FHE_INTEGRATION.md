# FHE Integration for CypherLink Prover Node

## Overview

The CypherLink prover node now supports **Fully Homomorphic Encryption (FHE)** computations using [TFHE-rs (Concrete)](https://github.com/zama-ai/tfhe-rs), allowing provers to perform computations on encrypted data without ever decrypting it.

## Architecture

### High-Level Flow

```
┌─────────────┐           ┌─────────────┐           ┌─────────────┐
│   Client    │           │  Prover Node│           │   Solana    │
│             │           │             │           │   Chain     │
└──────┬──────┘           └──────┬──────┘           └──────┬──────┘
       │                         │                         │
       │ 1. Generate keypair     │                         │
       │    (client_key,         │                         │
       │     server_key)         │                         │
       │                         │                         │
       │ 2. Encrypt data         │                         │
       │    (with client_key)    │                         │
       │                         │                         │
       │ 3. Create FHE job ───────────────────────────────>│
       │    (encrypted data +    │                         │
       │     server_key)         │                         │
       │                         │                         │
       │                         │ 4. Poll for jobs        │
       │                         │<─────────────────────────│
       │                         │                         │
       │                         │ 5. Claim job            │
       │                         │─────────────────────────>│
       │                         │                         │
       │                         │ 6. Compute on           │
       │                         │    encrypted data       │
       │                         │    (using server_key)   │
       │                         │                         │
       │                         │ 7. Hash result          │
       │                         │    for consensus        │
       │                         │                         │
       │                         │ 8. Submit result hash   │
       │                         │─────────────────────────>│
       │                         │                         │
       │ 9. Download result      │                         │
       │<──────────────────────────────────────────────────│
       │                         │                         │
       │ 10. Decrypt result      │                         │
       │     (with client_key)   │                         │
       │                         │                         │
```

### Components

#### 1. FHE Engine (`src/fhe_engine.rs`)

Core FHE computation engine providing:
- Homomorphic addition, multiplication, subtraction
- Addition of two encrypted values
- Result hashing for consensus
- Key serialization/deserialization

#### 2. Prover Node Integration (`src/main.rs`)

Extended prover node to handle `CircuitType::FheComputation`:
- Loads FHE server key on startup
- Processes FHE jobs alongside ZK proof jobs
- Executes FHE operations in blocking threads (CPU-intensive)
- Submits encrypted results to chain

#### 3. Key Management

FHE keys are large (~50MB for server key) and require careful management:
- **Client Key**: Private, used for encryption/decryption
- **Server Key**: Public, used for homomorphic computations
- Keys generated using `generate-fhe-keys` binary

## Setup and Configuration

### 1. Generate FHE Keys

```bash
# Generate keypair
cd prover-node
cargo run --bin generate-fhe-keys -- --output-dir ./keys --test-value 100

# Output:
#   keys/fhe_client_key.bin  (~50MB) - KEEP PRIVATE
#   keys/fhe_server_key.bin  (~50MB) - Distribute to provers
#   keys/test_encrypted_value.bin (optional test data)
```

### 2. Configure Prover Node

Start prover with FHE support:

```bash
cypherlink-prover \
  --program-id <PROGRAM_ID> \
  --rpc-url <RPC_URL> \
  --fhe-server-key-path ./keys/fhe_server_key.bin \
  --keypair ~/.config/solana/id.json
```

**Important**: Without `--fhe-server-key-path`, the prover will skip FHE jobs.

### 3. Create FHE Job (Client Side)

```rust
use cypherlink_sdk::CypherlinkClient;
use cypherlink_types::{CircuitType, FheOperation};
use tfhe::{prelude::*, FheUint8, ConfigBuilder, generate_keys};

// 1. Generate keys
let config = ConfigBuilder::default().build();
let (client_key, server_key) = generate_keys(config);

// 2. Encrypt input data
let plaintext = 100u8;
let encrypted = FheUint8::try_encrypt(plaintext, &client_key)?;
let encrypted_bytes = bincode::serialize(&encrypted)?;

// 3. Create FHE job
let circuit_type = CircuitType::FheComputation(FheOperation::Add(50));

let job_id = client.create_fhe_job(
    circuit_type,
    encrypted_bytes,
    server_key,
    price_lamports,
).await?;

// 4. Wait for result and decrypt
let result_bytes = client.get_fhe_result(job_id).await?;
let result: FheUint8 = bincode::deserialize(&result_bytes)?;
let decrypted: u8 = result.decrypt(&client_key);

assert_eq!(decrypted, 150); // 100 + 50
```

## Supported Operations

### Add Constant
```rust
FheOperation::Add(constant: u8)
```
Adds a constant to encrypted value: `encrypt(x) + c = encrypt(x + c)`

**Performance**: ~150ms

### Multiply Constant
```rust
FheOperation::Multiply(constant: u8)
```
Multiplies encrypted value by constant: `encrypt(x) * c = encrypt(x * c)`

**Performance**: ~200ms

### Subtract Constant
```rust
FheOperation::Subtract(constant: u8)
```
Subtracts constant from encrypted value: `encrypt(x) - c = encrypt(x - c)`

**Performance**: ~150ms

### Add Two Encrypted Values
```rust
engine.compute_add_encrypted(encrypted_a, encrypted_b)
```
Adds two encrypted values: `encrypt(a) + encrypt(b) = encrypt(a + b)`

**Performance**: ~300ms

## Performance Characteristics

Based on benchmarks from `spike-fhe-prover`:

| Operation | Time | Notes |
|-----------|------|-------|
| Key Generation | 5-10s | One-time, cache keys |
| Encryption | 10-20ms | Client-side |
| FHE Addition | 120-150ms | Prover-side |
| FHE Multiplication | 150-200ms | Prover-side |
| Decryption | 5-10ms | Client-side |

**Data Sizes**:
- Client key: ~50MB
- Server key: ~50MB
- Encrypted u8: ~65KB
- Result: ~65KB

## Multi-Prover Consensus

FHE jobs support consensus checking across multiple provers:

1. **Multiple provers** compute the same encrypted operation
2. Each prover submits a **hash of their result**
3. Chain verifies **2-of-3 consensus** (configurable)
4. Result is accepted if threshold met

**Hash Function**: SHA3-256 of encrypted result bytes

```rust
let result_hash = FheEngine::hash_result(&encrypted_result_bytes);
// Submit hash to chain for consensus
```

## Testing

### Unit Tests

```bash
# Test FHE engine operations
cargo test --lib fhe_engine

# Run all tests
cargo test
```

### Integration Tests

```bash
# Test complete FHE workflow
cargo test --test fhe_integration_test

# Run with output
cargo test --test fhe_integration_test -- --nocapture

# Run slow multi-prover test
cargo test --test fhe_integration_test multi_prover -- --ignored --nocapture
```

### Performance Benchmarks

```bash
# Benchmark FHE operations
cargo test --test fhe_integration_test performance_benchmark -- --nocapture
```

## Key Management Strategies

### Option A: Job-Embedded (POC - Not Recommended)
Store server key in job metadata or witness data.

**Pros**: Simple
**Cons**: 50MB per job, expensive

### Option B: Config File (Current Implementation)
Store server key in local file, pass to prover via CLI.

**Pros**: Simple, works for demo
**Cons**: Manual key distribution

### Option C: Per-Job Generation (Secure)
Client generates fresh keypair for each job.

**Pros**: Most secure
**Cons**: 5-10s overhead per job

### Option D: Key Registry (Production)
Store server keys in on-chain registry or IPFS.

**Pros**: Scalable, decentralized
**Cons**: Requires infrastructure

**Recommendation**: Start with Option B (config file) for POC, migrate to Option D for production.

## Security Considerations

### Client Key Protection
- **Never share client key** - it can decrypt all data
- Store securely (hardware wallet, encrypted storage)
- Rotate regularly for long-term jobs

### Server Key Distribution
- Server keys are public (safe to share with provers)
- Can be stored on-chain, IPFS, or distributed directly
- Verify integrity with hash/signature

### Result Verification
- Always use multi-prover consensus for critical operations
- Minimum 2-of-3 provers recommended
- Higher thresholds (3-of-5) for high-value operations

### Timing Attacks
- FHE operations have constant time (by design)
- No information leakage from computation time
- Hash-based consensus prevents result manipulation

## Limitations and Future Work

### Current Limitations

1. **Data Type**: Only `u8` (0-255) supported
   - **Future**: u16, u32, u64, signed integers

2. **Operations**: Basic arithmetic only
   - **Future**: Comparisons, conditionals, bitwise ops

3. **Key Size**: 50MB per keypair
   - **Future**: Key compression, bootstrapping optimization

4. **Performance**: 150ms per operation
   - **Future**: GPU acceleration, circuit optimization

### Roadmap

- [ ] Support larger integer types (u16, u32, u64)
- [ ] Implement comparison operations (>, <, ==)
- [ ] Add conditional operations (if/else on encrypted data)
- [ ] GPU acceleration for faster computation
- [ ] Key registry smart contract
- [ ] Off-chain result storage (IPFS/Arweave)
- [ ] Advanced circuits (ML inference, private voting)

## Troubleshooting

### "FHE engine not initialized"
**Cause**: Prover started without `--fhe-server-key-path`

**Solution**: Generate keys and provide path:
```bash
cargo run --bin generate-fhe-keys
cypherlink-prover --fhe-server-key-path ./keys/fhe_server_key.bin ...
```

### "Failed to deserialize input ciphertext"
**Cause**: Encrypted data format mismatch

**Solution**: Ensure client uses same TFHE version and `FheUint8` type

### Slow Performance
**Cause**: FHE operations are CPU-intensive

**Solutions**:
- Use release build: `cargo build --release`
- Enable CPU features: `RUSTFLAGS="-C target-cpu=native"`
- Increase prover resources (more cores)

### Consensus Failures
**Cause**: Different provers producing different hashes

**Possible Reasons**:
- Different TFHE versions
- Different server keys
- Corrupted encrypted data

**Solution**: Ensure all provers use identical server keys and TFHE version

## Example: End-to-End FHE Job

See `tests/fhe_integration_test.rs` for complete working examples.

### Minimal Example

```rust
use prover_node::{generate_fhe_keys, FheEngine};
use tfhe::{prelude::*, FheUint8};

// Client: Encrypt
let (client_key, server_key) = generate_fhe_keys()?;
let encrypted = FheUint8::try_encrypt(42u8, &client_key)?;
let encrypted_bytes = bincode::serialize(&encrypted)?;

// Prover: Compute
let engine = FheEngine::new(server_key);
let result_bytes = engine.compute_add(&encrypted_bytes, 10)?;

// Client: Decrypt
let result: FheUint8 = bincode::deserialize(&result_bytes)?;
let decrypted: u8 = result.decrypt(&client_key);
assert_eq!(decrypted, 52);
```

## References

- [TFHE-rs Documentation](https://docs.zama.ai/tfhe-rs)
- [Concrete Whitepaper](https://whitepaper.zama.ai/)
- [CypherLink Architecture](../README.md)
- [Spike Results](../../spike-fhe-prover/REPORT.md)

## Support

For questions or issues:
- Open GitHub issue
- Check integration tests for examples
- Review spike results for performance baselines
