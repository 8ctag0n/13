# Guía de Integración del SDK

Aprende cómo integrar ZyberLink en tus aplicaciones Rust para externalizar computaciones FHE y ZK a una red descentralizada de provers.

## Resumen

El SDK de ZyberLink proporciona una API de alto nivel en Rust para:

- Crear y gestionar trabajos de computación
- Integrar operaciones FHE en tus aplicaciones  
- Monitorear progreso de trabajos y obtener resultados
- Gestionar interacciones con provers

## Instalación

Agrega el SDK de ZyberLink a tu `Cargo.toml`:

```toml
[dependencies]
zyberlink-sdk = { git = "https://github.com/yourusername/zyberlink", branch = "main" }
zyberlink-types = { git = "https://github.com/yourusername/zyberlink", branch = "main" }
solana-sdk = "1.18"
solana-client = "1.18"
anyhow = "1.0"
tokio = { version = "1.35", features = ["full"] }
```

## Inicio Rápido

### 1. Inicializar el Cliente

```rust
use zyberlink_sdk::MarketplaceClient;
use solana_sdk::pubkey::Pubkey;

// Conectar al validador local (desarrollo)
let client = MarketplaceClient::new(
    "http://localhost:8899".to_string(),
    program_id, // Tu program ID desplegado
);

// O conectar a devnet/mainnet
let client = MarketplaceClient::new(
    "https://api.devnet.solana.com".to_string(),
    program_id,
);
```

### 2. Crear Tu Primer Trabajo

```rust
use zyberlink_types::{CircuitType, FheConsensusConfig};
use solana_sdk::signature::{Keypair, Signer};

#[tokio::main]
async fn main() -> Result<()> {
    let client = MarketplaceClient::new(
        "http://localhost:8899".to_string(),
        program_id,
    );

    let creator = Keypair::new();
    let job_id = 1;

    // Preparar datos de witness encriptados
    let witness_commitment = [0u8; 32]; // Hash de tus datos encriptados
    let witness_size = 1024; // Tamaño en bytes

    // Configurar trabajo FHE con consenso multi-prover
    let fhe_config = Some(FheConsensusConfig {
        required_provers: 3,
        consensus_threshold: 2, // Consenso 2-de-3
    });

    // Construir instrucción
    let ix = client.create_job_instruction(
        &creator.pubkey(),
        job_id,
        CircuitType::FheAdd, // Operación de suma FHE
        witness_commitment,
        witness_size,
        1_000_000_000, // Pago de 1 SOL
        600, // Timeout de 10 minutos
        fhe_config,
    )?;

    // Enviar transacción
    let signature = client.send_and_confirm_transaction(
        &[ix],
        &[&creator],
    )?;

    println!("Trabajo creado: {}", signature);

    Ok(())
}
```

## Conceptos Centrales

### Ciclo de Vida del Trabajo

Los trabajos pasan por estos estados:
1. **Pending** - Trabajo creado, esperando provers
2. **Claimed** - Prover(s) han reclamado el trabajo
3. **Computing** - Computación en progreso
4. **Completed** - Trabajo completado exitosamente
5. **Failed** - Trabajo falló (timeout o fallo de consenso)

### Tipos de Trabajos

**Trabajos de Pruebas ZK**
- Modelo de prover único
- Verificación de prueba on-chain o del lado del cliente
- Finalización más rápida (10-15 segundos)

**Trabajos de Computación FHE**
- Consenso multi-prover (2-de-3 o 3-de-5)
- Verificación de resultados on-chain vía comparación de hash
- Mayor seguridad mediante tolerancia a fallas bizantinas

## Arquitectura del SDK

El SDK está organizado en tres capas:

### Capa 1: Constructores de Instrucciones

Constructores puros de instrucciones que no requieren keypairs (compatible con wallets):

```rust
use zyberlink_sdk::instructions::InstructionBuilder;

let builder = InstructionBuilder::new(program_id);

// Construir instrucción sin firmar
let ix = builder.create_job(
    creator_pubkey,
    job_id,
    CircuitType::FheAdd,
    witness_commitment,
    witness_size,
    price_lamports,
    timeout_seconds,
    fhe_config,
)?;

// La wallet firma la instrucción después
```

### Capa 2: Compositores de Transacciones

Compone transacciones de múltiples instrucciones.

### Capa 3: Cliente de Alto Nivel

Cliente completo con integración RPC:

```rust
use zyberlink_sdk::MarketplaceClient;

let client = MarketplaceClient::new(rpc_url, program_id);

// Operaciones de alto nivel
let signature = client.submit_fhe_result(&prover, &job_pda, result_hash)?;
```

## Patrones Comunes de Integración

### Patrón 1: Suma FHE Simple

Encripta dos números y súmalos usando FHE:

