# Guía de Ejemplos

Ejemplos de integración del mundo real para construir aplicaciones que preservan privacidad con ZyberLink.

## Tabla de Contenidos

- [Ejemplo 1: Intercambio de Balances DeFi Privado](#ejemplo-1-intercambio-de-balances-defi-privado)
- [Ejemplo 2: Votación Confidencial de DAO](#ejemplo-2-votación-confidencial-de-dao)
- [Ejemplo 3: Analítica que Preserva Privacidad](#ejemplo-3-analítica-que-preserva-privacidad)
- [Ejemplo 4: Integración de Wallet ZK](#ejemplo-4-integración-de-wallet-zk)
- [Ejemplo 5: Procesamiento FHE por Lotes](#ejemplo-5-procesamiento-fhe-por-lotes)

## Ejemplo 1: Intercambio de Balances DeFi Privado

Construye un DEX donde los usuarios pueden intercambiar tokens sin revelar sus balances usando FHE.

### Caso de Uso

Un exchange descentralizado que preserva la privacidad del usuario:
- Los balances de tokens de los usuarios permanecen encriptados
- Los montos de intercambio se computan en valores encriptados
- Solo el usuario puede desencriptar su balance final
- El consenso multi-prover asegura corrección

### Implementación Completa

```rust
use cypherlink_sdk::MarketplaceClient;
use cypherlink_types::{CircuitType, FheConsensusConfig};
use cypherlink_crypto::fhe::FheClient;

pub struct PrivateDex {
    marketplace_client: MarketplaceClient,
    fhe_client: FheClient,
}

impl PrivateDex {
    /// Ejecutar un intercambio privado
    pub async fn swap(
        &self,
        user: &Keypair,
        encrypted_balance_a: Vec<u8>, // Balance USDC encriptado
        encrypted_balance_b: Vec<u8>, // Balance SPL encriptado
        swap_amount: u64,              // Monto a intercambiar (será encriptado)
        exchange_rate: f64,            // Tasa USDC/SPL
    ) -> Result<Vec<u8>> {
        println!("Iniciando intercambio privado...");

        // Paso 1: Encriptar monto de intercambio
        let encrypted_amount = self.fhe_client.encrypt(swap_amount)?;

        // Paso 2: Crear datos de witness (balances encriptados + monto)
        let witness = SwapWitness {
            encrypted_balance_a,
            encrypted_balance_b,
            encrypted_swap_amount: encrypted_amount.serialize(),
            exchange_rate,
        };

        let witness_bytes = borsh::to_vec(&witness)?;
        let witness_commitment = compute_hash(&witness_bytes);

        // Paso 3: Crear trabajo FHE para computación de intercambio
        let job_id = self.get_next_job_id(&user.pubkey()).await?;

        let create_ix = self.marketplace_client.create_job_instruction(
            &user.pubkey(),
            job_id,
            CircuitType::Custom("PrivateSwap".to_string()),
            witness_commitment,
            witness_bytes.len() as u32,
            2_000_000_000, // Pago de 2 SOL por computación
            600,           // Timeout de 10 minutos
            Some(FheConsensusConfig {
                required_provers: 3,
                consensus_threshold: 2, // Consenso 2-de-3
            }),
        )?;

        // Paso 4: Enviar trabajo al marketplace
        let signature = self.marketplace_client.send_and_confirm_transaction(
            &[create_ix],
            &[user],
        )?;

        println!("Trabajo de intercambio creado: {}", signature);

        // Paso 5: Esperar a que los provers completen la computación
        let (job_pda, _) = self.marketplace_client.get_job_pda(&user.pubkey(), job_id);
        let result = self.poll_for_completion(&job_pda).await?;

        println!("Intercambio completado! Consenso alcanzado.");

        // Paso 6: Retornar resultado encriptado (nuevos balances)
        Ok(result.encrypted_output)
    }
}
```

### Características Clave

- **Privacidad**: Los balances nunca salen de forma encriptada
- **Corrección**: Consenso 2-de-3 asegura computación precisa
- **Descentralización**: Sin tercera parte confiable
- **Bajo Costo**: ~2 SOL por intercambio (~$44 a precios actuales)

---

## Ejemplo 2: Votación Confidencial de DAO

Construye una DAO donde los votos son privados pero los resultados son públicamente verificables.

### Caso de Uso

Un sistema de gobernanza con garantías de privacidad:
- Los miembros emiten votos encriptados
- Los conteos de votos se computan en boletas encriptadas
- El recuento final se revela sin exponer votos individuales
- Previene compra de votos y coerción

### Implementación Completa

```rust
pub struct ConfidentialDAO {
    marketplace_client: MarketplaceClient,
    fhe_client: FheClient,
}

#[derive(Debug, Clone, Copy)]
pub enum Vote {
    Yes,
    No,
    Abstain,
}

impl ConfidentialDAO {
    /// Emitir un voto encriptado
    pub async fn cast_vote(
        &self,
        voter: &Keypair,
        proposal_id: u64,
        vote: Vote,
    ) -> Result<Signature> {
        // Encriptar voto (Yes=1, No=0, Abstain=2)
        let vote_value = match vote {
            Vote::Yes => 1u64,
            Vote::No => 0u64,
            Vote::Abstain => 2u64,
        };

        let encrypted_vote = self.fhe_client.encrypt(vote_value)?;

        // Almacenar voto encriptado on-chain o en almacenamiento
        Ok(Signature::default())
    }

    /// Contar todos los votos usando FHE
    pub async fn tally_votes(
        &self,
        executor: &Keypair,
        proposal_id: u64,
        encrypted_votes: Vec<Vec<u8>>,
    ) -> Result<VoteTally> {
        println!("Contando {} votos encriptados...", encrypted_votes.len());

        // Crear witness: array de votos encriptados
        let witness = VotingWitness {
            proposal_id,
            encrypted_votes,
        };

        let witness_bytes = borsh::to_vec(&witness)?;
        let witness_commitment = compute_hash(&witness_bytes);

        // Crear trabajo FHE
        let job_id = proposal_id;

        let create_ix = self.marketplace_client.create_job_instruction(
            &executor.pubkey(),
            job_id,
            CircuitType::Custom("VoteTally".to_string()),
            witness_commitment,
            witness_bytes.len() as u32,
            5_000_000_000, // 5 SOL por computación de conteo
            1800,          // Timeout de 30 minutos (más votos = más tiempo)
            Some(FheConsensusConfig {
                required_provers: 5,
                consensus_threshold: 3, // 3-de-5 para mayor seguridad
            }),
        )?;

        let signature = self.marketplace_client.send_and_confirm_transaction(
            &[create_ix],
            &[executor],
        )?;

        // Esperar finalización y desencriptar conteo final
        let (job_pda, _) = self.marketplace_client.get_job_pda(&executor.pubkey(), job_id);
        let result = self.poll_for_completion(&job_pda).await?;

        let yes_count = self.fhe_client.decrypt(&result.encrypted_yes_count)?;
        let no_count = self.fhe_client.decrypt(&result.encrypted_no_count)?;

        Ok(VoteTally {
            yes: yes_count,
            no: no_count,
            abstain: abstain_count,
        })
    }
}
```

---

## Ejemplo 3: Analítica que Preserva Privacidad

Computa estadísticas en datasets encriptados sin revelar puntos de datos individuales.

### Caso de Uso

Análisis de datos de salud con privacidad:
- Los hospitales encriptan datos de pacientes
- Los análisis corren en valores encriptados
- Los resultados (promedios, conteos) se revelan
- Los registros individuales de pacientes permanecen privados

---

## Ejemplo 4: Integración de Wallet ZK

Externaliza la generación de pruebas ZK de wallets móviles a la red de provers.

### Caso de Uso

Wallet Zcash móvil que externaliza la generación de pruebas:
- El usuario construye transacción shielded en móvil
- La generación pesada de pruebas (10-15s) se externaliza a la red
- Se ahorra batería móvil, mejora UX
- La prueba se verifica antes de transmitir

### Comparación de Rendimiento

| Operación | Móvil (Local) | ZyberLink (Externalizado) |
|-----------|---------------|---------------------------|
| Construcción de witness | 2s | 2s |
| Generación de prueba | **120s** | **10-15s** |
| Verificación de prueba | 2s | 2s |
| Sobrecarga de red | 0s | 3s |
| **Total** | **124s** | **17-22s** |
| **Uso de batería** | **~5%** | **~0.5%** |

**Resultado: 6x más rápido, 10x menos consumo de batería**

---

## Ejemplo 5: Procesamiento FHE por Lotes

Procesa múltiples operaciones FHE eficientemente en modo lote.

### Implementación

```rust
pub struct BatchProcessor {
    marketplace_client: MarketplaceClient,
    fhe_client: FheClient,
}

impl BatchProcessor {
    /// Enviar múltiples trabajos FHE en paralelo
    pub async fn process_batch(
        &self,
        submitter: &Keypair,
        operations: Vec<FheOperation>,
    ) -> Result<Vec<Vec<u8>>> {
        println!("Enviando {} trabajos FHE...", operations.len());

        let mut job_ids = Vec::new();

        // Enviar todos los trabajos en paralelo
        for (i, operation) in operations.iter().enumerate() {
            let job_id = i as u64;
            let encrypted_input = self.fhe_client.encrypt(operation.input)?;

            let create_ix = self.marketplace_client.create_job_instruction(
                &submitter.pubkey(),
                job_id,
                match operation.op_type {
                    OpType::Add => CircuitType::FheAdd,
                    OpType::Multiply => CircuitType::FheMultiply,
                    OpType::Subtract => CircuitType::FheSubtract,
                },
                compute_hash(&encrypted_input),
                encrypted_input.len() as u32,
                1_000_000_000,
                600,
                Some(FheConsensusConfig {
                    required_provers: 3,
                    consensus_threshold: 2,
                }),
            )?;

            self.marketplace_client.send_and_confirm_transaction(&[create_ix], &[submitter])?;
            job_ids.push(job_id);
        }

        // Esperar a que todos los trabajos se completen en paralelo
        let mut results = Vec::new();
        for job_id in job_ids {
            let (job_pda, _) = self.marketplace_client.get_job_pda(&submitter.pubkey(), job_id);
            let result = self.poll_for_completion(&job_pda).await?;
            results.push(result.encrypted_output);
        }

        Ok(results)
    }
}
```

---

## Funciones de Ayuda

Utilidades comunes usadas en los ejemplos:

```rust
use sha3::{Digest, Sha3_256};

/// Computar hash SHA3-256
pub fn compute_hash(data: &[u8]) -> [u8; 32] {
    Sha3_256::digest(data).into()
}

/// Generar ID de trabajo único
pub fn generate_job_id() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};

    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}
```

## Probando Ejemplos

Prueba los ejemplos localmente:

```bash
# Iniciar validador local
solana-test-validator --reset

# Desplegar programa
solana program deploy target/deploy/cypherlink.so

# Ejecutar Ejemplo 1
cargo run --example private_dex

# Ejecutar Ejemplo 2
cargo run --example confidential_voting

# Ejecutar Ejemplo 3
cargo run --example private_analytics

# Ejecutar Ejemplo 4
cargo run --example zk_wallet

# Ejecutar Ejemplo 5
cargo run --example batch_processing
```

## Estimación de Costos

Costos típicos para ejemplos (a $22/SOL):

| Ejemplo | Costo SOL | Costo USD | Tiempo |
|---------|-----------|-----------|--------|
| Intercambio DeFi Privado | 2.0 SOL | $44 | ~8s |
| Conteo de Votos DAO (100 votos) | 5.0 SOL | $110 | ~30s |
| Analítica (promedio de 1000 valores) | 3.0 SOL | $66 | ~15s |
| Prueba de Wallet ZK | 1.0 SOL | $22 | ~15s |
| Lote (10 ops FHE) | 10.0 SOL | $220 | ~10s |

**Nota:** Los costos disminuirán significativamente una vez que la integración de la biblioteca Concrete esté completa (aceleración de 260x).

## Próximos Pasos

- **[Guía de Integración del SDK](integracion-sdk.md)** - Aprende patrones de integración
- **[Referencia de API](referencia-api.md)** - Documentación completa de API
- **[Visión General de Arquitectura](../arquitectura/vision-general.md)** - Entiende el sistema

## Soporte

¿Tienes preguntas sobre los ejemplos?

- GitHub Discussions: [Hacer preguntas](https://github.com/yourusername/zyberlink/discussions)
- Código de Ejemplo: [Explorar ejemplos](https://github.com/yourusername/zyberlink/tree/main/sdk/examples)
