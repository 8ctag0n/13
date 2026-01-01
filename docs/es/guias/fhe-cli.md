# Guía del CLI de FHE

Herramientas de línea de comandos para encriptar datos localmente y desencriptar resultados de computaciones usando Encriptación Totalmente Homomórfica (FHE).

## Descripción General

El CLI de FHE de ZyberLink provee dos herramientas esenciales para computaciones que preservan privacidad:

1. **zyb fhe encrypt** - Encripta tus datos localmente antes de enviar a ZyberLink
2. **zyb fhe decrypt** - Desencripta resultados de computaciones después de que los provers procesen tu trabajo

**Garantía de Privacidad:** Tus datos en texto plano nunca salen de tu máquina. Solo el texto cifrado encriptado es subido a la plataforma.

### ¿Por Qué Usar el CLI?

**Beneficios de Seguridad:**
- La encriptación ocurre localmente (tú controlas las claves)
- Ningún tercero de confianza ve tus datos
- La clave de cliente nunca se transmite por la red
- Control criptográfico completo

**Flexibilidad:**
- Encripta cualquier valor 0-255 (FheUint8)
- Soporta encriptación por lotes (múltiples valores)
- Compatible con todas las operaciones de ZyberLink
- Funciona offline (no se requiere red para encriptación)

## Instalación

### Requisitos Previos

- Rust 1.75 o superior
- Gestor de paquetes Cargo
- ~50 MB de espacio en disco (para el binario compilado)

### Compilar desde Fuente

```bash
# Clona el repositorio de ZyberLink
git clone https://github.com/8ctag0n/13.git zyberlink
cd zyberlink

# Navega al directorio del CLI de FHE
cd src/zyb-cli

# Compila versión release
cargo build --release

# Binarios ubicados en:
# - target/release/zyb
```

**Verifica la instalación:**
```bash
./target/release/zyb --version
# Esperado: zyb 0.1.0
```

### Agregar a PATH (Opcional)

```bash
# Copia el binario a la ruta del sistema
sudo cp target/release/zyb /usr/local/bin/

# Ahora úsalo desde cualquier lugar
zyb fhe --help
```

## Inicio Rápido

### 1. Encripta Datos

```bash
cd src/zyb-cli

# Encripta un solo valor
cargo run --release --bin zyb fhe encrypt --values 42

# Encripta múltiples valores (separados por comas)
cargo run --release --bin zyb fhe encrypt --values 10,20,30,40,50
```

**Salida:**
```
Herramienta de Encriptación FHE de ZyberLink
================================

Generando keypair FHE...
Keypair generado (tomó 1.2s)

Encriptando 5 valores...
Valores encriptados exitosamente

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
ARCHIVOS GENERADOS:
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Directorio de salida: ./fhe-output/

Archivos creados:
  - witness.bin (52.4 MB) - Subir a ZyberLink
  - client_key.bin (0.5 MB) - MANTENER SECRETO

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

IMPORTANTE: Mantén client_key.bin seguro
Lo necesitas para desencriptar resultados.

Próximos pasos:
   1. Subir witness.bin a la plataforma ZyberLink
   2. Enviar trabajo de computación
   3. Descargar resultado encriptado
   4. Desencriptar usando client_key.bin
```

### 2. Sube a la Plataforma

1. Navega a https://demo.zyberlink.fun
2. Elige tu operación (Analytics o Proof of Innocence)
3. Sube `witness.bin`
4. Envía el trabajo y espera finalización

### 3. Desencripta el Resultado

Después de que tu trabajo se complete:

```bash
# Descarga resultado encriptado de la plataforma (ej., result.bin)

# Desencripta el resultado
cargo run --release --bin zyb fhe decrypt \
  --result-path ~/Downloads/result.bin \
  --client-key-path ./fhe-output/client_key.bin
```

