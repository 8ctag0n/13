# ZyberLink Wallet - Quick Start Guide

## Installation

### 1. Install Dependencies

```bash
cd src/wallet-extension
npm install
```

### 2. Build Extension

For development with hot reload:
```bash
npm run dev
```

For production build:
```bash
npm run build
```

### 3. Load in Chrome

1. Open Chrome
2. Navigate to `chrome://extensions`
3. Enable "Developer mode" (toggle in top-right)
4. Click "Load unpacked"
5. Select the `dist/` folder from this directory

### 4. Pin Extension

Click the puzzle icon in Chrome toolbar and pin ZyberLink Wallet for easy access.

## First Time Setup

### Create New Wallet

1. Click the ZyberLink extension icon
2. Click `[CREATE NEW WALLET]`
3. **IMPORTANT**: Write down your 12-word seed phrase on paper
4. Store it in a secure location (safe, password manager, etc)
5. Check the confirmation box
6. Click `[CONTINUE]`
7. Set a strong password (min 8 characters)
8. Click `[CREATE WALLET]`

### Using Your Wallet

#### View Balance
- Default view shows your Solana balance
- Click chain tabs (SOL/STRK/ZEC) to switch networks
- Click `[REFRESH]` to update balance

#### Send Transaction
1. Click `[SEND]`
2. Enter recipient address
3. Enter amount
4. Click `[SEND]`
5. Confirm in popup

#### Custom RPC
1. Click `[SETTINGS]`
2. Select chain tab
3. Click `[+ ADD CUSTOM RPC]`
4. Enter name and URL
5. Click `[ADD]`
6. Click `[USE]` to activate

#### Lock Wallet
Click `[LOCK]` button to lock wallet manually.
Auto-locks after 15 minutes of inactivity.

## Web3 Provider API

Websites can interact with your wallet via injected providers:

### Solana
```javascript
// Connect
const { publicKey } = await window.solana.connect();

// Sign message
const { signature } = await window.solana.signMessage(message);

// Sign transaction
const signedTx = await window.solana.signTransaction(transaction);

// Sign and send
const { signature } = await window.solana.signAndSendTransaction(transaction);
```

### Starknet
```javascript
// Connect
const { publicKey } = await window.starknet.connect();

// Sign message
const { signature } = await window.starknet.signMessage(message);
```

### ZyberLink Unified
```javascript
// Access all chains
window.zyberlink.solana
window.zyberlink.starknet
window.zyberlink.version // "0.1.0"
```

## Architecture Overview

```
Extension Components:
├── Background (service-worker.ts)
│   └── Handles wallet state, signing, RPC calls
├── Content Script (inject.ts)
│   └── Injects window.solana, window.starknet providers
└── Popup (Svelte UI)
    └── User interface for wallet management

Data Flow:
Web Page → Content Script → Background → Storage
                              ↓
                         Chain Adapters → RPC
```

## Security Best Practices

1. **Seed Phrase**: Never share, never type online, store offline
2. **Password**: Use strong, unique password
3. **Lock**: Lock wallet when not in use
4. **RPC**: Only add trusted RPC endpoints
5. **dApps**: Review transaction details before signing

## Troubleshooting

### Extension not appearing
- Ensure `npm run build` completed successfully
- Check `dist/` folder exists
- Try reloading extension in chrome://extensions

### Cannot connect to RPC
- Check RPC endpoint URL is correct
- Try switching to default endpoint
- Check network connection

### Transaction failing
- Verify sufficient balance for transaction + fees
- Check recipient address is valid
- Ensure RPC endpoint is healthy

### Balance showing 0
- Click `[REFRESH]` button
- Try switching RPC endpoint
- Check address has received funds

## Development

### File Structure
```
lib/
├── crypto/          # Keyring, encryption
├── chains/          # Chain-specific adapters
├── storage/         # Chrome storage wrapper
├── rpc/            # RPC endpoint management
└── messaging/      # Background message handlers

popup/
├── routes/         # Svelte pages
├── App.svelte      # Main router
└── styles.css      # TUI/cyberpunk theme
```

### Adding New Chain

1. Create adapter in `lib/chains/newchain.ts`:
```typescript
export class NewChainAdapter implements ChainAdapter {
  chainId = 'newchain';
  name = 'NewChain';
  symbol = 'NEW';
  // Implement interface methods...
}
```

2. Add to `lib/chains/index.ts`:
```typescript
import { NewChainAdapter } from './newchain';

export const CHAIN_ADAPTERS = {
  // ...existing
  newchain: new NewChainAdapter()
};
```

3. Update types in `lib/chains/types.ts`:
```typescript
export type SupportedChain = 'solana' | 'starknet' | 'zcash' | 'newchain';
```

4. Add derivation path in `lib/crypto/keyring.ts`
5. Add UI tab in popup components

## Resources

- [Chrome Extension API](https://developer.chrome.com/docs/extensions/reference/)
- [Solana Web3.js](https://solana-labs.github.io/solana-web3.js/)
- [Starknet.js](https://www.starknetjs.com/)
- [BIP39 Spec](https://github.com/bitcoin/bips/blob/master/bip-0039.mediawiki)
- [BIP44 Spec](https://github.com/bitcoin/bips/blob/master/bip-0044.mediawiki)

## Support

For issues, check the logs:
1. Right-click extension icon
2. Click "Inspect popup" (for popup logs)
3. Check chrome://extensions → ZyberLink → background.html (for background logs)

## Next Steps

After basic setup works:
- [ ] Test wallet creation and unlock
- [ ] Test balance fetching for all chains
- [ ] Test sending transactions
- [ ] Add custom RPC endpoints
- [ ] Test dApp integration
- [ ] Implement transaction history
- [ ] Add token support
- [ ] Add hardware wallet integration
