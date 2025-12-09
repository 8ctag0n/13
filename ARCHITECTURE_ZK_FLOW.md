# Arquitectura del Sistema ZK Attestation

**Versión**: 1.0 | **Fecha**: 2025-12-09

---

## VISIÓN GENERAL

### Trustless Proof Verification Flow

```
┌─────────────────────────────────────────────────────────────────────────┐
│                     ZYBERLINK ZK ATTESTATION SYSTEM                     │
└─────────────────────────────────────────────────────────────────────────┘

Creator                 Backend               AttestationService      Blockchain
   │                       │                         │                    │
   ├─(1) POST /validate───►│                         │                    │
   │    {circuit: 10,      │                         │                    │
   │     witness_hash,     │                         │                    │
   │     public_inputs}    │                         │                    │
   │                       │                         │                    │
   │◄──(2) job_id + tx────┤                         │                    │
   │       parameters      │                         │                    │
   │                       │                         │                    │
   ├─(3) Sign & Submit────┼─────────────────────────┼───────────────────►│
   │      Transaction      │                         │                    │
   │                       │                         │                (on-chain
   │                       │                         │                 job created)
   ├─(4) POST /confirm────►│                         │                    │
   │    {tx_signature}     │                         │                    │
   │                       │                         │                    │
   │◄──(5) Job Active─────┤                         │                    │
   │                       │                         │                    │
   │                       │                         │                    │
Prover                    │                         │                    │
   │                       │                         │                    │
   ├─(6) Generate Proof───┤                         │                    │
   │    (off-chain)        │                         │                    │
   │    using zkey         │                         │                    │
   │                       │                         │                    │
   ├─(7) POST /submit─────►│                         │                    │
   │    {proof,            │──(8) verify_proof()────►│                    │
   │     public_inputs}    │                         │                    │
   │                       │                         ├─ Load VK           │
   │                       │                         ├─ Verify Groth16    │
   │                       │                         └─ Generate witness  │
   │                       │                         │                    │
   │                       │◄────(9) AttestResult────┤                    │
   │                       │    {valid: true,        │                    │
   │                       │     witness,            │                    │
   │                       │     vk_hash}            │                    │
   │                       │                         │                    │
   │                       ├─(10) Save Attestation───►│                    │
   │                       │       to DB             │                    │
   │                       │                         │                    │
   │◄──(11) Success───────┤                         │                    │
   │    {attestation_id,   │                         │                    │
   │     verification_ms}  │                         │                    │
   │                       │                         │                    │
   │                       │                         │                    │
Anyone                    │                         │                    │
   │                       │                         │                    │
   ├─(12) GET /attest/{id}►│                         │                    │
   │                       │                         │                    │
   │◄──(13) Attestation────┤                         │                    │
   │    {proof_points,     │                         │                    │
   │     vk_hash,          │                         │                    │
   │     verified: true}   │                         │                    │
   │                       │                         │                    │
   └───────────────────────┴─────────────────────────┴────────────────────┘
```

---

## COMPONENTES DEL SISTEMA

### 1. Blink Server (Backend API)

**Puerto**: 3000
**Responsabilidades**:
- Validar y crear ZK jobs
- Coordinar con blockchain
- Verificar proofs via AttestationService
- Persistir attestations en PostgreSQL

**Endpoints clave**:
```
POST   /api/jobs/zk/validate-and-build
POST   /api/jobs/zk/{id}/confirm
POST   /api/jobs/zk/{id}/submit-proof
GET    /api/attestations/{job_id}
GET    /api/jobs/zk/{id}/proof  (PHASE 4 - opcional)
```

**Estado de Jobs**:
```
pending_tx ──confirm──► active ──claim──► proving ──submit──► completed
                                                      └──verify_fail──► failed
```

### 2. AttestationService (Verification Engine)

**Tecnología**: Rust + arkworks-rs
**Inicialización**:
```rust
VK_DIRECTORY=/app/verification_keys
AttestationService::new(vk_dir)
  ├─ Load circuit_10_vkey.json
  ├─ Load circuit_11_vkey.json
  │  ...
  └─ Load circuit_49_vkey.json

PreparedVerifyingKey cached per circuit_type
```

