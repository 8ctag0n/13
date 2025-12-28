# ZyberLink Wallet - Browser Extension

Multi-chain browser wallet extension supporting Solana, Starknet, and Zcash with custom RPC configuration.

## Features

- Multi-chain support (Solana, Starknet, Zcash)
- BIP39/44 mnemonic-based key derivation
- AES-GCM encrypted vault storage
- Custom RPC endpoints
- Send/Receive transactions
- Auto-lock for security
- Cyberpunk TUI design

## Tech Stack

- TypeScript
- Svelte 5
- Manifest V3
- Vite + CRXJS
- Web Crypto API

## Architecture

```
wallet-extension/
├── manifest.json              # Extension manifest
├── background/
│   └── service-worker.ts     # Background service worker
├── content/
│   └── inject.ts             # Provider injection (window.solana, etc)
├── popup/
│   ├── App.svelte            # Main app router
│   └── routes/               # Popup views
├── lib/
│   ├── crypto/               # Keyring + encryption
│   ├── chains/               # Chain adapters
│   ├── storage/              # Chrome storage wrapper
│   ├── rpc/                  # RPC manager
│   └── messaging/            # Message handlers
└── assets/
    └── icons/                # Extension icons
```

## Development

### Install Dependencies

```bash
cd src/wallet-extension
npm install
```

### Development Mode

```bash
npm run dev
```

This will start Vite in watch mode and build the extension to `dist/`.

### Load in Chrome

1. Open Chrome and navigate to `chrome://extensions`
2. Enable "Developer mode" (top right)
3. Click "Load unpacked"
4. Select the `dist/` folder

### Build for Production

```bash
npm run build
```

## Usage

### Creating a Wallet

1. Click the extension icon
2. Click "CREATE NEW WALLET"
3. Save your 12-word seed phrase securely
4. Set a strong password
5. Wallet is ready to use

### Switching Chains

Click on the chain tabs (SOL, STRK, ZEC) to switch between networks.

### Sending Transactions

1. Click "SEND"
2. Enter recipient address
3. Enter amount
4. Click "SEND" to sign and broadcast

### Custom RPC Endpoints

1. Click "SETTINGS"
2. Select chain
3. Click "+ ADD CUSTOM RPC"
4. Enter name and URL
5. Click "USE" to activate

## Security

- All private keys derived from encrypted mnemonic
- Password-protected vault using AES-GCM
- PBKDF2 key derivation (100k iterations)
- Auto-lock after 15 minutes of inactivity
- No private keys ever leave the extension

## Provider API

The extension injects providers into web pages:

### Solana

```javascript
// Access via window.solana
await window.solana.connect();
await window.solana.signMessage(message);
await window.solana.signTransaction(tx);
await window.solana.signAndSendTransaction(tx);
```

### Starknet

```javascript
// Access via window.starknet
await window.starknet.connect();
await window.starknet.signMessage(message);
```

### ZyberLink

```javascript
// Access all providers
window.zyberlink.solana
window.zyberlink.starknet
window.zyberlink.version
```

## Message Types

Background service worker handles these message types:

- `GET_WALLET_STATE` - Get current wallet state
- `CREATE_WALLET` - Create new wallet
- `IMPORT_WALLET` - Import from mnemonic
- `UNLOCK_WALLET` - Unlock with password
- `LOCK_WALLET` - Lock wallet
- `CONNECT` - Connect to dApp
- `DISCONNECT` - Disconnect from dApp
- `GET_ADDRESS` - Get address for chain
- `GET_BALANCE` - Get balance for chain
- `SIGN_MESSAGE` - Sign message
- `SIGN_TRANSACTION` - Sign transaction
- `SIGN_AND_SEND_TRANSACTION` - Sign and send transaction

## Chain Adapters

Each chain implements the `ChainAdapter` interface:

```typescript
interface ChainAdapter {
  chainId: string;
  name: string;
  symbol: string;

  getAddress(keypair: any): string;
  getBalance(address: string, rpcUrl: string): Promise<string>;
  buildTransaction(params: TxParams): Promise<any>;
  signTransaction(tx: any, keypair: any): Promise<any>;
  sendTransaction(signedTx: any, rpcUrl: string): Promise<string>;
}
```

## Default RPC Endpoints

### Solana
- Mainnet: `https://api.mainnet-beta.solana.com`
- Devnet: `https://api.devnet.solana.com`

### Starknet
- Mainnet: `https://starknet-mainnet.public.blastapi.io`
- Testnet: `https://starknet-testnet.public.blastapi.io`

### Zcash
- Mainnet: `https://zcash.example.com` (placeholder)

## TODO

- [ ] Import wallet from mnemonic UI
- [ ] Transaction history
- [ ] Token support (SPL, ERC20, etc)
- [ ] NFT support
- [ ] Address book
- [ ] Multi-account support
- [ ] Hardware wallet support
- [ ] Zcash shielded transactions
- [ ] Network health monitoring
- [ ] dApp connection management
- [ ] Export private keys (with warning)

## Icons

Replace placeholder icons in `assets/icons/` with actual PNG icons:
- `icon-16.png` - 16x16px
- `icon-48.png` - 48x48px
- `icon-128.png` - 128x128px

## License

MIT
