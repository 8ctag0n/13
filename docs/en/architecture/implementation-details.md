# ZyberLink Architecture

## System Overview

```
┌──────────────┐       ┌──────────────┐       ┌──────────────┐
│   Creator    │       │   Prover     │       │  Any User    │
│  (HTTP CLI)  │       │   (Node)     │       │  (Web/API)   │
└──────┬───────┘       └──────┬───────┘       └──────┬───────┘
       │                      │                      │
       └──────────────────────┴──────────────────────┘
                              │
                              v
╔══════════════════════════════════════════════════════════════════════════╗
║                   BLINK SERVER (Port 3000 - Actix-web)                   ║
╠══════════════════════════════════════════════════════════════════════════╣
║                                                                          ║
║  ┌─ ZK Jobs API ─────────────┐    ┌─ FHE Jobs API ─────────────┐       ║
║  │ POST /validate-and-build  │    │ POST /validate-and-build   │       ║
║  │ POST /confirm             │    │ POST /confirm              │       ║
║  │ POST /submit-proof        │    │ GET  /compute-data/{id}    │       ║
║  │ GET  /attestations/{job}  │    │ GET  /status/{id}          │       ║
║  │ GET  /{job_id}/proof      │    └────────────────────────────┘       ║
║  └───────────────────────────┘                                          ║
║                                                                          ║
║  ┌─ AppState (shared) ──────────────────────────────────────────┐      ║
║  │ db_pool │ attestation_service │ program_id │ x402_url        │      ║
║  └──────────────────────────────────────────────────────────────┘      ║
╚══════════════════════════════════════════════════════════════════════════╝
        │                   │                   │                  │
        v                   v                   v                  v
┌─────────────┐   ┌──────────────────┐  ┌──────────────┐  ┌────────────┐
│ PostgreSQL  │   │ Attestation      │  │ x402-server  │  │ Solana RPC │
│   (5432)    │   │ Service          │  │  (8081)      │  │  (8899)    │
├─────────────┤   ├──────────────────┤  ├──────────────┤  ├────────────┤
│ zk_jobs     │   │ Groth16 Verify   │  │ Payment      │  │ On-chain   │
│ fhe_jobs    │   │ (arkworks)       │  │ validation   │  │ job state  │
│ attestations│   │                  │  │              │  │            │
│ witnesses   │   │ VK Cache:        │  │ Pricing:     │  │ Finality   │
│ provers     │   │ - circuit 10-52  │  │ 0.01-0.1 SOL │  │ Consensus  │
│ nonces      │   │ - 1-5ms verify   │  │              │  │            │
└─────────────┘   └──────────────────┘  └──────────────┘  └────────────┘
```

---

## Components

### Blink Server (Primary Backend)

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/jobs/zk/validate-and-build` | POST | Create ZK job |
| `/api/jobs/zk/confirm` | POST | Confirm job after TX |
| `/api/jobs/zk/{id}/submit-proof` | POST | Submit proof for verification |
| `/api/jobs/zk/{id}/attestations` | GET | Get attestation for job |
| `/api/jobs/zk/{id}/proof` | GET | Download proof JSON |
| `/api/jobs/fhe/validate-and-build` | POST | Create FHE job |
| `/api/jobs/fhe/{id}/compute-data` | GET | Get encrypted data for computation |
| `/health` | GET | Health check |

### Background Tasks

```
┌────────────────────┐  ┌───────────────────┐  ┌──────────────────┐  ┌─────────────────┐
│  cleanup.rs        │  │ chain_sync.rs     │  │ prover_sync.rs   │  │ job_finalizer   │
│  (3600s interval)  │  │ (~10s interval)   │  │ (~30s interval)  │  │ (~10s interval) │
├────────────────────┤  ├───────────────────┤  ├──────────────────┤  ├─────────────────┤
│ - Delete expired   │  │ - Fetch on-chain  │  │ - Track prover   │  │ - Auto-finalize │
│   jobs (24h+)      │  │   job accounts    │  │   reputation     │  │   FHE jobs on   │
│ - Delete nonces    │  │ - Sync to local   │  │ - Update metrics │  │   consensus     │
│   (1 day+)         │  │   DB              │  │ - Cache ROI      │  │                 │
│ - Clear proof_json │  │                   │  │                  │  │                 │
│   (30 days)        │  │                   │  │                  │  │                 │
│ - Remove witnesses │  │                   │  │                  │  │                 │
└────────────────────┘  └───────────────────┘  └──────────────────┘  └─────────────────┘
```

### x402 Anti-Spam Service (Port 8081)

Payment validation layer with circuit-based pricing:

| Circuit Type | Category | Price (SOL) |
|--------------|----------|-------------|
| 0-9 | FHE Operations | 0.01 |
| 10-19 | ZK Core | 0.05 |
| 20-29 | Voting | 0.05 |
| 30-39 | Market | 0.075 |
| 40-49 | Portfolio | 0.1 |

- Quote expiry: 5 minutes
- Token expiry: 24 hours

---

## ZK Proof Flow

```
Creator                 Blink Server           AttestationSvc         Blockchain
   │                         │                       │                     │
   │ 1. POST /validate       │                       │                     │
   ├────────────────────────>│                       │                     │
   │                         │ validate, gen job_id  │                     │
   │<────────────────────────┤                       │                     │
   │                         │                       │                     │
   │ 2. Sign & submit TX     │                       │                     │
   ├─────────────────────────────────────────────────────────────────────>│
   │                         │                       │   create job acct   │
   │<─────────────────────────────────────────────────────────────────────│
   │                         │                       │                     │
   │ 3. POST /confirm        │                       │                     │
   ├────────────────────────>│                       │                     │
   │                         │ verify on-chain       │                     │
   │<────────────────────────┤ Job: active           │                     │
   │                         │                       │                     │

