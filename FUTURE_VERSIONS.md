# Future Versions & Improvements

Este documento contiene ideas, mejoras y features que se consideraron pero se pospusieron para versiones futuras del proyecto.

---

## 🔐 Witness Backend Security (v0.2.0 o posterior)

### Problema
Actualmente el witness backend (`witness-storage/`) acepta cualquier dato sin verificación:
- No hay autenticación
- No verifica que el witness corresponda a un job real
- Vulnerable a spam/DoS
- Cualquiera puede subir basura

### Solución Propuesta: Commit-Reveal Pattern

**Descripción:**
Implementar verificación on-chain de que el witness corresponde a un job REAL antes de aceptarlo.

**Flow:**
```
1. Cliente genera witness encriptado
2. Cliente calcula commitment = Blake2b(encrypted_witness)
3. CreateJob(commitment, ...) on-chain  ← Job created FIRST
4. Cliente POST /witness/:commitment con encrypted_witness
5. Backend calcula hash del witness recibido
6. Backend verifica on-chain que existe job con ese commitment
7. Si matchea → acepta y guarda
8. Si NO matchea → rechaza (400 Bad Request)
```

**Cambios necesarios:**

1. **Backend necesita conexión RPC a Solana:**
```rust
struct AppState {
    storage: Arc<RwLock<WitnessStorage>>,
    solana_client: Arc<RpcClient>,  // NUEVO
    program_id: Pubkey,
}

impl AppState {
    async fn verify_job_exists(&self, commitment: &[u8; 32]) -> Result<bool> {
        // Buscar jobs on-chain con este commitment
        let accounts = self.solana_client.get_program_accounts(&self.program_id)?;

        for (pubkey, account) in accounts {
            let job: JobAccount = borsh::from_slice(&account.data)?;
            if job.witness_commitment == *commitment {
                return Ok(true);  // Job existe!
            }
        }

        Ok(false)  // No hay job con este commitment
    }
}
```

2. **Endpoint actualizado:**
```rust
POST /witness/:commitment  // commitment en URL, no en response

async fn upload_witness(
    State(state): State<AppState>,
    Path(commitment_hex): Path<String>,
    body: axum::body::Bytes,
) -> impl IntoResponse {
    let encrypted_witness = body.to_vec();

    // Calcular commitment del witness recibido
    let computed = compute_commitment(&encrypted_witness);

    // Parsear commitment esperado
    let expected = hex::decode(&commitment_hex)?;

    // Verificar match
    if computed != expected {
        return (StatusCode::BAD_REQUEST, "Commitment mismatch");
    }

    // Verificar que existe job on-chain
    if !state.verify_job_exists(&computed).await? {
        return (StatusCode::FORBIDDEN, "No job found");
    }

    // Guardar
    state.storage.write().await.store(computed, encrypted_witness);

    Json({"status": "stored"})
}
```

3. **Cliente actualizado:**
```rust
// Cliente primero crea job, LUEGO sube witness
let commitment = blake2b_hash(&encrypted_witness);
client.create_job(commitment, ...)?;  // On-chain first
backend.upload_witness(&commitment, &encrypted_witness)?;  // Then upload
```

**Beneficios:**
- ✅ Solo acepta witness de jobs REALES on-chain
- ✅ Commitment matchea cryptográficamente
- ✅ No se puede spamear sin pagar por CreateJob
- ✅ Descentralizado (verifica contra blockchain)
- ✅ Estándar en blockchain (commit-reveal pattern)

**Trade-offs:**
- Backend necesita RPC connection (costo/latencia)
- Más complejo de implementar
- Cliente debe crear job ANTES de subir

**Alternativas consideradas:**
1. Signed uploads (autenticación pero no previene spam)
2. API keys (centralizado)
3. Payment-gated (usuario paga dos veces)
4. Rate limiting (solo mitiga, no resuelve)

**Prioridad:** Media-Alta (necesario para mainnet)

---

## 🔑 Encryption Key Management (v0.2.0)

### Problema
Actualmente el prover genera un keypair X25519 nuevo en cada inicio y se pierde al reiniciar.

### Solución Propuesta: Key Persistence

**Opciones:**

1. **Guardar en disco:**
```rust
// ~/.cypherlink/encryption_key
impl WitnessEncryption {
    pub fn save_to_file(&self, path: &Path) -> Result<()> {
        let key_bytes = self.private_key.to_bytes();
        // Encrypt with password antes de guardar
        fs::write(path, encrypted_key)?;
        Ok(())
    }

    pub fn load_from_file(path: &Path) -> Result<Self> {
        let encrypted = fs::read(path)?;
        // Decrypt with password
        let key = decrypt_key(&encrypted)?;
        Ok(Self::from_bytes(&key))
    }
}
```

