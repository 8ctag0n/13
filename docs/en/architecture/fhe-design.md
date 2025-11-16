# FHE Design

Understanding how ZyberLink handles Fully Homomorphic Encryption computations.

## What is FHE?

**Fully Homomorphic Encryption (FHE)** allows computations to be performed directly on encrypted data without decryption.

**Example:**
```
Traditional approach:
encrypted(5) → decrypt → 5 + 7 = 12 → encrypt → encrypted(12)
                ↑ Data exposed during computation

FHE approach:
encrypted(5) + encrypted(7) = encrypted(12)
     ↑ Data NEVER decrypted, always encrypted
```

**Why this matters:**
- Client never exposes sensitive data to provers
- Computations happen on encrypted values
- Only the client can decrypt final results

## FHE in ZyberLink

### Supported Operations

**Current (Phase 1):**
- **Addition:** `enc(a) + enc(b) = enc(a + b)`
- **Multiplication:** `enc(a) × enc(b) = enc(a × b)`
- **Subtraction:** `enc(a) - enc(b) = enc(a - b)`

**Future (Phase 2+):**
- Comparisons (greater than, less than, equal)
- Boolean logic (AND, OR, NOT)
- Advanced circuits (division, modulo)

### FHE Library: TFHE-rs

ZyberLink uses **TFHE-rs** by Zama for FHE computations.

**Why TFHE-rs:**
- Production-ready and actively maintained
- Comprehensive operation support
- Good performance for modern hardware
- Strong Rust integration
- Well-documented

**Version:** 0.6+

## Architecture

### Client Side

**1. Key Generation**

Client generates FHE keys:
```rust
// Client generates keys locally
let client_key = ClientKey::new(&config);
let server_key = ServerKey::new(&client_key);
```

- **Client Key:** Private, never shared (for encryption/decryption)
- **Server Key:** Public, sent to provers (for computation only)

**2. Encryption**

Client encrypts inputs before posting job:
```rust
let encrypted_a = client_key.encrypt(5u64);
let encrypted_b = client_key.encrypt(7u64);

// Post encrypted job to marketplace
marketplace.create_fhe_job(
    FheOperation::Add,
    vec![encrypted_a, encrypted_b],
    server_key
);
```

**3. Decryption**

After consensus, client retrieves and decrypts result:
```rust
let encrypted_result = marketplace.get_result(job_id);
let result: u64 = client_key.decrypt(&encrypted_result);
// result = 12
```

### Prover Side

**1. Receive Job**

Prover claims FHE job from marketplace:
```rust
let job = marketplace.claim_job(job_id);
let server_key = job.server_key;
let encrypted_inputs = job.encrypted_inputs;
```

**2. Compute on Encrypted Data**

Prover executes operation WITHOUT seeing plaintext:
```rust
let encrypted_result = match job.operation {
    FheOperation::Add => {
        server_key.add(&encrypted_inputs[0], &encrypted_inputs[1])
    },
    FheOperation::Multiply => {
        server_key.mul(&encrypted_inputs[0], &encrypted_inputs[1])
    },
    // ...
};
```

**Critical:** Prover **never** sees plaintext values, only encrypted ciphertexts.

**3. Submit Result**

Prover submits encrypted result hash:
```rust
let result_hash = hash(&encrypted_result);
marketplace.submit_fhe_result(job_id, result_hash, encrypted_result);
```

## Consensus Mechanism

Since FHE results cannot be cryptographically verified on-chain (unlike ZK proofs), ZyberLink uses **multi-prover consensus**.

### How It Works

**Step 1:** Multiple provers (typically 3) claim the same job

**Step 2:** Each prover computes independently:
```
Prover A: enc(5) + enc(7) → enc(12) → hash_A
Prover B: enc(5) + enc(7) → enc(12) → hash_B
Prover C: enc(5) + enc(7) → enc(12) → hash_C
```

**Step 3:** On-chain consensus verification:
```rust
if hash_A == hash_B == hash_C {
    // 3/3 consensus - all provers agree
    job.status = JobStatus::Completed;
    pay_all_provers();
} else if hash_A == hash_B {
    // 2/3 consensus - majority agrees
    job.status = JobStatus::Completed;
    pay_matching_provers(A, B);
    penalize_prover(C);
} else {
    // No consensus - job fails
    job.status = JobStatus::Failed;
    refund_client();
}
```

**Threshold:** Minimum 2-of-3 agreement required.

### Why Consensus?

**Problem:** Can't verify FHE computations cryptographically
**Solution:** Economic security through majority voting

**Trust Model:**
- Don't need to trust any single prover
- Byzantine fault tolerant (works even with dishonest provers)
- Economic disincentive for dishonesty (reputation loss + potential slashing)

## Security Guarantees

### Data Privacy

**What provers CAN'T see:**
- ❌ Input values (always encrypted)
- ❌ Output values (encrypted until client decrypts)
- ❌ Intermediate computation results (encrypted throughout)

