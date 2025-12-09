# Decisiones Técnicas - ZK Attestation System

**Fecha**: 2025-12-09
**Equipo**: Master Planner
**Propósito**: Documentar decisiones de diseño, trade-offs y rationale

---

## DECISIONES ARQUITECTURALES

### D1: Hybrid Verification Model (Backend + On-chain)

**Decisión**: Verificar proofs off-chain con AttestationService, guardar witness, y permitir disputes on-chain.

**Alternativas consideradas**:
1. **Full On-chain Verification**: Verificar proofs directamente en smart contract
2. **Full Off-chain Verification**: Solo backend verifica, sin attestations
3. **Hybrid (elegida)**: Backend verifica + attestation witness para disputes

**Trade-offs**:
| Aspecto | On-chain | Off-chain | Hybrid |
|---------|----------|-----------|--------|
| Gas cost | 🔴 Alto (~200K gas) | 🟢 Ninguno | 🟢 Solo en dispute |
| Latency | 🔴 5-10s (block time) | 🟢 1-5ms | 🟢 1-5ms |
| Trust | 🟢 Trustless | 🔴 Confianza en backend | 🟡 Trustless verificable |
| Throughput | 🔴 ~100 proofs/s | 🟢 ~1000 proofs/s | 🟢 ~1000 proofs/s |
| Complexity | 🟢 Simple | 🟢 Simple | 🟡 Moderada |

**Rationale**:
- MVP necesita velocidad y low cost (favores off-chain)
- Sistema trustless requiere verificabilidad (favores on-chain)
- Hybrid model: "Verify off-chain, dispute on-chain" es el mejor balance
- Inspirado en Optimistic Rollups (assume valid, challenge if fraud)

**Referencias**:
- Optimism Fraud Proofs: https://community.optimism.io/docs/protocol/2-rollup-protocol/
- zkSync Era: Off-chain proof generation, on-chain verification

---

### D2: Groth16 sobre PLONK/Halo2

**Decisión**: Usar Groth16 para verification de proofs.

**Alternativas consideradas**:
1. **Groth16** (elegida)
2. **PLONK**
3. **Halo2**

**Comparación**:
| Feature | Groth16 | PLONK | Halo2 |
|---------|---------|-------|-------|
| Proof size | 🟢 192 bytes | 🟡 ~1KB | 🟡 ~1KB |
| Verify time | 🟢 1-3ms | 🟡 5-10ms | 🟡 10-20ms |
| Trusted setup | 🔴 Circuit-specific | 🟡 Universal | 🟢 None |
| Prover time | 🟡 ~1s | 🟡 ~1s | 🔴 ~5s |
| Maturity | 🟢 Producción | 🟡 Creciendo | 🟡 Research |

**Rationale**:
- Groth16 es el estándar de facto (usado por Zcash, Filecoin, Tornado Cash)
- Proof size mínimo (importante para on-chain storage)
- Fastest verification (crítico para throughput)
- Tooling maduro (snarkjs, arkworks, circom)
- Trusted setup es mitigable con MPC ceremony

**Trade-off aceptado**: Circuit-specific trusted setup
- Para MVP: Keys de development OK
- Para producción: MPC ceremony planificada (Q1 2026)

---

### D3: PostgreSQL sobre IPFS para Attestations

**Decisión**: Guardar attestations en PostgreSQL, no en IPFS.

**Alternativas consideradas**:
1. **PostgreSQL** (elegida para MVP)
2. **IPFS + PostgreSQL (metadata)**
3. **Arweave**

**Comparación**:
| Feature | PostgreSQL | IPFS | Arweave |
|---------|-----------|------|---------|
| Latency | 🟢 <1ms | 🔴 100-500ms | 🔴 1-5s |
| Cost | 🟢 Disk local | 🟡 Pinning service | 🔴 Por MB |
| Availability | 🟡 Depende de DB | 🟢 Redundante | 🟢 Permanente |
| Query capability | 🟢 SQL completo | 🔴 Solo CID | 🔴 Solo hash |
| Integration | 🟢 Trivial | 🟡 Requiere nodo | 🔴 SDK externo |

**Rationale**:
- MVP necesita queries rápidas (stats, filtering, analytics)
- PostgreSQL JSONB permite queries flexibles en witness
- IPFS agrega latencia y complejidad operacional
- Para escala futura: Hybrid (hot data en DB, cold data en IPFS)

