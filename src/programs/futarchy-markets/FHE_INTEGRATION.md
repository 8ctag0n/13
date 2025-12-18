# FHE Integration - Encrypted Pool Management

## Overview

El programa `futarchy-markets` integra con `fhe-generator` para mantener pools encriptados usando **TFHE (Fully Homomorphic Encryption)**. Esto permite:

- ✅ Pools completamente privados (nadie puede ver cuánto hay en YES vs NO)
- ✅ Computación off-chain por provers descentralizados
- ✅ Consensus multi-prover (2/3 para seguridad)
- ✅ Slashing automático de provers deshonestos

## Diferencial Competitivo

**Polymarket, Augur, etc**: Pools transparentes 👁️
**Futarchy Markets**: Pools encriptados usando FHE + marketplace descentralizado 🔒

Esto es **único en el ecosistema** - nadie más tiene un marketplace de computación FHE descentralizado.

---

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    USER FLOW                                 │
└─────────────────────────────────────────────────────────────┘
                          │
                          ▼
         1. PlaceBet (on-chain)
            - User submits encrypted_bet_amount
            - Market creates FHE job via CPI
            - Job: add(current_pool, new_bet)
                          │
                          ▼
┌─────────────────────────────────────────────────────────────┐
│                  FHE-GENERATOR                               │
│    Creates FheJob + FheConsensusData                        │
│    - Required provers: 3                                     │
│    - Consensus threshold: 2/3                                │
│    - Operation: CIRCUIT_FHE_ADD (4)                         │
└─────────────────────────────────────────────────────────────┘
                          │
                          ▼
         2. Provers Claim Job (off-chain)
            - Prover 1, 2, 3 claim the job
            - Download encrypted witness from IPFS
                          │
                          ▼
┌─────────────────────────────────────────────────────────────┐
│                  PROVER NODES (off-chain)                    │
│                                                              │
│  let pool = tfhe::deserialize(&current_pool);               │
│  let bet = tfhe::deserialize(&new_bet);                     │
│  let new_pool = tfhe::ServerKey::add(&pool, &bet);          │
│  let result_hash = blake2s(&new_pool.serialize());          │
│                                                              │
│  submit_result(job_id, result_hash);                        │
└─────────────────────────────────────────────────────────────┘
                          │
                          ▼
         3. Consensus Reached (on-chain)
            - 2 out of 3 provers agree on result_hash
            - FheConsensusData stores consensus
                          │
                          ▼
         4. UpdatePool (anyone can call)
            - Reads consensus result from FHE job
            - Updates market.encrypted_pool_yes/no
            - Clears pending_pool_update_job
```

---

## State Modifications

### Market Account (src/state/market.rs)

```rust
pub struct Market {
    // ... existing fields ...

    /// Encrypted pool for YES (FHE ciphertext)
    pub encrypted_pool_yes: Vec<u8>,

    /// Encrypted pool for NO (FHE ciphertext)
    pub encrypted_pool_no: Vec<u8>,

    /// Pending FHE job ID (if pool update in progress)
    pub pending_pool_update_job: Option<u64>,
}
```

**Size Impact:**
- Before: 3,388 bytes
- After: 4,429 bytes (+1,041 bytes for encrypted pools)

---

## Instructions

### PlaceBet (Modified)

**New Flow:**

```rust
// 1. Validate bet (ZK proof)
verify_market_bet_proof(&proof, &public_inputs, circuit_type)?;

// 2. Create FHE job to add bet to pool
let fhe_job_id = generate_job_id();

cpi::create_fhe_pool_addition_job(
    fhe_program_info,
    market_authority_info,  // Market program signs
    fhe_job_info,
    fhe_consensus_info,
    fhe_escrow_info,
    system_program_info,
    fhe_job_id,
    market.encrypted_pool_yes.clone(),  // Current pool
    encrypted_bet_amount,               // New bet (from user)
    100_000,                            // 0.0001 SOL payment to provers
)?;

// 3. Mark job as pending
market.pending_pool_update_job = Some(fhe_job_id);