**What provers CAN see:**
- ✅ Operation type (Add, Multiply, etc.)
- ✅ Number of inputs
- ✅ Encrypted ciphertexts (opaque blobs)
- ✅ Server key (public, used for computation only)

**Privacy Level:** Same as local computation (data never exposed).

### Result Correctness

**Threat:** Dishonest prover submits wrong result

**Protection:**
1. **Consensus:** Need 2+ matching results
2. **Reputation:** Dishonest provers lose reputation
3. **Slashing (future):** Stake is slashed for bad behavior

**Attack Cost:** Must corrupt 2+ independent provers simultaneously (expensive and detectable).

### Known Limitations

**What we DON'T protect against:**
- ❌ Client-side key compromise
- ❌ Byzantine majority (all provers colluding)
- ❌ Side-channel attacks on prover hardware

**Mitigation:**
- Client security is client's responsibility
- Economic incentives prevent collusion
- Use secure hardware where available

## Performance

### Computation Times

**Typical performance on modern hardware (8-core CPU):**

| Operation | Time |
|-----------|------|
| Addition | 2-3 seconds |
| Multiplication | 4-6 seconds |
| Subtraction | 2-3 seconds |

**With GPU acceleration (future):**
- 10-100x speedup expected
- CUDA/ROCm support planned

### Network Overhead

**Data sizes:**
- Encrypted input: ~4-8 KB per value
- Server key: ~50-100 KB
- Encrypted result: ~4-8 KB
- Result hash: 32 bytes (on-chain)

**Total bandwidth per job:** ~100-200 KB

### Cost Analysis

**Per FHE operation:**
- Client pays: ~0.002 SOL (~$0.02)
- Prover earns: ~0.0018 SOL (~$0.018)
- Platform fee: ~0.0002 SOL (~$0.002)

## Use Cases

### 1. Private DeFi

**Problem:** DeFi reveals transaction amounts on-chain

**Solution:** FHE-powered private swaps
```rust
// User wants to swap 100 USDC for ETH privately
let encrypted_amount = encrypt(100);
let encrypted_result = fhe_swap(encrypted_amount, USDC, ETH);
// Blockchain sees: encrypted swap occurred
// Blockchain can't see: how much was swapped
```

### 2. Confidential Voting

**Problem:** Vote tallies reveal partial results during voting

**Solution:** Encrypted vote accumulation
```rust
let encrypted_votes = votes.map(|v| encrypt(v));
let encrypted_total = encrypted_votes.reduce(|a, b| fhe_add(a, b));
// Votes are never decrypted until voting ends
```

### 3. Private Analytics

**Problem:** Can't compute on sensitive datasets without exposing data

**Solution:** FHE-powered analytics
```rust
let encrypted_salaries = employee_data.map(|e| encrypt(e.salary));
let encrypted_average = fhe_average(encrypted_salaries);
// Compute average salary without seeing individual salaries
```

## Future Improvements

### Phase 2: Advanced Operations

- **Comparisons:** `enc(a) > enc(b)`
- **Boolean logic:** `enc(a) AND enc(b)`
- **Conditional execution:** `if enc(condition) then enc(x) else enc(y)`

### Phase 3: Hardware Acceleration

- **GPU support:** 10-100x speedup
- **FPGA co-processors:** Custom FHE accelerators
- **Specialized silicon:** Dedicated FHE chips

### Phase 4: Advanced Features

- **Threshold decryption:** Multi-party key sharing
- **Homomorphic operations on floating point:** More versatile computations
- **Circuit privacy:** Hide the computation itself (not just data)

## Developer Guide

### Creating an FHE Job

```rust
use zyberlink_sdk::MarketplaceClient;
use tfhe::prelude::*;

// 1. Generate keys
let config = ConfigBuilder::default().build();
let (client_key, server_key) = generate_keys(config);

// 2. Encrypt inputs
let a = client_key.encrypt(42u64);
let b = client_key.encrypt(58u64);

// 3. Create job
let client = MarketplaceClient::new(...);
let job_id = client.create_fhe_job(
    FheOperation::Add,
    vec![a, b],
    server_key,
    reward_lamports,
).await?;

// 4. Wait for consensus
let result = client.wait_for_job(job_id).await?;

// 5. Decrypt result
let plaintext: u64 = client_key.decrypt(&result);
println!("Result: {}", plaintext); // 100
```

### Running an FHE Prover

See [Deployment Guide](../guides/deployment.md) for setting up a prover node.

## References

- **TFHE-rs Documentation:** https://docs.zama.ai/tfhe-rs
- **FHE.org:** https://fhe.org (FHE educational resources)
- **Zama:** https://zama.ai (TFHE-rs creators)

## Next Steps

- **[System Overview](overview.md)** - Complete architecture
- **[Tech Stack](tech-stack.md)** - All technologies used
- **[Quickstart](../getting-started/quickstart.md)** - Try it yourself

---

**FHE in ZyberLink:** Compute on encrypted data, preserve privacy, maintain correctness through consensus.
