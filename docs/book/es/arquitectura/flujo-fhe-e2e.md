# Documentación del Flujo FHE de Extremo a Extremo

## Descripción General

El sistema FHE (Fully Homomorphic Encryption) de ZyberLink permite el cómputo seguro sobre datos encriptados con consenso multi-prover. Este documento describe el flujo completo de extremo a extremo desde la creación del trabajo hasta la finalización del resultado.

### Componentes Principales

- **Job Creator**: Genera datos FHE encriptados y envía trabajos on-chain
- **Blink Server**: Servicio backend para almacenamiento de witness y gestión de resultados
- **Prover Nodes**: Trabajadores autónomos que ejecutan cómputos FHE
- **Solana Program**: Contrato inteligente on-chain que gestiona el ciclo de vida del trabajo y el consenso
- **FHE Engine**: Motor de cómputo basado en TFHE ejecutándose en los provers

### Características Clave

- **Consenso Multi-Prover**: Los trabajos requieren acuerdo de múltiples provers (ej. 2 de 3)
- **Precios Dinámicos**: Sistema de precios basado en niveles (Tier 1-4) según la complejidad de la operación
- **Almacenamiento de Resultados**: Almacenamiento off-chain con hashes de commitment on-chain
- **Hashing Blake2s256**: Utilizado para commitments de witness y resultados
- **Selección Basada en ROI**: Los provers evalúan trabajos según rentabilidad

## Diagrama de Secuencia de Arquitectura

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

## Formato de Datos Witness

El formato witness varía según el tipo de operación:

### Operaciones de Valor Único (Add, Multiply, Threshold)

```
[encrypted_data_len: 4 bytes LE] [encrypted_data: N bytes] [server_key: M bytes]
```

**Ejemplo en código (job-creator):**

```rust
let mut encrypted_input = Vec::new();
encrypted_input.extend_from_slice(&(encrypted_data.len() as u32).to_le_bytes());
encrypted_input.extend_from_slice(&encrypted_data);
encrypted_input.extend_from_slice(&server_key);
```

### Operaciones Multi-Valor (Sum, Average, CountIf, Histogram)

```
[serialized_vec_len: 4 bytes LE] [Vec<Vec<u8>> bincode serialized] [server_key: M bytes]
```

**Ejemplo para operación Sum:**

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

## Implementación Paso a Paso

### Paso 1: Generar Datos FHE (Job Creator)

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

### Paso 2: Subir Witness al Backend

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

### Paso 3: Obtener Recomendación de Precio

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

### Paso 4: Crear Trabajo On-Chain

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

### Paso 5: Prover Descarga el Witness

```rust
// Download witness from backend
let witness_url = format!("{}/witness/{}", backend_url, hex::encode(witness_hash));
let witness_bytes = http_client.get(&witness_url).send().await?.bytes().await?;
```

### Paso 6: Prover Parsea el Formato Witness

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

### Paso 7: Prover Ejecuta el Cómputo FHE

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

### Paso 8: Prover Sube el Resultado FHE

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

### Paso 9: Prover Envía el Resultado On-Chain

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

### Paso 10: Consenso y Finalización

El programa on-chain maneja automáticamente el consenso:

1. **Envío de Resultados**: Cada prover envía su hash de resultado
2. **Verificación de Consenso**: El programa cuenta los hashes coincidentes
3. **Umbral Alcanzado**: Si `consensus_threshold` provers están de acuerdo (ej. 2 de 3):
   - El estado del trabajo se actualiza a `Completed`
   - Se almacena el hash de consenso
   - Los provers ganadores reciben el pago
   - Los provers en desacuerdo reciben reembolso
4. **Timeout**: Si no se alcanza consenso dentro de `submission_timeout_secs`:
   - El trabajo se marca como `Failed`
   - El creador recibe un reembolso

## Referencia de Endpoints de API

### Gestión de Witness

#### POST /witness

Subir datos witness encriptados.

**Request:**
- Body: Bytes crudos (application/octet-stream)
- Format: `[len][encrypted_data][server_key]`

**Response:**
```json
{
  "commitment": "hex-encoded-blake2s256-hash"
}
```

#### GET /witness/{commitment}

Descargar datos witness por hash de commitment.

**Response:**
- Body: Bytes crudos (application/octet-stream)