**Verification Flow**:
```
verify_proof(circuit_type, proof_json, public_inputs)
  │
  ├─(1) Lookup PreparedVK from cache
  │     If not found → Warning + Accept (backward compatible)
  │
  ├─(2) Parse snarkjs JSON to arkworks types
  │     Proof { pi_a, pi_b, pi_c } → Proof<Bn254>
  │
  ├─(3) Convert public inputs (strings → Fr)
  │
  ├─(4) Call arkworks Groth16::verify()
  │     e(A,B) = e(α,β) * e(vk_x,γ) * e(C,δ)
  │
  ├─(5) Generate attestation witness
  │     {proof_points, vk_hash, inputs_hash, result}
  │
  └─(6) Return AttestationResult
        {valid: bool, witness: JSON, time_ms: u64}
```

**Performance**:
- Verification: ~1-5ms (con PreparedVK)
- VK loading: ~10-50ms (una vez al inicio)
- Memory: ~50KB por VK cached

### 3. PostgreSQL Database

**Tablas principales**:

**zk_jobs**:
```sql
CREATE TABLE zk_jobs (
    id SERIAL PRIMARY KEY,
    job_id BIGINT UNIQUE NOT NULL,
    creator_pubkey VARCHAR(44),
    circuit_type SMALLINT CHECK (circuit_type BETWEEN 10 AND 49),
    witness_commitment VARCHAR(64),  -- Blake2s256 hash
    public_inputs JSONB,
    proof_hash VARCHAR(64),          -- Set on submit
    status VARCHAR(20),               -- pending_tx, active, proving, completed, failed
    prover_pubkey VARCHAR(44),
    created_at TIMESTAMP,
    completed_at TIMESTAMP,
    -- PHASE 4 (opcional):
    -- proof_json JSONB,
    -- retention_expires_at TIMESTAMP
);
```

**attestations**:
```sql
CREATE TABLE attestations (
    id BIGSERIAL PRIMARY KEY,
    job_id BIGINT REFERENCES zk_jobs(job_id),
    circuit_type SMALLINT,
    witness JSONB,                   -- Full attestation witness
    vk_hash VARCHAR(64),             -- Keccak256 de VK
    verification_result BOOLEAN,     -- true/false
    verification_time_ms INTEGER,    -- Performance metric
    created_at TIMESTAMP,
    used_for_dispute BOOLEAN,        -- On-chain dispute flag
    dispute_tx_signature VARCHAR(88)
);
```

**Queries críticas**:
```sql
-- Get attestation by job
SELECT * FROM attestations WHERE job_id = $1;

-- Stats de verificación
SELECT
    circuit_type,
    COUNT(*) as total,
    SUM(CASE WHEN verification_result THEN 1 ELSE 0 END) as valid
FROM attestations
GROUP BY circuit_type;

-- Avg verification time
SELECT AVG(verification_time_ms)
FROM attestations
WHERE verification_result = true;
```

### 4. Circuit: VERIFY_GROTH16

**Archivo**: `circuits/verify-groth16/circuit.circom`
**Constraints**: 228,394
**Curve**: BN254 (bn128)

**Public Inputs** (3):
```circom
signal input vk_hash;              // Poseidon(VK)
signal input public_inputs_hash;   // Poseidon(inputs)
signal input verification_result;  // 0 o 1
```

**Private Inputs** (122):
```circom
signal input proof_a[2];           // A ∈ G1
signal input proof_b[2][2];        // B ∈ G2
signal input proof_c[2];           // C ∈ G1
signal input vk_alpha[2];          // VK components
signal input vk_beta[2][2];
signal input vk_gamma[2][2];
signal input vk_delta[2][2];
signal input vk_ic[33][2];         // MAX_IC = 33
signal input num_public_inputs;    // Cantidad de inputs
signal input public_inputs[32];    // MAX = 32
signal input verification_witness; // Backend attestation
```

**Flujo de verificación**:
```
(1) vk_hash === Poseidon(vk_alpha, vk_beta, vk_gamma, vk_delta, vk_ic)
(2) public_inputs_hash === Poseidon(public_inputs[0..num_public_inputs])
(3) vk_x = IC[0] + Σ(public_inputs[i] · IC[i+1])
(4) verification_result === Groth16PairingCheckSimple()
    ├─ e(-A, B) * e(α, β) * e(vk_x, γ) * e(C, δ) = 1
    └─ Usa attestation_witness para optimizar
```

**Keys**:
- `circuit_final.zkey` (98 MB) - Para provers (generar proofs)
- `verification_key.json` (3.3 KB) - Para verifiers (verificar proofs)

---

