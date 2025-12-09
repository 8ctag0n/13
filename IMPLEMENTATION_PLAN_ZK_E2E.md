# Plan de Implementación E2E - Sistema ZK Attestation

**Generado**: 2025-12-09
**Branch**: feature/api-simplification-v1
**Objetivo**: Implementar flujo completo de verificación ZK con attestation

---

## ANÁLISIS DE SITUACIÓN ACTUAL

### Estado del Sistema

**IMPLEMENTADO Y FUNCIONANDO**:
- blink-server (puerto 3000) con API REST para jobs ZK
- x402-server (puerto 8081) anti-spam funcional
- PostgreSQL con 4 tablas: zk_jobs, attestations, x402_quotes, x402_tokens
- Migración 20251209000020 aplicada (tabla attestations)
- AttestationService con verificación Groth16 integrado en submit_zk_proof
- Circuito verify-groth16 compilado (228K constraints)
- Keys generadas: circuit_final.zkey (98 MB), verification_key.json (3.3 KB)
- WASM compilado: circuit.wasm (3.9 MB)
- Script generate_proof.sh funcional

**GAPS CRÍTICOS IDENTIFICADOS**:

1. **Circuit Testing** (BLOQUEADOR):
   - input.json tiene valores de prueba inválidos
   - No se ha generado witness ni proof válido nunca
   - No hay test vectors documentados
   - Pairing check puede no estar funcionando correctamente

2. **Proof Storage** (FUNCIONALIDAD INCOMPLETA):
   - proof_json NO existe en schema zk_jobs
   - retention_expires_at NO existe en schema
   - No hay endpoint GET /api/jobs/zk/{job_id}/proof
   - No hay cleanup service para proofs expirados

3. **Verification Keys Directory** (CONFIGURACIÓN):
   - /home/deploy/2025q4/13/verification_keys/ NO existe
   - AttestationService busca VKs en ./verification_keys/
   - Necesita circuit_10_vkey.json a circuit_49_vkey.json
   - Solo existe verification_key.json en circuits/verify-groth16/

4. **Integration Testing** (VALIDACIÓN):
   - No hay test E2E del flujo completo
   - No hay documentación de uso trustless
   - No se ha probado el flujo POST job → submit proof → verify

---

## ESTRATEGIA DE EJECUCIÓN

### Principios de Priorización

1. **Desbloquear antes de construir**: Resolver circuit testing primero
2. **Mínimo viable antes de completo**: Hybrid storage (DB + cleanup manual)
3. **Paralelizable cuando es independiente**: VK setup puede ir en paralelo
4. **Integrar antes de optimizar**: E2E test antes de production hardening

### Dependencias Críticas

```
PHASE 1 (Circuit Testing) ← BLOQUEADOR DE TODO
    ↓
PHASE 2 (VK Directory) ← Necesario para verificación
    ↓
PHASE 3 (Integration E2E) ← Valida todo funciona
    ↓
PHASE 4 (Proof Storage) ← Feature adicional, no bloqueante
    ↓
PHASE 5 (Production Hardening) ← Opcional
```

---

## PLAN DE IMPLEMENTACIÓN

## PHASE 0: Validación Pre-vuelo (15 min)

**Objetivo**: Confirmar que el entorno está listo.

**Pasos**:
```bash
# 1. Verificar DB disponible
cd /home/deploy/2025q4/13
docker compose ps postgres
# Si no está corriendo: docker compose up -d postgres

# 2. Verificar migración attestations aplicada
psql $DATABASE_URL -c "\d attestations"
# Debe mostrar la tabla con columnas: id, job_id, circuit_type, witness, vk_hash, etc.

# 3. Verificar compilación Rust OK
cd src/blink-server
cargo check --release
# Debe compilar sin errores (warnings OK)

# 4. Verificar snarkjs disponible
cd /home/deploy/2025q4/13/circuits
bun run snarkjs --version
# Debe mostrar versión (ej: 0.7.x)
```

**Criterio de Éxito**: Todos los comandos pasan sin errores críticos.

**Bloqueadores**: Si DB no está disponible o migración no aplicada, aplicar primero.