### Gestión de Resultados FHE

#### POST /fhe-result

Subir resultado de cómputo FHE.

**Request:**
- Body: Bytes crudos (resultado encriptado)

**Response:**
```json
{
  "commitment": "hex-encoded-blake2s256-hash"
}
```

#### GET /fhe-result/{commitment}

Descargar resultado FHE por hash de commitment.

**Response:**
- Body: Bytes crudos (resultado encriptado)

### Recomendación de Precio

#### POST /api/price-recommendation

Obtener recomendación de precio para operación FHE.

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

## Operaciones FHE y Niveles

### Tier 1: Operaciones Básicas (20M lamports/prover, 300s timeout)
- **Add**: Agregar constante a valor encriptado
- **Multiply**: Multiplicar valor encriptado por constante

### Tier 2: Agregación (20M lamports/prover, 600s timeout)
- **Sum**: Sumar múltiples valores encriptados
- **Threshold**: Verificar si el valor cumple un umbral

### Tier 3: Verificaciones Avanzadas (30M lamports/prover, 900s timeout)
- **RangeCheck**: Verificar valor dentro de un rango

### Tier 4: Operaciones Complejas (50M lamports/prover, 1800s timeout)
- **Average**: Calcular promedio de valores encriptados
- **CountIf**: Contar valores que cumplen un predicado
- **Histogram**: Generar bins de histograma

## Solución de Problemas

### Problema: "Witness too short to contain length prefix"

**Causa:** Formato witness inválido o descarga corrupta.

**Solución:**
1. Verificar que la carga del witness se completó exitosamente
2. Verificar que el hash de commitment coincide en la carga y descarga
3. Asegurar que la red no corrompió la transferencia

### Problema: "Failed to deserialize server key from witness"

**Causa:** La extracción del server key del witness falló.

**Solución:**
1. Verificar formato witness: `[len: 4 bytes LE][encrypted_data][server_key]`
2. Verificar que `encrypted_data_len` es correcto
3. Asegurar que el server key fue serializado con `bincode::serialize`

### Problema: "Thread-local key not set" o errores TFHE

**Causa:** TFHE utiliza almacenamiento thread-local para el server key.

**Solución:**
```rust
// ALWAYS call this in the thread performing FHE computation
engine.set_key_for_thread();
```

### Problema: "Expected N inputs for Sum operation, got M"

**Causa:** Discrepancia entre `expected_count` en la configuración de la operación y los valores encriptados reales.

**Solución:**
1. Verificar que el job creator genera el número correcto de valores encriptados
2. Verificar que `expected_count` coincide en `FheOperation::Sum { expected_count }`
3. Asegurar que el formato witness es correcto para operaciones multi-valor

### Problema: "Price too low for operation"

**Causa:** El precio del trabajo no cumple los requisitos mínimos del nivel.

**Solución:**
1. Usar `/api/price-recommendation` para obtener rangos de precio válidos
2. Asegurar que precio total ≥ `min_price_per_prover × required_provers`
3. Usar precio recomendado para ~95% de tasa de aceptación de provers

### Problema: "Consensus not reached - timeout"

**Causa:** No suficientes provers enviaron resultados coincidentes dentro del timeout.

**Solución:**
1. Incrementar `submission_timeout_secs` para operaciones complejas
2. Ofrecer precio más alto para atraer más provers
3. Verificar logs de provers para errores de cómputo
4. Verificar que los datos witness son válidos y consistentes

### Problema: "Commitment mismatch! Local vs Backend"

**Causa:** El hash Blake2s256 calculado por el creador no coincide con el backend.

**Solución:**
1. Asegurar que ambos usan Blake2s256 (no Blake2b)
2. Verificar que los bytes witness son idénticos en la carga
3. Verificar corrupción de red o problemas de codificación

## Documentación Relacionada

- [Referencia de API](./referencia-api.md) - Documentación completa de endpoints de API
- [Guía de Integración del Slider de Precios](../guias/integracion-slider-precio.md) - Integración frontend
- [Especificación de Operaciones FHE](./operaciones-fhe.md) - Especificaciones detalladas de operaciones
- [Configuración de Nodo Prover](../guias/configuracion-prover.md) - Ejecutar un nodo prover
- [Guía de Job Creator](../guias/job-creator.md) - Crear y enviar trabajos