## FLUJOS DE DATOS

### Flujo 1: Job Creation (Creator)

```
┌─────────┐
│ Creator │
└────┬────┘
     │
     ├─(1) Genera witness localmente (off-chain)
     │     witness = {private_data, randomness}
     │     commitment = Blake2s256(witness)
     │
     ├─(2) Prepara public inputs
     │     public_inputs = extract_public(witness)
     │
     ├─(3) POST /api/jobs/zk/validate-and-build
     │     {
     │       creator_pubkey,
     │       circuit_type: 10,
     │       witness_commitment,
     │       public_inputs: ["100", "200"],
     │       timeout_seconds: 3600,
     │       x402_token_id (opcional)
     │     }
     │
     ├─(4) Backend valida y retorna unsigned tx
     │     Response: {
     │       job_id: 123,
     │       unsigned_tx: "...",
     │       status: "pending_tx"
     │     }
     │
     ├─(5) Creator firma tx con wallet
     │     signature = wallet.sign(unsigned_tx)
     │
     ├─(6) Creator submite a blockchain
     │     tx_signature = blockchain.send(signed_tx)
     │
     ├─(7) POST /api/jobs/zk/123/confirm
     │     {tx_signature}
     │
     └─(8) Backend confirma
           Response: {
             job_id: 123,
             status: "active"
           }
```

### Flujo 2: Proof Generation & Verification (Prover)

```
┌────────┐
│ Prover │
└───┬────┘
    │
    ├─(1) Query active jobs
    │     GET /api/jobs/zk?status=active&circuit_type=10
    │
    ├─(2) Claim job (opcional)
    │     POST /api/jobs/zk/123/claim
    │     {prover_pubkey}
    │
    ├─(3) Obtiene witness del Creator (off-chain)
    │     witness = creator.send_witness()
    │
    ├─(4) Genera proof localmente
    │     # Usando circuit_final.zkey
    │     snarkjs wtns calculate circuit.wasm witness.json witness.wtns
    │     snarkjs groth16 prove circuit_final.zkey witness.wtns proof.json public.json
    │
    ├─(5) Verifica localmente (opcional)
    │     snarkjs groth16 verify verification_key.json public.json proof.json
    │
    ├─(6) Submit proof a backend
    │     POST /api/jobs/zk/123/submit-proof
    │     {
    │       prover_pubkey,
    │       proof: {pi_a, pi_b, pi_c, protocol, curve},
    │       public_inputs: ["100", "200"]
    │     }
    │
    │     Backend:
    │     ├─ Valida job en estado active/proving
    │     ├─ Llama AttestationService.verify_proof()
    │     ├─ Si válido:
    │     │   ├─ Guarda attestation en DB
    │     │   ├─ Calcula proof_hash = Keccak256(proof)
    │     │   └─ Marca job como completed
    │     └─ Si inválido:
    │         ├─ Marca job como failed
    │         └─ Retorna HTTP 400
    │
    └─(7) Recibe confirmación
          Response: {
            job_id: 123,
            status: "completed",
            attestation_id: 456,
            verification_time_ms: 3
          }
```

### Flujo 3: Dispute Resolution (Auditor)

```
┌─────────┐
│ Auditor │
└────┬────┘
     │
     ├─(1) Sospecha de proof inválido
     │     (detectado on-chain o por monitoring)
     │
     ├─(2) Query attestation
     │     GET /api/attestations/123
     │
     ├─(3) Recibe attestation witness
     │     Response: {
     │       id: 456,
     │       job_id: 123,
     │       witness: {
     │         proof_a: [...],
     │         proof_b: [...],
     │         proof_c: [...],
     │         vk_hash: "...",
     │         public_inputs: [...],
     │         verification_result: true
     │       },
     │       verification_time_ms: 3
     │     }
     │
     ├─(4) Re-verifica off-chain
     │     local_result = verify(witness.proof, witness.inputs, VK)
     │
     ├─(5) Si discrepancia detectada:
     │     │
     │     ├─ Genera P2 (proof de attestation)
     │     │   usando VERIFY_GROTH16 circuit
     │     │
     │     ├─ Submit P2 a smart contract dispute
     │     │   dispute_tx = contract.challenge(job_id, P2)
     │     │
     │     └─ Backend marca attestation
     │         UPDATE attestations
     │         SET used_for_dispute = TRUE,
     │             dispute_tx_signature = 'xxx'
     │         WHERE job_id = 123
     │
     └─(6) Disputa resuelta on-chain
           (slashing del prover malicioso)
```