---

## PHASE 1: Circuit Testing & Proof Generation (CRÍTICO - 2-3h)

**Objetivo**: Generar y verificar el PRIMER proof válido.

**Problema actual**: input.json tiene valores hardcoded inválidos que no satisfacen constraints del circuito.

### Paso 1.1: Analizar Constraints del Circuito (30 min)

**Archivo**: `/home/deploy/2025q4/13/circuits/verify-groth16/circuit.circom`

**Constraints críticas a entender**:
- Línea 74: `vk_hash === vk_hasher.out` (hash del VK debe coincidir)
- Línea 84: `public_inputs_hash === pi_hasher.out` (hash de inputs debe coincidir)
- Línea 141: `verification_result === pairing_check.out` (pairing debe pasar)
- Línea 148: `verification_result * (1 - verification_result) === 0` (booleano)

**Inputs necesarios**:
- Proof Groth16 válido de otro circuito (P1)
- VK correspondiente a ese proof
- Public inputs que coincidan con P1
- Hashes calculados correctamente (Poseidon)

**Acción**:
```bash
# Crear circuit_analysis.md con estructura de inputs
cd /home/deploy/2025q4/13/circuits/verify-groth16
cat > circuit_analysis.md << 'EOF'
# Análisis de Constraints - VERIFY_GROTH16

## Input Structure

### Public Signals (van on-chain)
1. vk_hash: Poseidon hash del VK
2. public_inputs_hash: Poseidon hash de los public_inputs
3. verification_result: 1 (válido) o 0 (inválido)

### Private Witness
- proof_a, proof_b, proof_c: Puntos del proof P1
- vk_alpha, vk_beta, vk_gamma, vk_delta: VK del circuito P1
- vk_ic[33]: IC points del VK
- num_public_inputs: Cantidad de inputs en P1
- public_inputs[32]: Los inputs del proof P1
- verification_witness: Attestation backend

## Flujo de Validación
1. Verificar vk_hash = Poseidon(VK)
2. Verificar public_inputs_hash = Poseidon(inputs)
3. Calcular vk_x = IC[0] + Σ(inputs[i] * IC[i+1])
4. Verificar pairing: e(A,B) = e(α,β) * e(vk_x,γ) * e(C,δ)
EOF
```

### Paso 1.2: Generar Circuito Simple de Test (1h)

**Objetivo**: Crear un circuito P1 trivial para usar como input.

**Acción**:
```bash
cd /home/deploy/2025q4/13/circuits/verify-groth16

# Crear circuito simple que solo verifica un input
cat > test_simple.circom << 'EOF'
pragma circom 2.1.6;

template SimpleCheck() {
    signal input a;
    signal input b;
    signal output c;

    c <== a + b;
}

component main {public [a, b]} = SimpleCheck();
EOF

# Compilar circuito simple
circom test_simple.circom --r1cs --wasm --sym -o ./test_simple_build

# Generar keys para test_simple
cd test_simple_build
snarkjs groth16 setup test_simple.r1cs ../../ptau/pot18_final.ptau test_simple_0000.zkey
snarkjs zkey contribute test_simple_0000.zkey test_simple_final.zkey --name="Test" -v -e="random"
snarkjs zkey export verificationkey test_simple_final.zkey test_simple_vk.json

# Generar input válido
cat > test_simple_input.json << 'EOF'
{
    "a": "5",
    "b": "10"
}
EOF

# Generar proof
snarkjs wtns calculate test_simple_js/test_simple.wasm test_simple_input.json test_simple_witness.wtns
snarkjs groth16 prove test_simple_final.zkey test_simple_witness.wtns test_simple_proof.json test_simple_public.json

# VERIFICAR (debe pasar)
snarkjs groth16 verify test_simple_vk.json test_simple_public.json test_simple_proof.json
```

**Output esperado**: "OK! Proof verified successfully."

### Paso 1.3: Crear Input Válido para VERIFY_GROTH16 (1h)