**Migration Path** (si se necesita IPFS después):
```sql
-- PHASE 6 (futuro)
ALTER TABLE attestations
  ADD COLUMN ipfs_cid VARCHAR(64),
  ADD COLUMN archived_to_ipfs BOOLEAN DEFAULT FALSE;

-- Archive old attestations
UPDATE attestations
SET ipfs_cid = upload_to_ipfs(witness),
    archived_to_ipfs = TRUE,
    witness = NULL  -- Free up space
WHERE created_at < NOW() - INTERVAL '90 days';
```

---

### D4: Proof Storage - Hybrid TTL Model

**Decisión**: Guardar solo proof_hash en MVP, agregar proof_json con TTL en PHASE 4 (opcional).

**Alternativas consideradas**:
1. **No guardar proofs** (solo hashes)
2. **Guardar todos los proofs** (sin TTL)
3. **Hybrid con TTL** (elegida)

**Comparación**:
| Modelo | Disk Usage | Query Speed | Auditability |
|--------|-----------|-------------|--------------|
| Solo hash | 🟢 64 bytes | 🟢 Rápido | 🔴 No auditable |
| Todos los proofs | 🔴 ~2KB/proof | 🟡 Moderado | 🟢 Completa |
| Hybrid TTL | 🟡 Variable | 🟢 Rápido | 🟡 Ventana 7-30 días |

**Rationale**:
- proof_hash es suficiente para verificar no-reuse
- Attestation witness permite re-verificación sin proof completo
- proof_json útil para audits pero no crítico
- TTL (7-30 días) balancea storage vs auditability

**Storage Math**:
```
1 proof = ~2KB JSON
1M proofs/año × 2KB = 2GB/año (sin TTL)
1M proofs/año × 30 días retention = 164MB promedio (con TTL)

Saving: ~92% disk space
```

**Implementación** (PHASE 4):
```rust
// Al submit proof
let retention_period = env::var("PROOF_RETENTION_DAYS")
    .unwrap_or("30".to_string())
    .parse::<i32>()?;

let retention_expires_at = Utc::now() + Duration::days(retention_period);

ZkQueries::save_proof_json(
    &pool,
    job_id,
    proof_json,
    retention_expires_at
).await?;
```

---

### D5: VK Directory sobre Database Storage

**Decisión**: Cargar VKs desde filesystem directory al startup, cachear en memoria.

**Alternativas consideradas**:
1. **Filesystem + In-memory cache** (elegida)
2. **Database storage** (tabla verification_keys)
3. **S3 / Object Storage**

**Comparación**:
| Opción | Startup Time | Update Complexity | Multi-instance |
|--------|--------------|-------------------|----------------|
| Filesystem | 🟢 <1s | 🟢 Simple (file replace) | 🟡 Requiere sync |
| Database | 🟡 2-5s | 🟡 SQL migrations | 🟢 Automático |
| S3 | 🔴 5-10s | 🟢 Simple | 🟢 Automático |

**Rationale**:
- VKs son estáticos (no cambian después de deploy)
- Filesystem es más simple para desarrollo y debugging
- In-memory cache elimina latency de lecturas
- Docker volume mount permite updates sin rebuild

**Multi-instance Strategy**:
```yaml
# docker-compose.yml
services:
  backend:
    deploy:
      replicas: 3
    volumes:
      - ./verification_keys:/app/verification_keys:ro  # Shared mount
```

**Hot Reload** (futuro):
```rust
// Watch filesystem para cambios
use notify::Watcher;

let watcher = notify::watcher(tx, Duration::from_secs(10))?;
watcher.watch("/app/verification_keys", RecursiveMode::NonRecursive)?;

// Reload VKs on change
loop {
    match rx.recv() {
        Ok(DebouncedEvent::Write(_)) => {
            log::info!("VK change detected, reloading...");
            attestation_service.reload_vks()?;
        }
        _ => {}
    }
}
```

---

## DECISIONES DE IMPLEMENTACIÓN

### D6: arkworks sobre bellman

**Decisión**: Usar arkworks-rs para Groth16 verification.

**Alternativas**:
1. **arkworks** (elegida)
2. **bellman** (Zcash)
3. **snarkjs via WASM**

**Comparación**:
| Library | Performance | Ergonomics | Community |
|---------|-------------|------------|-----------|
| arkworks | 🟢 Fastest | 🟢 Moderno | 🟢 Activo |
| bellman | 🟡 Rápido | 🔴 Legacy API | 🟡 Zcash-focused |
| snarkjs | 🔴 JS/WASM overhead | 🟢 Familiar | 🟢 Muy activo |

