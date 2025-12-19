# E2E FHE Flow Documentation

## Overview

The ZyberLink FHE (Fully Homomorphic Encryption) system enables secure computation over encrypted data with multi-prover consensus. This document describes the complete end-to-end flow from job creation through result finalization.

### Core Components

- **Job Creator**: Generates encrypted FHE data and submits jobs on-chain
- **Blink Server**: Backend service for witness storage and result management
- **Prover Nodes**: Autonomous workers that execute FHE computations
- **Solana Program**: On-chain smart contract managing job lifecycle and consensus
- **FHE Engine**: TFHE-based computation engine running on provers

### Key Features

- **Multi-Prover Consensus**: Jobs require agreement from multiple provers (e.g., 2 out of 3)
- **Dynamic Pricing**: Tier-based pricing system (Tier 1-4) based on operation complexity
- **Result Storage**: Off-chain storage with on-chain commitment hashes
- **Blake2s256 Hashing**: Used for witness and result commitments
- **ROI-Based Selection**: Provers evaluate jobs based on profitability

## Architecture Sequence Diagram

```mermaid
sequenceDiagram
    participant JC as Job Creator
    participant BE as Blink Server
    participant BC as Blockchain (Solana)
    participant P1 as Prover 1
    participant P2 as Prover 2
    participant P3 as Prover 3

    Note over JC: Step 1: Generate FHE Data
    JC->>JC: Create FHE keys (client_key, server_key)
    JC->>JC: Encrypt data (FheUint8)
    JC->>JC: Package witness format

    Note over JC,BE: Step 2: Upload Witness
    JC->>BE: POST /witness (encrypted_data + server_key)
    BE->>BE: Compute Blake2s256 hash
    BE->>BE: Store witness in database
    BE-->>JC: Return commitment hash

    Note over JC,BC: Step 3: Get Price Recommendation
    JC->>BE: POST /api/price-recommendation
    BE->>BE: Calculate min/recommended/max price
    BE-->>JC: Return price ranges + slider params

    Note over JC,BC: Step 4: Create Job On-Chain
    JC->>JC: Get next job ID from config
    JC->>JC: Build FheConsensusConfig
    JC->>BC: Submit create_fhe_job transaction
    BC->>BC: Create Job account (status=Pending)
    BC->>BC: Create FheConsensusData account
    BC-->>JC: Transaction confirmed

    Note over P1,P2,P3: Step 5: Provers Poll for Jobs
    loop Every 5 seconds
        P1->>BC: Query pending jobs
        P2->>BC: Query pending jobs
        P3->>BC: Query pending jobs
    end

    Note over P1,P2,P3: Step 6: Evaluate Profitability
    P1->>P1: Calculate ROI (price vs cost)
    P2->>P2: Calculate ROI (price vs cost)
    P3->>P3: Calculate ROI (price vs cost)

    Note over P1,BC: Step 7: Claim Job
    P1->>BC: claim_fhe_job_instruction
    BC->>BC: Add P1 to claimed_provers[]
    P2->>BC: claim_fhe_job_instruction
    BC->>BC: Add P2 to claimed_provers[]
    P3->>BC: claim_fhe_job_instruction
    BC->>BC: Add P3 to claimed_provers[]
    BC->>BC: Update status to Claimed

    Note over P1,BE: Step 8: Download Witness
    P1->>BE: GET /witness/{hash}
    BE-->>P1: Return witness bytes
    P2->>BE: GET /witness/{hash}
    BE-->>P2: Return witness bytes
    P3->>BE: GET /witness/{hash}
    BE-->>P3: Return witness bytes

    Note over P1: Step 9: Parse Witness
    P1->>P1: Read length prefix (4 bytes LE)
    P1->>P1: Extract encrypted_data
    P1->>P1: Extract server_key (remaining bytes)
    P2->>P2: Parse witness format
    P3->>P3: Parse witness format

    Note over P1: Step 10: Execute FHE Computation
    P1->>P1: Deserialize server_key
    P1->>P1: Create FheEngine
    P1->>P1: set_key_for_thread() (TFHE context)
    P1->>P1: Execute operation (Add/Multiply/Sum/etc)
    P1->>P1: Serialize result
    P2->>P2: Execute FHE computation
    P3->>P3: Execute FHE computation

    Note over P1,BE: Step 11: Upload FHE Result
    P1->>BE: POST /fhe-result (encrypted result)
    BE->>BE: Compute Blake2s256 hash
    BE->>BE: Store result in database
    BE-->>P1: Return commitment
    P2->>BE: POST /fhe-result
    BE-->>P2: Return commitment
    P3->>BE: POST /fhe-result
    BE-->>P3: Return commitment

    Note over P1,BC: Step 12: Submit Result On-Chain
    P1->>BC: submit_fhe_result_instruction(hash)
    BC->>BC: Store result_hash in FheConsensusData
    BC->>BC: Set result_submitted[P1] = true
    P2->>BC: submit_fhe_result_instruction(hash)
    BC->>BC: Store result_hash, mark submitted
    P3->>BC: submit_fhe_result_instruction(hash)
    BC->>BC: Store result_hash, mark submitted

    Note over BC: Step 13: Consensus Check
    BC->>BC: Count matching result hashes
    alt Consensus Reached (2 out of 3)
        BC->>BC: Update status to Completed
        BC->>BC: Pay winning provers
        BC->>BC: Refund disagreeing provers
    else Timeout
        BC->>BC: Mark job as Failed
        BC->>BC: Refund creator
    end

    Note over JC,BC: Step 14: Retrieve Result
    JC->>BC: Query job status
    BC-->>JC: Status=Completed, consensus_hash
    JC->>BE: GET /fhe-result/{consensus_hash}
    BE-->>JC: Return encrypted result
    JC->>JC: Decrypt with client_key
```

