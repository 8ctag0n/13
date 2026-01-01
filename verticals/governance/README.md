# Governance Vertical

Futarchy-based governance, private voting, and proposal systems.

## Components

### Circuits (`circuits/`)
- `vote/private_vote.circom` - Anonymous voting with ZK proofs
- `vote/private_vote_poi.circom` - Voting with proof-of-innocence

### Solana (`solana/`)
- **Programs**
  - `futarchy-markets` - Prediction markets for governance decisions
- **SDK**: On-chain interaction library

### Aptos (`aptos/`)
- **Contracts**
  - `marketplace.move` - Futarchy marketplace implementation

### SDK (`sdk/`)
- **Rust**: `governance-sdk` - Multi-chain governance SDK
- **TypeScript**: `@zyberlink/private-vote` - Browser/Node.js SDK

## Building

### Circuits
```bash
cd circuits/vote
circom private_vote.circom --r1cs --wasm --sym
```

### Solana
```bash
cd solana
cargo build-sbf
```

### Aptos
```bash
cd aptos
aptos move compile
```

### SDKs
```bash
# Rust
cd sdk/rust && cargo build

# TypeScript
cd sdk/typescript && npm install && npm run build
```

## CLI (Future)

Governance-specific CLI commands will be extracted from `../../cli/core/` to `cli/`.

## Prover Handler (Future)

Governance-specific proof generation logic will be extracted from `../../prover/core/` to `prover-handler/`.