2. **Derivar del keypair de Solana:**
```rust
// Derivar X25519 key del Ed25519 keypair
let x25519_key = derive_x25519_from_ed25519(&solana_keypair);
```
   - Pro: Una sola key de la que derivar todo
   - Con: Ed25519 y X25519 son curvas diferentes, derivación no es trivial

3. **Hardware Security Module (HSM) / SGX:**
   - Para producción enterprise
   - Keys nunca salen del enclave

**Prioridad:** Media

---

## 🌐 Descentralizar Witness Storage (v0.3.0)

### Problema
Backend HTTP es centralizado, single point of failure.

### Soluciones:

1. **Light Protocol (ZK Compression)**
   - Diseño original del proyecto
   - Witness comprimido off-chain, commitment on-chain
   - Requiere integrar Light SDK
   - Prioridad: Alta (roadmap original)

2. **IPFS**
   - Cliente sube a IPFS → obtiene CID
   - CID va en `witness_commitment`
   - Prover descarga de IPFS
   - Pro: Descentralizado
   - Con: Latencia variable, pinning necesario

3. **Arweave**
   - Storage permanente
   - Cliente paga una vez, permanece forever
   - Pro: Inmutable, censorship-resistant
   - Con: Costo upfront

4. **Filecoin**
   - Storage descentralizado con proofs
   - Pro: Económico para datos grandes
   - Con: Latencia de retrieval

**Prioridad:** Alta (v0.3.0 target)

---

## 🔒 Post-Quantum Cryptography (v1.0.0)

### Problema
X25519 es vulnerable a computadoras cuánticas (futuro).

### Solución: Hybrid Encryption

**Approach:**
```rust
// Combinar X25519 (ahora) + ML-KEM (post-quantum)
pub struct HybridEncryption {
    x25519: X25519Encryption,
    mlkem: MLKEMEncryption,  // Kyber
}

impl HybridEncryption {
    pub fn encrypt(&self, data: &[u8]) -> Result<Vec<u8>> {
        // Generar dos shared secrets
        let secret1 = self.x25519.ecdh(...);
        let secret2 = self.mlkem.encapsulate(...);

        // Combinar (XOR o KDF)
        let combined = kdf(secret1, secret2);

        // Encriptar con combined key
        chacha20poly1305::encrypt(data, &combined)
    }
}
```

**Beneficios:**
- Seguro contra computadoras clásicas (X25519)
- Seguro contra computadoras cuánticas (ML-KEM)
- Si una falla, la otra protege

**Crates:**
- `pqcrypto-kyber` o `ml-kem`

**Prioridad:** Baja (quantum computers están lejos)

---

## 📊 Metrics & Observability (v0.2.0)

### Necesidades:

1. **Prover metrics:**
   - Proofs/hour
   - Average proof time
   - Success rate
   - Earnings tracking
   - Resource usage (CPU, RAM)

2. **Backend metrics:**
   - Uploads/downloads per second
   - Storage usage
   - Request latency
   - Error rates

3. **Smart contract events:**
   - Jobs created/completed/failed
   - Prover registrations
   - Slashing events

**Stack sugerido:**
- Prometheus (metrics collection)
- Grafana (dashboards)
- Loki (logs)

**Prioridad:** Media-Alta (necesario para producción)

---

## 🧪 Circuit Improvements (v0.4.0)

### Actual: Orchard-like Simplificado
El circuito actual es una simplificación de Orchard:
- Value conservation ✅
- Merkle path validation ✅ (simplificada)
- Value commitment ✅ (simplificado, no Pedersen real)

### Falta para Orchard Completo:
1. **Nullifier derivation** - Prevenir double-spend
2. **Full Merkle tree constraints** - Validación completa
3. **Pedersen commitments** - Value hiding real
4. **Spend authorization** - Validar firma
5. **Note encryption** - Output note handling

### Approach:
- Usar `orchard` crate directamente cuando API sea estable
- O implementar constraints faltantes en halo2_proofs

**Prioridad:** Media (actual es suficiente para PoC)

---

## 🚀 Performance Optimizations (v0.3.0)

### Prover Performance

1. **Persistent Proving Keys:**
```rust
// Guardar proving keys (140MB) en disco
// Cargar al inicio en vez de generar cada vez
// Ahorro: ~10 segundos por inicio
```

2. **GPU Acceleration:**
   - halo2 soporta GPU para MSM (Multi-Scalar Multiplication)
   - Reducción de 17s → ~5s por proof

