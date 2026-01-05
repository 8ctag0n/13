# Attestation Service Integration with ZK Jobs Flow

## Overview

The `AttestationService` has been successfully integrated into the ZK jobs flow to verify Groth16 proofs before they are accepted by the network.

## Implemented Changes

### 1. AttestationService Initialization (main.rs)

**Location**: `zyb-services/blink-server/src/main.rs`

- Added `VK_DIRECTORY` environment variable (default: `./verification_keys`).
- Initialization of the `AttestationService` at server startup.
- Wrapped in `Arc<AttestationService>` for cross-thread sharing.
- Added as `app_data` in the Actix HttpServer.
- Complete logging of available circuits and warnings if no VKs are found.

**Behavior**:
- No VKs: Warning logged, but server starts (backward compatible).
- VKs found: Logs list of available circuit types.
- Initialization failure: Warning logged and service disabled.

### 2. New Endpoint: Submit Proof (zk_handlers.rs)

**Location**: `zyb-services/blink-server/src/zk_handlers.rs`

**Endpoint**: `POST /api/jobs/zk/{job_id}/submit-proof`

**Request Body**:
```json
{
  "prover_pubkey": "string",
  "proof": "string (JSON proof)",
  "public_inputs": ["string array"]
}
```

**Verification Flow**:
1. Validate that the job exists and is in `active` or `proving` status.
2. Claim the job if it is `active` (transitions to `proving`).
3. Verify proof using `AttestationService` (if available).
   - If no VK for the circuit: Log warning and accept without verification.
   - If VK exists: Verify using arkworks Groth16.
4. If proof is **invalid**: 
   - Mark job as `failed`.
   - Return HTTP 400 with details.
5. If proof is **valid**:
   - Save attestation to Database.
   - Compute proof hash (Keccak256).
   - Complete the job (transitions to `completed`).
   - Return attestation_id and verification time.

**Response**:
```json
{
  "job_id": 123,
  "status": "completed",
  "attestation_id": 456,
  "verification_time_ms": 234
}
```

### 3. Attestation Query Endpoint

**Endpoint**: `GET /api/attestations/{job_id}`

**Response**:
```json
{
  "id": 1,
  "job_id": 123,
  "circuit_type": 10,
  "witness": {...},
  "vk_hash": "...",
  "verification_result": true,
  "verification_time_ms": 234,
  "created_at": "...",
  "used_for_dispute": false,
  "dispute_tx_signature": null
}
```

---

## Full ZK Job Flow with Attestation

```
1. Creator: POST /api/jobs/zk/validate-and-build
   ↓ (Job created with status: pending_tx)
   
2. Creator: Sign & Submit TX on-chain
   ↓
   
3. Creator: POST /api/jobs/zk/{job_id}/confirm
   ↓ (Job confirmed with status: active)
   
4. Prover: Generate proof off-chain (SnarkJS/Noir)
   ↓
   
5. Prover: POST /api/jobs/zk/{job_id}/submit-proof
   ↓
   ├─ Backend verifies with AttestationService
   ├─ If valid: Save attestation + complete job
   └─ If invalid: Mark job as failed + return error
   
6. Optional: GET /api/attestations/{job_id}
   (Used for disputes or auditing)
```

## Configuration

### Environment Variables

```bash
# Optional: directory with verification keys
VK_DIRECTORY=./verification_keys

# VK files must follow this naming convention:
# circuit_10_vkey.json
# circuit_11_vkey.json
# ... up to circuit_49_vkey.json
```

### VK Format

Verification keys must be in SnarkJS JSON format:
```json
{
  "protocol": "groth16",
  "curve": "bn128",
  "nPublic": 2,
  "vk_alpha_1": ["...", "...", "1"],
  "vk_beta_2": [["...", "..."], ["...", "..."], ["1", "0"]],
  ...
}
```

## Backward Compatibility

The integration is **100% backward compatible**:

1. If no VKs are loaded, proofs are accepted without verification (with a warning).
2. If `AttestationService` fails to initialize, the server starts normally.
3. If a specific circuit lacks a VK, it accepts the proof for that circuit.

## Metrics Available

The `attestations` table allows for analytical queries:

```sql
-- Valid vs Invalid proof rate
SELECT 
  verification_result,
  COUNT(*) as count
FROM attestations
GROUP BY verification_result;

-- Average verification time per circuit
SELECT 
  circuit_type,
  AVG(verification_time_ms) as avg_ms
FROM attestations
WHERE verification_result = true
GROUP BY circuit_type;
```
