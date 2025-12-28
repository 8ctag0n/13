# Futarchy SDK - Unsigned Transaction Flow

## Overview

El SDK ahora soporta la construcción de transacciones sin firmar para el flujo de Blinks, permitiendo que el blink-server prepare transacciones que luego serán firmadas por el cliente.

## API Changes

### PlaceBet Instruction

Se agregó el campo `ciphertext_hash` para soportar el flujo FHE:

```rust
pub enum FutarchyInstruction {
    PlaceBet {
        market_id: u64,
        bet_commitment: [u8; 32],
        proof: Vec<u8>,
        public_inputs: Vec<u8>,
        amount: u64,
        circuit_type: u8,
        ciphertext_hash: Option<[u8; 32]>,  // NUEVO
        encrypted_bet_amount: Option<Vec<u8>>,
        side: Option<bool>,
    },
    // ...
}
```

### Instruction Builder

```rust
pub fn build_place_bet_ix(
    program_id: &Pubkey,
    bettor: &Pubkey,
    market_id: u64,
    bet_commitment: [u8; 32],
    proof: Vec<u8>,
    public_inputs: Vec<u8>,
    amount: u64,
    circuit_type: u8,
    ciphertext_hash: Option<[u8; 32]>,      // NUEVO
    encrypted_bet_amount: Option<Vec<u8>>,
    side: Option<bool>,
    zk_generator_program: &Pubkey,
    fhe_accounts: Option<FheAccounts>,
) -> Result<Instruction>
```

### Transaction Preparation

Nueva función para preparar transacciones sin firmar:

```rust
pub fn prepare_unsigned_transaction(
    instructions: &[Instruction],
    payer: &Pubkey,
) -> Result<String>
```

Esta función:
1. Crea un `Message` con las instrucciones y el payer
2. Construye un `Transaction` sin firmar
3. Serializa usando `bincode`
4. Retorna el resultado codificado en base64

## Usage in Blink Server

### Handler `/prepare`

```rust
use futarchy_sdk::*;
use solana_sdk::pubkey::Pubkey;

async fn prepare_transaction(
    program_id: Pubkey,
    bettor: Pubkey,
    market_id: u64,
    ciphertext_hash: [u8; 32],
    side: bool,
) -> Result<String, Error> {
    // Construir la instrucción
    let ix = build_place_bet_ix(
        &program_id,
        &bettor,
        market_id,
        bet_commitment,
        proof,
        public_inputs,
        amount,
        circuit_type,
        Some(ciphertext_hash),  // Solo el hash, no el ciphertext completo
        None,                   // No incluir encrypted_bet_amount aquí
        Some(side),
        &zk_program,
        fhe_accounts,
    )?;

    // Preparar transacción sin firmar
    let unsigned_tx = prepare_unsigned_transaction(&[ix], &bettor)?;

    Ok(unsigned_tx)  // Retorna base64 string
}
```

### Complete Flow

```
1. Cliente → POST /prepare { ciphertext_hash }
2. Blink-server:
   - Genera proof ZK
   - Construye instrucción con build_place_bet_ix()
   - Prepara TX sin firmar con prepare_unsigned_transaction()
3. Blink-server → Cliente: { transaction: "base64..." }
4. Cliente firma y envía TX a la red
```

## All Available Builders

El SDK incluye builders para todas las operaciones principales:

### Market Management
- `build_create_market_ix()` - Crear un nuevo mercado
- `build_settle_market_ix()` - Resolver un mercado
- `build_cancel_market_ix()` - Cancelar un mercado

### Betting Operations
- `build_place_bet_ix()` - Colocar una apuesta (con soporte FHE)
- `build_claim_payout_ix()` - Reclamar ganancias

### FHE Operations
- `build_update_pool_ix()` - Actualizar pools encriptados

### Governance
- `build_create_market_with_governance_ix()` - Mercado con acción ejecutable
- `build_register_user_ix()` - Registro de usuario con prueba de elegibilidad

### Escrow
- `build_deposit_to_market_ix()` - Depositar a escrow del mercado
- `build_withdraw_from_escrow_ix()` - Retirar de escrow

## Testing

Todos los builders están completamente testeados:

```bash
# Run all SDK tests
cargo test -p futarchy-sdk

# Run specific example
cargo run -p futarchy-sdk --example prepare_transaction
```

## Notes

- El `ciphertext_hash` es opcional para mantener retrocompatibilidad
- Para operaciones FHE, usar `ciphertext_hash` en lugar de `encrypted_bet_amount` en el flujo de Blinks
- El `encrypted_bet_amount` se puede usar para flows que no sean Blinks
- Todas las transacciones retornadas están serializadas en base64 para facilitar el transporte HTTP
