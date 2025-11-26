# Self-Executing Tests

## Overview

Los tests self-executing usan `solana-program-test` para correr un validator in-memory, eliminando la necesidad de tener un validator externo corriendo.

**Ventajas:**
- No requiere `solana-test-validator` corriendo
- Rápido (carga programa en memoria)
- Determinístico (mismo estado inicial siempre)
- Perfecto para CI/CD
- Fácil debugging

## Setup

El setup está en `tests/common/program_test.rs` y provee helpers:

```rust
use common::{
    setup_test_environment,
    setup_initialized_marketplace,
    register_test_prover,
    create_test_fhe_job,
};
```

## Uso

### 1. Test Básico

```rust
use common::setup_initialized_marketplace;

#[tokio::test]
async fn test_my_feature() -> Result<()> {
    // Setup auto-ejecutable (no validator externo)
    let mut ctx = setup_initialized_marketplace().await?;

    // Usar ctx.banks_client para queries
    // Usar ctx.execute_transaction() para txs

    Ok(())
}
```

### 2. Registrar Prover

```rust
use common::{setup_initialized_marketplace, register_test_prover};
use solana_sdk::signature::Keypair;

#[tokio::test]
async fn test_with_prover() -> Result<()> {
    let mut ctx = setup_initialized_marketplace().await?;

    let prover = Keypair::new();
    let prover_pda = register_test_prover(
        &mut ctx,
        &prover,
        5_000_000_000,  // stake
    ).await?;

    // prover está registrado y funded
    Ok(())
}
```

### 3. Crear FHE Job

```rust
use common::{setup_initialized_marketplace, create_test_fhe_job};

#[tokio::test]
async fn test_fhe_job() -> Result<()> {
    let mut ctx = setup_initialized_marketplace().await?;

    let creator = Keypair::new();
    let job_pda = create_test_fhe_job(
        &mut ctx,
        &creator,
        3,  // required_provers
        2,  // consensus_threshold
    ).await?;

    // Job está creado y funded
    Ok(())
}
```

### 4. Ejecutar Transacciones

```rust
#[tokio::test]
async fn test_custom_instruction() -> Result<()> {
    let mut ctx = setup_initialized_marketplace().await?;

    let sdk = ctx.sdk_client();
    let prover = Keypair::new();
    let job_pda = /* ... */;

    // Crear instrucción
    let claim_ix = sdk.claim_job_instruction(
        &prover.pubkey(),
        &job_pda,
    )?;

    // Ejecutar
    ctx.execute_transaction(&[claim_ix], &[&prover]).await?;

    Ok(())
}
```

### 5. Verificar Estado On-Chain

```rust
use borsh::BorshDeserialize;
use zyberlink_types::JobAccount;

#[tokio::test]
async fn test_verify_state() -> Result<()> {
    let mut ctx = setup_initialized_marketplace().await?;
    let job_pda = /* ... */;

    // Query account
    let account = ctx.banks_client
        .get_account(job_pda)
        .await?
        .expect("Account should exist");

    // Deserialize
    let job: JobAccount = borsh::from_slice(&account.data)?;

    // Assert state
    assert_eq!(job.status, JobStatus::Claimed);

    Ok(())
}
```

## Ejemplos Completos

Ver `tests/fhe_self_executing.rs` para ejemplos completos:

- `test_create_fhe_job_self_executing` - Crear job FHE
- `test_multi_prover_claiming_self_executing` - Multi-prover claiming
- `test_fhe_result_submission_self_executing` - Submit FHE results
- `test_consensus_finalization_self_executing` - Consenso completo

## Ejecutar Tests

```bash
# Todos los tests self-executing
cargo test --test fhe_self_executing

# Test específico
cargo test --test fhe_self_executing test_create_fhe_job_self_executing -- --nocapture

# Con output verbose
cargo test --test fhe_self_executing -- --nocapture --test-threads=1
```

## Debugging

Los tests self-executing usan el mismo programa que production:

```rust
ProgramTest::new(
    "zyberlink",
    program_id,
    processor!(zyberlink::entrypoint::process_instruction),
)
```

Si un test falla:
1. Revisar logs del test (usar `--nocapture`)
2. Los errores del programa aparecen en `ProcessTransactionError`
3. Usar `msg!()` en el programa para debugging

## Diferencias vs Tests con Validator

| Feature | Self-Executing | External Validator |
|---------|---------------|-------------------|
| Speed | Rápido (~1-2s) | Lento (~10-30s) |
| Setup | Automático | Manual (start validator) |
| State | Fresh cada test | Compartido |
| Debugging | Fácil | Más complejo |
| CI/CD | Perfecto | Requiere setup |

## Cuándo Usar Cada Tipo

**Self-Executing (Preferir):**
- Tests unitarios del programa
- CI/CD pipelines
- Desarrollo rápido
- Tests determinísticos

**External Validator:**
- Tests de integración full-stack
- Testing con prover nodes reales
- Debugging de issues específicos de devnet/mainnet
- Performance testing bajo carga

## Troubleshooting

### Error: "Program account not found"

El programa no está cargado. Verificar que `zyberlink` está en dependencias del Cargo.toml.

### Error: "Transaction failed"

Revisar logs con `--nocapture`. El error del programa aparecerá en el output.

### Tests muy lentos

Verificar que no estés ejecutando FHE computations en el test path crítico. Esas pueden tomar ~40s.

### BanksClient vs RpcClient

`BanksClient` es la versión in-memory de `RpcClient`:
- Usa `get_account()` en lugar de `get_account_data()`
- Usa `process_transaction()` en lugar de `send_and_confirm_transaction()`
- No soporta airdrop (usar `fund_account()` helper)

---

**Tip:** Para máxima velocidad en CI, correr tests self-executing en paralelo y tests con validator secuencialmente.
