# Identity Vertical

Proof-of-Innocence (POI), compliance, portfolio verification, and identity primitives.

## Components

### Circuits (`circuits/`)
- `poi/proof_of_innocence.circom` - Privacy-preserving compliance proofs
- `portfolio/compliance.circom` - Portfolio compliance verification
- `portfolio/net_worth.circom` - Net worth proofs

### SDK (`sdk/`)
- **TypeScript**: `@zyberlink/compliance-kit` - Compliance SDK

## Future

### Solana
POI program for Solana (planned).

### Aptos
Identity registry contract (planned).

### Starknet
POI verification on Starknet (planned).

## Building

### Circuits
```bash
cd circuits/poi
circom proof_of_innocence.circom --r1cs --wasm --sym
```

### SDK
```bash
cd sdk/typescript && npm install && npm run build
```