**Acción**:
```bash
cd /home/deploy/2025q4/13/circuits/verify-groth16

# Script para generar input válido a partir de test_simple
cat > generate_valid_input.js << 'EOF'
const fs = require('fs');
const { buildPoseidon } = require('circomlibjs');

async function main() {
    // Cargar proof y VK de test_simple
    const proof = JSON.parse(fs.readFileSync('./test_simple_build/test_simple_proof.json'));
    const vk = JSON.parse(fs.readFileSync('./test_simple_build/test_simple_vk.json'));
    const publicInputs = JSON.parse(fs.readFileSync('./test_simple_build/test_simple_public.json'));

    // Calcular vk_hash con Poseidon
    const poseidon = await buildPoseidon();

    // Hash de VK (simplificado - solo alpha y beta para demo)
    const vkHashInput = [
        BigInt(vk.vk_alpha_1[0]),
        BigInt(vk.vk_alpha_1[1])
    ];
    const vkHash = poseidon(vkHashInput);

    // Hash de public inputs
    const piHashInput = publicInputs.map(x => BigInt(x));
    const piHash = poseidon(piHashInput);

    // Construir input completo
    const input = {
        vk_hash: poseidon.F.toString(vkHash),
        public_inputs_hash: poseidon.F.toString(piHash),
        verification_result: "1",  // Proof es válido

        proof_a: proof.pi_a.slice(0, 2),
        proof_b: [
            proof.pi_b[0].slice(0, 2),
            proof.pi_b[1].slice(0, 2)
        ],
        proof_c: proof.pi_c.slice(0, 2),

        vk_alpha: vk.vk_alpha_1.slice(0, 2),
        vk_beta: [
            vk.vk_beta_2[0].slice(0, 2),
            vk.vk_beta_2[1].slice(0, 2)
        ],
        vk_gamma: [
            vk.vk_gamma_2[0].slice(0, 2),
            vk.vk_gamma_2[1].slice(0, 2)
        ],
        vk_delta: [
            vk.vk_delta_2[0].slice(0, 2),
            vk.vk_delta_2[1].slice(0, 2)
        ],

        // IC array (rellenar con zeros si faltan)
        vk_ic: Array(33).fill(null).map((_, i) =>
            vk.IC[i] ? vk.IC[i].slice(0, 2) : ["0", "0"]
        ),

        num_public_inputs: publicInputs.length.toString(),
        public_inputs: Array(32).fill("0").map((_, i) =>
            publicInputs[i] || "0"
        ),

        verification_witness: "12345"  // Dummy para MVP
    };

    fs.writeFileSync('valid_input.json', JSON.stringify(input, null, 2));
    console.log('✓ Generated valid_input.json');
}

main().catch(console.error);
EOF

# Ejecutar generador
node generate_valid_input.js

# PROBAR: Generar proof con input válido
./generate_proof.sh valid_input.json ./test_output
```

**Criterio de éxito**:
- Script genera valid_input.json sin errores
- generate_proof.sh completa sin errores
- snarkjs verify retorna "OK!"

**Si falla**: Iterar ajustando hashes Poseidon hasta que constraints se satisfagan.

---

## PHASE 2: VK Directory Setup (30 min - PARALELO)

**Objetivo**: Preparar directorio de verification keys para AttestationService.

**Puede ejecutarse en paralelo con PHASE 1 Paso 1.3**.

### Paso 2.1: Crear Directorio y Copiar VK Base

```bash
cd /home/deploy/2025q4/13

# Crear directorio
mkdir -p verification_keys

# Copiar VK existente como circuit_10 (circuito de test)
cp circuits/verify-groth16/verification_key.json verification_keys/circuit_10_vkey.json

# Crear VKs dummy para otros circuitos (11-49) si es necesario
# Para MVP, solo circuit_10 es suficiente

# Verificar formato JSON válido
cat verification_keys/circuit_10_vkey.json | jq '.protocol'
# Debe retornar: "groth16"
```

### Paso 2.2: Configurar Variable de Entorno

```bash
# Agregar a .env o docker-compose.yml
echo 'VK_DIRECTORY=/home/deploy/2025q4/13/verification_keys' >> src/blink-server/.env.example

# Para docker-compose
# Agregar en environment del servicio backend:
#   - VK_DIRECTORY=/app/verification_keys
# Y en volumes:
#   - ./verification_keys:/app/verification_keys:ro
```

