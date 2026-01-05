# Identity Vertical

Privacy-first identity and compliance solutions.

## Features

- **Proof of Innocence (POI)**: Proving a user is not in a blacklist (e.g., sanctions list) without revealing their identity.
- **Portfolio Compliance**: Verifying portfolio requirements (e.g., "net worth > $X") without revealing the exact amount.

## Architecture

### Circuits (`zyb-circuits/`)
- `zyb-circuits/poi/proof_of_innocence.circom`: The core circuit for generating POI proofs.
- `zyb-circuits/portfolio/compliance.circom`: Circuit for checking portfolio constraints.
- `zyb-circuits/portfolio/net_worth.circom`: Circuit for net worth validation.

### SDK
- `zyb-sdks/identity-sdk`: Rust/TypeScript tools for generating proofs and interacting with verifiers.

## Technical Status

- [x] **Proof of Innocence**: Baseline POI flow with FHE CountIf support.
- [x] **Compliance Kit**: SDK for easy integration with financial apps.
- [ ] **Biometric ZK**: Identity anchoring via biometric proofs (Research).
- [x] **Sanctions Verification**: Automated checks against global lists.

