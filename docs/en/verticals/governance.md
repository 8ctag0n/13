# Governance Vertical

This vertical implements Futarchy-based governance and private voting systems.

## Features

- **Anonymous Voting**: Users can vote without revealing their specific choice publicly, while ensuring the vote is counted correctly.
- **Proof of Innocence (POI) Voting**: Voting gates that require compliance proofs.
- **Futarchy Markets**: Prediction markets used to decide governance outcomes based on asset price performance.

## Architecture

### Circuits (`circuits/vote`)
- `private_vote.circom`: Core logic for anonymous voting using ZK proofs.
- `private_vote_poi.circom`: Extended voting logic incorporating Proof-of-Innocence checks.

### Chains

#### Solana
- `futarchy-markets`: Rust-based programs for handling prediction markets on SVM.

#### Aptos
- `marketplace.move`: Move smart contracts implementing the prediction marketplace.

## Status
- **Circuits**: Private vote circuits implemented.
- **Solana**: Futarchy market programs in development.
- **Aptos**: Marketplace contracts available.
- **SDK**: TypeScript and Rust SDKs available for integration.
