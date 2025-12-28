# Chain - Aptos Core Contracts

Core infrastructure contracts used across all verticals.

## Contracts

- `zk_verifier.move` - Groth16 ZK proof verification
- `jobs.move` - Job registration and management system

## Building

```bash
cd chain/aptos
aptos move compile --skip-fetch-latest-git-deps
```

## Testing

```bash
aptos move test
```

## Deployment

See `../../deploy/scripts/deploy-aptos-core.sh`
