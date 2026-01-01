# Chain Layer

Core blockchain infrastructure.

## Directory Structure

```
chain/
├── solana/                    # Solana BPF workspace
│   └── programs/
│       ├── bedrock/           # Main ZK verifier
│       ├── zk-generator/      # ZK proof generation
│       ├── fhe-generator/     # FHE encryption
│       ├── threshold/         # Threshold cryptography
│       ├── futarchy-markets/  # Governance markets
│       ├── zyberlink/         # DEPRECATED - legacy monolith
│       ├── sdks/              # Program SDKs
│       └── e2e/               # Integration tests
├── aptos/                     # Aptos Move contracts
│   └── sources/
│       ├── zk_verifier.move   # Groth16 verification
│       └── jobs.move          # Job management
└── starknet/                  # Starknet Cairo contracts
    └── src/
        ├── fhe_verifier.cairo
        ├── jobs.cairo
        └── crypto_lib.cairo
```

## Notes

- `zyberlink/` is deprecated, use modular programs instead
- `futarchy-markets/` is governance-specific but kept here for Solana workspace cohesion

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
