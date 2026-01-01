# DeFi Vertical

Decentralized finance primitives: lending, markets, liquidity, and private Bitcoin finance (pBTCFi).

## Components

### Circuits (`circuits/`)
- `market/` - Market betting circuits
- `blind/` - Blind betting (privacy-preserving)
- `private_balance/` - Private balance management

### Aptos (`aptos/`)
- **Contracts**
  - `lending_manager.move` - Lending protocol manager
  - `collateral_vault.move` - Collateral management
  - `plst_token.move` - Protocol token

### Starknet (`starknet/`)
- **Contracts** (pBTCFi - Private Bitcoin Finance)
  - `core/pbtcfi_core.cairo` - Core pBTCFi logic
  - `managers/collateral_manager.cairo` - Collateral management
  - `managers/loan_manager.cairo` - Loan lifecycle
  - `liquidation.cairo` - Liquidation engine
  - `tokens/` - Token implementations (pLST, mock WBTC)
- **Types**: Rust types for Starknet interaction

## Building

### Circuits
```bash
cd circuits/market
circom market_bet.circom --r1cs --wasm --sym
```

### Aptos
```bash
cd aptos
aptos move compile
```

### Starknet
```bash
cd starknet
scarb build
```

### SDK
```bash
cd sdk/typescript && npm install && npm run build
```

## Multi-Chain Support

| Feature | Aptos | Starknet |
|---------|-------|----------|
| Lending | ✅ | ✅ (pBTCFi) |
| Collateral Management | ✅ | ✅ |
| Private Markets | ⚠️ (partial) | ✅ |
