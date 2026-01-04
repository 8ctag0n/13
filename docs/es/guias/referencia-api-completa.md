# Referencia API de ZyberLink

**Documentación Completa de la API REST para el Marketplace de ZyberLink**

---

## Tabla de Contenidos

1. [Descripción General](#descripción-general)
2. [Autenticación](#autenticación)
3. [URLs Base](#urls-base)
4. [Endpoints de Gestión de Jobs](#endpoints-de-gestión-de-jobs)
5. [Endpoints de Precios](#endpoints-de-precios)
6. [Endpoints de Almacenamiento de Datos](#endpoints-de-almacenamiento-de-datos)
7. [Manejo de Errores](#manejo-de-errores)
8. [Límites de Tasa](#límites-de-tasa)
9. [Ejemplos](#ejemplos)

---

## Descripción General

La API de ZyberLink proporciona endpoints RESTful para interactuar con el marketplace de computación descentralizada. La API gestiona:

- Validación de jobs y construcción de transacciones
- Estimación de costos y recomendaciones de precios dinámicos
- Seguimiento de estado de jobs
- Entrega de datos de computación para provers
- Almacenamiento de witness y resultados FHE con commitments basados en contenido

**Versión API:** v1.1
**Content-Type:** application/json (endpoints API), application/octet-stream (almacenamiento de datos)
**Protocolo:** HTTPS (producción), HTTP (desarrollo)

---

## Autenticación

Actualmente, la API no requiere autenticación para endpoints de solo lectura. Las operaciones de escritura (creación de jobs) requieren firmas on-chain vía transacciones de Solana.

**Futuro:** Se puede agregar autenticación OAuth2 o API key para límites de tasa y análisis.

---

## URLs Base

| Ambiente | URL |
|----------|-----|
| Desarrollo Local | `http://localhost:8080` |
| Testnet | `https://api-testnet.zyberlink.io` |
| Mainnet | `https://api.zyberlink.io` |

---

## Endpoints de Gestión de Jobs

### POST /api/jobs/validate-and-build

**Descripción:** Valida los datos del job y construye una transacción de Solana sin firmar para la creación del job.

**Request Body:**
```json
{
  "creator_pubkey": "ABC123...",
  "encrypted_data": "base64_encoded_data",
  "server_key": "base64_encoded_key",
  "operation": "add",
  "operation_value": 5,
  "price_lamports": 3000000,
  "required_provers": 3,
  "consensus_threshold": 2,
  "payment_method": "SOL",
  "payment_token_mint": null,
  "signature": "signature_hex"
}
```

**Request Schema:**
```typescript
interface ValidateJobRequest {
  creator_pubkey: string;          // Base58 Solana pubkey
  encrypted_data: string;          // Base64 encoded ciphertext
  server_key: string;              // Base64 encoded FHE server key
  operation: string;               // "add" | "multiply" | "sum" | etc.
  operation_value: number;         // Operation parameter
  price_lamports: number;          // Total payment in lamports
  required_provers: number;        // 2-10 provers
  consensus_threshold: number;     // Min matching results
  payment_method: string;          // "SOL" | "wZEC"
  payment_token_mint?: string;     // Token mint if not SOL
  signature: string;               // Ed25519 signature
}
```

**Response (Success):**
```json
{
  "job_id": 12345,
  "transaction": "base64_encoded_transaction",
  "status": "pending_signature"
}
```

**Response Schema:**
```typescript
interface ValidateJobResponse {
  job_id: number;
  transaction: string;  // Base64 serialized Transaction
  status: string;       // "pending_signature"
}
```

**Códigos de Estado:**
- `200 OK` - Job validado y transacción construida
- `400 Bad Request` - Validación fallida (ver mensaje de error)
- `500 Internal Server Error` - Error de base de datos o serialización

**Ejemplo:**
```bash
curl -X POST http://localhost:8080/api/jobs/validate-and-build \
  -H "Content-Type: application/json" \
  -d '{
    "creator_pubkey": "11111111111111111111111111111111",
    "encrypted_data": "ZW5jcnlwdGVk",
    "server_key": "c2VydmVy",
    "operation": "add",
    "operation_value": 5,
    "price_lamports": 3000000,
    "required_provers": 3,
    "consensus_threshold": 2,
    "payment_method": "SOL",
    "signature": "abcd1234..."
  }'
```

---

### GET /api/jobs/{job_id}/compute-data

**Descripción:** Recupera datos de computación cifrados para provers. Solo devuelve datos si el estado del job es "active".

**Path Parameters:**
- `job_id` (integer, required) - Job ID

**Response (Success):**
```json
{
  "job_id": 12345,
  "encrypted_data": "base64_encoded_data",
  "server_key": "base64_encoded_key",
  "operation": "add",
  "operation_value": 5
}
```

**Response Schema:**
```typescript
interface ComputeDataResponse {
  job_id: number;
  encrypted_data: string;  // Base64
  server_key: string;      // Base64
  operation: string;
  operation_value: number;
}
```

**Códigos de Estado:**
- `200 OK` - Datos de computación devueltos
- `400 Bad Request` - Job no está en estado "active"
- `404 Not Found` - El job no existe
- `500 Internal Server Error` - Error de base de datos

**Ejemplo:**
```bash
curl -X GET http://localhost:8080/api/jobs/12345/compute-data
```

---

### POST /api/jobs/{job_id}/confirm

**Descripción:** Confirma que una transacción de job fue enviada exitosamente on-chain. Actualiza el estado del job de "pending_tx" a "active".

**Path Parameters:**
- `job_id` (integer, required) - Job ID

**Request Body:**
```json
{
  "signature": "transaction_signature_base58"
}
```

**Request Schema:**
```typescript
interface ConfirmJobRequest {
  signature: string;  // Transaction signature
}
```

**Response (Success):**
```json
{
  "job_id": 12345,
  "status": "active",
  "message": "Job confirmed and ready for provers"
}
```

**Códigos de Estado:**
- `200 OK` - Job confirmado
- `404 Not Found` - El job no existe
- `500 Internal Server Error` - Actualización de base de datos fallida

**Ejemplo:**
```bash
curl -X POST http://localhost:8080/api/jobs/12345/confirm \
  -H "Content-Type: application/json" \
  -d '{"signature": "5J7Zx..."}'
```

**Nota:** En producción, este endpoint debe verificar la firma de la transacción on-chain antes de actualizar el estado.

---

### GET /api/jobs/{job_id}/status

**Descripción:** Obtiene el estado actual de un job.

**Path Parameters:**
- `job_id` (integer, required) - Job ID

**Response (Success):**
```json
{
  "job_id": 12345,
  "status": "active",
  "created_at": "2025-11-21T18:00:00Z"
}
```

**Response Schema:**
```typescript
interface JobStatusResponse {
  job_id: number;
  status: string;      // "pending_tx" | "active" | "completed" | "failed"
  created_at: string;  // ISO 8601 timestamp
}
```

**Códigos de Estado:**
- `200 OK` - Estado devuelto
- `404 Not Found` - El job no existe
- `500 Internal Server Error` - Error de base de datos

**Ejemplo:**
```bash
curl -X GET http://localhost:8080/api/jobs/12345/status
```

---

### DELETE /api/jobs/{job_id}

**Descripción:** Elimina datos de job. Solo permitido si el job está en estado terminal (completed/failed).

**Path Parameters:**
- `job_id` (integer, required) - Job ID

**Response (Success):**
```json
{
  "message": "Job deleted successfully"
}
```

**Códigos de Estado:**
- `200 OK` - Job eliminado
- `400 Bad Request` - Job no está en estado terminal
- `404 Not Found` - El job no existe
- `500 Internal Server Error` - Error de base de datos

**Ejemplo:**
```bash
curl -X DELETE http://localhost:8080/api/jobs/12345
```

---

## Endpoints de Precios

### POST /api/estimate-cost

**Descripción:** Estima el costo y tiempo de espera para una operación FHE dada. Este es el **endpoint principal** para la integración de precios dinámicos.

**Request Body:**
```json
{
  "operation": "histogram",
  "operation_value": 0,
  "expected_count": null,
  "bins": 10,
  "required_provers": 3
}
```

**Request Schema:**
```typescript
interface EstimateCostRequest {
  operation: string;           // FHE operation name
  operation_value: number;     // Constant value (for Add, Multiply, Threshold)
  expected_count?: number;     // Item count (for Sum, Average, CountIf)
  bins?: number;               // Bin count (for Histogram)
  required_provers: number;    // 2-10 provers
}
```

**Operaciones Soportadas:**

| Operación | Campos Requeridos | Campos Opcionales | Descripción |
|-----------|-------------------|-------------------|-------------|
| `add` | `operation_value` | - | Agregar constante al valor cifrado |
| `multiply` | `operation_value` | - | Multiplicar valor cifrado por constante |
| `sum` | - | `expected_count` | Suma de múltiples valores cifrados |
| `threshold` | `operation_value` | - | Verificar si valor >= umbral |
| `range_check` | `operation_value` | - | Verificar si valor en rango [0, value] |
| `average` | - | `expected_count` | Promedio de valores cifrados |
| `count_if` | `operation_value` | `expected_count` | Contar valores que coinciden con predicado |
| `histogram` | - | `bins` | Distribución a través de bins |

**Response (Success):**
```json
{
  "operation": "Histogram",
  "complexity_tier": 5,
  "min_payment_lamports": 732050808,
  "min_payment_sol": 0.732050808,
  "total_min_payment_lamports": 2196152424,
  "total_min_payment_sol": 2.196152424,
  "timeout_seconds": 1600,
  "estimated_compute_ms": 30500
}
```

**Response Schema:**
```typescript
interface EstimateCostResponse {
  operation: string;                    // Operation name
  complexity_tier: number;              // 1-5 tier
  min_payment_lamports: number;         // Cost per prover (lamports)
  min_payment_sol: number;              // Cost per prover (SOL)
  total_min_payment_lamports: number;   // Total cost (lamports)
  total_min_payment_sol: number;        // Total cost (SOL)
  timeout_seconds: number;              // Dynamic timeout
  estimated_compute_ms: number;         // Estimated execution time
}
```

**Niveles de Complejidad:**

| Nivel | Complejidad | Costo Base/Prover | Operaciones Ejemplo |
|-------|-------------|-------------------|---------------------|
| 1 | O(1) | 0.001 SOL | Add, Multiply |
| 2 | O(n) | 0.001 + n×0.0001 SOL | Sum |
| 3 | O(1) + bootstrap | 0.005 SOL | Threshold, RangeCheck |
| 4 | O(n) + predicates | Variable | Average, CountIf |
| 5 | O(n×m) | 0.1 + 0.02×m^1.5 SOL | Histogram |

**Códigos de Estado:**
- `200 OK` - Estimación de costo devuelta
- `400 Bad Request` - Operación o parámetros inválidos
- `500 Internal Server Error` - Error de cálculo

**Ejemplo 1: Suma Simple (Nivel 1)**
```bash
curl -X POST http://localhost:8080/api/estimate-cost \
  -H "Content-Type: application/json" \
  -d '{
    "operation": "add",
    "operation_value": 5,
    "required_provers": 3
  }'
```

**Response:**
```json
{
  "operation": "Add",
  "complexity_tier": 1,
  "min_payment_lamports": 1000000,
  "min_payment_sol": 0.001,
  "total_min_payment_lamports": 3000000,
  "total_min_payment_sol": 0.003,
  "timeout_seconds": 60,
  "estimated_compute_ms": 150
}
```

**Ejemplo 2: Verificación de Edad (Nivel 3)**
```bash
curl -X POST http://localhost:8080/api/estimate-cost \
  -H "Content-Type: application/json" \
  -d '{
    "operation": "threshold",
    "operation_value": 18,
    "required_provers": 3
  }'
```

**Response:**
```json
{
  "operation": "Threshold",
  "complexity_tier": 3,
  "min_payment_lamports": 5000000,
  "min_payment_sol": 0.005,
  "total_min_payment_lamports": 15000000,
  "total_min_payment_sol": 0.015,
  "timeout_seconds": 300,
  "estimated_compute_ms": 300
}
```

**Ejemplo 3: Suma de Censo (Nivel 2 - Escala con Conteo)**
```bash
curl -X POST http://localhost:8080/api/estimate-cost \
  -H "Content-Type: application/json" \
  -d '{
    "operation": "sum",
    "expected_count": 10000,
    "required_provers": 3
  }'
```

**Response:**
```json
{
  "operation": "Sum",
  "complexity_tier": 2,
  "min_payment_lamports": 1001000000,
  "min_payment_sol": 1.001,
  "total_min_payment_lamports": 3003000000,
  "total_min_payment_sol": 3.003,
  "timeout_seconds": 20060,
  "estimated_compute_ms": 500150
}
```

**Ejemplo 4: Histograma de Votación (Nivel 5)**
```bash
curl -X POST http://localhost:8080/api/estimate-cost \
  -H "Content-Type: application/json" \
  -d '{
    "operation": "histogram",
    "bins": 5,
    "required_provers": 3
  }'
```

**Response:**
```json
{
  "operation": "Histogram",
  "complexity_tier": 5,
  "min_payment_lamports": 324193547,
  "min_payment_sol": 0.324193547,
  "total_min_payment_lamports": 972580641,
  "total_min_payment_sol": 0.972580641,
  "timeout_seconds": 1100,
  "estimated_compute_ms": 15500
}
```

**Notas de Integración:**

1. **Siempre estimar antes de crear el job** para prevenir rechazos on-chain
2. **Cachear estimaciones** por 1 minuto para reducir llamadas a la API
3. **Manejar cambios de parámetros** - re-estimar cuando el usuario modifique provers/bins/counts
4. **Mostrar advertencias** para operaciones de Nivel 5 (> 0.5 SOL)
5. **Validar entradas** antes de enviar el request (nombre de operación, rangos de valores)

**Integración TypeScript:**
```typescript
async function estimateAndCreateJob(params: JobParams) {
  // 1. Estimate cost
  const estimate = await fetch('/api/estimate-cost', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      operation: params.operation,
      operation_value: params.value,
      bins: params.bins,
      required_provers: params.provers,
    }),
  }).then(r => r.json());

  // 2. Warn user if expensive
  if (estimate.total_min_payment_sol > 0.5) {
    if (!confirm(`This will cost ${estimate.total_min_payment_sol} SOL. Continue?`)) {
      return;
    }
  }

  // 3. Create job with validated price
  await createJob({
    ...params,
    price_lamports: estimate.total_min_payment_lamports,
    timeout: estimate.timeout_seconds,
  });
}
```

---

### POST /api/price-recommendation

**Descripción:** Obtiene recomendación de precio con precios mínimo, recomendado y máximo para operaciones FHE. Este endpoint está diseñado para **integración con UI** (sliders de precio, indicadores de aceptación) mientras que `/api/estimate-cost` proporciona el costo mínimo teórico.

**Diferencias Clave con `/api/estimate-cost`:**
- **estimate-cost**: Devuelve precio mínimo teórico (precio piso)
- **price-recommendation**: Devuelve un rango (min/recomendado/max) basado en economía de provers y tasas de aceptación esperadas

**Request Body:**
```json
{
  "operation": "histogram",
  "operation_value": 0,
  "expected_count": null,
  "bins": 10,
  "required_provers": 3
}
```

**Request Schema:**
```typescript
interface PriceRecommendationRequest {
  operation: string;           // FHE operation name
  operation_value?: number;    // Constant value (for Add, Multiply, Threshold)
  expected_count?: number;     // Item count (for Sum, Average, CountIf)
  bins?: number;               // Bin count (for Histogram)
  required_provers: number;    // 2-10 provers
}
```

**Response (Success):**
```json
{
  "operation": "Histogram",
  "complexity_tier": 5,
  "required_provers": 3,
  "min_price_lamports": 972580641,
  "min_price_sol": 0.972580641,
  "recommended_price_lamports": 1749444000,
  "recommended_price_sol": 1.749444,
  "max_suggested_lamports": 3498888000,
  "max_suggested_sol": 3.498888,
  "acceptance_at_min": "low",
  "acceptance_at_recommended": "high",
  "slider_min": 972580641,
  "slider_max": 3498888000,
  "slider_recommended": 1749444000,
  "slider_step": 1000000,
  "estimated_time_seconds": 1600,
  "prover_overhead_multiplier": 1.5,
  "prover_min_roi_percent": 20.0
}
```

**Response Schema:**
```typescript
interface PriceRecommendationResponse {
  operation: string;
  complexity_tier: number;              // 1-5 tier
  required_provers: number;

  // Price levels (total for all provers)
  min_price_lamports: number;           // Theoretical minimum
  min_price_sol: number;
  recommended_price_lamports: number;   // ~95% prover acceptance
  recommended_price_sol: number;
  max_suggested_lamports: number;       // Premium pricing
  max_suggested_sol: number;

  // Acceptance indicators
  acceptance_at_min: string;            // "low" (~20%)
  acceptance_at_recommended: string;    // "high" (~95%)

  // UI slider configuration
  slider_min: number;                   // Minimum value
  slider_max: number;                   // Maximum value
  slider_recommended: number;           // Default/recommended position
  slider_step: number;                  // Step increment (lamports)

  // Additional info
  estimated_time_seconds: number;
  prover_overhead_multiplier: number;   // 1.5 = 50% overhead
  prover_min_roi_percent: number;       // 20.0 = 20% ROI
}
```

**Fórmula de Precios:**

El endpoint utiliza la economía de provers para calcular precios realistas:

| Nivel de Precio | Fórmula | Aceptación Esperada | Caso de Uso |
|-----------------|---------|---------------------|-------------|
| **Mínimo** | Costo base del nivel de operación | ~20% (bajo) | Jobs económicos, puede esperar más tiempo |
| **Recomendado** | Base × 1.5 × 1.2 = Base × 1.8 | ~95% (alto) | Jobs estándar, ejecución rápida |
| **Máximo** | Recomendado × 2 | ~99% (muy alto) | Jobs prioritarios, procesamiento inmediato |

**Economía de Provers:**
- **Multiplicador de Overhead**: 1.5 (cubre 50% de costos operacionales)
- **ROI Mínimo**: 20% (provers esperan al menos 20% de margen de ganancia)
- **Fórmula**: `recomendado = costo_base × 1.5 × (1 + 0.20) = costo_base × 1.8`

**Códigos de Estado:**
- `200 OK` - Recomendación de precio devuelta
- `400 Bad Request` - Operación o parámetros inválidos
- `500 Internal Server Error` - Error de cálculo

**Ejemplo 1: Suma Simple**
```bash
curl -X POST http://localhost:8080/api/price-recommendation \
  -H "Content-Type: application/json" \
  -d '{
    "operation": "add",
    "operation_value": 5,
    "required_provers": 3
  }'
```

**Response:**
```json
{
  "operation": "Add",
  "complexity_tier": 1,
  "required_provers": 3,
  "min_price_lamports": 3000000,
  "min_price_sol": 0.003,
  "recommended_price_lamports": 5400000,
  "recommended_price_sol": 0.0054,
  "max_suggested_lamports": 10800000,
  "max_suggested_sol": 0.0108,
  "acceptance_at_min": "low",
  "acceptance_at_recommended": "high",
  "slider_min": 3000000,
  "slider_max": 10800000,
  "slider_recommended": 5400000,
  "slider_step": 100000,
  "estimated_time_seconds": 60,
  "prover_overhead_multiplier": 1.5,
  "prover_min_roi_percent": 20.0
}
```

**Ejemplo 2: Histograma con 10 Bins**
```bash
curl -X POST http://localhost:8080/api/price-recommendation \
  -H "Content-Type: application/json" \
  -d '{
    "operation": "histogram",
    "bins": 10,
    "required_provers": 3
  }'
```

**Integración con UI Slider:**
```typescript
async function setupPriceSlider(operation: string, provers: number) {
  // Fetch price recommendation
  const rec = await fetch('/api/price-recommendation', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      operation,
      required_provers: provers,
      bins: 10,
    }),
  }).then(r => r.json());

  // Configure slider
  const slider = document.getElementById('price-slider');
  slider.min = rec.slider_min;
  slider.max = rec.slider_max;
  slider.value = rec.slider_recommended;
  slider.step = rec.slider_step;

  // Show acceptance indicator
  updateAcceptanceIndicator(slider.value, rec);
}

function updateAcceptanceIndicator(price: number, rec: PriceRecommendationResponse) {
  let indicator = '';
  if (price < rec.slider_recommended * 0.8) {
    indicator = 'Low acceptance (~20%)';
  } else if (price >= rec.slider_recommended) {
    indicator = 'High acceptance (~95%)';
  } else {
    indicator = 'Medium acceptance (~50-70%)';
  }
  document.getElementById('acceptance').textContent = indicator;
}
```

**Cuándo Usar:**
- **Sliders de Precio UI**: Usar este endpoint para configurar valores min/max/default
- **Indicadores de Aceptación**: Mostrar a los usuarios tasas de aceptación de provers esperadas
- **Estimación de Costos**: Para validación final, usar `/api/estimate-cost` para asegurar que se cumplan los requisitos mínimos
- **Precios Dinámicos**: Actualizar recomendaciones cuando el usuario cambie el número de provers o parámetros de operación

---

## Endpoints de Almacenamiento de Datos

### POST /witness

**Descripción:** Sube datos de witness cifrados antes de crear un job. Los datos witness se almacenan en la base de datos y se referencian por su hash de commitment Blake2s256. Los creadores de jobs deben subir datos witness antes de crear jobs que lo requieran.

**Request:** Raw bytes (application/octet-stream)

**Content-Type:** `application/octet-stream`

**Response (Success):**
```json
{
  "commitment": "a3f5d8e2c1b4..."
}
```

**Response Schema:**
```typescript
interface WitnessUploadResponse {
  commitment: string;  // Hex-encoded Blake2s256 hash
}
```

**Códigos de Estado:**
- `200 OK` - Witness subido exitosamente
- `400 Bad Request` - Datos witness vacíos
- `500 Internal Server Error` - Fallo de almacenamiento

**Ejemplo (Usando curl):**
```bash
# Upload witness from file
curl -X POST http://localhost:8080/witness \
  -H "Content-Type: application/octet-stream" \
  --data-binary "@witness.bin"
```

**Response:**
```json
{
  "commitment": "a3f5d8e2c1b4567890abcdef1234567890abcdef1234567890abcdef12345678"
}
```

**Ejemplo de Integración:**
```typescript
async function uploadWitness(witnessData: Uint8Array): Promise<string> {
  const response = await fetch('/witness', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/octet-stream',
    },
    body: witnessData,
  });

  if (!response.ok) {
    throw new Error(`Failed to upload witness: ${response.status}`);
  }

  const { commitment } = await response.json();
  console.log('Witness uploaded, commitment:', commitment);
  return commitment;
}

// Use the commitment when creating a job
const commitment = await uploadWitness(myWitnessData);
// Then reference this commitment in your job creation
```

**Notas:**
- El commitment es un hash Blake2s256 de los datos witness
- Este hash debe coincidir con el commitment almacenado on-chain en el job
- Los provers usarán el commitment para descargar los datos witness vía GET /witness/{commitment}
- Los datos witness se almacenan como bytes raw (no codificados en base64)

---

### GET /witness/{commitment}

**Descripción:** Descarga datos de witness cifrados por su hash de commitment. Usado por provers para recuperar datos witness necesarios para la computación FHE.

**Path Parameters:**
- `commitment` (string, required) - Hash Blake2s256 codificado en hex del response de subida

**Response (Success):** Raw bytes (application/octet-stream)

**Content-Type:** `application/octet-stream`

**Códigos de Estado:**
- `200 OK` - Datos witness devueltos
- `404 Not Found` - Commitment no encontrado
- `500 Internal Server Error` - Error de base de datos

**Ejemplo:**
```bash
# Download witness to file
curl -X GET http://localhost:8080/witness/a3f5d8e2c1b4567890abcdef1234567890abcdef1234567890abcdef12345678 \
  --output witness.bin
```

**Ejemplo de Integración:**
```typescript
async function downloadWitness(commitment: string): Promise<Uint8Array> {
  const response = await fetch(`/witness/${commitment}`);

  if (!response.ok) {
    if (response.status === 404) {
      throw new Error('Witness not found');
    }
    throw new Error(`Failed to download witness: ${response.status}`);
  }

  const arrayBuffer = await response.arrayBuffer();
  return new Uint8Array(arrayBuffer);
}

// Prover downloads witness before computation
const witnessData = await downloadWitness(job.witness_commitment);
const result = await computeFheOperation(witnessData, job.encrypted_data);
```

---

### POST /fhe-result

**Descripción:** Sube resultado de computación FHE. Usado por provers para enviar sus resultados de computación cifrados después de procesar un job. El resultado se almacena y se referencia por su hash de commitment Blake2s256.

**Request:** Raw bytes (application/octet-stream)

**Content-Type:** `application/octet-stream`

**Response (Success):**
```json
{
  "commitment": "b7e9c3f1d2a5..."
}
```

**Response Schema:**
```typescript
interface FheResultUploadResponse {
  commitment: string;  // Hex-encoded Blake2s256 hash
}
```

**Códigos de Estado:**
- `200 OK` - Resultado FHE subido exitosamente
- `400 Bad Request` - Datos de resultado vacíos
- `500 Internal Server Error` - Fallo de almacenamiento

**Ejemplo (Usando curl):**
```bash
# Upload FHE result from file
curl -X POST http://localhost:8080/fhe-result \
  -H "Content-Type: application/octet-stream" \
  --data-binary "@fhe_result.bin"
```

**Response:**
```json
{
  "commitment": "b7e9c3f1d2a5678901bcdef234567890abcdef1234567890abcdef123456789"
}
```

**Ejemplo de Integración:**
```typescript
async function uploadFheResult(resultData: Uint8Array): Promise<string> {
  const response = await fetch('/fhe-result', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/octet-stream',
    },
    body: resultData,
  });

  if (!response.ok) {
    throw new Error(`Failed to upload FHE result: ${response.status}`);
  }

  const { commitment } = await response.json();
  console.log('FHE result uploaded, commitment:', commitment);
  return commitment;
}

// Prover uploads result after computation
const resultCommitment = await uploadFheResult(computedResult);
// Then submit this commitment on-chain to complete the job
```

**Notas:**
- El commitment es un hash Blake2s256 del resultado FHE cifrado
- Los provers deben enviar este commitment on-chain para probar su trabajo
- El commitment vincula los datos off-chain a la transacción on-chain
- Los creadores de jobs pueden descargar el resultado usando GET /fhe-result/{commitment}

---

### GET /fhe-result/{commitment}

**Descripción:** Descarga resultado de computación FHE por su hash de commitment. Usado por creadores de jobs para recuperar los resultados cifrados después de que los provers hayan completado la computación.

**Path Parameters:**
- `commitment` (string, required) - Hash Blake2s256 codificado en hex del response de subida

**Response (Success):** Raw bytes (application/octet-stream)

**Content-Type:** `application/octet-stream`

**Códigos de Estado:**
- `200 OK` - Resultado FHE devuelto
- `404 Not Found` - Commitment no encontrado
- `500 Internal Server Error` - Error de base de datos

**Ejemplo:**
```bash
# Download FHE result to file
curl -X GET http://localhost:8080/fhe-result/b7e9c3f1d2a5678901bcdef234567890abcdef1234567890abcdef123456789 \
  --output fhe_result.bin
```

**Ejemplo de Integración:**
```typescript
async function downloadFheResult(commitment: string): Promise<Uint8Array> {
  const response = await fetch(`/fhe-result/${commitment}`);

  if (!response.ok) {
    if (response.status === 404) {
      throw new Error('FHE result not found');
    }
    throw new Error(`Failed to download result: ${response.status}`);
  }

  const arrayBuffer = await response.arrayBuffer();
  return new Uint8Array(arrayBuffer);
}

// Job creator downloads result after consensus is reached
const encryptedResult = await downloadFheResult(consensusCommitment);
const decryptedResult = await decryptFheResult(encryptedResult, clientKey);
console.log('Final result:', decryptedResult);
```

**Ejemplo de Flujo Completo:**
```typescript
// === PROVER WORKFLOW ===

// 1. Prover claims a job and gets compute data
const job = await fetch(`/api/jobs/${jobId}/compute-data`).then(r => r.json());

// 2. Download witness if needed
const witness = await downloadWitness(job.witness_commitment);

// 3. Perform FHE computation
const result = await performFheComputation(
  job.encrypted_data,
  job.server_key,
  witness,
  job.operation
);

// 4. Upload encrypted result
const resultCommitment = await uploadFheResult(result);

// 5. Submit commitment on-chain to complete job
await submitProofOnChain(jobId, resultCommitment);

// === JOB CREATOR WORKFLOW ===

// 1. Upload witness before creating job
const witnessCommitment = await uploadWitness(witnessData);

// 2. Create job with witness commitment
await createJob({
  witness_commitment: witnessCommitment,
  // ... other params
});

// 3. Wait for consensus
await waitForConsensus(jobId);

// 4. Download final result
const consensusCommitment = await getConsensusCommitment(jobId);
const encryptedResult = await downloadFheResult(consensusCommitment);

// 5. Decrypt result with private key
const finalResult = decryptFheResult(encryptedResult, clientKey);
```

---

## Manejo de Errores

### Formato de Response de Error

Todos los responses de error siguen este formato:

```json
{
  "error": "Human-readable error message"
}
```

### Códigos de Error Comunes

| Código de Estado | Significado | Causas Comunes |
|------------------|-------------|----------------|
| 400 | Bad Request | Parámetros inválidos, fallo de validación, job con precio muy bajo |
| 404 | Not Found | Job ID no existe |
| 500 | Internal Server Error | Error de base de datos, fallo de serialización |
| 503 | Service Unavailable | Conexión de base de datos perdida, nodo RPC caído |

### Ejemplos de Responses de Error

**Error de Validación:**
```json
{
  "error": "Validation failed: Invalid signature"
}
```

**Precio Muy Bajo:**
```json
{
  "error": "Price too low for operation 'Histogram' (tier 5): 100000000 < 972580641 lamports"
}
```

**Operación Desconocida:**
```json
{
  "error": "Unknown operation: substract"
}
```

**Job No Activo:**
```json
{
  "error": "Job not ready, status: pending_tx"
}
```

---

## Límites de Tasa

**Actual:** Sin límites de tasa aplicados

**Futuro:** Límites de tasa planeados:
- 100 requests/minuto por IP (estimación de costos)
- 10 creaciones de jobs/minuto por creator pubkey
- 1000 verificaciones de estado/minuto por IP

Se agregarán headers de límite de tasa:
```
X-RateLimit-Limit: 100
X-RateLimit-Remaining: 95
X-RateLimit-Reset: 1732213200
```

---

## Ejemplos

### Flujo Completo de Creación de Job

```bash
#!/bin/bash

# Step 1: Estimate cost
ESTIMATE=$(curl -s -X POST http://localhost:8080/api/estimate-cost \
  -H "Content-Type: application/json" \
  -d '{
    "operation": "add",
    "operation_value": 5,
    "required_provers": 3
  }')

echo "Cost estimate: $ESTIMATE"

PRICE=$(echo $ESTIMATE | jq -r '.total_min_payment_lamports')
TIMEOUT=$(echo $ESTIMATE | jq -r '.timeout_seconds')

# Step 2: Validate and build transaction
VALIDATE=$(curl -s -X POST http://localhost:8080/api/jobs/validate-and-build \
  -H "Content-Type: application/json" \
  -d '{
    "creator_pubkey": "11111111111111111111111111111111",
    "encrypted_data": "ZW5jcnlwdGVk",
    "server_key": "c2VydmVy",
    "operation": "add",
    "operation_value": 5,
    "price_lamports": '$PRICE',
    "required_provers": 3,
    "consensus_threshold": 2,
    "payment_method": "SOL",
    "signature": "abcd1234"
  }')

echo "Validation response: $VALIDATE"

JOB_ID=$(echo $VALIDATE | jq -r '.job_id')
TX_BASE64=$(echo $VALIDATE | jq -r '.transaction')

# Step 3: Sign transaction (using Solana CLI)
echo $TX_BASE64 | base64 -d > unsigned_tx.bin
solana sign-transaction unsigned_tx.bin > signed_tx.sig

# Step 4: Submit transaction
# (In real scenario, use solana-cli or web3.js)

# Step 5: Confirm transaction
curl -X POST http://localhost:8080/api/jobs/$JOB_ID/confirm \
  -H "Content-Type: application/json" \
  -d '{"signature": "5J7Zx..."}'

# Step 6: Check status
curl -X GET http://localhost:8080/api/jobs/$JOB_ID/status

# Step 7: Provers fetch compute data
curl -X GET http://localhost:8080/api/jobs/$JOB_ID/compute-data
```

### Ejemplo de Cliente Python

```python
import requests

class ZyberLinkClient:
    def __init__(self, base_url="http://localhost:8080"):
        self.base_url = base_url

    def estimate_cost(self, operation, **params):
        """Estimate cost for FHE operation."""
        response = requests.post(
            f"{self.base_url}/api/estimate-cost",
            json={"operation": operation, **params}
        )
        response.raise_for_status()
        return response.json()

    def create_job(self, job_data):
        """Validate and build job transaction."""
        response = requests.post(
            f"{self.base_url}/api/jobs/validate-and-build",
            json=job_data
        )
        response.raise_for_status()
        return response.json()

    def confirm_job(self, job_id, signature):
        """Confirm job transaction."""
        response = requests.post(
            f"{self.base_url}/api/jobs/{job_id}/confirm",
            json={"signature": signature}
        )
        response.raise_for_status()
        return response.json()

    def get_job_status(self, job_id):
        """Get job status."""
        response = requests.get(
            f"{self.base_url}/api/jobs/{job_id}/status"
        )
        response.raise_for_status()
        return response.json()

# Usage
client = ZyberLinkClient()

# Estimate
estimate = client.estimate_cost("add", operation_value=5, required_provers=3)
print(f"Cost: {estimate['total_min_payment_sol']} SOL")

# Create job
job = client.create_job({
    "creator_pubkey": "11111111111111111111111111111111",
    "encrypted_data": "ZW5jcnlwdGVk",
    "server_key": "c2VydmVy",
    "operation": "add",
    "operation_value": 5,
    "price_lamports": estimate["total_min_payment_lamports"],
    "required_provers": 3,
    "consensus_threshold": 2,
    "payment_method": "SOL",
    "signature": "abcd1234"
})

print(f"Job ID: {job['job_id']}")
```

---

## Documentación Relacionada

- **[Visión General de Precios Dinámicos](./modelo-precios-dinamicos.md)** - Visión general del sistema y niveles de precios
- **[Implementación Técnica](./precios-dinamicos-tecnico.md)** - Análisis profundo del algoritmo
- **[Guía de Uso](./precios-dinamicos-uso.md)** - Ejemplos de integración
- **[Guía de Pagos wZEC](./wzec-api.md)** - Documentación de pagos con tokens

---

## Changelog

### Version 1.1.0 (2025-11-27)
- Agregado endpoint `/api/price-recommendation` para integración con slider UI
- Agregados endpoints de almacenamiento witness (`POST /witness`, `GET /witness/{commitment}`)
- Agregados endpoints de almacenamiento de resultados FHE (`POST /fhe-result`, `GET /fhe-result/{commitment}`)
- Agregada sección de Endpoints de Almacenamiento de Datos
- Mejorada documentación de precios con economía de provers
- Agregados ejemplos de flujos completos para provers y creadores de jobs

### Version 1.0.0 (2025-11-21)
- Documentación inicial de API
- Agregado endpoint `/api/estimate-cost`
- Agregados endpoints de gestión de jobs
- Agregada documentación de manejo de errores
- Agregados ejemplos completos

---

**Preguntas o Problemas?**
- GitHub: [github.com/yourorg/zyberlink](https://github.com/yourorg/zyberlink)
- Discord: [discord.gg/zyberlink](https://discord.gg/zyberlink)

---

**Última Actualización:** 2025-11-27
**Versión:** 1.1.0
**Mantenido Por:** ZyberLink Core Team
