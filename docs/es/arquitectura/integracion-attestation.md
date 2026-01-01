# Integración AttestationService con ZK Jobs Flow

## Resumen

Se ha integrado exitosamente el `AttestationService` con el flujo de ZK jobs para verificar proofs Groth16 antes de aceptarlos.

## Cambios Implementados

### 1. Inicialización de AttestationService (main.rs)

**Ubicación**: `/home/deploy/2025q4/13/src/blink-server/src/main.rs`

- Agregada variable de entorno `VK_DIRECTORY` (default: `./verification_keys`)
- Inicialización del `AttestationService` al arranque del servidor
- Wrapped en `Arc<AttestationService>` para compartir entre threads
- Agregado como `app_data` en el HttpServer
- Logging completo de circuitos disponibles y warnings si no hay VKs

**Comportamiento**:
- Si no hay VKs: Warning pero el servidor arranca (backward compatible)
- Si hay VKs: Logs de circuitos disponibles
- Si falla inicialización: Warning y servicio deshabilitado

### 2. Nuevo Endpoint: Submit Proof (zk_handlers.rs)

**Ubicación**: `/home/deploy/2025q4/13/src/blink-server/src/zk_handlers.rs`

**Endpoint**: `POST /api/jobs/zk/{job_id}/submit-proof`

**Request Body**:
```json
{
  "prover_pubkey": "string",
  "proof": "string (JSON del proof)",
  "public_inputs": ["string array"]
}
```

**Flujo de Verificación**:
1. Validar que el job existe y está en estado `active` o `proving`
2. Clamar el job si está `active` (transición a `proving`)
3. Verificar proof con `AttestationService` (si disponible)
   - Si no hay VK para el circuito: Warning + acepta sin verificar
   - Si hay VK: Verifica con arkworks Groth16
4. Si proof es **inválido**: 
   - Marcar job como `failed`
   - Retornar HTTP 400 con detalles
5. Si proof es **válido**:
   - Guardar attestation en la DB
   - Calcular proof hash (Keccak256)
   - Completar el job (estado `completed`)
   - Retornar attestation_id y tiempo de verificación

**Response**:
```json
{
  "job_id": 123,
  "status": "completed",
  "attestation_id": 456,
  "verification_time_ms": 234
}
```

### 3. Endpoint de Consulta de Attestations

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

### 4. Imports y Dependencias

Limpiados imports no utilizados en:
- `src/db/mod.rs`
- `src/services/mod.rs` 
- `src/services/attestation_service.rs`
- `src/api_handlers.rs`

## Flujo Completo de ZK Job con Attestation

```
1. Creator: POST /api/jobs/zk/validate-and-build
   ↓ (job creado con status: pending_tx)
   
2. Creator: Firma y submite tx on-chain
   ↓
   
3. Creator: POST /api/jobs/zk/{job_id}/confirm
   ↓ (job confirmado con status: active)
   
4. Prover: Genera proof off-chain
   ↓
   
5. Prover: POST /api/jobs/zk/{job_id}/submit-proof
   ↓
   ├─ Backend verifica con AttestationService
   ├─ Si válido: guarda attestation + completa job
   └─ Si inválido: marca job como failed + retorna error
   
6. Opcional: GET /api/attestations/{job_id}
   (Para disputes o auditing)
```

## Configuración Necesaria

### Variables de Entorno

```bash
# Opcional: directorio con verification keys
VK_DIRECTORY=./verification_keys

# Los VKs deben tener nombres:
# circuit_10_vkey.json
# circuit_11_vkey.json
# ... hasta circuit_49_vkey.json
```

### Formato de VKs

Los verification keys deben estar en formato snarkjs JSON:
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

La integración es **100% backward compatible**:

1. Si no hay VKs cargados, los proofs se aceptan sin verificación (con warning)
2. Si AttestationService falla al inicializar, el servidor arranca normal
3. Si un circuito no tiene VK, acepta el proof para ese circuito específico

Esto permite migración gradual y no rompe flujos existentes.

## Testing

### Sin DB (esperado)
```bash
cargo check
# Error: set DATABASE_URL... (OK, es esperado)
```

### Con DB
```bash
export DATABASE_URL=postgresql://user:pass@localhost/db
cargo build --release
./target/release/blink-server
```

### Verificar Logs al Iniciar

```
[INFO] Initializing AttestationService...
[INFO] AttestationService ready with 4 circuit types: [10, 11, 12, 13]
```

O si no hay VKs:
```
[WARN] AttestationService initialized but NO verification keys loaded!
[WARN] Proofs will not be verified. Set VK_DIRECTORY env var with VK files.
```

## Archivos Modificados

1. `/home/deploy/2025q4/13/src/blink-server/src/main.rs`
   - Inicialización de AttestationService
   - Agregado como app_data

2. `/home/deploy/2025q4/13/src/blink-server/src/zk_handlers.rs`
   - Nuevo endpoint: `submit_zk_proof`
   - Nuevo endpoint: `get_attestation`
   - Estructuras: `SubmitProofRequest`, `SubmitProofResponse`

3. `/home/deploy/2025q4/13/src/blink-server/src/db/mod.rs`
   - Re-export de `AttestationQueries`

4. Limpieza de imports en varios archivos

## Próximos Pasos

1. Agregar VKs para circuitos 10-49 en `./verification_keys/`
2. Testing con proofs reales
3. Monitorear performance de verificación
4. Implementar cache de VKs si hay muchos circuitos
5. Agregar métricas de attestations (válidos vs inválidos)

## Métricas Disponibles

La tabla `attestations` permite queries analíticas:

```sql
-- Tasa de proofs válidos vs inválidos
SELECT 
  verification_result,
  COUNT(*) as count
FROM attestations
GROUP BY verification_result;

-- Tiempo promedio de verificación por circuito
SELECT 
  circuit_type,
  AVG(verification_time_ms) as avg_ms
FROM attestations
WHERE verification_result = true
GROUP BY circuit_type;
```

Estas queries están implementadas en `AttestationQueries`:
- `get_verification_stats()`
- `get_avg_verification_time()`