**Salida:**
```
Herramienta de Desencriptación FHE de ZyberLink
=================================

Cargando clave de cliente...
Clave de cliente cargada

Cargando resultado encriptado...
Resultado encriptado cargado (256 bytes)

Desencriptando resultado...
Resultado desencriptado exitosamente

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
RESULTADO: 150
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Operación: Sum
Valores de entrada: 5 (encriptados)
Computación: 10 + 20 + 30 + 40 + 50 = 150

Tus datos permanecieron encriptados en todo momento
```

## Referencia de Comandos

### encrypt - Encriptar Datos

**Sintaxis:**
```bash
zyb fhe encrypt --values <VALUES> [--output <PATH>]
```

**Argumentos:**

| Argumento | Tipo | Requerido | Descripción |
|-----------|------|-----------|-------------|
| `--values` | String | Sí | Valores separados por comas para encriptar (0-255) |
| `--output` | Path | No | Directorio de salida (por defecto: ./fhe-output) |

**Ejemplos:**

```bash
# Valor único
zyb fhe encrypt --values 42

# Múltiples valores
zyb fhe encrypt --values 10,20,30,40,50

# Directorio de salida personalizado
zyb fhe encrypt --values 100,200 --output ~/encrypted-data/

# Dataset grande
zyb fhe encrypt --values 5,15,25,35,45,55,65,75,85,95
```

**Archivos de Salida:**

1. **witness.bin** (~52 MB)
   - Contiene: Datos encriptados + Server key
   - Propósito: Subir a ZyberLink para computación
   - Seguro para compartir: Sin información de texto plano

2. **client_key.bin** (~0.5 MB)
   - Contiene: Clave de desencriptación del cliente
   - Propósito: Desencriptar resultados de computación
   - **MANTENER SECRETO:** Cualquiera con esto puede desencriptar tus resultados

**Rendimiento:**

| Tamaño del Dataset | Generación de Claves | Tiempo de Encriptación | Tiempo Total |
|-------------------|---------------------|------------------------|--------------|
| 1 valor | 1-2 segundos | <0.1 segundos | ~2 segundos |
| 10 valores | 1-2 segundos | 0.2 segundos | ~2 segundos |
| 100 valores | 1-2 segundos | 1-2 segundos | ~4 segundos |

**Nota:** La generación de claves domina el tiempo total. Encriptar más valores es prácticamente gratis una vez que las claves están generadas.

### decrypt - Desencriptar Resultado

**Sintaxis:**
```bash
zyb fhe decrypt --result-path <PATH> --client-key-path <PATH>
```

**Argumentos:**

| Argumento | Tipo | Requerido | Descripción |
|-----------|------|-----------|-------------|
| `--result-path` | Path | Sí | Ruta al archivo de resultado encriptado |
| `--client-key-path` | Path | Sí | Ruta a client_key.bin |

**Ejemplos:**

```bash
# Desencriptación básica
zyb fhe decrypt \
  --result-path result.bin \
  --client-key-path ./fhe-output/client_key.bin

# Con rutas absolutas
zyb fhe decrypt \
  --result-path ~/Downloads/result_abc123.bin \
  --client-key-path ~/.zyberlink/keys/client_key.bin

# Desencripta resultado de trabajo específico
zyb fhe decrypt \
  --result-path ./results/job_42_result.bin \
  --client-key-path ./fhe-output/client_key.bin
```

**Tipos de Resultado:**

**Sum/Average (valor único):**
```
RESULTADO: 150
```

**CountIf (conteo):**
```
RESULTADO: 3
(3 valores coincidieron con el predicado)
```

**Histogram (múltiples bins):**
```
RESULTADOS:
  Bin 0 (0-18): 5
  Bin 1 (19-35): 12
  Bin 2 (36-65): 8
  Bin 3 (66+): 2
```

## Flujos de Trabajo Completos

### Flujo de Trabajo 1: Analytics Privado (Sum)

