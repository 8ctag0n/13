# pBTCFi (Private Bitcoin Finance)

A specialized vertical for private Bitcoin-backed finance on Starknet.

## Overview

pBTCFi enables lending and borrowing using Bitcoin as collateral while preserving user privacy regarding loan positions and liquidation thresholds.

## Architecture

### Starknet Contracts (`verticals/pbtcfi/contracts/`)
The core logic is implemented in Cairo for the Starknet zk-rollup.

- **Core Logic**: Handles the main interaction between borrowers and the protocol.
- **Collateral Manager**: Manages BTC collateral deposits and withdrawals.
- **Loan Manager**: Tracks loan health, interest accumulation, and repayment.
- **Liquidation**: Handles the logic for liquidating under-collateralized positions securely.

## Status
- **Contracts**: Cairo contracts structure is in place.
- **Integration**: Integrated with the broader DeFi vertical for cross-chain capabilities.
