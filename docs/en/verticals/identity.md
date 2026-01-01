# Identity Vertical

Privacy-first identity and compliance solutions.

## Features

- **Proof of Innocence (POI)**: Proving a user is not in a blacklist (e.g., sanctions list) without revealing their identity.
- **Portfolio Compliance**: Verifying portfolio requirements (e.g., "net worth > $X") without revealing the exact amount.

## Architecture

### Circuits (`circuits/`)
- `poi/proof_of_innocence.circom`: The core circuit for generating POI proofs.
- `portfolio/compliance.circom`: Circuit for checking portfolio constraints.
- `portfolio/net_worth.circom`: Circuit for net worth validation.

### SDK
- `@zyberlink/compliance-kit`: TypeScript tools for generating proofs and interacting with verifiers.

## Status
- **Circuits**: Core POI and Portfolio circuits implemented.
- **Integration**: Planned integration with Solana, Aptos, and Starknet verifiers.