3. **Parallel Proof Generation:**
   - Procesar múltiples jobs en paralelo
   - Usar todos los cores CPU

4. **Pre-computation:**
   - Cachear parámetros comunes
   - Lazy static para estructuras inmutables

### Backend Performance

1. **Redis/PostgreSQL en vez de HashMap:**
   - Persistencia
   - Clustering
   - Mejor performance

2. **CDN para witness distribution:**
   - CloudFlare Workers
   - Geolocation-based routing

**Prioridad:** Media

---

## 💰 Economic Improvements (v0.2.0)

### Dynamic Pricing
Actualmente precio es fijo. Mejorar con:

1. **Auction-based pricing:**
   - Clientes ofrecen precio
   - Provers aceptan o rechazan

2. **Reputation-based pricing:**
   - Provers con alta reputación pueden cobrar más
   - Descuentos para clientes frecuentes

3. **Circuit-specific pricing:**
   - Orchard proof: $0.02
   - Custom circuit: $0.05
   - Más complejo = más caro

**Prioridad:** Media

---

## 🔐 Privacy Improvements (v0.4.0)

### Metadata Privacy
Backend actual ve:
- IP addresses
- Timing (cuándo se sube/descarga)
- Correlaciones (cliente X → prover Y)

### Soluciones:

1. **Tor/Mixnet Integration:**
   - Cliente usa Tor para upload
   - Prover usa Tor para download
   - Oculta IPs

2. **Timing Obfuscation:**
   - Random delays
   - Batching de uploads
   - Previene timing analysis

3. **Dummy Traffic:**
   - Generar requests fake
   - Dificulta correlación

**Prioridad:** Baja-Media (no crítico para MVP)

---

## 🧪 Testing Improvements (v0.2.0)

### Necesidades:

1. **Property-based testing:**
   - `proptest` crate
   - Fuzzing de inputs

2. **Load testing:**
   - Simular 1000 provers
   - Stress test del backend

3. **Security audits:**
   - Formal verification de circuitos
   - Penetration testing
   - Code review externo

4. **Chaos engineering:**
   - Simular network partitions
   - RPC failures
   - Byzantine provers

**Prioridad:** Alta (antes de mainnet)

---

## 📱 Client SDK Improvements (v0.2.0)

### Necesidades:

1. **Flutter SDK completo:**
   - Wrapper de Rust via FFI
   - Witness generation mobile
   - UI components

2. **React SDK:**
   - Web wallet integration
   - Browser-based proving (WASM)

3. **CLI tool para testing:**
   - Generar witness desde CLI
   - Upload/download manualmente
   - Debug helpers

**Prioridad:** Media-Alta

---

## 🌍 Multi-chain Support (v1.0.0)

Actualmente solo Solana. Expandir a:

1. **Ethereum L2s:**
   - Optimism, Arbitrum, Base
   - Mismo modelo, diferente blockchain

2. **Cosmos chains:**
   - IBC para cross-chain proving

3. **Bitcoin layers:**
   - Lightning Network integration
   - Fedimint modules

**Prioridad:** Baja (focus en Solana primero)

---

## 📝 Governance (v2.0.0)

### DAO para CypherLink

1. **Token governance:**
   - CYPH token para voting
   - Protocol parameter changes
   - Treasury management

2. **Slashing governance:**
   - Community vote para penalidades
   - Dispute resolution

3. **Fee adjustments:**
   - Governance sobre platform fee
   - Revenue sharing con stakers

**Prioridad:** Baja (futuro lejano)

---

## ⚡ RISC0 / zkVM Integration (v0.5.0)

### Problema
Actualmente solo Halo2/Orchard. Queremos proofs generales.

### Solución: RISC0 zkVM

**Casos de uso:**
```rust
// Arbitrary computation en ZK
CircuitType::Custom("sha256_preimage") → RISC0 guest program
CircuitType::Custom("ML_inference") → Run model en zkVM
CircuitType::Custom("regex_match") → Prove string match
```

**Implementación:**
```rust
// prover-node/src/risc0_prover.rs
pub struct RISC0Prover {
    // ...
}

impl RISC0Prover {
    pub fn prove_custom(&self, program: &[u8], input: &[u8]) -> Result<Proof> {
        // Ejecutar programa en zkVM
        // Generar proof de ejecución correcta
    }
}
```

**Prioridad:** Media-Alta (roadmap mencionado en conversación original)

---

## 📄 Notas

- Este documento es LIVING: agregamos ideas a medida que surgen
- Prioridades son estimaciones, pueden cambiar
- Versiones son tentativas
- No todo se implementará necesariamente

**Última actualización:** 2025-11-11

