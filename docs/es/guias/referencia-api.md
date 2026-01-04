# Referencia de API

Documentación de referencia completa para el SDK de ZyberLink.

## Tabla de Contenidos

- [MarketplaceClient](#marketplaceclient)
- [InstructionBuilder](#instructionbuilder)
- [Tipos de Datos](#tipos-de-datos)
- [Instrucciones](#instrucciones)
- [Manejo de Errores](#manejo-de-errores)

## MarketplaceClient

El cliente principal para interactuar con el programa marketplace de ZyberLink.

### Constructor

#### `new`

Crea un nuevo cliente marketplace con nivel de compromiso por defecto (confirmed).

```rust
pub fn new(rpc_url: String, program_id: Pubkey) -> Self
```

**Parámetros:**
- `rpc_url`: URL del endpoint RPC de Solana
- `program_id`: ID del programa marketplace desplegado

**Retorna:** Instancia de `MarketplaceClient`

**Ejemplo:**
```rust
let client = MarketplaceClient::new(
    "http://localhost:8899".to_string(),
    program_id,
);
```

#### `new_with_commitment`

Crea un cliente con nivel de compromiso personalizado.

```rust
pub fn new_with_commitment(
    rpc_url: String,
    program_id: Pubkey,
    commitment: CommitmentConfig,
) -> Self
```

### Constructores de Instrucciones

#### `initialize_instruction`

Construye una instrucción para inicializar la configuración del marketplace.

```rust
pub fn initialize_instruction(
    &self,
    authority: &Pubkey,
    fee_basis_points: u16,
    min_stake_amount: u64,
    min_reputation_score: u32,
    default_job_timeout_seconds: i64,
) -> Result<Instruction>
```

**Parámetros:**
- `authority`: Clave pública de autoridad del marketplace (debe firmar)
- `fee_basis_points`: Comisión del protocolo en puntos base (ej. 1000 = 10%)
- `min_stake_amount`: Stake mínimo requerido para provers (lamports)
- `min_reputation_score`: Puntuación mínima de reputación para reclamar trabajos
- `default_job_timeout_seconds`: Timeout por defecto para trabajos

#### `create_job_instruction`

Construye una instrucción para crear un nuevo trabajo de computación.

```rust
pub fn create_job_instruction(
    &self,
    job_creator: &Pubkey,
    job_id: u64,
    circuit_type: CircuitType,
    witness_commitment: [u8; 32],
    witness_size: u32,
    price_lamports: u64,
    timeout_seconds: i64,
    fhe_config: Option<FheConsensusConfig>,
) -> Result<Instruction>
```

**Parámetros:**
- `job_creator`: Clave pública del creador del trabajo (debe firmar y pagar)
- `job_id`: Identificador único del trabajo
- `circuit_type`: Tipo de computación (FHE o ZK)
- `witness_commitment`: Compromiso hash del witness encriptado
- `witness_size`: Tamaño de datos del witness en bytes
- `price_lamports`: Pago por finalización del trabajo
- `timeout_seconds`: Timeout del trabajo en segundos
- `fhe_config`: Configuración opcional de consenso FHE

#### `submit_fhe_result_instruction`

Construye una instrucción para enviar un resultado de computación FHE.

```rust
pub fn submit_fhe_result_instruction(
    &self,
    prover_authority: &Pubkey,
    job_pda: &Pubkey,
    result_hash: [u8; 32],
) -> Result<Instruction>
```

#### `finalize_fhe_job_instruction`

Construye una instrucción para finalizar un trabajo FHE después del consenso.

```rust
pub fn finalize_fhe_job_instruction(
    &self,
    finalizer: &Pubkey,
    job_pda: &Pubkey,
    job_creator: &Pubkey,
    prover_accounts: &[Pubkey],
) -> Result<Instruction>
```

### Métodos FHE de Alto Nivel

#### `submit_fhe_result`

Método de alto nivel para enviar un resultado FHE.

```rust
pub fn submit_fhe_result(
    &self,
    prover: &Keypair,
    job_pda: &Pubkey,
    result_hash: [u8; 32],
) -> Result<Signature>
```

#### `finalize_fhe_job`

Método de alto nivel para finalizar un trabajo FHE.

```rust
pub fn finalize_fhe_job(
    &self,
    finalizer: &Keypair,
    job_pda: &Pubkey,
    job_creator: &Pubkey,
    matching_prover_pubkeys: &[Pubkey],
) -> Result<Signature>
```

### Helpers de PDA

#### `get_config_pda`

Deriva el PDA de configuración del marketplace.

```rust
pub fn get_config_pda(&self) -> (Pubkey, u8)
```

#### `get_prover_pda`

Deriva un PDA de cuenta de prover.

```rust
pub fn get_prover_pda(&self, prover_authority: &Pubkey) -> (Pubkey, u8)
```

#### `get_job_pda`

Deriva un PDA de cuenta de trabajo.

```rust
pub fn get_job_pda(&self, job_creator: &Pubkey, job_id: u64) -> (Pubkey, u8)
```

## InstructionBuilder

Constructor puro de instrucciones para integración con wallets (sin dependencia RPC).

### Constructor

```rust
pub fn new(program_id: Pubkey) -> Self
```

### Métodos de Instrucciones

Todos los métodos de instrucción coinciden con `MarketplaceClient` pero aceptan `Pubkey` en lugar de `&Pubkey` para compatibilidad con wallets.

## Tipos de Datos

### CircuitType

Enum que representa el tipo de computación.

```rust
pub enum CircuitType {
    ZcashOrchard,
    AnonymousVote,
    Credential,
    FheAdd,
    FheMultiply,
    FheSubtract,
    Custom(String),
}
```

**Variantes:**
- `ZcashOrchard`: Transacción shielded de Zcash Orchard
- `AnonymousVote`: Circuito de votación anónima
- `FheAdd`: Operación de suma FHE
- `FheMultiply`: Operación de multiplicación FHE
- `FheSubtract`: Operación de resta FHE
- `Custom(String)`: Tipo de circuito personalizado

### FheConsensusConfig

Configuración para consenso multi-prover FHE.

```rust
pub struct FheConsensusConfig {
    pub required_provers: u8,
    pub consensus_threshold: u8,
}
```

**Campos:**
- `required_provers`: Número de provers requeridos para reclamar el trabajo
- `consensus_threshold`: Número de resultados coincidentes necesarios para consenso

**Ejemplo:**
```rust
let config = FheConsensusConfig {
    required_provers: 3,
    consensus_threshold: 2, // Consenso 2-de-3
};
```

### JobStatus

Enum que representa el estado actual de un trabajo.

```rust
pub enum JobStatus {
    Pending,    // Creado, esperando provers
    Claimed,    // Prover(s) han reclamado el trabajo
    Computing,  // Computación en progreso
    Completed,  // Completado exitosamente
    Failed,     // Falló (timeout o fallo de consenso)
    Cancelled,  // Cancelado por el creador
}
```

### Job

Estructura de cuenta de trabajo on-chain.

```rust
pub struct Job {
    pub creator: Pubkey,
    pub circuit_type: CircuitType,
    pub witness_commitment: [u8; 32],
    pub price_lamports: u64,
    pub status: JobStatus,
    pub created_at: i64,
    pub timeout_at: i64,
    pub claimed_provers: Vec<Pubkey>,
    pub fhe_config: Option<FheConsensusConfig>,
    pub fhe_results: Vec<FheResult>,
}
```

### FheResult

Resultado de computación FHE enviado por un prover.

```rust
pub struct FheResult {
    pub prover: Pubkey,
    pub result_hash: [u8; 32],
    pub submitted_at: i64,
}
```

## Instrucciones

### MarketplaceInstruction

Enum de todas las instrucciones del programa.

```rust
pub enum MarketplaceInstruction {
    Initialize {
        fee_basis_points: u16,
        min_stake_amount: u64,
        min_reputation_score: u32,
        default_job_timeout_seconds: i64,
    },
    RegisterProver {
        stake_amount: u64,
        encryption_pubkey: [u8; 32],
    },
    CreateJob {
        circuit_type: CircuitType,
        witness_commitment: [u8; 32],
        witness_size: u32,
        price_lamports: u64,
        timeout_seconds: i64,
        fhe_config: Option<FheConsensusConfig>,
    },
    ClaimJob,
    SubmitProof {
        proof_commitment: [u8; 32],
        proof_size: u32,
    },
    SubmitFheResult {
        result_hash: [u8; 32],
    },
    FinalizeFheJob,
    CancelJob,
}
```

## Manejo de Errores

El SDK usa `anyhow::Result` para manejo flexible de errores.

### Errores Comunes

**Errores RPC:**
```rust
// Falló conexión
Err: RpcError: Failed to get account

// Cuenta no encontrada
Err: RpcError: AccountNotFound
```

**Errores de Programa:**
```rust
// Errores personalizados del programa (del programa on-chain)
Err: ProgramError: Custom(1001) // Stake insuficiente
Err: ProgramError: Custom(1002) // Trabajo no está pendiente
```

### Patrón de Manejo de Errores

```rust
use anyhow::{Context, Result};

fn create_job() -> Result<()> {
    let signature = client
        .send_and_confirm_transaction(&[ix], &[&creator])
        .context("Falló crear trabajo")?;

    Ok(())
}

// Uso
match create_job() {
    Ok(_) => println!("Éxito!"),
    Err(e) => {
        eprintln!("Error: {:?}", e);
        for cause in e.chain() {
            eprintln!("  Causado por: {}", cause);
        }
    }
}
```

## Información de Versión

**Versión Actual del SDK:** 0.2.0

**Versión del SDK de Solana:** 1.18+

**Versión Mínima de Rust:** 1.75+

## Próximos Pasos

- **[Guía de Integración del SDK](integracion-sdk.md)** - Patrones de integración y mejores prácticas
- **[Guía de Ejemplos](ejemplos.md)** - Ejemplos de código del mundo real
- **[Visión General de Arquitectura](../arquitectura/vision-general.md)** - Arquitectura del sistema

## Soporte

Para preguntas sobre la API:
- GitHub Issues: ver el [Mapa de Repositorios](../primeros-pasos/repositorios.md)
- Documentación: [Docs completos](https://docs.zyberlink.io)