```rust
async fn fhe_addition_example() -> Result<()> {
    let fhe_client = FheClient::new();

    // Encriptar valores
    let encrypted_a = fhe_client.encrypt(42)?;
    let encrypted_b = fhe_client.encrypt(58)?;

    // Crear compromiso de witness
    let witness = vec![encrypted_a.clone(), encrypted_b.clone()];
    let witness_commitment = hash_witness(&witness);

    // Crear trabajo
    let client = MarketplaceClient::new(rpc_url, program_id);
    let job_id = get_next_job_id();

    let ix = client.create_job_instruction(
        &creator.pubkey(),
        job_id,
        CircuitType::FheAdd,
        witness_commitment,
        witness.len() as u32,
        1_000_000_000,
        600,
        Some(FheConsensusConfig {
            required_provers: 3,
            consensus_threshold: 2,
        }),
    )?;

    let signature = client.send_and_confirm_transaction(&[ix], &[&creator])?;

    // Esperar finalización
    let (job_pda, _) = client.get_job_pda(&creator.pubkey(), job_id);
    let result = poll_for_completion(&client, &job_pda).await?;

    // Desencriptar resultado
    let decrypted = fhe_client.decrypt(&result.encrypted_output)?;
    println!("Resultado: {} + {} = {}", 42, 58, decrypted);

    Ok(())
}
```

### Patrón 2: Procesamiento por Lotes

Envía múltiples trabajos eficientemente:

```rust
async fn batch_jobs_example() -> Result<()> {
    let client = MarketplaceClient::new(rpc_url, program_id);
    let mut jobs = Vec::new();

    // Crear múltiples instrucciones de trabajo
    for i in 0..10 {
        let ix = client.create_job_instruction(/* ... */)?;
        jobs.push(ix);
    }

    // Enviar todos los trabajos en paralelo (lote)
    for chunk in jobs.chunks(10) {
        let signature = client.send_and_confirm_transaction(chunk, &[&creator])?;
        println!("Lote enviado: {}", signature);
    }

    Ok(())
}
```

### Patrón 3: Monitoreo de Trabajos

Rastrea el progreso del trabajo en tiempo real:

```rust
async fn monitor_job(client: &MarketplaceClient, job_pda: &Pubkey) -> Result<JobStatus> {
    loop {
        let account = client.rpc_client.get_account(job_pda)?;
        let job: Job = borsh::from_slice(&account.data)?;

        match job.status {
            JobStatus::Pending => println!("Trabajo pendiente..."),
            JobStatus::Claimed => println!("Trabajo reclamado por {} provers", job.claimed_provers.len()),
            JobStatus::Computing => println!("Provers computando..."),
            JobStatus::Completed => {
                println!("Trabajo completado!");
                return Ok(JobStatus::Completed);
            }
            JobStatus::Failed => {
                println!("Trabajo falló");
                return Ok(JobStatus::Failed);
            }
        }

        sleep(Duration::from_secs(2)).await;
    }
}
```

## Mejores Prácticas

### 1. Usar Niveles de Compromiso Sabiamente

```rust
use solana_sdk::commitment_config::CommitmentConfig;

// Para producción: usar confirmed (rápido, seguro)
let client = MarketplaceClient::new_with_commitment(
    rpc_url,
    program_id,
    CommitmentConfig::confirmed(),
);

// Para garantías de finalidad: usar finalized (más lento, más seguro)
let client = MarketplaceClient::new_with_commitment(
    rpc_url,
    program_id,
    CommitmentConfig::finalized(),
);
```

### 2. Implementar Lógica de Reintentos

```rust
async fn send_with_retry(
    client: &MarketplaceClient,
    ix: &Instruction,
    signer: &Keypair,
    max_retries: u32,
) -> Result<Signature> {
    for attempt in 0..max_retries {
        match client.send_and_confirm_transaction(&[ix.clone()], &[signer]) {
            Ok(sig) => return Ok(sig),
            Err(e) if attempt < max_retries - 1 => {
                eprintln!("Intento {} falló: {}. Reintentando...", attempt + 1, e);
                sleep(Duration::from_secs(2_u64.pow(attempt))).await;
            }
            Err(e) => return Err(e),
        }
    }
    unreachable!()
}
```

### 3. Validar Entradas

```rust
fn validate_job_params(
    price_lamports: u64,
    timeout_seconds: i64,
    fhe_config: &Option<FheConsensusConfig>,
) -> Result<()> {
    if price_lamports < 1_000_000 {
        anyhow::bail!("Precio muy bajo: mínimo 0.001 SOL");
    }

    if timeout_seconds < 60 || timeout_seconds > 3600 {
        anyhow::bail!("Timeout debe estar entre 60 y 3600 segundos");
    }

    if let Some(config) = fhe_config {
        if config.consensus_threshold > config.required_provers {
            anyhow::bail!("Umbral de consenso no puede exceder provers requeridos");
        }
    }

    Ok(())
}
```

## Próximos Pasos

- **[Referencia de API](referencia-api.md)** - Documentación completa de la API
- **[Guía de Ejemplos](ejemplos.md)** - Ejemplos de integración del mundo real
- **[Visión General de Arquitectura](../arquitectura/vision-general.md)** - Entender el diseño del sistema

## Soporte

Para preguntas sobre el SDK:
- GitHub Issues: [Reportar un bug](https://github.com/yourusername/zyberlink/issues)
- Discussions: [Hacer preguntas](https://github.com/yourusername/zyberlink/discussions)