## Witness Data Format

The witness format varies based on operation type:

### Single-Value Operations (Add, Multiply, Threshold)

```
[encrypted_data_len: 4 bytes LE] [encrypted_data: N bytes] [server_key: M bytes]
```

**Example in code (dev-job):**

```rust
let mut encrypted_input = Vec::new();
encrypted_input.extend_from_slice(&(encrypted_data.len() as u32).to_le_bytes());
encrypted_input.extend_from_slice(&encrypted_data);
encrypted_input.extend_from_slice(&server_key);
```

### Multi-Value Operations (Sum, Average, CountIf, Histogram)

```
[serialized_vec_len: 4 bytes LE] [Vec<Vec<u8>> bincode serialized] [server_key: M bytes]
```

**Example for Sum operation:**

```rust
// Create multiple encrypted values
let mut encrypted_values: Vec<Vec<u8>> = Vec::new();
for i in 0..count {
    let value = ((i % 10) + 1) as u8;
    let encrypted = FheUint8::encrypt(value, &client_key);
    let enc_bytes = bincode::serialize(&encrypted)?;
    encrypted_values.push(enc_bytes);
}

// Serialize the vector
let encrypted_bytes = bincode::serialize(&encrypted_values)?;
```

## Step-by-Step Implementation

### Step 1: Generate FHE Data (Job Creator)

```rust
use tfhe::{prelude::*, ConfigBuilder, FheUint8, generate_keys};

fn create_fhe_data(operation: &FheOperation) -> Result<(Vec<u8>, Vec<u8>)> {
    // Generate FHE keys
    let config = ConfigBuilder::default().build();
    let (client_key, server_key) = generate_keys(config);

    // Create encrypted data
    let value = 10u8;
    let encrypted = FheUint8::encrypt(value, &client_key);
    let encrypted_bytes = bincode::serialize(&encrypted)?;

    // Serialize server key
    let server_key_bytes = bincode::serialize(&server_key)?;

    Ok((encrypted_bytes, server_key_bytes))
}
```