**Escenario:** Calcular donaciones totales sin revelar montos individuales

#### Paso 1: Encriptar Valores de Donaciones

```bash
cd src/zyb-cli

# Encripta montos de donaciones (en dólares)
cargo run --release --bin zyb fhe encrypt \
  --values 100,250,75,500,125

# Salida:
# witness.bin creado (52.4 MB)
# client_key.bin creado (0.5 MB)
```

#### Paso 2: Envía a ZyberLink

```
1. Abre https://demo.zyberlink.fun/analytics
2. Conecta wallet de Solana
3. Sube witness.bin
4. Selecciona operación: SUM
5. Establece precio: 0.0054 SOL (recomendado)
6. Haz clic en "Submit Analytics Job"
7. Espera finalización (~60-120 segundos)
```

#### Paso 3: Descarga y Desencripta Resultado

```bash
# Descarga result.bin de la plataforma

# Desencripta
cargo run --release --bin zyb fhe decrypt \
  --result-path ~/Downloads/result.bin \
  --client-key-path ./fhe-output/client_key.bin

# Salida:
# RESULTADO: 1050
# (Donaciones totales: $1,050)
```

**Privacidad Preservada:**
- Montos individuales nunca expuestos
- Solo suma encriptada revelada a ti
- Provers nunca vieron valores de texto plano

### Flujo de Trabajo 2: Proof of Innocence (CountIf)

**Escenario:** Probar que no has interactuado con direcciones sancionadas

#### Paso 1: Mapear Historial de Transacciones a Índices

```python
# Ejemplo: Convertir direcciones a índices
transactions = [
    "0xABC...123",  # → Índice 10
    "0xDEF...456",  # → Índice 25
    "0xGHI...789",  # → Índice 50
    "0xJKL...012",  # → Índice 75
]

indices = [10, 25, 50, 75]

# Lista de sanciones: [66, 77, 88, 99]
# Resultado esperado: 0 coincidencias (inocente)
```

#### Paso 2: Encriptar Índices de Transacciones

```bash
cd src/zyb-cli

# Encripta tus índices de transacciones
cargo run --release --bin zyb fhe encrypt \
  --values 10,25,50,75

# Salida:
# witness.bin creado
# client_key.bin creado
```

#### Paso 3: Envía Trabajo de Proof of Innocence

```
1. Abre https://demo.zyberlink.fun/proof-of-innocence
2. Conecta wallet
3. Sube witness.bin
4. Selecciona índice sancionado: 66
5. Establece precio: 0.0054 SOL
6. Haz clic en "Verify Innocence"
7. Espera finalización
```

#### Paso 4: Desencripta Resultado de Verificación

```bash
# Descarga resultado de la plataforma

cargo run --release --bin zyb fhe decrypt \
  --result-path result.bin \
  --client-key-path ./fhe-output/client_key.bin

# Salida:
# RESULTADO: 0
# (No se detectaron interacciones sancionadas)
# Proof of Innocence verificado
```

### Flujo de Trabajo 3: Procesamiento por Lotes de Múltiples Trabajos

**Escenario:** Analizar diferentes métricas sobre el mismo dataset

#### Paso 1: Encripta Dataset Una Vez

```bash
# Encripta tu dataset
cargo run --release --bin zyb fhe encrypt \
  --values 15,22,28,35,42,48,55,62,68,75

# Guarda ubicación de clave de cliente
CLIENT_KEY=./fhe-output/client_key.bin
```

#### Paso 2: Envía Múltiples Trabajos

```
Usando el mismo witness.bin:

Trabajo 1: SUM - Total de todos los valores
Trabajo 2: AVERAGE - Valor promedio
Trabajo 3: COUNTIF (>= 50) - Valores por encima del umbral
Trabajo 4: HISTOGRAM - Análisis de distribución
```

#### Paso 3: Desencripta Todos los Resultados