---

## ARQUITECTURA DE ARCHIVOS

```
/home/deploy/2025q4/13/
│
├── circuits/
│   ├── lib/
│   │   ├── bn254.circom           # BN254 curve ops
│   │   ├── poseidon_utils.circom  # Hashing
│   │   └── pairing.circom         # Pairing check
│   │
│   ├── verify-groth16/
│   │   ├── circuit.circom         # Main circuit (228K constraints)
│   │   ├── build/
│   │   │   └── circuit_js/
│   │   │       └── circuit.wasm   # 3.9 MB
│   │   ├── circuit_final.zkey     # 98 MB (proving key)
│   │   ├── verification_key.json  # 3.3 KB (verifying key)
│   │   ├── generate_proof.sh      # Helper script
│   │   └── input.json             # Example input
│   │
│   └── ptau/
│       └── pot18_final.ptau       # 289 MB (trusted setup)
│
├── verification_keys/             # 🆕 PHASE 2
│   ├── circuit_10_vkey.json       # Core circuits
│   ├── circuit_11_vkey.json
│   └── ...
│
├── src/
│   ├── blink-server/
│   │   ├── src/
│   │   │   ├── main.rs            # Init AttestationService
│   │   │   ├── zk_handlers.rs     # POST /submit-proof, GET /attestation
│   │   │   ├── api_handlers.rs    # General API
│   │   │   ├── db/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── zk_queries.rs  # ZK job queries
│   │   │   │   └── attestation_queries.rs
│   │   │   └── services/
│   │   │       ├── mod.rs
│   │   │       └── attestation_service.rs  # Groth16 verification
│   │   │
│   │   └── migrations/
│   │       ├── 20251209000019_create_zk_jobs_table.sql
│   │       ├── 20251209000020_create_attestations.sql
│   │       └── 20251210000021_add_proof_storage.sql  # 🆕 PHASE 4 (opcional)
│   │
│   └── x402-server/               # Anti-spam layer (puerto 8081)
│
├── IMPLEMENTATION_PLAN_ZK_E2E.md  # Plan detallado
├── IMPLEMENTATION_SUMMARY.md      # Resumen ejecutivo
└── ARCHITECTURE_ZK_FLOW.md        # Este documento
```

---

## CONFIGURACIÓN DEL SISTEMA

### Variables de Entorno (blink-server)

```bash
# Database
DATABASE_URL=postgresql://user:pass@localhost:5432/zyberlink

# Blockchain
SOLANA_RPC_URL=http://localhost:8899
PROGRAM_ID=CypherLinkProgram11111111111111111111111111

# Server
HOST=0.0.0.0
PORT=3000
RUST_LOG=info

# ZK Attestation (🆕)
VK_DIRECTORY=/app/verification_keys

# Keys
SERVER_KEYPAIR_PATH=/app/keypair.json
```

### Docker Compose (snippet)

```yaml
services:
  backend:
    build:
      context: .
      dockerfile: src/blink-server/Dockerfile
    environment:
      - DATABASE_URL=postgresql://...
      - VK_DIRECTORY=/app/verification_keys
    volumes:
      - ./verification_keys:/app/verification_keys:ro  # 🆕 Read-only mount
    ports:
      - "3000:3000"
```

---

## MÉTRICAS Y MONITOREO

### Performance Metrics

**Verificación de Proofs**:
```sql
-- Tiempo promedio por circuit_type
SELECT
    circuit_type,
    COUNT(*) as total_verifications,
    AVG(verification_time_ms) as avg_ms,
    MAX(verification_time_ms) as max_ms,
    MIN(verification_time_ms) as min_ms
FROM attestations
WHERE verification_result = true
GROUP BY circuit_type
ORDER BY circuit_type;
```

**Tasa de Éxito**:
```sql
-- Proofs válidos vs inválidos
SELECT
    verification_result,
    COUNT(*) as count,
    ROUND(100.0 * COUNT(*) / SUM(COUNT(*)) OVER (), 2) as percentage
FROM attestations
GROUP BY verification_result;
```

**Jobs por Estado**:
```sql
-- Distribution de jobs
SELECT
    status,
    COUNT(*) as count
FROM zk_jobs
GROUP BY status
ORDER BY
    CASE status
        WHEN 'completed' THEN 1
        WHEN 'proving' THEN 2
        WHEN 'active' THEN 3
        WHEN 'pending_tx' THEN 4
        WHEN 'failed' THEN 5
    END;
```