### Step 2: Upload Witness to Backend

```rust
use blake2::{Blake2s256, Digest};

// Combine encrypted data and server key
let mut encrypted_input = Vec::new();
encrypted_input.extend_from_slice(&(encrypted_data.len() as u32).to_le_bytes());
encrypted_input.extend_from_slice(&encrypted_data);
encrypted_input.extend_from_slice(&server_key);

// Compute local commitment for verification
let mut hasher = Blake2s256::new();
hasher.update(&encrypted_input);
let local_commitment = hex::encode(hasher.finalize());

// Upload to backend
let upload_url = format!("{}/witness", backend_url);
let response = http_client
    .post(&upload_url)
    .body(encrypted_input.clone())
    .header("Content-Type", "application/octet-stream")
    .send()
    .await?;

let backend_commitment = response.json::<WitnessUploadResponse>().await?;

// Verify commitment matches
assert_eq!(backend_commitment.commitment, local_commitment);
```

### Step 3: Get Price Recommendation

```rust
#[derive(Deserialize)]
struct PriceRecommendationResponse {
    min_price_lamports: u64,
    recommended_price_lamports: u64,
    max_suggested_lamports: u64,
    slider_min: u64,
    slider_max: u64,
    slider_recommended: u64,
}

let request = json!({
    "operation": "sum",
    "expected_count": 5,
    "required_provers": 3
});

let response = http_client
    .post(&format!("{}/api/price-recommendation", backend_url))
    .json(&request)
    .send()
    .await?
    .json::<PriceRecommendationResponse>()
    .await?;

// Use recommended price (or let user adjust with slider)
let total_price_lamports = response.recommended_price_lamports;
```

### Step 4: Create Job On-Chain

```rust
use zyberlink_sdk::MarketplaceSDK;
use zyberlink_types::fhe::{FheConsensusConfig, FheOperation};

// Get next job ID from on-chain config
let job_id = sdk.get_next_job_id(&rpc_client)?;

// Create FHE consensus config
let fhe_config = FheConsensusConfig {
    required_provers: 3,
    consensus_threshold: 2,
    submission_timeout_secs: 600,
    operation: FheOperation::Sum { expected_count: 5 },
};

// Build create job instruction
let create_job_ix = sdk.create_fhe_job(
    user_keypair.pubkey(),
    job_id,
    &encrypted_input,
    fhe_config,
    total_price_lamports,
    600, // timeout_seconds
)?;

// Submit transaction
let recent_blockhash = rpc_client.get_latest_blockhash()?;
let mut tx = Transaction::new_with_payer(&[create_job_ix], Some(&user_keypair.pubkey()));
tx.sign(&[&user_keypair], recent_blockhash);

let signature = rpc_client.send_and_confirm_transaction(&tx)?;
```

### Step 5: Prover Downloads Witness

```rust
// Download witness from backend
let witness_url = format!("{}/witness/{}", backend_url, hex::encode(witness_hash));
let witness_bytes = http_client.get(&witness_url).send().await?.bytes().await?;
```

### Step 6: Prover Parses Witness Format

```rust
// Parse witness format: [len][encrypted_data][server_key]
if witness_bytes.len() < 4 {
    return Err(anyhow::anyhow!("Witness too short"));
}

let encrypted_data_len = u32::from_le_bytes([
    witness_bytes[0],
    witness_bytes[1],
    witness_bytes[2],
    witness_bytes[3],
]) as usize;

let header_size = 4;
let encrypted_data_end = header_size + encrypted_data_len;

let encrypted_data = &witness_bytes[header_size..encrypted_data_end];
let server_key_bytes = &witness_bytes[encrypted_data_end..];
```

### Step 7: Prover Executes FHE Computation

