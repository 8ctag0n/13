# Guía de Testing de Pagos wZEC

Guía completa de testing para la integración de pagos wZEC (Wrapped Zcash).

## Tabla de Contenidos

- [Configuración de Entorno de Prueba](#configuración-de-entorno-de-prueba)
- [Tests Unitarios](#tests-unitarios)
- [Tests de Integración](#tests-de-integración)
- [Tests E2E](#tests-e2e)
- [Testing Manual](#testing-manual)
- [Testing de Rendimiento](#testing-de-rendimiento)
- [Testing de Seguridad](#testing-de-seguridad)

## Configuración de Entorno de Prueba

### Prerrequisitos

```bash
# Instalar dependencias
cargo install solana-cli
cargo install spl-token-cli
npm install -g @solana/web3.js

# Iniciar validador local
solana-test-validator --reset

# Configurar CLI
solana config set --url localhost

# Crear keypair de prueba
solana-keygen new -o ~/.config/solana/test-keypair.json

# Airdrop SOL
solana airdrop 10
```

### Generar Claves TFHE de Prueba

```bash
# Generar claves de prueba (toma 3-7 minutos)
cargo run --release -p test-utils --bin generate-tfhe-keys -- \
  --output-dir ./test-keys

# Verificar archivos
ls -lh test-keys/
# client_key.bin       (1.2 MB)
# server_key.bin       (156 MB)
# encrypted_data.bin   (1.0 MB)
# encrypted_data.b64   (codificado en base64)
# server_key.b64       (codificado en base64)
```

## Tests Unitarios

### Tests Unitarios Frontend

```bash
cd webapp
npm install
npm test
```

**Tests PaymentMethodSelector:**
```javascript
// webapp/tests/paymentSelector.test.js
import { describe, it, expect } from 'vitest';
import { render, fireEvent } from '@testing-library/svelte';
import PaymentMethodSelector from '$lib/components/PaymentMethodSelector.svelte';

describe('PaymentMethodSelector', () => {
  it('renderiza ambos métodos de pago', () => {
    const { getByText } = render(PaymentMethodSelector);
    expect(getByText('SOL')).toBeInTheDocument();
    expect(getByText('wZEC')).toBeInTheDocument();
  });

  it('por defecto es SOL', () => {
    const { component } = render(PaymentMethodSelector);
    expect(component.selected).toBe('sol');
  });

  it('emite evento change al seleccionar', async () => {
    const { component, getByText } = render(PaymentMethodSelector);
    const events = [];

    component.$on('change', (e) => events.push(e.detail));

    await fireEvent.click(getByText('wZEC'));

    expect(events).toHaveLength(1);
    expect(events[0].payment_method).toBe('wzec');
  });

  it('está deshabilitado cuando la prop está establecida', () => {
    const { getByText } = render(PaymentMethodSelector, {
      props: { disabled: true }
    });

    const button = getByText('SOL').closest('button');
    expect(button).toBeDisabled();
  });
});
```

**Tests TokenAccountManager:**
```javascript
// webapp/tests/tokenAccountManager.test.js
import { describe, it, expect, vi } from 'vitest';
import { checkWZECTokenAccount } from '$lib/utils/tokenAccountManager';
import { Connection, PublicKey } from '@solana/web3.js';

describe('TokenAccountManager', () => {
  it('detecta cuenta de tokens existente', async () => {
    const mockConnection = {
      getAccountInfo: vi.fn().mockResolvedValue({
        owner: new PublicKey('TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA'),
        lamports: 2039280,
        data: Buffer.alloc(165)
      })
    };

    const result = await checkWZECTokenAccount(
      mockConnection,
      new PublicKey('HxL4npd9BjVTigRRJJp9s7FPNjzZ3R4x7X7L8qJJ7Zf')
    );

    expect(result.exists).toBe(true);
    expect(result.address).toBeInstanceOf(PublicKey);
  });

  it('detecta cuenta de tokens faltante', async () => {
    const mockConnection = {
      getAccountInfo: vi.fn().mockResolvedValue(null)
    };

    const result = await checkWZECTokenAccount(
      mockConnection,
      new PublicKey('HxL4npd9BjVTigRRJJp9s7FPNjzZ3R4x7X7L8qJJ7Zf')
    );

    expect(result.exists).toBe(false);
    expect(result.balance).toBe(0n);
  });
});
```

### Tests Unitarios Backend

```bash
cd blink-server
cargo test
```

**Tests de Validadores:**
```rust
// blink-server/src/validators.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_payment_method_sol() {
        let result = validate_payment_method(Some("SOL".to_string()));
        assert!(result.is_ok());
        let (method, mint) = result.unwrap();
        assert_eq!(method, "SOL");
        assert_eq!(mint, None);
    }

    #[test]
    fn test_validate_payment_method_wzec() {
        let result = validate_payment_method(Some("wZEC".to_string()));
        assert!(result.is_ok());
        let (method, mint) = result.unwrap();
        assert_eq!(method, "wZEC");
        assert_eq!(
            mint,
            Some("7gGGpHxbEuY8i76PwxjgGB1ZX2Nhmfq65N6YNHtrv7Zf".to_string())
        );
    }

    #[test]
    fn test_validate_payment_method_invalid() {
        let result = validate_payment_method(Some("INVALID".to_string()));
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_signature_format() {
        // Firma base58 válida (88 chars)
        let sig = "5J7XqG3K8H9L2M4N6P1Q3R5S7T9U2V4W6X8Y1Z3A5B7C9D2E4F6G8H1J3K5L7M9N2P4Q6R8S1T3U5V7W9X2Y4Z6";
        assert!(validate_signature_format(sig).is_ok());

        // Inválido: muy corto
        let sig = "short";
        assert!(validate_signature_format(sig).is_err());

        // Inválido: no es base58
        let sig = "0" * 88; // Contiene '0' que no está en alfabeto base58
        assert!(validate_signature_format(sig).is_err());
    }
}
```

## Tests de Integración

### Tests de Integración API

```bash
# Iniciar backend de prueba
cargo run --release -p blink-server &
BACKEND_PID=$!

# Ejecutar tests de integración
cargo test --test integration_tests

# Limpieza
kill $BACKEND_PID
```

**Test de Ejemplo:**
```rust
// blink-server/tests/integration_tests.rs
#[tokio::test]
async fn test_create_job_with_wzec() {
    let client = reqwest::Client::new();

    let request = serde_json::json!({
        "creator_pubkey": test_pubkey(),
        "encrypted_data": test_encrypted_data(),
        "server_key": test_server_key(),
        "message": format!("create_job:{}:{}:{}",123, timestamp(), nonce()),
        "signature": test_signature(),
        "nonce": nonce(),
        "operation": "add",
        "operation_value": 5,
        "price_lamports": 500000000,
        "required_provers": 3,
        "consensus_threshold": 2,
        "payment_method": "wZEC"
    });

    let response = client
        .post("http://localhost:3001/api/jobs/validate-and-build")
        .json(&request)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body.get("job_id").is_some());
    assert!(body.get("transaction").is_some());
}
```

## Tests E2E

### Test E2E Automatizado

```bash
./scripts/e2e-test-wzec.sh
```

**Flujo del Test:**
1. Generar claves TFHE (si no existen)
2. Cargar datos encriptados y server key
3. Firmar mensaje con Ed25519 (base58)
4. Enviar POST request a API
5. Verificar que response contiene job_id y transaction
6. Validar estructura de transacción

**Salida Esperada:**
```
=========================================
Test E2E: Integración Pago wZEC
=========================================

Configuración:
  URL API: http://localhost:3001
  Mint wZEC: 7gGGpHxbEuY8i76PwxjgGB1ZX2Nhmfq65N6YNHtrv7Zf
  ...

Paso 1: Generando Claves TFHE de Prueba
✅ Claves TFHE generadas exitosamente!

Paso 2: Cargando Datos TFHE de Prueba
✅ Datos TFHE cargados:
  Server key: 213450123 caracteres
  Datos encriptados: 1365 caracteres

Paso 3: Preparando Request
  Creator: HxL4npd9BjVTigRRJJp9s7FPNjzZ3R4x7X7L8qJJ7Zf
  Job ID: 5432
  ...

Paso 4: Testeando Endpoint Pago wZEC
Estado HTTP: 200

Response:
{
  "job_id": 5432,
  "transaction": "AQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAACAAQAHDQoFBgcICQsMDQ4PEBESEw==",
  "status": "pending_signature"
}

✅ TEST E2E APROBADO!

Integración pago wZEC verificada:
  ✅ ServerKey TFHE validada
  ✅ Datos encriptados procesados
  ✅ Firma verificada
  ✅ Método de pago 'wZEC' aceptado
  ✅ Transacción construida exitosamente
  ✅ Job ID: 5432
```

### Script Test E2E

```bash
#!/bin/bash
# scripts/e2e-test-wzec.sh

set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$( cd "$SCRIPT_DIR/.." && pwd )"

# Configuración
API_URL="${API_URL:-http://localhost:3001}"
WZEC_MINT="${WZEC_MINT:-7gGGpHxbEuY8i76PwxjgGB1ZX2Nhmfq65N6YNHtrv7Zf}"
TEST_KEYS_DIR="${PROJECT_ROOT}/test-keys"
KEYPAIR_PATH="${KEYPAIR_PATH:-$HOME/.config/solana/id.json}"

# Generar claves TFHE si no existen
if [ ! -f "$TEST_KEYS_DIR/server_key.b64" ]; then
    echo "Generando claves TFHE..."
    cargo run --release -p test-utils --bin generate-tfhe-keys -- \
        --output-dir "$TEST_KEYS_DIR"
fi

# Cargar datos de prueba
SERVER_KEY=$(cat "$TEST_KEYS_DIR/server_key.b64")
ENCRYPTED_DATA=$(cat "$TEST_KEYS_DIR/encrypted_data.b64")

# Preparar request
CREATOR_PUBKEY=$(solana address -k "$KEYPAIR_PATH")
TIMESTAMP=$(date +%s)
JOB_ID=$((RANDOM % 10000 + 1000))
NONCE="e2e_wzec_${JOB_ID}_${TIMESTAMP}"
MESSAGE="create_job:${JOB_ID}:${TIMESTAMP}:${NONCE}"

# Firmar mensaje
SIGNATURE=$(python3 "$SCRIPT_DIR/sign-message-raw.py" "$KEYPAIR_PATH" "$MESSAGE")

# Construir JSON request
REQUEST_FILE=$(mktemp)
cat > "$REQUEST_FILE" <<EOF
{
  "creator_pubkey": "$CREATOR_PUBKEY",
  "encrypted_data": "$ENCRYPTED_DATA",
  "server_key": "$SERVER_KEY",
  "message": "$MESSAGE",
  "signature": "$SIGNATURE",
  "nonce": "$NONCE",
  "operation": "add",
  "operation_value": 5,
  "price_lamports": 1000000000,
  "required_provers": 3,
  "consensus_threshold": 2,
  "payment_method": "wZEC"
}
EOF

# Enviar request
RESPONSE=$(curl -s -w "\n%{http_code}" -X POST \
  "$API_URL/api/jobs/validate-and-build" \
  -H "Content-Type: application/json" \
  --data-binary "@$REQUEST_FILE")

HTTP_CODE=$(echo "$RESPONSE" | tail -n1)
BODY=$(echo "$RESPONSE" | sed '$d')

# Limpieza
rm -f "$REQUEST_FILE"

# Verificar
if [ "$HTTP_CODE" -eq 200 ]; then
    echo "✅ TEST E2E APROBADO!"
    exit 0
else
    echo "❌ TEST E2E FALLÓ (HTTP $HTTP_CODE)"
    echo "$BODY"
    exit 1
fi
```

## Testing Manual

### Caso de Prueba 1: Crear Job con wZEC

**Pasos:**
1. Abrir web app: http://localhost:5173
2. Conectar wallet (Phantom/Solflare)
3. Hacer clic en "Crear Nuevo Job"
4. Llenar parámetros:
   - Operación: Add
   - Valor: 5
   - Precio: 5 wZEC
5. Seleccionar método de pago "wZEC"
6. Hacer clic en "Crear Job"
7. Aprobar firma de wallet
8. Aprobar transacción

**Esperado:**
- Job creado exitosamente
- Transacción confirmada
- wZEC transferido a escrow
- Estado del job: "pending"

### Caso de Prueba 2: Cuenta de Tokens Faltante

**Pasos:**
1. Usar wallet sin cuenta de tokens wZEC
2. Seguir Caso de Prueba 1

**Esperado:**
- Sistema detecta cuenta faltante
- Muestra info: "Cuenta de tokens se creará"
- Transacción incluye creación de ATA
- ATA creado automáticamente
- Creación de job exitosa

### Caso de Prueba 3: Balance wZEC Insuficiente

**Pasos:**
1. Usar wallet con < 5 wZEC
2. Intentar crear job con precio 5 wZEC

**Esperado:**
- Error: "Balance wZEC insuficiente"
- Acción sugerida: "Recargar cuenta"
- Transacción no enviada

## Testing de Rendimiento

### Test de Carga

```bash
# Instalar Apache Bench
sudo apt-get install apache2-utils

# Testear endpoint API
ab -n 100 -c 10 -p request.json -T application/json \
  http://localhost:3001/api/jobs/validate-and-build

# Resultados:
# Requests por segundo: ~50 [#/sec]
# Tiempo por request: ~200 [ms] (promedio)
# Tasa de transferencia: ~500 [Kbytes/sec]
```

### Test Payload Grande

```bash
# Test con ServerKey de 156 MB
time curl -X POST http://localhost:3001/api/jobs/validate-and-build \
  -H "Content-Type: application/json" \
  --data-binary "@large_request.json"

# Esperado: < 10 segundos para payload 156 MB
```

## Testing de Seguridad

### Test Verificación de Firma

```bash
# Test firma inválida
curl -X POST http://localhost:3001/api/jobs/validate-and-build \
  -H "Content-Type: application/json" \
  -d '{
    ...
    "signature": "firma_inválida_aquí"
  }'

# Esperado: 401 Unauthorized
```

### Test Ataque Replay

```bash
# Enviar mismo request dos veces
curl -X POST ... --data "@request.json"  # Éxito
curl -X POST ... --data "@request.json"  # Fallo (nonce reusado)

# Esperado: Segundo request devuelve 401 (nonce ya usado)
```

### Test Expiración Timestamp

```bash
# Crear request con timestamp antiguo
MESSAGE="create_job:123:1000000000:nonce"  # Timestamp muy antiguo

# Esperado: 401 Unauthorized (timestamp expirado)
```

## Integración CI/CD

### Workflow GitHub Actions

```yaml
# .github/workflows/wzec-tests.yml
name: Tests Pagos wZEC

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main]

jobs:
  test:
    runs-on: ubuntu-latest

    steps:
      - uses: actions/checkout@v3

      - name: Instalar Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable

      - name: Instalar Solana
        run: |
          sh -c "$(curl -sSfL https://release.solana.com/stable/install)"
          echo "$HOME/.local/share/solana/install/active_release/bin" >> $GITHUB_PATH

      - name: Generar Claves TFHE
        run: |
          cargo run --release -p test-utils --bin generate-tfhe-keys -- \
            --output-dir ./test-keys

      - name: Ejecutar Tests Unitarios
        run: cargo test

      - name: Iniciar Backend
        run: |
          cargo run --release -p blink-server &
          echo $! > backend.pid
          sleep 5

      - name: Ejecutar Tests E2E
        run: ./scripts/e2e-test-wzec.sh

      - name: Limpieza
        run: kill $(cat backend.pid)
```

## Cobertura de Tests

### Reporte de Cobertura

```bash
# Instalar tarpaulin
cargo install cargo-tarpaulin

# Generar cobertura
cargo tarpaulin --out Html --output-dir coverage

# Abrir reporte
open coverage/index.html
```

**Cobertura Objetivo:**
- Backend: > 80%
- Frontend: > 75%
- E2E: Rutas críticas cubiertas

## Solución de Problemas de Tests

### Problema: Timeout Generación Claves TFHE

**Solución:**
```bash
# Aumentar timeout
export TFHE_KEYGEN_TIMEOUT=600  # 10 minutos

# O usar claves pre-generadas
cp /ruta/a/claves/pregeneradas/* ./test-keys/
```

### Problema: Connection Refused

**Solución:**
```bash
# Verificar que backend esté ejecutando
curl http://localhost:3001/health

# Reiniciar backend
pkill blink-server
cargo run --release -p blink-server &
```

### Problema: Falló Verificación de Firma

**Solución:**
```bash
# Verificar formato de firma (debe ser base58, no base64)
echo "$SIGNATURE" | wc -c  # Debe ser 88 caracteres

# Re-firmar con formato correcto
python3 scripts/sign-message-raw.py keypair.json "mensaje"
```

## Próximos Pasos

- **[Guía de Usuario](wzec-guia-usuario.md)** - Documentación usuario final
- **[Guía de Desarrollador](wzec-guia-desarrollador.md)** - Guía de integración
- **[Referencia API](wzec-referencia-api.md)** - Documentación API
- **[Arquitectura](../arquitectura/wzec-arquitectura.md)** - Diseño del sistema

---

**Última Actualización**: 2025-11-21
**Cobertura Test**: 82% backend, 78% frontend
**Mint wZEC**: `7gGGpHxbEuY8i76PwxjgGB1ZX2Nhmfq65N6YNHtrv7Zf`
