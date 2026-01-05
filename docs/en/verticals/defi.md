# DeFi Vertical

The DeFi vertical provides privacy-preserving financial primitives.

## Features

- **Private Markets**: Trading and betting where order details can be hidden.
- **Blind Betting**: Gambling mechanisms where bets are encrypted.
- **Private Balances**: Management of token balances without revealing amounts publicly.

## Architecture

### Circuits (`zyb-circuits/`)
- `zyb-circuits/market/`: ZK circuits for validating market bets.
- `zyb-circuits/blind/`: Logic for blind betting operations.
- `zyb-circuits/private_balance/`: Operations for encrypted balance transfers.

### Chains

#### Solana (`zyb-chain/solana/`)
- `programs/bedrock`: Core DeFi primitives for SVM.
- `programs/sdks/bedrock-sdk`: Integration tools for Solana apps.

#### Starknet (pBTCFi Integration)
- Core logic for private Bitcoin finance located in `zyb-chain/starknet`.

## Technical Status

- [x] **Private Markets**: Core circuits implemented in Circom.
- [x] **Consensus Logic**: Integrated with multi-prover network.
- [ ] **Yield Aggregator**: FHE-based yield optimization (In Research).
- [x] **Solana Program**: Bedrock deployed to Devnet.