```rust
use tfhe::FheUint8;

// Deserialize server key
let server_key = bincode::deserialize::<tfhe::ServerKey>(server_key_bytes)?;

// Create FHE engine
let engine = FheEngine::new(server_key);

// CRITICAL: Set server key in thread-local context
engine.set_key_for_thread();

// Parse encrypted input based on operation
let result_bytes = match operation {
    FheOperation::Add(constant) => {
        let encrypted: FheUint8 = bincode::deserialize(encrypted_data)?;
        let result = encrypted + constant;
        bincode::serialize(&result)?
    }

    FheOperation::Sum { expected_count } => {
        // Deserialize vector of encrypted values
        let inputs: Vec<Vec<u8>> = bincode::deserialize(encrypted_data)?;

        // Convert to slice references
        let input_refs: Vec<&[u8]> = inputs.iter().map(|v| v.as_slice()).collect();

        // Compute sum using u16 for safety
        let mut sum = FheUint16::encrypt(0u16, &client_key);
        for input_bytes in input_refs {
            let encrypted: FheUint8 = bincode::deserialize(input_bytes)?;
            let as_u16 = encrypted.cast_into();
            sum = sum + as_u16;
        }

        bincode::serialize(&sum)?
    }

    FheOperation::Threshold { threshold, greater_or_equal } => {
        let encrypted: FheUint8 = bincode::deserialize(encrypted_data)?;
        let threshold_enc = FheUint8::encrypt(threshold, &client_key);
        let result = if greater_or_equal {
            encrypted.ge(threshold_enc)
        } else {
            encrypted.gt(threshold_enc)
        };
        bincode::serialize(&result)?
    }

    _ => return Err(anyhow::anyhow!("Unsupported operation"))
};

// Hash result for consensus
let result_hash = FheEngine::hash_result(&result_bytes);
```

### Step 8: Prover Uploads FHE Result

```rust
// Upload encrypted result to backend
let upload_url = format!("{}/fhe-result", backend_url);
let response = http_client
    .post(&upload_url)
    .body(result_bytes.clone())
    .header("Content-Type", "application/octet-stream")
    .send()
    .await?;

let upload_response: Value = response.json().await?;
let stored_commitment = upload_response["commitment"].as_str().unwrap();
```

### Step 9: Prover Submits Result On-Chain

```rust
// Build SubmitFheResult instruction
let submit_ix = client.submit_fhe_result_instruction(
    &keypair.pubkey(),
    &job_pda,
    job_id,
    result_hash
)?;

// Submit transaction
let signature = client.send_and_confirm_transaction(&[submit_ix], &[&keypair])?;
```

### Step 10: Consensus and Finalization

The on-chain program automatically handles consensus:

1. **Result Submission**: Each prover submits their result hash
2. **Consensus Check**: Program counts matching hashes
3. **Threshold Met**: If `consensus_threshold` provers agree (e.g., 2 out of 3):
   - Job status updated to `Completed`
   - Consensus hash stored
   - Winning provers receive payment
   - Disagreeing provers receive refund
4. **Timeout**: If consensus not reached within `submission_timeout_secs`:
   - Job marked as `Failed`
   - Creator receives refund

## API Endpoints Reference

### Witness Management

#### POST /witness

Upload encrypted witness data.

**Request:**
- Body: Raw bytes (application/octet-stream)
- Format: `[len][encrypted_data][server_key]`

**Response:**
```json
{
  "commitment": "hex-encoded-blake2s256-hash"
}
```

#### GET /witness/{commitment}

Download witness data by commitment hash.

**Response:**
- Body: Raw bytes (application/octet-stream)

### FHE Result Management

#### POST /fhe-result

Upload FHE computation result.

**Request:**
- Body: Raw bytes (encrypted result)

**Response:**
```json
{
  "commitment": "hex-encoded-blake2s256-hash"
}
```

#### GET /fhe-result/{commitment}

Download FHE result by commitment hash.

**Response:**
- Body: Raw bytes (encrypted result)

### Price Recommendation

#### POST /api/price-recommendation

Get recommended pricing for FHE operation.