```bash
# Desencripta resultado del trabajo 1 (Sum)
zyb fhe decrypt --result-path result_job1.bin --client-key-path $CLIENT_KEY
# Resultado: 450

# Desencripta resultado del trabajo 2 (Average)
zyb fhe decrypt --result-path result_job2.bin --client-key-path $CLIENT_KEY
# Resultado: 45 (promedio)

# Desencripta resultado del trabajo 3 (CountIf)
zyb fhe decrypt --result-path result_job3.bin --client-key-path $CLIENT_KEY
# Resultado: 5 (valores >= 50)

# Desencripta resultado del trabajo 4 (Histogram)
zyb fhe decrypt --result-path result_job4.bin --client-key-path $CLIENT_KEY
# Resultados: [3, 4, 3] (bins)
```

## Uso Avanzado

### Directorios de Salida Personalizados

Organiza datos encriptados por proyecto:

```bash
# Crea directorio de proyecto
mkdir -p ~/projects/analytics-demo/encrypted

# Encripta con salida personalizada
zyb fhe encrypt \
  --values 10,20,30 \
  --output ~/projects/analytics-demo/encrypted

# Archivos creados:
# ~/projects/analytics-demo/encrypted/witness.bin
# ~/projects/analytics-demo/encrypted/client_key.bin
```

### Encriptar Valores Máximos

FHE soporta valores 0-255 (enteros sin signo de 8 bits):

```bash
# Valor mínimo
zyb fhe encrypt --values 0

# Valor máximo
zyb fhe encrypt --values 255

# Mezcla de valores
zyb fhe encrypt --values 0,50,100,150,200,255

# Inválido (dará error)
zyb fhe encrypt --values 256  # Fuera de rango
zyb fhe encrypt --values -1   # Negativo no soportado
```

**Solución para valores más grandes:**
Usa escalado:

```python
# Valores originales
values = [1000, 2000, 3000]

# Escala hacia abajo a 0-255
scale_factor = 10
scaled = [v // scale_factor for v in values]
# scaled = [100, 200, 255] (limitado a 255)

# Encripta valores escalados
# Después de computación, escala el resultado de vuelta
```

### Script de Encriptación por Lotes

Automatiza encriptación de múltiples datasets:

```bash
#!/bin/bash
# encrypt-batch.sh

datasets=(
  "10,20,30,40,50"
  "15,25,35,45,55"
  "20,30,40,50,60"
)

for i in "${!datasets[@]}"; do
  echo "Encriptando dataset $((i+1))..."

  zyb fhe encrypt \
    --values "${datasets[$i]}" \
    --output "./encrypted/dataset_$i"

  echo "Dataset $((i+1)) encriptado"
done

echo "Todos los datasets encriptados"
ls -lh ./encrypted/
```

### Mejores Prácticas de Gestión de Claves

```bash
# Crea directorio de almacenamiento seguro de claves
mkdir -p ~/.zyberlink/keys
chmod 700 ~/.zyberlink/keys

# Mueve claves de cliente a ubicación segura
mv fhe-output/client_key.bin ~/.zyberlink/keys/project1_$(date +%Y%m%d).bin
chmod 400 ~/.zyberlink/keys/project1_*.bin

# Respalda claves encriptadas
gpg --symmetric --cipher-algo AES256 \
  ~/.zyberlink/keys/project1_20251204.bin

# Guarda backup .gpg en unidad externa
cp ~/.zyberlink/keys/project1_20251204.bin.gpg /media/usb/backups/

# Nunca hagas commit de claves a git
echo "client_key.bin" >> .gitignore
echo "*.bin" >> .gitignore
```

## Solución de Problemas

### La Encriptación Falla

**Error:** "Failed to generate keypair"

**Causa:** Memoria insuficiente o proceso interrumpido

**Solución:**
```bash
# Verifica memoria disponible
free -h
# Se necesitan al menos 2GB libres

# Cierra otras aplicaciones
# Reintentar encriptación

# Si persiste, reinicia el sistema
sudo reboot
```