### Paso 2.3: Validar Carga de VKs

```bash
cd src/blink-server

# Test de inicialización
export VK_DIRECTORY=/home/deploy/2025q4/13/verification_keys
cargo run --bin blink-server 2>&1 | grep "AttestationService"

# Debe mostrar:
# [INFO] Initializing AttestationService...
# [INFO] Loaded VK for circuit type 10
# [INFO] AttestationService ready with 1 circuit types: [10]
```

**Criterio de éxito**: Servidor arranca y carga al menos 1 VK.

---

## PHASE 3: Integration E2E Testing (2-3h)

**Objetivo**: Validar flujo completo con proof real.

### Paso 3.1: Levantar Stack Completo (15 min)

```bash
cd /home/deploy/2025q4/13

# Iniciar servicios
docker compose up -d postgres
docker compose up -d blink-server x402-server

# Verificar salud
curl http://localhost:3000/health
curl http://localhost:8081/health

# Ver logs
docker compose logs -f blink-server | grep -i attestation
```

### Paso 3.2: Test Manual del Flujo (1h)

**Script de test**:
```bash
cd /home/deploy/2025q4/13

cat > test_e2e_zk_flow.sh << 'EOF'
#!/bin/bash
set -e

API_BASE="http://localhost:3000/api"
CREATOR_PUBKEY="TestCreator1111111111111111111111111111"
PROVER_PUBKEY="TestProver11111111111111111111111111111"

echo "=== ZK Job E2E Test ==="

# 1. Crear job
echo "[1/5] Creating ZK job..."
JOB_RESPONSE=$(curl -s -X POST "$API_BASE/jobs/zk/validate-and-build" \
  -H "Content-Type: application/json" \
  -d "{
    \"creator_pubkey\": \"$CREATOR_PUBKEY\",
    \"circuit_type\": 10,
    \"witness_commitment\": \"1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef\",
    \"public_inputs\": [\"100\", \"200\", \"300\"],
    \"timeout_seconds\": 3600
  }")

echo "$JOB_RESPONSE" | jq .

JOB_ID=$(echo "$JOB_RESPONSE" | jq -r '.job_id')
echo "Created job_id: $JOB_ID"

# 2. Confirmar job (simular tx on-chain)
echo "[2/5] Confirming job..."
curl -s -X POST "$API_BASE/jobs/zk/$JOB_ID/confirm" \
  -H "Content-Type: application/json" \
  -d "{
    \"tx_signature\": \"5KTest111111111111111111111111111111111111111111111111111111111111111111111111\"
  }" | jq .

# 3. Generar proof off-chain (usando proof de PHASE 1)
echo "[3/5] Generating proof..."
cd circuits/verify-groth16
./generate_proof.sh valid_input.json /tmp/test_proof
PROOF_JSON=$(cat /tmp/test_proof/proof.json)
PUBLIC_JSON=$(cat /tmp/test_proof/public.json)

# 4. Submit proof
echo "[4/5] Submitting proof..."
PROOF_RESPONSE=$(curl -s -X POST "$API_BASE/jobs/zk/$JOB_ID/submit-proof" \
  -H "Content-Type: application/json" \
  -d "{
    \"prover_pubkey\": \"$PROVER_PUBKEY\",
    \"proof\": $(echo $PROOF_JSON | jq -c .),
    \"public_inputs\": $(echo $PUBLIC_JSON | jq -c .)
  }")

echo "$PROOF_RESPONSE" | jq .

ATTESTATION_ID=$(echo "$PROOF_RESPONSE" | jq -r '.attestation_id')
echo "Created attestation_id: $ATTESTATION_ID"

# 5. Verificar attestation
echo "[5/5] Querying attestation..."
curl -s "$API_BASE/attestations/$JOB_ID" | jq .

echo ""
echo "=== Test Complete ==="
EOF

chmod +x test_e2e_zk_flow.sh
./test_e2e_zk_flow.sh
```

