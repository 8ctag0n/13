# Chain - Starknet Core Contracts

Core infrastructure contracts (Cairo) used across all verticals.

## Contracts

- `fhe_verifier.cairo` - FHE proof verification
- `jobs.cairo` - Job registration and management
- `crypto_lib.cairo` - Cryptographic utilities (Poseidon, etc.)

## Building

```bash
cd chain/starknet
scarb build
```

## Testing

```bash
scarb test
```

## Deployment

See `../../deploy/scripts/deploy-starknet-core.sh`