// 4. Transfer bet to escrow (same as before)
invoke(
    &system_instruction::transfer(user_info.key, escrow_info.key, amount),
    &[user_info, escrow_info, system_program],
)?;
```

**Additional Accounts Required:**
- `[writable]` FHE job account (PDA)
- `[writable]` FHE consensus account (PDA)
- `[writable]` FHE escrow account (PDA)
- `[]` FHE-generator program

### UpdatePool (New Instruction)

```rust
UpdatePool {
    market_id: u64,
    fhe_job_id: u64,
    encrypted_result: Vec<u8>,
    side: bool,  // true = YES, false = NO
}
```

**Process:**
1. Verify market has pending job matching `fhe_job_id`
2. Verify FHE job has reached consensus (read FheConsensusData)
3. Update `market.encrypted_pool_yes` or `encrypted_pool_no`
4. Clear `pending_pool_update_job`

**Who Can Call:** Anyone (permissionless after consensus)

**Why Permissionless:**
- Consensus already verified by fhe-generator
- No trust needed - result is cryptographically guaranteed

---

## CPI Helper

### create_fhe_pool_addition_job

Located in `src/cpi.rs`:

```rust
pub fn create_fhe_pool_addition_job<'a>(
    fhe_program_info: &AccountInfo<'a>,
    creator_info: &AccountInfo<'a>,
    job_info: &AccountInfo<'a>,
    consensus_info: &AccountInfo<'a>,
    escrow_info: &AccountInfo<'a>,
    system_program_info: &AccountInfo<'a>,
    job_id: u64,
    current_pool: Vec<u8>,
    new_bet: Vec<u8>,
    price_lamports: u64,
) -> ProgramResult
```

**What it does:**
1. Concatenates `current_pool || new_bet` as witness
2. Hashes witness for storage
3. Creates `FheGeneratorInstruction::CreateJob`:
   - `circuit_type: 4` (CIRCUIT_FHE_ADD)
   - `required_provers: 3`
   - `consensus_threshold: 2` (2/3)
   - `timeout_seconds: 300` (5 minutes)
4. Invokes fhe-generator via CPI

---

## Security Considerations

### 1. Multi-Prover Consensus

**Problem:** Single prover could lie about result

**Solution:**
- Require 3 provers
- Need 2/3 to agree on result_hash
- Provers who disagree get slashed

### 2. Witness Privacy

**Problem:** Witness contains encrypted data that needs to stay private

**Solution:**
- Witness stored off-chain (IPFS/Arweave)
- Only hash stored on-chain
- Provers download encrypted witness
- TFHE ensures data stays encrypted during computation

### 3. Timeout Protection

**Problem:** Provers might not complete job

**Solution:**
- 5 minute timeout
- After timeout, creator can cancel and refund
- Failed provers lose reputation

### 4. Result Verification

**Problem:** How to verify encrypted result is correct?

**Solution:**
- Multiple provers compute independently
- All submit `result_hash = blake2s(encrypted_result)`
- Consensus on matching hashes
- Provers with minority hash get slashed

---

## Gas Costs

### Traditional Transparent Pool

```rust
market.total_yes_bets += amount;  // ~5,000 compute units
```

**Cost:** ~0.000005 SOL per bet

### FHE Encrypted Pool

```rust
// On-chain (PlaceBet)
create_fhe_job(...);  // ~50,000 compute units

// Off-chain (Provers)
tfhe::ServerKey::add(&pool, &bet);  // FREE (provers pay)

// On-chain (UpdatePool)
update_pool(...);  // ~30,000 compute units
```

**Cost:**
- PlaceBet: ~0.00005 SOL
- Prover payment: ~0.0001 SOL (goes to provers)
- UpdatePool: ~0.00003 SOL (anyone can call, claim fee from market)

**Total:** ~0.00018 SOL per bet (~10x transparent)

**Trade-off:**
- 10x more expensive
- But **100% private pools**
- Unique competitive advantage

---

## Future Enhancements

### Phase 2: Encrypted Odds Display

**Problem:** Users can't see odds without revealing pools

**Solution:** FHE comparison

```rust
// Circuit FHE_COMPARISON
let yes_is_winning = tfhe::ServerKey::gt(&pool_yes, &pool_no);

// Decrypt boolean (reveals nothing about amounts)
if yes_is_winning {
    "YES is currently winning"
} else {
    "NO is currently winning"
}
```

### Phase 3: Private Payout Calculation

**Problem:** Payout reveals your bet amount

**Solution:** ZK proof of payout + FHE division

```rust
// Circuit: FHE_DIVIDE
your_share = fhe::divide(
    your_encrypted_bet,
    encrypted_total_pool
);

// Prove share is correct without revealing amounts
```

---

## Testing Plan

### Unit Tests

1. ✅ Market creation with empty encrypted pools
2. ✅ UpdatePool with valid consensus
3. ⚠️ UpdatePool rejects invalid job_id
4. ⚠️ UpdatePool rejects without consensus

### Integration Tests

1. ⚠️ Full flow: PlaceBet → Provers → Consensus → UpdatePool
2. ⚠️ Multiple bets in sequence
3. ⚠️ Consensus failure (1/3 provers disagree)
4. ⚠️ Timeout handling

### E2E Tests

1. ⚠️ Real TFHE computation (3 provers)
2. ⚠️ Encrypted result matches expected
3. ⚠️ Slashing of dishonest prover

---

## Deployment Checklist

- [ ] Deploy fhe-generator to devnet
- [ ] Deploy futarchy-markets to devnet
- [ ] Run 3 prover nodes
- [ ] Test full bet cycle with FHE
- [ ] Verify consensus works
- [ ] Verify slashing works
- [ ] Monitor gas costs
- [ ] Document for users

---

## References

- **TFHE Library:** https://github.com/zama-ai/tfhe-rs
- **FHE-Generator:** `src/programs/fhe-generator/`
- **Circuit 4 (FHE_ADD):** `src/programs/fhe-generator/src/state/job.rs:9`
- **Multi-Prover Consensus:** `src/programs/fhe-generator/src/state/consensus.rs`

---

**Status:** ✅ Infrastructure Ready (CPI helpers + state)
**Next Step:** Implement actual PlaceBet FHE integration
**Timeline:** 1-2 days to wire up + test