**Criterio de éxito**:
- Job se crea con status pending_tx
- Job se confirma a status active
- Proof se verifica y crea attestation
- Job se marca como completed
- Attestation se puede consultar

### Paso 3.3: Documentar Test Cases (30 min)

Crear `/home/deploy/2025q4/13/TEST_CASES_ZK_E2E.md` con:
- Casos de éxito (proof válido)
- Casos de error (proof inválido)
- Casos de timeout
- Casos de VK no disponible

---

## PHASE 4: Proof Storage Enhancement (OPCIONAL - 3-4h)

**Objetivo**: Agregar almacenamiento de proofs completos con TTL.

**Nota**: Esta fase NO es bloqueante para MVP. Se puede implementar después.

### Paso 4.1: Schema Migration (30 min)

```bash
cd /home/deploy/2025q4/13/src/blink-server/migrations

cat > 20251210000021_add_proof_storage.sql << 'EOF'
-- Add proof storage to zk_jobs table
ALTER TABLE zk_jobs
  ADD COLUMN proof_json JSONB,
  ADD COLUMN retention_expires_at TIMESTAMP;

-- Index for cleanup queries
CREATE INDEX idx_zk_jobs_retention_expires
  ON zk_jobs(retention_expires_at)
  WHERE retention_expires_at IS NOT NULL;

COMMENT ON COLUMN zk_jobs.proof_json IS 'Full proof in snarkjs format (optional, for download)';
COMMENT ON COLUMN zk_jobs.retention_expires_at IS 'When to delete proof_json (NULL = keep forever)';
EOF

# Aplicar migración
export DATABASE_URL="postgresql://..."
sqlx migrate run
```

### Paso 4.2: Update ZkQueries (1h)

Modificar `/home/deploy/2025q4/13/src/blink-server/src/db/zk_queries.rs`:
- Agregar campo proof_json en struct ZkJob
- Actualizar save_proof para guardar proof completo
- Agregar query get_proof_by_job_id
- Agregar query list_expired_proofs

### Paso 4.3: Endpoint GET Proof (30 min)

Agregar en `/home/deploy/2025q4/13/src/blink-server/src/zk_handlers.rs`:
```rust
pub async fn get_proof(
    pool: web::Data<PgPool>,
    path: web::Path<i64>,
) -> Result<HttpResponse, Error> {
    let job_id = path.into_inner();

    let proof = ZkQueries::get_proof_by_job_id(&pool, job_id)
        .await
        .map_err(|_| ErrorNotFound("Proof not found or expired"))?;

    Ok(HttpResponse::Ok().json(proof))
}
```

### Paso 4.4: Cleanup Service (1h)

Crear `/home/deploy/2025q4/13/src/blink-server/src/services/proof_cleanup_service.rs`:
- Background task que corre cada hora
- Query proofs con retention_expires_at < NOW()
- Eliminar proof_json (UPDATE SET proof_json = NULL)
- Logging de proofs eliminados

**MVP Alternativo**: Script cron manual en vez de background service.

---

## PHASE 5: Production Hardening (OPCIONAL - 4-5h)

**Objetivo**: Preparar para producción real.

### Recomendaciones:

1. **Trusted Setup Ceremony** (fuera de scope)
   - Coordinar MPC ceremony para production keys
   - No usar keys de development

2. **Circuit Audit** (fuera de scope)
   - Auditar verify-groth16 circuit
   - Validar security assumptions

3. **Performance Optimization** (2h)
   - Benchmark verification times
   - Agregar caché de PreparedVK
   - Optimizar queries con EXPLAIN ANALYZE

4. **Monitoring & Metrics** (2h)
   - Prometheus metrics para attestations
   - Alertas para verification failures
   - Dashboard de tasa de éxito

5. **Rate Limiting** (1h)
   - Limitar submit-proof por IP
   - Prevenir spam de proofs inválidos

---

## CRONOGRAMA SUGERIDO

### Día 1 (4-5h)
- PHASE 0: Validación Pre-vuelo (15 min)
- PHASE 1: Circuit Testing completo (2-3h)
- PHASE 2: VK Directory (30 min en paralelo)
- Checkpoint: Primer proof válido generado