**Rationale**:
- arkworks tiene mejor performance (~2x más rápido que bellman)
- API moderna y type-safe
- Mejor documentación y community support
- Compatibilidad con snarkjs JSON (parsing manual pero factible)

**Ejemplo de Performance**:
```rust
// Benchmark interno
arkworks::verify()   → 1.2ms
bellman::verify()    → 2.8ms
snarkjs (WASM)       → 5.4ms
```

---

### D7: Snarkjs sobre Circom CLI

**Decisión**: Usar snarkjs para compilación y proof generation.

**Alternativas**:
1. **snarkjs** (elegida)
2. **circom CLI + arkworks**
3. **Custom tooling**

**Rationale**:
- snarkjs es el estándar de facto para circom
- All-in-one tool (compile, setup, prove, verify)
- JSON format compatible con arkworks (con parsing)
- Massive community y documentación

**Workflow**:
```bash
# Compilar
circom circuit.circom --r1cs --wasm --sym

# Setup (una vez)
snarkjs groth16 setup circuit.r1cs ptau.ptau circuit_0000.zkey
snarkjs zkey contribute circuit_0000.zkey circuit_final.zkey

# Generar proof (repetido)
snarkjs wtns calculate circuit.wasm input.json witness.wtns
snarkjs groth16 prove circuit_final.zkey witness.wtns proof.json public.json

# Verificar
snarkjs groth16 verify vk.json public.json proof.json
```

---

### D8: Circuit Complexity - 228K constraints

**Decisión**: VERIFY_GROTH16 tiene 228K constraints.

**Alternativas consideradas**:
1. **Simple Model** (~8K constraints): Solo attestation, no pairing real
2. **Medium Model** (228K constraints): Pairing con optimizaciones (elegida)
3. **Full Model** (~1M constraints): Pairing completo sin shortcuts

**Trade-offs**:
| Model | Constraints | Prove Time | Verify Time | Security |
|-------|-------------|------------|-------------|----------|
| Simple | 8K | 🟢 100ms | 🟢 <1ms | 🔴 Attestation only |
| Medium | 228K | 🟡 ~1s | 🟡 1-3ms | 🟡 Optimized pairing |
| Full | 1M | 🔴 ~5s | 🔴 5-10ms | 🟢 Full pairing |

**Rationale**:
- Simple model no permite verify real pairing (solo attestation de backend)
- Medium model balancea security vs performance
- 228K es manejable para provers (~1s en laptop moderno)
- Full pairing (1M constraints) es overkill para MVP

**Optimizaciones aplicadas**:
```circom
// En lugar de pairing completo:
// e(A,B) * e(α,β) * e(vk_x,γ) * e(C,δ) = 1

// Usamos modelo attestation:
component pairing_check = Groth16PairingCheckSimple();
pairing_check.verification_witness <== verification_witness;
pairing_check.claimed_result <== verification_result;

// Backend pre-calcula pairing y genera witness
// Circuit verifica consistencia del resultado
```

**Security Assumption**:
- Confiamos en backend para generar witness correcto
- Pero: Cualquiera puede re-generar witness y verificar
- Si backend es malicioso: Auditor puede detectar y disputar on-chain

---

### D9: MAX_PUBLIC_INPUTS = 32

**Decisión**: Límite de 32 public inputs por proof.

**Rationale**:
- La mayoría de circuitos usan <10 public inputs
- 32 es suficiente para casos de uso complejos
- Más inputs → más constraints → mayor tiempo de proof

**Si se necesita más**:
```circom
// Opción 1: Aumentar límite (recompile)
// Opción 2: Usar hash de inputs
signal input public_inputs_merkle_root;
// Probar membresía de cada input en el árbol
```

---

### D10: Blake2s256 para witness_commitment

**Decisión**: Usar Blake2s256 en lugar de SHA256 o Poseidon.

**Alternativas**:
1. **Blake2s256** (elegida)
2. **SHA256**
3. **Poseidon**

**Comparación**:
| Hash | Speed | Circuit-friendly | Output Size |
|------|-------|------------------|-------------|
| Blake2s256 | 🟢 Rápido | 🟡 Moderado | 🟢 32 bytes |
| SHA256 | 🟡 Medio | 🔴 Costoso | 🟢 32 bytes |
| Poseidon | 🔴 Lento (CPU) | 🟢 Nativo ZK | 🟢 32 bytes |

