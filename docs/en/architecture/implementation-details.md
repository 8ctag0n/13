# ZyberLink Architecture

## System Overview

ZyberLink employs a **3-Layer Microservices Architecture** to ensure security, scalability, and separation of concerns.

```mermaid
graph TD
    User((User/Client)) -->|HTTPS :3000| L1[Layer 1: Public API]
    
    subgraph "Private Network (Docker Internal)"
        L1 -->|HTTP :8081| L2[Layer 2: x402 Gateway]
        L2 -->|HTTP :8080| L3[Layer 3: Blink Server]
        
        L3 --> DB[(PostgreSQL)]
        L3 --> AT[Attestation Svc]
        L3 --> RPC[Solana RPC]
    end
    
    style L1 fill:#4ade80,stroke:#333,stroke-width:2px,color:black
    style L2 fill:#facc15,stroke:#333,stroke-width:2px,color:black
    style L3 fill:#60a5fa,stroke:#333,stroke-width:2px,color:black
```

### The 3-Layer Defense Model

```
INTERNET
   │
   ▼
┌─────────────────────────────────────────────────────────────┐
│  LAYER 1: PUBLIC API GATEWAY (Port 3000)                    │
│  Type: Axum / Tokio                                         │
│  Role: Entry Point, Rate Limiting, Request Validation       │
│  Access: 0.0.0.0:3000 (Public)                              │
└────────────┬────────────────────────────────────────────────┘
             │ (Internal Traffic Only)
             ▼
┌─────────────────────────────────────────────────────────────┐
│  LAYER 2: x402 ANTI-SPAM GATEWAY (Port 8081)                │
│  Type: Rust Middleware                                      │
│  Role: Payment Validation, Token Gating, DDoS Protection    │
│  Access: Internal :8081                                     │
└────────────┬────────────────────────────────────────────────┘
             │ (Authenticated Traffic)
             ▼
┌─────────────────────────────────────────────────────────────┐
│  LAYER 3: BLINK SERVER (Port 8080)                          │
│  Type: Core Logic / Actix-web                               │
│  Role: Business Logic, DB Access, Blockchain Interactions   │
│  Access: Internal :8080                                     │
└─────────────────────────────────────────────────────────────┘
```

---

## Components Detail

### 1. Public API (Gateway)
*   **Repo:** `zyb-services/public-api`
*   **Responsibility:** Handles all incoming HTTP requests, sanitizes inputs, and forwards valid requests to the x402 layer.
*   **Security:** Implements rate limiting (`RATE_LIMIT_PER_MIN=100`) and basic CORS policies.

### 2. x402 Server (Payment & Protection)
*   **Repo:** `zyb-services/x402-server`
*   **Responsibility:** Enforces the "Pay-to-Compute" model. It verifies that requests have valid payment tokens or signatures before reaching the core logic.
*   **Pricing:**
    *   FHE Operations: 0.01 SOL
    *   ZK Core: 0.05 SOL
    *   Voting: 0.05 SOL

### 3. Blink Server (Core)
*   **Repo:** `zyb-services/blink-server`
*   **Responsibility:** The brain of the operation.
    *   **ZK Jobs**: Handles proof submission and verification via `AttestationService`.
    *   **FHE Jobs**: Manages encrypted witness storage and consensus.
    *   **Database**: Exclusive write access to PostgreSQL.
    *   **Chain Sync**: Runs background tasks to sync on-chain state.

### Background Tasks (Running in Blink Server)

```
┌────────────────────┐  ┌───────────────────┐  ┌──────────────────┐
│  cleanup.rs        │  │ chain_sync.rs     │  │ prover_sync.rs   │
│  (3600s interval)  │  │ (~10s interval)   │  │ (~30s interval)  │
├────────────────────┤  ├───────────────────┤  ├──────────────────┤
│ - Delete expired   │  │ - Fetch on-chain  │  │ - Track prover   │
│   jobs             │  │   job accounts    │  │   reputation     │
└────────────────────┘  └───────────────────┘  └──────────────────┘
```

---

## ZK Proof Flow (Updated)

```
Creator                 Public API (3000)        x402 (8081)           Blink (8080)
   │                         │                       │                     │
   │ 1. POST /validate       │                       │                     │
   ├────────────────────────>│                       │                     │
   │                         │ 2. Forward            │                     │
   │                         ├──────────────────────>│                     │
   │                         │                       │ 3. Check Payment    │
   │                         │                       │ & Forward           │
   │                         │                       ├────────────────────>│
   │                         │                       │                     │ 4. DB & Logic
   │<──────────────────────────────────────────────────────────────────────┤
   │       200 OK (Job Created)                                            │
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