Prover                       │                       │                     │
   │ 4. Generate proof       │                       │                     │
   │    (off-chain snarkjs)  │                       │                     │
   │                         │                       │                     │
   │ 5. POST /submit-proof   │                       │                     │
   ├────────────────────────>│                       │                     │
   │                         │ 6. verify_proof()     │                     │
   │                         ├──────────────────────>│                     │
   │                         │                       │ Load VK             │
   │                         │                       │ Parse snarkjs JSON  │
   │                         │                       │ ark-groth16 verify  │
   │                         │<──────────────────────┤ Ok(true) ~2ms       │
   │                         │                       │                     │
   │                         │ 7. Store attestation  │                     │
   │<────────────────────────┤ {attestation_id}      │                     │
```

---

## FHE Computation Flow

```
Creator                 Blink Server              Prover(s)           Blockchain
   │                         │                        │                    │
   │ 1. POST /validate       │                        │                    │
   │    {encrypted_data,     │                        │                    │
   │     server_key,         │                        │                    │
   │     operation}          │                        │                    │
   ├────────────────────────>│                        │                    │
   │                         │ Store encrypted data   │                    │
   │<────────────────────────┤ job_id                 │                    │
   │                         │                        │                    │
   │ 2. Sign & submit TX     │                        │                    │
   ├────────────────────────────────────────────────────────────────────>│
   │                         │                        │                    │
   │ 3. POST /confirm        │                        │                    │
   ├────────────────────────>│                        │                    │
   │                         │                        │                    │
   │                         │       4. Fetch job     │                    │
   │                         │<───────────────────────┤                    │
   │                         │       encrypted_data   │                    │
   │                         │                        │                    │
   │                         │       5. TFHE compute  │                    │
   │                         │       (off-chain)      │                    │
   │                         │                        │                    │
   │                         │       6. Submit result │                    │
   │                         │<───────────────────────┤                    │
   │                         │                        │                    │
   │                         │       7. Consensus     │                    │
   │                         │       (3 provers agree)│                    │
   │                         │                        │                    │
   │                         │       8. Finalize      │                    │
   │                         ├────────────────────────────────────────────>│
   │<────────────────────────┤       Job: completed   │                    │
```

---

## Database Schema

```
  zk_jobs                    attestations              x402_tokens
  ┌────────────────┐         ┌────────────────┐       ┌────────────────┐
  │ job_id (PK)    │         │ id (PK)        │       │ id (PK)        │
  │ circuit_type   │────────>│ job_id (FK)    │       │ quote_id       │
  │ witness_commit │         │ proof_json     │       │ tx_signature   │
  │ public_inputs  │         │ vk_hash        │       │ expires_at     │
  │ status         │         │ verified       │       │ used_at        │
  │ x402_token_id  │         │ witness        │       └────────────────┘
  └────────────────┘         │ retention_     │
                             │ expires_at     │       witnesses
  blockchain_jobs            └────────────────┘       ┌────────────────┐
  ┌────────────────┐                                  │ commitment(PK) │
  │ job_id (PK)    │         provers                  │ witness_data   │
  │ creator        │         ┌────────────────┐       │ created_at     │
  │ status         │         │ pubkey (PK)    │       └────────────────┘
  │ synced_at      │         │ reputation     │
  └────────────────┘         │ jobs_completed │
                             └────────────────┘
```

### Job Status Flow

**ZK Jobs:**
```
pending_tx → active → proving → completed/failed
```

**FHE Jobs:**
```
pending_tx → active → pending_proof → consensus_reached → finalized → completed
```

---

## Key Files

| File | Purpose |
|------|---------|
| `src/blink-server/src/main.rs` | Server entry point, spawns background tasks |
| `src/blink-server/src/zk_handlers.rs` | ZK job endpoints |
| `src/blink-server/src/api_handlers.rs` | FHE job endpoints |
| `src/blink-server/src/services/attestation_service.rs` | Groth16 verification |
| `src/blink-server/src/db/attestation_queries.rs` | Attestation DB operations |
| `src/blink-server/src/cleanup.rs` | Background cleanup service |
| `src/blink-server/src/chain_sync.rs` | Blockchain sync service |
| `src/x402-server/` | Anti-spam payment service |
| `verification_keys/` | VK files for circuits 10-52 |

---

## Performance

| Operation | Time |
|-----------|------|
| Groth16 verification (ark-groth16) | ~2ms |
| VK loading (cached) | ~1ms |
| DB query (attestation) | ~1-5ms |
| Proof JSON parsing | ~1ms |

---

## Environment Variables

```bash
PROGRAM_ID="GVCw9MYL6YywDPwsQkqC3xsvw5ETRH7KGkgxCDpeYfeu"
DATABASE_URL="postgresql://user:pass@localhost:5432/zyberlink"
VK_DIRECTORY="/path/to/verification_keys"
RUST_LOG=info
```

---

## Development Commands

```bash
# Run unit tests
cargo test --release -p zyberlink-blink-server --bin blink-server

# Run attestation service tests
cargo test --release services::attestation_service::tests

# Start server
./target/release/blink-server

# Create ZK job (example)
curl -X POST http://localhost:3000/api/jobs/zk/validate-and-build \
  -H "Content-Type: application/json" \
  -d '{"circuit_type":10,"witness_commitment":"<64-hex>","public_inputs":["15"],"creator":"wallet"}'

# Download proof
curl http://localhost:3000/api/jobs/zk/{job_id}/proof
```