**Request:**
```json
{
  "operation": "sum",
  "expected_count": 10,
  "required_provers": 3
}
```

**Response:**
```json
{
  "operation": "Sum",
  "complexity_tier": 2,
  "required_provers": 3,
  "min_price_lamports": 60000000,
  "min_price_sol": 0.06,
  "recommended_price_lamports": 108000000,
  "recommended_price_sol": 0.108,
  "max_suggested_lamports": 216000000,
  "max_suggested_sol": 0.216,
  "slider_min": 60000000,
  "slider_max": 216000000,
  "slider_recommended": 108000000,
  "slider_step": 1000000,
  "estimated_time_seconds": 600,
  "prover_overhead_multiplier": 1.5,
  "prover_min_roi_percent": 20.0
}
```

## FHE Operations and Tiers

### Tier 1: Basic Operations (20M lamports/prover, 300s timeout)
- **Add**: Add constant to encrypted value
- **Multiply**: Multiply encrypted value by constant

### Tier 2: Aggregation (20M lamports/prover, 600s timeout)
- **Sum**: Sum multiple encrypted values
- **Threshold**: Check if value meets threshold

### Tier 3: Advanced Checks (30M lamports/prover, 900s timeout)
- **RangeCheck**: Verify value within range

### Tier 4: Complex Operations (50M lamports/prover, 1800s timeout)
- **Average**: Calculate average of encrypted values
- **CountIf**: Count values matching predicate
- **Histogram**: Generate histogram bins

## Troubleshooting

### Problem: "Witness too short to contain length prefix"

**Cause:** Invalid witness format or corrupted download.

**Solution:**
1. Verify witness upload completed successfully
2. Check commitment hash matches on upload and download
3. Ensure network didn't corrupt transfer

### Problem: "Failed to deserialize server key from witness"

**Cause:** Server key extraction from witness failed.

**Solution:**
1. Verify witness format: `[len: 4 bytes LE][encrypted_data][server_key]`
2. Check `encrypted_data_len` is correct
3. Ensure server key was serialized with `bincode::serialize`

### Problem: "Thread-local key not set" or TFHE errors

**Cause:** TFHE uses thread-local storage for server key.

**Solution:**
```rust
// ALWAYS call this in the thread performing FHE computation
engine.set_key_for_thread();
```

### Problem: "Expected N inputs for Sum operation, got M"

**Cause:** Mismatch between `expected_count` in operation config and actual encrypted values.

**Solution:**
1. Verify job creator generates correct number of encrypted values
2. Check `expected_count` matches in `FheOperation::Sum { expected_count }`
3. Ensure witness format is correct for multi-value operations

### Problem: "Price too low for operation"

**Cause:** Job price doesn't meet minimum tier requirements.

**Solution:**
1. Use `/api/price-recommendation` to get valid price ranges
2. Ensure total price ≥ `min_price_per_prover × required_provers`
3. Use recommended price for ~95% prover acceptance rate

### Problem: "Consensus not reached - timeout"

**Cause:** Not enough provers submitted matching results within timeout.

**Solution:**
1. Increase `submission_timeout_secs` for complex operations
2. Offer higher price to attract more provers
3. Check prover logs for computation errors
4. Verify witness data is valid and consistent

### Problem: "Commitment mismatch! Local vs Backend"

**Cause:** Blake2s256 hash computed by creator doesn't match backend.

**Solution:**
1. Ensure both use Blake2s256 (not Blake2b)
2. Verify witness bytes are identical on upload
3. Check for network corruption or encoding issues

## Related Documentation

- [API Reference](./api-reference.md) - Complete API endpoint documentation
- [Price Slider Integration Guide](../guides/price-slider-integration.md) - Frontend integration
- [FHE Operations Specification](./fhe-operations.md) - Detailed operation specs
- [Prover Node Setup](../guides/prover-setup.md) - Running a prover node
- Dev job runner: use `zyb dev-job` for creating and submitting jobs
