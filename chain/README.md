# Chain Layer

Core blockchain infrastructure used across ALL verticals.

## Directory Structure

```
chain/
├── solana/          # Solana BPF programs (core)
│   ├── bedrock/          # Main ZK verifier
│   ├── zk-generator/     # ZK proof generation service
│   ├── fhe-generator/    # FHE encryption service
│   └── threshold/        # Threshold cryptography
├── aptos/           # Aptos Move contracts (core)
│   ├── zk_verifier.move  # Groth16 verification
│   └── jobs.move         # Job management
└── starknet/        # Starknet Cairo contracts (core)
    ├── fhe_verifier.cairo
    ├── jobs.cairo
    └── crypto_lib.cairo
```

## Principles

**Chain layer contains ONLY infrastructure used by MULTIPLE verticals.**

If a contract/program is vertical-specific (e.g., futarchy-markets for governance), it belongs in `verticals/<domain>/`.

## Building

### Solana
```bash
cd chain/solana
cargo build-sbf
```

### Aptos
```bash
cd chain/aptos
aptos move compile
```

### Starknet
```bash
cd chain/starknet
scarb build
```

## Testing

```bash
# Solana
cd chain/solana && cargo test

# Aptos
cd chain/aptos && aptos move test

# Starknet
cd chain/starknet && scarb test
```