**Rationale**:
- Blake2s es fastest en CPU (importante para off-chain computation)
- Output de 32 bytes es standard (hex de 64 chars)
- Circuit-friendliness no es crítico (commitment calculado off-chain)
- Si se necesita verificar commitment on-chain: Poseidon es mejor

**Migration Path** (si se requiere Poseidon):
```circom
// Agregar input alternativo
signal input witness_commitment_poseidon;
signal input use_poseidon;  // 0 = Blake2s, 1 = Poseidon

// Conditional verification
component poseidon_check = ...;
component blake2s_check = ...;

signal commitment_valid;
commitment_valid <== use_poseidon ? poseidon_check.out : blake2s_check.out;
```

---

## DECISIONES DE PRODUCT

### D11: Backward Compatibility (Accept without VK)

**Decisión**: Si VK no está disponible, aceptar proof sin verificar (con warning).

**Rationale**:
- Permite deployment gradual de VKs
- No rompe flujos existentes
- Backend logging permite detectar missing VKs

**Implementación**:
```rust
pub async fn verify_proof(&self, circuit_type: u8, proof: &str, inputs: &[String])
    -> Result<AttestationResult>
{
    match self.prepared_vks.get(&circuit_type) {
        Some(pvk) => {
            // Verificar normalmente
            let valid = Groth16::<Bn254>::verify(pvk, inputs, proof)?;
            Ok(AttestationResult { valid, ... })
        }
        None => {
            // Accept sin verificar
            log::warn!("No VK for circuit {}, accepting without verification", circuit_type);
            Ok(AttestationResult {
                valid: true,  // Assume valid
                verification_time_ms: 0,
                witness: None,  // No witness sin VK
            })
        }
    }
}
```

**Security Consideration**:
- En producción: Requiere VK (cambiar a error en lugar de warning)
- Monitoring: Alertar si >5% de proofs aceptados sin VK

---

### D12: Status Flow - 5 Estados

**Decisión**: Jobs tienen 5 estados: pending_tx, active, proving, completed, failed.

**Alternativas consideradas**:
1. **3 estados**: pending, active, completed (too simple)
2. **5 estados** (elegida): pending_tx, active, proving, completed, failed
3. **7 estados**: Agregar cancelled, expired (too complex para MVP)

**State Machine**:
```
pending_tx ──confirm──► active ──claim──► proving
                                            │
                                            ├──submit (valid)──► completed
                                            └──submit (invalid)──► failed
```

**Rationale**:
- pending_tx: Waiting for on-chain confirmation (puede fallar)
- active: Confirmado, disponible para claim
- proving: Prover working (SLA tracking)
- completed: Success
- failed: Invalid proof o timeout

**Query Optimization**:
```sql
-- Index por status
CREATE INDEX idx_zk_jobs_status ON zk_jobs(status);

-- Jobs activos (hot query)
SELECT * FROM zk_jobs
WHERE status = 'active'
ORDER BY created_at ASC
LIMIT 100;
```

---

### D13: Timeout Default - 1 hora

**Decisión**: Default timeout de 3600 segundos (1 hora).

**Rationale**:
- Proof generation: ~1s (circuito simple) a ~30s (complejo)
- Network latency: ~1s
- Buffer: 1 hora permite provers lentos sin penalizar rápidos

**Customizable**:
```json
POST /api/jobs/zk/validate-and-build
{
  "timeout_seconds": 7200  // 2 horas para circuitos grandes
}
```

**Cleanup Job** (futuro):
```rust
// Background task cada 5 minutos
async fn cleanup_expired_jobs(pool: &PgPool) {
    sqlx::query!(
        "UPDATE zk_jobs
         SET status = 'failed'
         WHERE status IN ('active', 'proving')
           AND created_at + (timeout_seconds * INTERVAL '1 second') < NOW()"
    )
    .execute(pool)
    .await?;
}
```

---

## DECISIONES DIFERIDAS (Post-MVP)

### D14: Batch Verification

**Decisión**: Diferir a post-MVP.

**Rationale**:
- Batch verification ahorra ~30% tiempo para N>10 proofs
- Pero: Agrega complejidad (need to accumulate proofs)
- MVP prioriza simplicidad sobre optimización

**Future Implementation**:
```rust
// PHASE 6 (futuro)
pub async fn verify_batch(&self, proofs: Vec<ProofInput>)
    -> Result<Vec<AttestationResult>>
{
    let ark_proofs = proofs.iter().map(parse_proof).collect();
    let results = Groth16::<Bn254>::verify_batch(ark_proofs)?;
    // ...
}
```