### Día 2 (2-3h)
- PHASE 3: Integration E2E Testing (2-3h)
- Checkpoint: Flujo completo funciona end-to-end

### Día 3+ (Opcional)
- PHASE 4: Proof Storage (3-4h) - Solo si se requiere download
- PHASE 5: Production Hardening (4-5h) - Solo para producción

---

## MÉTRICAS DE ÉXITO

### MVP Mínimo (Día 1-2)
- [ ] Al menos 1 proof válido generado con VERIFY_GROTH16
- [ ] AttestationService carga VK y verifica proof
- [ ] Test E2E completo pasa sin errores
- [ ] Job transita: pending_tx → active → proving → completed

### MVP Completo (Día 3)
- [ ] Proof storage implementado
- [ ] Endpoint GET /api/jobs/zk/{id}/proof funciona
- [ ] Cleanup service elimina proofs expirados
- [ ] Documentación de uso publicada

### Production Ready (Día 4+)
- [ ] Ceremony keys generadas
- [ ] Circuit auditado
- [ ] Monitoring en producción
- [ ] Rate limiting activo

---

## DECISIONES DE DISEÑO

### Por qué PHASE 1 es crítico
Sin un proof válido, no podemos validar que el circuito funciona. Es un bloqueador absoluto.

### Por qué VK Directory es PHASE 2
AttestationService puede inicializar sin VKs (backward compatible), pero para verificar necesitamos al menos 1 VK.

### Por qué Proof Storage es opcional
El sistema funciona con solo attestations. El proof completo es útil para audits pero no bloquea el flujo trustless.

### Por qué no usar IPFS en MVP
IPFS agrega complejidad (nodo, pinning, retrieval) sin valor inmediato. Podemos iterar a IPFS después de validar MVP.

---

## ROLLBACK PLAN

Si algo falla en producción:

1. **Circuit no genera proofs válidos**
   - Rollback: AttestationService acepta sin verificar (modo backward compatible)
   - Fix: Iterar en PHASE 1 offline

2. **VK no carga correctamente**
   - Rollback: Modo sin VKs (warning pero funcional)
   - Fix: Corregir formato de VKs

3. **Performance inaceptable**
   - Rollback: Deshabilitar verificación en AttestationService
   - Fix: Implementar caché de PreparedVK

---

## CONTACTOS Y RECURSOS

**Documentación**:
- Circom: https://docs.circom.io
- Snarkjs: https://github.com/iden3/snarkjs
- Arkworks: https://github.com/arkworks-rs

**Archivos clave**:
- Circuit: `/home/deploy/2025q4/13/circuits/verify-groth16/circuit.circom`
- Service: `/home/deploy/2025q4/13/src/blink-server/src/services/attestation_service.rs`
- Handlers: `/home/deploy/2025q4/13/src/blink-server/src/zk_handlers.rs`
- Schema: `/home/deploy/2025q4/13/src/blink-server/migrations/20251209000019_create_zk_jobs_table.sql`

**Testing**:
- E2E Script: `/home/deploy/2025q4/13/test_e2e_zk_flow.sh` (crear en PHASE 3)
- Input Generator: `/home/deploy/2025q4/13/circuits/verify-groth16/generate_valid_input.js` (crear en PHASE 1)

---

## APÉNDICE: COMANDOS RÁPIDOS

```bash
# Compilar circuito
cd circuits/verify-groth16
circom circuit.circom --r1cs --wasm --sym -o ./build

# Generar proof
./generate_proof.sh input.json ./output

# Test backend
cd src/blink-server
cargo test --release attestation

# Test E2E
./test_e2e_zk_flow.sh

# Ver logs
docker compose logs -f blink-server | grep attestation

# Limpiar proofs manualmente
psql $DATABASE_URL -c "UPDATE zk_jobs SET proof_json = NULL WHERE retention_expires_at < NOW()"
```

---

**Fin del Plan de Implementación**
**Última actualización**: 2025-12-09
**Versión**: 1.0
