# Documentación de la API de Integración de Pagos wZEC

## Descripción General

El backend de ZyberLink ahora soporta pagos **wZEC (Wrapped Zcash)** además de pagos nativos SOL para jobs de computación FHE. Este documento describe el contrato de API para crear jobs con pagos wZEC.

## Endpoint API

### POST /api/jobs/validate-and-build

Crea un nuevo job de computación FHE con pago en SOL o wZEC.

#### Request Schema

```json
{
  "creator_pubkey": "string",        // Solana public key (base58)
  "encrypted_data": "string",        // Base64 encoded encrypted input
  "server_key": "string",            // Base64 encoded TFHE server key
  "message": "string",               // Format: "create_job:{job_id}:{timestamp}:{nonce}"
  "signature": "string",             // Base64 encoded signature (64 bytes)
  "nonce": "string",                 // Anti-replay nonce
  "operation": "string",             // "add" | "multiply"
  "operation_value": number,         // u8 (1-255)
  "price_lamports": number,          // u64 (price in lamports for SOL, zatoshis for wZEC)
  "required_provers": number,        // u8 (number of provers needed)
  "consensus_threshold": number,     // u8 (minimum matching results)
  "payment_method": "string"         // "SOL" | "wZEC" (defaults to "SOL")
}
```

#### Response Schema

```json
{
  "job_id": number,                  // i64
  "transaction": "string",           // Base64 serialized unsigned transaction
  "status": "pending_signature"      // Job status
}
```

#### Error Response

```json
{
  "error": "string"                  // Human-readable error message
}
```

## Métodos de Pago

### SOL (Pago Nativo)

- **payment_method**: `"SOL"`
- **payment_token_mint**: No usado (auto-establecido a `null`)
- **price_lamports**: Cantidad en lamports (1 SOL = 1,000,000,000 lamports)
- **Instrucción**: Usa instrucción on-chain `CreateJob`
- **Escrow**: Cuenta escrow SOL nativa (PDA)

### wZEC (Pago con Token)

- **payment_method**: `"wZEC"`
- **payment_token_mint**: Automáticamente establecido a `sXpG9BWgA6hxz9BTVLNTqWSHpbbQKa2LqKH6qD2fCAZ`
- **price_lamports**: Cantidad en zatoshis (1 wZEC = 100,000,000 zatoshis)
- **Instrucción**: Usa instrucción on-chain `CreateJobWithToken`
- **Escrow**: Cuenta escrow de token SPL (PDA)

## Constantes

```javascript
const WZEC_MINT = "sXpG9BWgA6hxz9BTVLNTqWSHpbbQKa2LqKH6qD2fCAZ";
const PROGRAM_ID = "7CZmQAqJyDJqjLeEw7Gthb7Wya3PmUUMM4FrPm2CrrqR";
const ZATOSHIS_PER_WZEC = 100_000_000;
```

## Schema de Base de Datos

El backend almacena los siguientes campos adicionales:

```sql
ALTER TABLE temp_job_data
ADD COLUMN payment_method TEXT NOT NULL DEFAULT 'SOL',
ADD COLUMN payment_token_mint TEXT NULL;
```

- **payment_method**: `"SOL"` o `"wZEC"`
- **payment_token_mint**: Dirección del mint de token SPL (solo para wZEC)

## Ejemplos de Uso

### Creando un job con pago SOL

```javascript
const response = await fetch('http://localhost:8080/api/jobs/validate-and-build', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    creator_pubkey: "YourPublicKeyHere...",
    encrypted_data: "base64_encoded_data",
    server_key: "base64_encoded_server_key",
    message: "create_job:123:1732104000:random_nonce",
    signature: "base64_encoded_signature",
    nonce: "random_nonce",
    operation: "add",
    operation_value: 5,
    price_lamports: 1000000000,  // 1 SOL
    required_provers: 3,
    consensus_threshold: 2,
    payment_method: "SOL"  // Optional, defaults to SOL
  })
});

const result = await response.json();
console.log('Job ID:', result.job_id);
console.log('Transaction:', result.transaction);
```

### Creando un job con pago wZEC

```javascript
const response = await fetch('http://localhost:8080/api/jobs/validate-and-build', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    creator_pubkey: "YourPublicKeyHere...",
    encrypted_data: "base64_encoded_data",
    server_key: "base64_encoded_server_key",
    message: "create_job:124:1732104100:random_nonce_2",
    signature: "base64_encoded_signature",
    nonce: "random_nonce_2",
    operation: "multiply",
    operation_value: 3,
    price_lamports: 500000000,  // 5 wZEC (in zatoshis)
    required_provers: 2,
    consensus_threshold: 2,
    payment_method: "wZEC"  // NEW: Specify wZEC payment
  })
});

const result = await response.json();
console.log('Job ID:', result.job_id);
console.log('Transaction:', result.transaction);

// Transaction will include wZEC token transfer to escrow
// Frontend must sign and submit this transaction
```

## Estructura de Transacción

### Transacción de Pago SOL

Accounts (5):
1. Creator (signer, writable)
2. Job PDA (writable)
3. Config PDA (readonly)
4. Escrow PDA (writable)
5. System Program (readonly)

