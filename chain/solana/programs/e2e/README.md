# E2E Integration Tests

Tests de integración end-to-end para verificar el flujo completo de ZK y FHE proofs, incluyendo las interacciones CPI entre programas.

## Estructura

```
e2e/
├── Cargo.toml                    # Configuración del package de tests
├── tests/
│   ├── e2e_tests.rs             # Entry point (único test binary que Cargo detecta)
│   ├── e2e/                      # Módulo con todos los tests (subdirectorio para evitar autodiscovery)
│   │   ├── mod.rs               # Declara submódulos
│   │   ├── common.rs            # Helpers y utilities compartidas
│   │   ├── zk_flow_tests.rs    # Tests del flujo ZK
│   │   ├── fhe_flow_tests.rs   # Tests del flujo FHE
│   │   └── cpi_integration_tests.rs  # Tests de CPI entre programas
│   └── example_implementation.rs.example  # Ejemplo de implementación
└── README.md                     # Este archivo
```

## Tests Disponibles

### ZK Flow Tests (`zk_flow_tests.rs`)

Tests del flujo completo de ZK proofs:

- `test_zk_full_flow()` - Flujo completo: create job → claim → submit proof
- `test_zk_claim_unregistered_prover_fails()` - Verificar que prover no registrado no puede hacer claim
- `test_zk_claim_insufficient_stake_fails()` - Verificar requisito de stake mínimo
- `test_zk_submit_invalid_proof_fails()` - Submit proof inválida y slashing
- `test_zk_multiple_claims_same_prover_fails()` - Prevenir múltiples claims del mismo prover
- `test_zk_claim_timeout()` - Manejo de timeouts
- `test_zk_proof_verification_success()` - Verificación exitosa de proof

### FHE Flow Tests (`fhe_flow_tests.rs`)

Tests del flujo FHE con coordinación multi-prover:

- `test_fhe_multi_prover_flow()` - Flujo completo con múltiples provers
- `test_fhe_threshold_not_reached()` - Threshold no alcanzado
- `test_fhe_claim_unregistered_prover_fails()` - Prover no registrado
- `test_fhe_submit_without_claim_fails()` - Submit sin claim previo
- `test_fhe_duplicate_claim_fails()` - Prevenir claims duplicados
- `test_fhe_finalize_insufficient_submissions()` - Finalize sin suficientes submissions
- `test_fhe_job_timeout()` - Manejo de timeouts
- `test_fhe_exact_threshold_success()` - Threshold exacto alcanzado
- `test_fhe_inconsistent_results()` - Manejo de resultados inconsistentes entre provers

### CPI Integration Tests (`cpi_integration_tests.rs`)

Tests de integración vía CPI entre programas:

- `test_bedrock_prover_registration()` - Registro de prover en bedrock
- `test_cpi_update_stats_success()` - Actualización de stats via CPI
- `test_cpi_slash_prover()` - Slashing via CPI por proof inválida
- `test_cpi_update_stats_from_fhe()` - Update stats desde FHE generator
- `test_cpi_validate_prover_on_claim()` - Validación de prover en claim
- `test_cpi_suspended_prover_cannot_claim()` - Prover suspendido no puede hacer claim
- `test_cpi_minimum_stake_enforcement()` - Enforcement de stake mínimo
- `test_multiple_cpi_calls_in_flow()` - Múltiples CPIs en un flujo
- `test_cpi_error_propagation()` - Propagación de errores de CPI
- `test_cpi_with_wrong_accounts()` - Error handling con accounts incorrectas
- `test_cpi_stats_increment_correctness()` - Verificar incrementos correctos de stats

## Common Helpers

El módulo `common.rs` provee:

### Funciones de Setup
- `setup_test_environment()` - Configura ProgramTest con todos los programas
- `create_and_fund_keypair()` - Crea y fondea keypairs para tests
- `initialize_bedrock()` - Inicializa bedrock con config básica
- `register_prover()` - Registra un prover en bedrock

### PDA Helpers
- `get_bedrock_config_pda()` - Deriva PDA de config de bedrock
- `get_prover_registry_pda()` - Deriva PDA de prover registry
- `get_zk_job_pda()` - Deriva PDA de job ZK
- `get_zk_claim_pda()` - Deriva PDA de claim ZK
- `get_fhe_job_pda()` - Deriva PDA de job FHE
- `get_fhe_worker_pda()` - Deriva PDA de worker FHE

### Constantes
- `BEDROCK_PROGRAM_ID` - Program ID para tests
- `ZK_GENERATOR_PROGRAM_ID` - Program ID para tests
- `FHE_GENERATOR_PROGRAM_ID` - Program ID para tests

## Ejecutar Tests

```bash
# Todos los tests E2E
cd src/programs
cargo test --package e2e-tests

# Tests específicos por módulo
cargo test --package e2e-tests --test e2e -- zk_flow_tests
cargo test --package e2e-tests --test e2e -- fhe_flow_tests
cargo test --package e2e-tests --test e2e -- cpi_integration_tests

# Test específico
cargo test --package e2e-tests --test e2e -- test_zk_full_flow

# Con output verboso
cargo test --package e2e-tests -- --nocapture

# Con logs
RUST_LOG=debug cargo test --package e2e-tests
```

## Próximos Pasos

Los tests actualmente están estructurados con `todo!()` macros. Para completarlos:

1. **Implementar helpers en `common.rs`**:
   - `initialize_bedrock()` - Crear instrucción de inicialización
   - `register_prover()` - Crear instrucción de registro

2. **Completar tests de ZK flow**:
   - Construir instrucciones usando zk-generator program
   - Generar proofs válidas para tests de verificación
   - Implementar verificación de estados esperados

3. **Completar tests de FHE flow**:
   - Construir instrucciones de FHE coordinator
   - Simular múltiples provers trabajando en paralelo
   - Implementar lógica de threshold y finalization

4. **Completar tests de CPI**:
   - Verificar que CPIs se ejecutan correctamente
   - Validar propagación de errores
   - Verificar actualizaciones de estado via CPI

## Arquitectura de Tests

Los tests usan `solana-program-test` que provee:

- **ProgramTest**: Environment de testing in-memory
- **BanksClient**: Cliente para interactuar con programs
- **ProgramTestContext**: Contexto con payer, blockhash, etc.

### Pattern Típico

```rust
#[tokio::test]
async fn test_example() {
    // 1. Setup environment
    let program_test = setup_test_environment();
    let mut context = program_test.start_with_context().await;

    // 2. Setup actors (admin, provers, users)
    let admin = Keypair::new();
    let prover = create_and_fund_keypair(&mut context, 10_000_000_000).await;

    // 3. Initialize bedrock
    initialize_bedrock(&mut context, &admin).await.unwrap();

    // 4. Execute test flow
    // ... create jobs, claims, submissions ...

    // 5. Verify final state
    let account = context.banks_client.get_account(pda).await.unwrap().unwrap();
    // assert on account data
}
```

## Notas

- Los Program IDs son específicos para tests (no son los de devnet/mainnet)
- Cada test debe ser independiente y no depender de estado de otros tests
- Usar `warp_to_slot()` para simular el paso del tiempo en tests de timeout
- Los tests pueden correr en paralelo, asegurar que no compartan estado