### Health Checks

**Backend Health**:
```bash
curl http://localhost:3000/health
# Response: {"status": "healthy", "timestamp": "..."}
```

**AttestationService Status**:
```bash
curl http://localhost:3000/api/internal/attestation-status
# Response: {
#   "circuits_loaded": 10,
#   "circuit_types": [10, 11, 12, ...],
#   "total_verifications": 1234,
#   "avg_time_ms": 3.5
# }
```

**Database Connection**:
```bash
psql $DATABASE_URL -c "SELECT COUNT(*) FROM attestations;"
```

---

## SEGURIDAD

### Threat Model

**Amenaza 1: Prover Malicioso**
- Submite proof inválido intentando engañar sistema
- **Mitigación**: AttestationService verifica con arkworks
- **Detección**: verification_result = false guardado en DB
- **Respuesta**: Job marcado como failed, prover puede ser penalizado

**Amenaza 2: Backend Comprometido**
- Backend acepta proofs sin verificar
- **Mitigación**: Attestation witness permite re-verificación off-chain
- **Detección**: Auditor puede regenerar verificación
- **Respuesta**: Dispute on-chain con P2

**Amenaza 3: VK Tampering**
- Attacker modifica verification_key.json
- **Mitigación**: vk_hash en attestation permite detectar cambios
- **Detección**: Hash no coincide con esperado
- **Respuesta**: Alert + rollback a VK correcto

**Amenaza 4: Replay Attack**
- Reusa proof de otro job
- **Mitigación**: witness_commitment único por job
- **Detección**: proof_hash duplicado en DB
- **Respuesta**: Rechazar proof

### Best Practices

1. **VK Storage**: Read-only mount en Docker
2. **Database**: Encrypted at rest + TLS connections
3. **API**: Rate limiting en submit-proof (1 por minuto por IP)
4. **Logging**: Audit log de todas las verificaciones
5. **Monitoring**: Alertas si tasa de fallos >5%

---

## SCALING CONSIDERATIONS

### Vertical Scaling (Single Instance)

**Bottlenecks actuales**:
- CPU: Groth16 verification (intensivo)
- Memory: PreparedVK cache (~50KB × 40 circuits = 2MB)
- Disk: Proof storage (si PHASE 4)

**Límites estimados**:
- ~1000 verificaciones/segundo (single core)
- ~10GB disk por millón de proofs (con JSON completo)

### Horizontal Scaling (Multiple Instances)

**Load Balancing**:
```
nginx → [backend-1, backend-2, backend-3]
         ↓         ↓         ↓
         └─────→ PostgreSQL (shared)
```

**Stateless Design**:
- AttestationService es stateless (VKs en memoria)
- Cada instancia carga VKs independientemente
- Sin necesidad de shared cache (Redis)

**Database Pooling**:
```rust
// en main.rs
let pool = PgPoolOptions::new()
    .max_connections(20)  // Por instancia
    .connect(&database_url)
    .await?;
```

### Caching Strategy

**VK Caching** (ya implementado):
```rust
pub struct AttestationService {
    prepared_vks: HashMap<u8, PreparedVerifyingKey<Bn254>>,  // In-memory
    vk_hashes: HashMap<u8, String>,
}
```

**Proof Caching** (si PHASE 4):
```sql
-- Query con caché de proof_json
SELECT proof_json
FROM zk_jobs
WHERE job_id = $1
  AND proof_json IS NOT NULL
  AND retention_expires_at > NOW();
```

---

## ROADMAP FUTURO

### Short Term (Post-MVP)
- [ ] PHASE 4: Proof Storage con TTL
- [ ] Rate limiting por prover_pubkey
- [ ] Prometheus metrics endpoint
- [ ] Circuit types 11-49 VKs

### Medium Term (Q1 2026)
- [ ] IPFS storage para proofs grandes
- [ ] Batch verification (múltiples proofs en paralelo)
- [ ] WebSocket para status updates real-time
- [ ] Dashboard de analytics

### Long Term (Q2 2026+)
- [ ] Trusted setup ceremony para production
- [ ] Circuit audit completo
- [ ] Recursive proof aggregation
- [ ] Cross-chain attestation bridge

---

**Fin de Arquitectura**
**Última actualización**: 2025-12-09
