# DeFi Vertical

The DeFi vertical provides privacy-preserving financial primitives.

## Features

- **Private Markets**: Trading and betting where order details can be hidden.
- **Blind Betting**: Gambling mechanisms where bets are encrypted.
- **Private Balances**: Management of token balances without revealing amounts publicly.

## Architecture

### Circuits (`circuits/`)
- `market/`: ZK circuits for validating market bets.
- `blind/`: Logic for blind betting operations.
- `private_balance/`: Operations for encrypted balance transfers.

### Chains

#### Aptos
- `lending_manager`: Manages the lending protocol state.
- `collateral_vault`: Securely holds collateral assets.
- `plst_token`: Implementation of the protocol token.

#### Starknet (pBTCFi Integration)
- Core logic for private Bitcoin finance.
- Managers for collateral and loan lifecycles.
- Liquidation engines.

## Status
- **Aptos**: Lending and Collateral management implemented.
- **Starknet**: Core logic and token implementations ready.
- **Circuits**: Market bet circuits available for compilation.