### Valores Fuera de Rango

**Error:** "Value X out of range (must be 0-255)"

**Causa:** Valores de entrada demasiado grandes para FheUint8

**Solución:**
```bash
# Usa escalado
# Original: 1000 → Escalado: 100 (dividir por 10)
# Después de computación, multiplica resultado por 10

# O usa múltiples valores encriptados
# 1000 = encrypt(255) + encrypt(255) + encrypt(255) + encrypt(235)
```

### La Desencriptación Falla

**Error:** "Failed to deserialize client key"

**Causa:** Archivo de clave corrupto o formato de archivo incorrecto

**Solución:**
```bash
# Verifica que el archivo exista y sea legible
ls -lh client_key.bin

# Verifica tamaño del archivo (debería ser ~0.5 MB)
du -h client_key.bin

# Intenta con diferente clave de cliente
# (si encriptaste múltiples veces)

# Re-encripta si es necesario
zyb fhe encrypt --values 10,20,30
```

### Resultado de Desencriptación Incorrecto

**Error:** El resultado no coincide con el valor esperado

**Causa:** Usando clave de cliente incorrecta (de diferente encriptación)

**Solución:**
```bash
# Las claves de cliente están vinculadas a archivos witness específicos
# Asegura que estás usando el client_key.bin coincidente

# Verifica timestamps
ls -lt fhe-output/
# witness.bin y client_key.bin deberían tener el mismo timestamp

# Si no estás seguro, re-encripta y reenvía trabajo
```

### "Witness.bin Not Found"

**Error:** La plataforma no puede encontrar el witness subido

**Causa:** Subida falló o está incompleta

**Solución:**
```bash
# Verifica tamaño del archivo antes de subir
ls -lh witness.bin
# Debería ser ~52 MB

# Verifica conexión a Internet
# Reintentar subida

# Re-encripta si el archivo está corrupto
zyb fhe encrypt --values <ORIGINAL_VALUES>
```

## Detalles Técnicos

### Parámetros FHE

**Biblioteca:** TFHE-rs (Concrete de Zama) versión 0.10

**Configuración:**
- **Tipo de datos:** FheUint8 (enteros sin signo encriptados de 8 bits)
- **Seguridad:** Nivel de seguridad de 128 bits
- **Budget de ruido:** Suficiente para ~10 operaciones FHE
- **Operaciones soportadas:** Adición, multiplicación, comparación

**Tamaños de Claves:**
- **Clave de cliente:** ~500 KB
- **Server key:** ~50 MB (incluida en witness.bin)
- **Valor encriptado:** ~256 bytes por valor

### Formato de Archivo Witness

```
[Estructura de Witness.bin]
  ├─ Prefijo de longitud (4 bytes, u32 little-endian)
  ├─ Datos encriptados (tamaño variable)
  │   ├─ Vec<Vec<u8>> serializado con Bincode
  │   └─ Cada elemento: FheUint8 serializado
  └─ Server key (~50 MB)
      └─ ServerKey de TFHE serializada
```

**Ejemplo:**
```
Tamaño total: 52,428,800 bytes (~52 MB)
- Prefijo de longitud: 4 bytes
- Datos encriptados: 1,256 bytes (5 valores × ~256 bytes cada uno)
- Server key: 52,427,540 bytes
```

### Consideraciones de Seguridad

**Lo que es seguro:**
- Clave de cliente nunca transmitida (permanece local)
- Texto plano nunca expuesto (encriptado localmente)
- Server key puede ser pública (sin info secreta)
- Valores encriptados indistinguibles de aleatorios

**Lo que debes proteger:**
- **client_key.bin** - Cualquiera con esto puede desencriptar tus resultados
- Valores originales de texto plano (no se lo digas a nadie)