### Transacción de Pago wZEC

Accounts (9):
1. Creator (signer, writable)
2. Job PDA (writable)
3. Config PDA (writable)
4. Token Escrow PDA (writable)
5. Creator's Token Account (writable)
6. Token Mint (readonly)
7. System Program (readonly)
8. Token Program (readonly)
9. Rent Sysvar (readonly)

## Reglas de Validación

### Validación de Método de Pago

```rust
match payment_method {
    "SOL" => Ok(("SOL", None)),
    "wZEC" => Ok(("wZEC", Some("sXpG9BWgA6hxz9BTVLNTqWSHpbbQKa2LqKH6qD2fCAZ"))),
    _ => Err("Invalid payment_method: must be 'SOL' or 'wZEC'")
}
```

### Validaciones Adicionales

Todas las validaciones existentes todavía aplican:
- Verificación de firma
- Expiración de timestamp (5 minutos)
- Anti-replay de nonce
- Límites de tamaño de datos cifrados
- Validación de formato de server key
- Validación de operación
- Validación de configuración de consenso

## Códigos de Error

| HTTP Code | Error | Razón |
|-----------|-------|-------|
| 200 | Success | Job creado exitosamente |
| 400 | Invalid payment_method | payment_method no es "SOL" o "wZEC" |
| 400 | Validation failed | Validación de firma, nonce o datos falló |
| 500 | Database error | Fallo al almacenar datos de job |
| 500 | Transaction build failed | Fallo al construir transacción |

## Guía de Integración Frontend

### Pasos Requeridos

1. **Verificar Cuenta de Token**: Antes de crear un job wZEC, verificar que el usuario tiene una cuenta de token asociada para wZEC:
   ```javascript
   const tokenAccount = await getAssociatedTokenAddress(
     new PublicKey(WZEC_MINT),
     wallet.publicKey
   );
   ```

2. **Construir Request**: Llamar `/api/jobs/validate-and-build` con `payment_method: "wZEC"`

3. **Deserializar Transacción**: Parsear la transacción base64 devuelta por la API

4. **Firmar Transacción**: Usar wallet para firmar la transacción

5. **Enviar Transacción**: Enviar a la red Solana

6. **Confirmar Job**: Llamar `/api/jobs/{job_id}/confirm` con la firma de la transacción

### Ejemplo de Código

```typescript
import { Connection, Transaction } from '@solana/web3.js';
import { getAssociatedTokenAddress } from '@solana/spl-token';

async function createWZECJob(wallet, jobData) {
  // 1. Verify token account exists
  const tokenAccount = await getAssociatedTokenAddress(
    new PublicKey(WZEC_MINT),
    wallet.publicKey
  );

  // 2. Call backend API
  const response = await fetch('/api/jobs/validate-and-build', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      ...jobData,
      payment_method: 'wZEC',
      creator_pubkey: wallet.publicKey.toString(),
    })
  });

  const { job_id, transaction } = await response.json();

  // 3. Deserialize and sign transaction
  const txBytes = Buffer.from(transaction, 'base64');
  const tx = Transaction.from(txBytes);
  const signedTx = await wallet.signTransaction(tx);

  // 4. Submit to network
  const connection = new Connection(RPC_URL);
  const signature = await connection.sendRawTransaction(signedTx.serialize());
  await connection.confirmTransaction(signature);

  // 5. Confirm with backend
  await fetch(`/api/jobs/${job_id}/confirm`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ signature })
  });

  return { job_id, signature };
}
```

## Testing

Ejecutar el script de test E2E:

```bash
chmod +x test_wzec_payment.sh
./test_wzec_payment.sh
```

Cobertura de tests:
- Creación de job wZEC
- Creación de job SOL (compatibilidad retroactiva)
- Rechazo de payment_method inválido
- Verificación de almacenamiento en base de datos

## Notas de Migración

### Compatibilidad Retroactiva

La integración es **totalmente compatible hacia atrás**:

- Requests existentes sin `payment_method` por defecto usan `"SOL"`
- Todos los flujos de pago SOL sin cambios
- Sin cambios incompatibles con contratos de API existentes

### Migración de Base de Datos

Migración ya aplicada: `20250120000007_add_wzec_payment_support.sql`

```sql
ALTER TABLE temp_job_data
ADD COLUMN payment_token_mint TEXT NULL,
ADD COLUMN payment_method TEXT NOT NULL DEFAULT 'SOL';

ALTER TABLE temp_job_data
ADD CONSTRAINT check_payment_method
CHECK (payment_method IN ('SOL', 'wZEC'));
```

## Soporte

Para problemas o preguntas:
- Revisar logs del programa: `/var/log/zyberlink/program.log`
- Revisar logs de API: `/var/log/zyberlink/api.log`
- Revisar transacción en Solana Explorer
- Verificar que la cuenta de token wZEC tenga saldo suficiente

## Changelog

### v0.1.0 (2025-11-20)

- Agregado campo `payment_method` al request de API
- Implementado constructor de instrucción `CreateJobWithToken`
- Agregada validación de mint wZEC
- Actualizado schema de base de datos
- Creado script de test E2E
- Mantenida compatibilidad retroactiva de pago SOL
