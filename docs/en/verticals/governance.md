# Governance Vertical

This vertical implements Futarchy-based governance and private voting systems.

## Features

- **Anonymous Voting**: Users can vote without revealing their specific choice publicly, while ensuring the vote is counted correctly.
- **Proof of Innocence (POI) Voting**: Voting gates that require compliance proofs.
- **Futarchy Markets**: Prediction markets used to decide governance outcomes based on asset price performance.

## Architecture

### Circuits (`zyb-circuits/vote`)
- `zyb-circuits/vote/private_vote.circom`: Core logic for anonymous voting using ZK proofs.
- `zyb-circuits/vote/private_vote_poi.circom`: Extended voting logic incorporating Proof-of-Innocence checks.

### Chains

#### Solana (`zyb-chain/solana/`)
- `programs/futarchy-markets`: Rust-based programs for handling prediction markets on SVM.

## Technical Status

- [x] **Anonymous Voting**: Circom circuits for privacy-preserving voting.
- [x] **Futarchy Support**: On-chain prediction markets for decision making.
- [ ] **Delegate Privacy**: FHE-based voting power delegation (Development).
- [x] **Solana Integration**: Registry for DAO configurations.