**Lo que es seguro compartir:**
- witness.bin (datos encriptados + server key)
- Server key (habilita computación, no desencriptación)
- Resultados encriptados (hasta que los desencriptes)

## Optimización de Rendimiento

### Minimizar Sobrecarga de Generación de Claves

```bash
# La generación de claves es lenta (1-2 segundos)
# Encripta múltiples valores a la vez para amortizar costo

# Ineficiente (genera claves 3 veces)
zyb fhe encrypt --values 10
zyb fhe encrypt --values 20
zyb fhe encrypt --values 30

# Eficiente (genera claves una vez)
zyb fhe encrypt --values 10,20,30
```

### Encriptación Paralela (Futuro)

Actualmente, la encriptación es secuencial. Versiones futuras pueden soportar:

```bash
# Encriptación paralela hipotética
zyb fhe encrypt --values 1,2,3,4,5,6,7,8,9,10 --parallel

# Encriptaría valores en paralelo usando múltiples núcleos CPU
```

## Integración con Aplicaciones

### Uso Programático (Rust)

```rust
use tfhe::prelude::*;
use tfhe::{ConfigBuilder, generate_keys, FheUint8};

fn encrypt_values(values: Vec<u8>) -> Result<(Vec<Vec<u8>>, Vec<u8>), Box<dyn std::error::Error>> {
    // Genera claves
    let config = ConfigBuilder::default().build();
    let (client_key, server_key) = generate_keys(config);

    // Encripta valores
    let encrypted: Vec<Vec<u8>> = values
        .iter()
        .map(|&v| {
            let encrypted = FheUint8::encrypt(v, &client_key);
            bincode::serialize(&encrypted).unwrap()
        })
        .collect();

    // Serializa server key
    let server_key_bytes = bincode::serialize(&server_key)?;

    Ok((encrypted, server_key_bytes))
}
```

### Integración de API (JavaScript/TypeScript)

```javascript
import { exec } from 'child_process';
import { promisify } from 'util';

const execPromise = promisify(exec);

async function encryptData(values) {
  const valuesStr = values.join(',');

  const { stdout } = await execPromise(
    `zyb fhe encrypt --values ${valuesStr} --output ./encrypted`
  );

  return {
    witness: './encrypted/witness.bin',
    clientKey: './encrypted/client_key.bin'
  };
}

async function decryptResult(resultPath, clientKeyPath) {
  const { stdout } = await execPromise(
    `zyb fhe decrypt --result-path ${resultPath} --client-key-path ${clientKeyPath}`
  );

  // Parsea resultado del stdout
  const match = stdout.match(/RESULT: (\d+)/);
  return match ? parseInt(match[1]) : null;
}
```

## Próximos Pasos

Ahora que entiendes el CLI de FHE:

- **[Guía de Analytics](analytics-privado.md)** - Usa datos encriptados para estadísticas
- **[Guía de Proof of Innocence](proof-of-innocence.md)** - Verifica cumplimiento de manera privada
- **[Configuración de Prover](configuracion-prover.md)** - Ejecuta tu propio nodo de computación
- **[Integración del SDK](integracion-sdk.md)** - Construye FHE en tu app

## Soporte

Para preguntas sobre el CLI de FHE:

- **Documentación:** https://docs.zyberlink.fun
- **GitHub Issues:** https://github.com/8ctag0n/13/issues
- **Discord:** [Únete a la comunidad]
- **Email:** support@zyberlink.fun

## Documentación Relacionada

- [Código Fuente del CLI de Zyb](/src/zyb-cli/) - Detalles de implementación
- [Documentación de TFHE-rs](https://docs.zama.ai/tfhe-rs) - Biblioteca FHE
- [Guía de Operaciones FHE](/docs/book/en/concepts/fhe-operations.md) - Operaciones soportadas

---

**CLI de FHE:** Encripta localmente. Computa sobre texto cifrado. Desencripta de manera privada. Tus datos nunca expuestos.