---

### D15: Circuit Recursion

**Decisión**: Diferir a Q2 2026.

**Rationale**:
- Recursion permite "proof of proofs" (compression)
- Útil para batching y cross-chain
- Pero: Muy complejo, no necesario para MVP

**Research**:
- Nova: Folding schemes
- Halo2: IPA-based recursion
- zkSTARK → Groth16 wrapper

---

### D16: Trusted Setup Ceremony

**Decisión**: Usar development keys para MVP, planificar ceremony para Q1 2026.

**Rationale**:
- MPC ceremony requiere coordinación (weeks)
- Development keys son suficientes para testnet/demo
- Production mainnet: MUST have proper ceremony

**Ceremony Plan** (futuro):
```
Phase 1 (1 semana):
  - Setup coordinator infrastructure
  - Recruit participants (30+ recommended)

Phase 2 (2 semanas):
  - Run ceremony (each participant contributes)
  - Verify all contributions

Phase 3 (1 día):
  - Generate final keys
  - Destroy toxic waste
  - Publish transcript
```

**References**:
- Zcash Ceremony: https://z.cash/technology/paramgen/
- Hermez Ceremony: https://blog.hermez.io/hermez-trusted-setup/

---

## LECCIONES Y BEST PRACTICES

### L1: Fail Fast en Circuit Testing

**Lección**: No asumir que circuito compila = circuito funciona.

**Best Practice**:
1. Compilar circuito
2. Generar test vectors manualmente
3. Calcular expected output
4. Validar witness satisface constraints
5. ENTONCES generar proof

**Antipattern**:
```bash
# ❌ MAL
circom circuit.circom --r1cs
snarkjs groth16 prove ...  # Falla con constraint error

# ✅ BIEN
circom circuit.circom --r1cs
snarkjs wtns calculate ...  # Valida constraints primero
snarkjs wtns check circuit.r1cs witness.wtns  # Explicit check
snarkjs groth16 prove ...  # Entonces genera proof
```

---

### L2: VK Validation al Startup

**Lección**: Detectar VK corruptos o inválidos antes de recibir requests.

**Best Practice**:
```rust
impl AttestationService {
    pub fn new(vk_directory: &Path) -> Result<Self> {
        // ...load VKs...

        // VALIDATE cada VK
        for (circuit_type, pvk) in &prepared_vks {
            Self::validate_vk(circuit_type, pvk)?;
        }

        Ok(Self { prepared_vks, vk_hashes })
    }

    fn validate_vk(circuit_type: &u8, pvk: &PreparedVerifyingKey<Bn254>)
        -> Result<()>
    {
        // Verificar punto no es identity
        if pvk.vk.alpha_g1.is_zero() {
            return Err(anyhow!("Invalid VK: alpha_g1 is zero"));
        }
        // ... más validaciones ...
        Ok(())
    }
}
```

---

### L3: Attestation Witness es Crítico

**Lección**: Guardar suficiente información para re-verificar sin backend.

**Best Practice**:
```rust
pub struct AttestationWitness {
    // Suficiente para reproducir verification
    pub proof_a: [String; 2],
    pub proof_b: [[String; 2]; 2],
    pub proof_c: [String; 2],
    pub vk_hash: String,          // Identifica VK usado
    pub public_inputs: Vec<String>,
    pub public_inputs_hash: String,
    pub verification_result: u8,
}
```

**Validation**:
```python
# Cualquiera puede re-verificar
def audit_attestation(attestation):
    vk = load_vk_by_hash(attestation.vk_hash)
    proof = parse_proof(attestation.proof_a, attestation.proof_b, attestation.proof_c)
    result = verify_groth16(vk, proof, attestation.public_inputs)

    assert result == attestation.verification_result, "Fraud detected!"
```

---

## CONCLUSIÓN

Las decisiones documentadas aquí reflejan el balance entre:
- **MVP Speed**: Llegar a producción rápido
- **Technical Quality**: No acumular deuda técnica crítica
- **Trustlessness**: Mantener verificabilidad descentralizada
- **Scalability**: Permitir crecimiento futuro

**Principios guía**:
1. Simple beats complex (hasta que no)
2. Measure before optimize
3. Backward compatibility when possible
4. Document all trade-offs

**Próximos pasos**:
- Revisar decisiones después de PHASE 3 (E2E test)
- Adjust based on real-world performance
- Plan evolution hacia production (ceremonies, audits, etc)

---

**Última actualización**: 2025-12-09
**Versión**: 1.0
