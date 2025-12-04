# Referencia API de Pagos wZEC

Referencia API completa para integrar pagos wZEC (Wrapped Zcash) en el marketplace ZyberLink.

## Tabla de Contenidos

- [Resumen](#resumen)
- [Autenticación](#autenticación)
- [Endpoints](#endpoints)
- [Tipos de Datos](#tipos-de-datos)
- [Códigos de Error](#códigos-de-error)
- [Límites de Tasa](#límites-de-tasa)
- [Ejemplos](#ejemplos)
- [SDKs](#sdks)

## Resumen

### URL Base

```
Producción:  https://api.zyberlink.io
Testnet:     https://api.testnet.zyberlink.io
Local:       http://localhost:3001
```

### Versión API

```
Versión: 1.0.0
Lanzamiento: 2025-11-20
```

### Tipo de Contenido

Todos los requests y responses usan `application/json`.

```http
Content-Type: application/json
Accept: application/json
```

## Autenticación

### Autenticación Basada en Firmas

La API usa autenticación basada en firmas Ed25519 en lugar de API keys:

1. El cliente firma un mensaje con su wallet Solana
2. El servidor verifica que la firma coincida con la clave pública del creator
3. El mensaje incluye timestamp y nonce para protección contra replay

**Formato del Mensaje:**
```
create_job:{job_id}:{timestamp}:{nonce}
```

**Ejemplo:**
```
create_job:12345:1732104000:e2e_wzec_5432_1732104000
```

**Formato de Firma:**
- Algoritmo: Ed25519
- Codificación: base58 (estándar Solana)
- Longitud: 88 caracteres

**Protección Anti-Replay:**
- El timestamp debe estar dentro de 5 minutos del tiempo del servidor
- El nonce debe ser único (almacenado en base de datos)
- Formato de nonce: cualquier string, recomendado: `{prefix}_{job_id}_{timestamp}`

## Endpoints

### POST /api/jobs/validate-and-build

Crea un nuevo job de computación FHE con pago en SOL o wZEC.

#### Request

```http
POST /api/jobs/validate-and-build HTTP/1.1
Host: api.zyberlink.io
Content-Type: application/json

{
  "creator_pubkey": "string",
  "encrypted_data": "string",
  "server_key": "string",
  "message": "string",
  "signature": "string",
  "nonce": "string",
  "operation": "string",
  "operation_value": number,
  "price_lamports": number,
  "required_provers": number,
  "consensus_threshold": number,
  "payment_method": "string"
}
```

#### Campos del Request

| Campo | Tipo | Requerido | Descripción |
|-------|------|-----------|-------------|
| `creator_pubkey` | string | Sí | Clave pública Solana (base58, 32-44 chars) |
| `encrypted_data` | string | Sí | Entrada encriptada codificada en Base64 (max 1 MB) |
| `server_key` | string | Sí | Server key TFHE codificada en Base64 (max 200 MB) |
| `message` | string | Sí | Mensaje firmado: `create_job:{job_id}:{timestamp}:{nonce}` |
| `signature` | string | Sí | Firma Ed25519 (base58, 88 chars) |
| `nonce` | string | Sí | Nonce anti-replay (único por request) |
| `operation` | string | Sí | Tipo de operación: `add`, `multiply`, `subtract` |
| `operation_value` | number | Sí | Parámetro de operación (u8: 1-255) |
| `price_lamports` | number | Sí | Monto de pago (u64: lamports SOL o zatoshis wZEC) |
| `required_provers` | number | Sí | Número de provers necesarios (u8: 1-255) |
| `consensus_threshold` | number | Sí | Mínimo de resultados coincidentes (u8: 1-255, <= required_provers) |
| `payment_method` | string | No | Tipo de pago: `SOL` o `wZEC` (default: `SOL`) |

#### Validación de Campos

**creator_pubkey:**
```regex
^[1-9A-HJ-NP-Za-km-z]{32,44}$
```
- Debe ser una clave pública base58 válida de Solana
- Ejemplo: `HxL4npd9BjVTigRRJJp9s7FPNjzZ3R4x7X7L8qJJ7Zf`

**encrypted_data:**
```
- Codificación: Base64
- Tamaño máx: 1 MB (1,048,576 bytes)
- Debe ser datos encriptados TFHE válidos
```

**server_key:**
```
- Codificación: Base64
- Tamaño máx: 200 MB (209,715,200 bytes)
- Debe ser ServerKey TFHE válida
- Tamaño típico: ~156 MB
```

**message:**
```
Formato: create_job:{job_id}:{timestamp}:{nonce}
  - job_id: entero positivo
  - timestamp: timestamp Unix (segundos)
  - nonce: cualquier string
Ejemplo: create_job:12345:1732104000:abc123
```

**signature:**
```
- Algoritmo: Ed25519
- Codificación: base58
- Longitud: exactamente 88 caracteres
- Debe ser firma válida del message
```

**nonce:**
```
- Longitud: 1-256 caracteres
- Debe ser único entre todos los requests
- Formato recomendado: {prefix}_{job_id}_{timestamp}
- Ejemplo: e2e_wzec_12345_1732104000
```

**operation:**
```
Valores válidos:
  - "add"       - Suma FHE
  - "multiply"  - Multiplicación FHE
  - "subtract"  - Resta FHE
```

**operation_value:**
```
- Tipo: u8
- Rango: 1-255
- La interpretación depende del tipo de operación
```

**price_lamports:**
```
- Tipo: u64
- Rango: 0-18,446,744,073,709,551,615
- Unidad: lamports (SOL) o zatoshis (wZEC)
- 1 SOL = 1,000,000,000 lamports
- 1 wZEC = 100,000,000 zatoshis
```

**required_provers:**
```
- Tipo: u8
- Rango: 1-255
- Típico: 3-5
- Debe ser >= consensus_threshold
```

**consensus_threshold:**
```
- Tipo: u8
- Rango: 1-255
- Debe ser <= required_provers
- Típico: 2 (para consenso 2-de-3)
```

**payment_method:**
```
Valores válidos:
  - "SOL"  - Pago nativo Solana (default)
  - "wZEC" - Pago Wrapped Zcash
Sensible a mayúsculas, requiere mayúsculas
```

#### Response

**Éxito (200 OK):**

```json
{
  "job_id": 12345,
  "transaction": "transacción_sin_firmar_serializada_base64",
  "status": "pending_signature"
}
```

**Campos del Response:**

| Campo | Tipo | Descripción |
|-------|------|-------------|
| `job_id` | number | Identificador único del job (i64) |
| `transaction` | string | Transacción Solana sin firmar serializada en Base64 |
| `status` | string | Estado del job: `pending_signature` |

**Error (400/500):**

```json
{
  "error": "Mensaje de error legible"
}
```

#### Ejemplo Request (Pago SOL)

```bash
curl -X POST https://api.zyberlink.io/api/jobs/validate-and-build \
  -H "Content-Type: application/json" \
  -d '{
    "creator_pubkey": "HxL4npd9BjVTigRRJJp9s7FPNjzZ3R4x7X7L8qJJ7Zf",
    "encrypted_data": "AQIDBA==",
    "server_key": "BQYHCA==",
    "message": "create_job:12345:1732104000:nonce123",
    "signature": "5J7XqG3K8H9L2M4N6P1Q3R5S7T9U2V4W6X8Y1Z3A5B7C9D2E4F6G8H1J3K5L7M9N2P4Q6R8S1T3U5V7W9X2Y4Z6",
    "nonce": "nonce123",
    "operation": "add",
    "operation_value": 5,
    "price_lamports": 1000000000,
    "required_provers": 3,
    "consensus_threshold": 2,
    "payment_method": "SOL"
  }'
```

#### Ejemplo Request (Pago wZEC)

```bash
curl -X POST https://api.zyberlink.io/api/jobs/validate-and-build \
  -H "Content-Type: application/json" \
  -d '{
    "creator_pubkey": "HxL4npd9BjVTigRRJJp9s7FPNjzZ3R4x7X7L8qJJ7Zf",
    "encrypted_data": "AQIDBA==",
    "server_key": "BQYHCA==",
    "message": "create_job:12346:1732104100:nonce456",
    "signature": "2A4B6C8D1E3F5G7H9J2K4L6M8N1P3Q5R7S9T2U4V6W8X1Y3Z5A7B9C2D4E6F8G1H3J5K7L9M2N4P6Q8R1S3T5U7V9W",
    "nonce": "nonce456",
    "operation": "multiply",
    "operation_value": 3,
    "price_lamports": 500000000,
    "required_provers": 3,
    "consensus_threshold": 2,
    "payment_method": "wZEC"
  }'
```

#### Ejemplo Response

```json
{
  "job_id": 12345,
  "transaction": "AQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAACAAQAHDQoFBgcICQsMDQ4PEBESEw==",
  "status": "pending_signature"
}
```

### POST /api/jobs/:id/confirm

Confirma creación de job después de que la transacción es firmada y enviada a la red Solana.

#### Request

```http
POST /api/jobs/12345/confirm HTTP/1.1
Host: api.zyberlink.io
Content-Type: application/json

{
  "signature": "string"
}
```

#### Campos del Request

| Campo | Tipo | Requerido | Descripción |
|-------|------|-----------|-------------|
| `signature` | string | Sí | Firma de transacción Solana (base58) |

#### Response

**Éxito (200 OK):**

```json
{
  "status": "confirmed",
  "job_id": 12345,
  "signature": "5J7XqG3K8H9L2M4N6P1Q3R5S7T9U2V4W6X8Y1Z3A5B7C9D2E4F6G8H1J3K5L7M9N2P4Q6R8S1T3U5V7W9X2Y4Z6"
}
```

**Error (400):**

```json
{
  "error": "Formato de firma inválido"
}
```

**Error (404):**

```json
{
  "error": "Job no encontrado"
}
```

### GET /api/jobs/:id

Recupera estado y detalles del job.

#### Request

```http
GET /api/jobs/12345 HTTP/1.1
Host: api.zyberlink.io
```

#### Response

**Éxito (200 OK):**

```json
{
  "job_id": 12345,
  "creator": "HxL4npd9BjVTigRRJJp9s7FPNjzZ3R4x7X7L8qJJ7Zf",
  "status": "completed",
  "payment_method": "wZEC",
  "payment_token_mint": "7gGGpHxbEuY8i76PwxjgGB1ZX2Nhmfq65N6YNHtrv7Zf",
  "price_lamports": 500000000,
  "operation": "add",
  "operation_value": 5,
  "required_provers": 3,
  "consensus_threshold": 2,
  "provers_claimed": 3,
  "results_submitted": 3,
  "consensus_reached": true,
  "created_at": "2025-11-20T10:30:00Z",
  "completed_at": "2025-11-20T10:30:45Z"
}
```

**Error (404):**

```json
{
  "error": "Job no encontrado"
}
```

## Tipos de Datos

### PaymentMethod

```typescript
type PaymentMethod = "SOL" | "wZEC";
```

### CircuitType

```typescript
type CircuitType = "add" | "multiply" | "subtract";
```

### JobStatus

```typescript
type JobStatus =
  | "pending_signature"  // Esperando firma del usuario
  | "pending"            // Transacción confirmada, esperando provers
  | "claimed"            // Provers han reclamado el job
  | "computing"          // Computación en progreso
  | "consensus"          // Verificando consenso
  | "completed"          // Job completado exitosamente
  | "failed"             // Job falló (timeout o falla de consenso)
  | "cancelled";         // Job cancelado por creator
```

### Estructura de Transacción

#### Transacción Pago SOL (5 Cuentas)

```
Cuentas:
  0. Creator (firmante, escribible) - Paga SOL
  1. Job PDA (escribible) - Cuenta de estado del job
  2. Config PDA (solo lectura) - Configuración marketplace
  3. Escrow PDA (escribible) - Escrow SOL
  4. System Program (solo lectura) - Transferencias SOL nativas

Instrucción: CreateJob
Data: CircuitType, WitnessCommitment, WitnessSize, PriceLamports, TimeoutSeconds, FheConfig
```

#### Transacción Pago wZEC (9 Cuentas)

```
Cuentas:
  0. Creator (firmante, escribible) - Firma transacción
  1. Job PDA (escribible) - Cuenta de estado del job
  2. Config PDA (escribible) - Configuración marketplace
  3. Token Escrow PDA (escribible) - Escrow wZEC (auto-creado)
  4. Creator Token Account (escribible) - Balance wZEC del creator
  5. Token Mint (solo lectura) - Dirección mint wZEC
  6. System Program (solo lectura) - Creación de cuentas
  7. Token Program (solo lectura) - Transferencias SPL Token
  8. Rent Sysvar (solo lectura) - Cálculos de rent

Instrucción: CreateJobWithToken
Data: CircuitType, WitnessCommitment, WitnessSize, PriceLamports, TimeoutSeconds, FheConfig
```

### Constantes

```typescript
// Mint Token wZEC
const WZEC_MINT = "7gGGpHxbEuY8i76PwxjgGB1ZX2Nhmfq65N6YNHtrv7Zf";

// Program ID
const PROGRAM_ID = "7CZmQAqJyDJqjLeEw7Gthb7Wya3PmUUMM4FrPm2CrrqR";

// Decimales Token
const SOL_DECIMALS = 9;  // 1 SOL = 1,000,000,000 lamports
const WZEC_DECIMALS = 8; // 1 wZEC = 100,000,000 zatoshis

// Límites
const MAX_ENCRYPTED_DATA_SIZE = 1_048_576;     // 1 MB
const MAX_SERVER_KEY_SIZE = 209_715_200;       // 200 MB
const MAX_NONCE_LENGTH = 256;                  // caracteres
const SIGNATURE_EXPIRY_SECONDS = 300;          // 5 minutos
```

## Códigos de Error

### Códigos de Estado HTTP

| Código | Significado | Descripción |
|--------|-------------|-------------|
| 200 | OK | Request exitoso |
| 400 | Bad Request | Parámetros de request inválidos |
| 401 | Unauthorized | Falló verificación de firma |
| 404 | Not Found | Recurso no encontrado |
| 429 | Too Many Requests | Límite de tasa excedido |
| 500 | Internal Server Error | Error del servidor |
| 503 | Service Unavailable | Servicio temporalmente no disponible |

### Mensajes de Error

**Errores de Validación (400):**

```
"payment_method inválido: debe ser 'SOL' o 'wZEC'"
"creator_pubkey inválido: debe ser clave pública Solana válida"
"signature inválida: debe ser firma Ed25519 codificada en base58"
"encrypted_data inválido: excede tamaño máximo de 1 MB"
"server_key inválido: excede tamaño máximo de 200 MB"
"operation inválida: debe ser 'add', 'multiply', o 'subtract'"
"consensus_threshold inválido: debe ser <= required_provers"
"Nonce ya usado"
"Timestamp expirado (debe estar dentro de 5 minutos)"
```

**Errores de Autenticación (401):**

```
"Falló verificación de firma"
"Formato de firma inválido"
"Firma no coincide con creator_pubkey"
```

**Errores del Servidor (500):**

```
"Error de base de datos: falló almacenar datos del job"
"Falló construcción de transacción"
"Falló derivar cuentas PDA"
"Error RPC: falló obtener cuenta"
```

### Formato Response Error

```json
{
  "error": "Mensaje de error",
  "code": "ERROR_CODE",
  "details": {
    "field": "nombre_campo",
    "value": "valor_inválido",
    "reason": "razón específica"
  }
}
```

## Límites de Tasa

### Límites

| Endpoint | Límite | Ventana |
|----------|--------|---------|
| POST /api/jobs/validate-and-build | 10 requests | por minuto |
| POST /api/jobs/:id/confirm | 20 requests | por minuto |
| GET /api/jobs/:id | 100 requests | por minuto |

### Headers Límite de Tasa

```http
X-RateLimit-Limit: 10
X-RateLimit-Remaining: 7
X-RateLimit-Reset: 1732104060
```

### Error Límite de Tasa

**429 Too Many Requests:**

```json
{
  "error": "Límite de tasa excedido",
  "retry_after": 45
}
```

## Ejemplos

### JavaScript/TypeScript

```typescript
import { Connection, PublicKey, Transaction } from '@solana/web3.js';
import bs58 from 'bs58';

const WZEC_MINT = new PublicKey('7gGGpHxbEuY8i76PwxjgGB1ZX2Nhmfq65N6YNHtrv7Zf');
const API_URL = 'https://api.zyberlink.io';

async function createJobWithWZEC(
  wallet: any,
  encryptedData: string,
  serverKey: string,
  priceLamports: number
): Promise<{ jobId: number; signature: string }> {
  // 1. Generar nonce y mensaje
  const jobId = Math.floor(Math.random() * 1000000);
  const timestamp = Math.floor(Date.now() / 1000);
  const nonce = `wzec_${jobId}_${timestamp}`;
  const message = `create_job:${jobId}:${timestamp}:${nonce}`;

  // 2. Firmar mensaje
  const messageBytes = new TextEncoder().encode(message);
  const signatureBytes = await wallet.signMessage(messageBytes);
  const signature = bs58.encode(signatureBytes);

  // 3. Construir request
  const response = await fetch(`${API_URL}/api/jobs/validate-and-build`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      creator_pubkey: wallet.publicKey.toString(),
      encrypted_data: encryptedData,
      server_key: serverKey,
      message,
      signature,
      nonce,
      operation: 'add',
      operation_value: 5,
      price_lamports: priceLamports,
      required_provers: 3,
      consensus_threshold: 2,
      payment_method: 'wZEC'
    })
  });

  if (!response.ok) {
    const error = await response.json();
    throw new Error(error.error);
  }

  const { job_id, transaction } = await response.json();

  // 4. Firmar y enviar transacción
  const connection = new Connection('https://api.mainnet-beta.solana.com');
  const tx = Transaction.from(Buffer.from(transaction, 'base64'));
  const signedTx = await wallet.signTransaction(tx);
  const txSignature = await connection.sendRawTransaction(signedTx.serialize());

  await connection.confirmTransaction(txSignature);

  // 5. Confirmar con backend
  await fetch(`${API_URL}/api/jobs/${job_id}/confirm`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ signature: txSignature })
  });

  return { jobId: job_id, signature: txSignature };
}
```

### Python

```python
import requests
import json
import base58
from nacl.signing import SigningKey
from solders.keypair import Keypair
from solders.transaction import Transaction

WZEC_MINT = "7gGGpHxbEuY8i76PwxjgGB1ZX2Nhmfq65N6YNHtrv7Zf"
API_URL = "https://api.zyberlink.io"

def create_job_with_wzec(
    keypair: Keypair,
    encrypted_data: str,
    server_key: str,
    price_lamports: int
) -> dict:
    # 1. Generar nonce y mensaje
    import time
    import random
    job_id = random.randint(1, 1000000)
    timestamp = int(time.time())
    nonce = f"wzec_{job_id}_{timestamp}"
    message = f"create_job:{job_id}:{timestamp}:{nonce}"

    # 2. Firmar mensaje
    message_bytes = message.encode('utf-8')
    signature_bytes = keypair.sign_message(message_bytes)
    signature = base58.b58encode(signature_bytes).decode('ascii')

    # 3. Construir request
    payload = {
        "creator_pubkey": str(keypair.pubkey()),
        "encrypted_data": encrypted_data,
        "server_key": server_key,
        "message": message,
        "signature": signature,
        "nonce": nonce,
        "operation": "add",
        "operation_value": 5,
        "price_lamports": price_lamports,
        "required_provers": 3,
        "consensus_threshold": 2,
        "payment_method": "wZEC"
    }

    response = requests.post(
        f"{API_URL}/api/jobs/validate-and-build",
        json=payload
    )

    if not response.ok:
        error = response.json()
        raise Exception(error["error"])

    result = response.json()
    job_id = result["job_id"]
    transaction = result["transaction"]

    # 4. Firmar y enviar transacción
    # (Implementación depende de librería Python Solana)

    return {"job_id": job_id, "signature": "..."}
```

### cURL

```bash
#!/bin/bash

KEYPAIR_PATH="$HOME/.config/solana/id.json"
API_URL="https://api.zyberlink.io"
WZEC_MINT="7gGGpHxbEuY8i76PwxjgGB1ZX2Nhmfq65N6YNHtrv7Zf"

# 1. Obtener pubkey creator
CREATOR=$(solana address -k "$KEYPAIR_PATH")

# 2. Generar mensaje
JOB_ID=$RANDOM
TIMESTAMP=$(date +%s)
NONCE="curl_${JOB_ID}_${TIMESTAMP}"
MESSAGE="create_job:${JOB_ID}:${TIMESTAMP}:${NONCE}"

# 3. Firmar mensaje (requiere helper Python)
SIGNATURE=$(python3 sign-message.py "$KEYPAIR_PATH" "$MESSAGE")

# 4. Cargar datos TFHE
ENCRYPTED_DATA=$(cat test-keys/encrypted_data.b64)
SERVER_KEY=$(cat test-keys/server_key.b64)

# 5. Construir request JSON
REQUEST=$(cat <<EOF
{
  "creator_pubkey": "$CREATOR",
  "encrypted_data": "$ENCRYPTED_DATA",
  "server_key": "$SERVER_KEY",
  "message": "$MESSAGE",
  "signature": "$SIGNATURE",
  "nonce": "$NONCE",
  "operation": "add",
  "operation_value": 5,
  "price_lamports": 500000000,
  "required_provers": 3,
  "consensus_threshold": 2,
  "payment_method": "wZEC"
}
EOF
)

# 6. Enviar request
RESPONSE=$(curl -s -X POST \
  "$API_URL/api/jobs/validate-and-build" \
  -H "Content-Type: application/json" \
  -d "$REQUEST")

echo "$RESPONSE" | jq '.'
```

## SDKs

### SDKs Oficiales

**JavaScript/TypeScript:**
```bash
npm install @zyberlink/sdk
```

**Rust:**
```toml
[dependencies]
zyberlink-sdk = "0.1.0"
```

**Python:**
```bash
pip install zyberlink-sdk
```

### SDKs Comunitarios

- **Go**: [zyberlink-go](https://github.com/zyberlink/zyberlink-go)
- **Ruby**: [zyberlink-ruby](https://github.com/zyberlink/zyberlink-ruby)
- **Java**: [zyberlink-java](https://github.com/zyberlink/zyberlink-java)

## Changelog

### Versión 1.0.0 (2025-11-20)

**Agregado:**
- Soporte de pago wZEC vía campo `payment_method`
- Instrucción `CreateJobWithToken` para pagos con tokens SPL
- Creación automática de cuenta de tokens
- PDA escrow de tokens para pagos wZEC

**Cambiado:**
- Campo `payment_method` ahora es opcional (default "SOL")
- Estructura de transacción varía basado en método de pago

**Corregido:**
- Formato de firma aclarado: base58 (no base64)
- Creación de cuenta de tokens ahora automática

## Soporte

Soporte API:

- **Documentación**: [docs.zyberlink.io](https://docs.zyberlink.io)
- **GitHub**: [Issues](https://github.com/zyberlink/zyberlink/issues)
- **Discord**: [#api-support](https://discord.gg/zyberlink)
- **Email**: api@zyberlink.io
- **Estado**: [status.zyberlink.io](https://status.zyberlink.io)

---

**Última Actualización**: 2025-11-21
**Versión API**: 1.0.0
**Mint wZEC**: `7gGGpHxbEuY8i76PwxjgGB1ZX2Nhmfq65N6YNHtrv7Zf`
