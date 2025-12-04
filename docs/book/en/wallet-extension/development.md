# ZyberLink Wallet Extension - Development Guide

## Executive Summary

Complete browser extension implementation for multi-chain wallet supporting Solana, Starknet, and Zcash. Built with TypeScript, Svelte 5, and Chrome Manifest V3.

## Development Setup

### Prerequisites

- Node.js 18+ and npm
- Chrome browser (or Chromium-based)
- Git

### Installation

```bash
# Clone repository
cd /home/deploy/experimental/demo-zyberlink-zcash/src/wallet-extension

# Install dependencies
npm install

# Start development server with hot reload
npm run dev

# Build for production
npm run build
```

### Loading in Browser

1. Open `chrome://extensions`
2. Enable "Developer mode"
3. Click "Load unpacked"
4. Select the `dist/` folder

## Core Features Implemented

### 1. Wallet Management
- **BIP39 Mnemonic Generation**: 12/24 word seed phrase generation
- **BIP44 HD Derivation**: Chain-specific key derivation paths
  - Solana: `m/44'/501'/0'/0'`
  - Starknet: `m/44'/9004'/0'/0/0`
  - Zcash: `m/44'/133'/0'`
- **AES-GCM Encryption**: Vault encryption with PBKDF2 key derivation (100k iterations)
- **Auto-lock**: 15-minute inactivity timeout

### 2. Multi-Chain Support
- **Solana Adapter**: Full integration with @solana/web3.js
  - Balance fetching
  - Transaction building
  - Transaction signing
  - Transaction broadcasting
- **Starknet Adapter**: JSON-RPC integration
  - Balance via RPC
  - Transaction structure
  - Signature generation
- **Zcash Adapter**: Base58 address encoding
  - Shielded address support
  - z_sendmany RPC calls
  - Memo field support

### 3. RPC Management
- **Multi-endpoint Support**: Multiple RPC endpoints per chain
- **Custom RPCs**: User-configurable endpoints
- **Health Checking**: RPC endpoint availability monitoring
- **Active Endpoint Switching**: Hot-swap between endpoints

### 4. User Interface
- **Cyberpunk TUI Design**: Monospace fonts, glow effects, cyber colors
- **Responsive Routing**: Multi-view navigation
  - CreateWallet: 3-step wallet creation flow
  - Unlock: Password-based unlock screen
  - Home: Balance display + quick actions
  - Send: Transaction composition + signing
  - Settings: RPC configuration
- **Chain Selector**: Tab-based chain switching
- **Real-time Balance**: Refresh on-demand

### 5. Web3 Provider Injection
- **window.solana**: Phantom/Solflare-compatible API
- **window.starknet**: Starknet wallet API
- **window.zyberlink**: Unified multi-chain interface

## Architecture Components

### Background Service Worker

```typescript
// Message routing
// Wallet state management
// Transaction signing
// RPC initialization
```

**Location**: `/src/wallet-extension/background/service-worker.ts`

**Responsibilities**:
- Handle all extension messages
- Manage encrypted vault state
- Coordinate chain adapters
- Process transaction signing requests

### Content Script

```typescript
// Provider injection into web pages
// Message bridging
// Event dispatching
```

**Location**: `/src/wallet-extension/content/inject.ts`

**Responsibilities**:
- Inject `window.solana` and `window.starknet` providers
- Forward messages between page and background
- Emit connection/disconnection events

### Popup Application

```typescript
// Svelte 5 reactive components
// State management
// User input validation
// Error handling
```

**Location**: `/src/wallet-extension/popup/`

**Key Components**:
- `App.svelte`: Main router and state coordinator
- `routes/CreateWallet.svelte`: Wallet creation flow
- `routes/Unlock.svelte`: Authentication screen
- `routes/Home.svelte`: Main dashboard
- `routes/Send.svelte`: Transaction builder
- `routes/Settings.svelte`: RPC configuration

### Crypto Library

```typescript
// keyring.ts: BIP39/44 implementation
// encryption.ts: AES-GCM vault encryption
```

**Location**: `/src/wallet-extension/lib/crypto/`

**Features**:
- Mnemonic generation and validation
- HD key derivation for multiple chains
- Secure vault encryption/decryption
- Password-based key derivation (PBKDF2)

### Chain Adapters

```typescript
// ChainAdapter interface
// Solana implementation (complete)
// Starknet implementation (basic)
// Zcash implementation (basic)
```

**Location**: `/src/wallet-extension/lib/chains/`

**Interface**:
```typescript
interface ChainAdapter {
  chainId: string;
  name: string;
  symbol: string;
  getAddress(keypair: KeyPair): Promise<string>;
  getBalance(address: string): Promise<string>;
  buildTransaction(params: TxParams): Promise<Transaction>;
  signTransaction(tx: Transaction, keypair: KeyPair): Promise<SignedTx>;
  sendTransaction(signedTx: SignedTx): Promise<string>;
}
```

### Storage Layer

```typescript
// Chrome storage wrapper
// Encrypted vault persistence
// RPC config storage
// Settings management
```

**Location**: `/src/wallet-extension/lib/storage/`

**API**:
- `saveVault(encryptedData: string): Promise<void>`
- `loadVault(): Promise<string | null>`
- `saveRPCConfig(config: RPCConfig): Promise<void>`
- `loadRPCConfig(): Promise<RPCConfig>`

### RPC Manager

```typescript
// Endpoint registration
// Health monitoring
// Active endpoint management
// Custom endpoint CRUD
```

**Location**: `/src/wallet-extension/lib/rpc/`

**Features**:
- Register default and custom RPC endpoints
- Monitor endpoint health and latency
- Switch active endpoints dynamically
- Persist RPC configuration

### Messaging System

```typescript
// Type-safe message definitions
// Handler registration
// Response formatting
// Error propagation
```

**Location**: `/src/wallet-extension/lib/messaging/`

**Message Types**:
- Wallet management (create, unlock, lock)
- Web3 operations (connect, sign, send)
- RPC management (add, remove, switch)

## File Structure

### Configuration Files (5)
- `manifest.json` - Chrome extension manifest
- `package.json` - NPM dependencies
- `tsconfig.json` - TypeScript config
- `vite.config.ts` - Build configuration
- `svelte.config.js` - Svelte compiler options

### TypeScript Files (17)
1. `background/service-worker.ts` - Background script
2. `content/inject.ts` - Provider injection
3. `lib/crypto/keyring.ts` - Key derivation
4. `lib/crypto/encryption.ts` - Vault encryption
5. `lib/crypto/index.ts` - Crypto exports
6. `lib/chains/types.ts` - Chain interfaces
7. `lib/chains/solana.ts` - Solana adapter
8. `lib/chains/starknet.ts` - Starknet adapter
9. `lib/chains/zcash.ts` - Zcash adapter
10. `lib/chains/index.ts` - Chain registry
11. `lib/storage/vault.ts` - Storage wrapper
12. `lib/rpc/manager.ts` - RPC management
13. `lib/messaging/types.ts` - Message types
14. `lib/messaging/handlers.ts` - Message handlers
15. `popup/main.ts` - Popup entry point
16. `popup/utils.ts` - Utility functions
17. `vite.config.ts` - Build config

### Svelte Components (6)
1. `popup/App.svelte` - Main app router
2. `popup/routes/CreateWallet.svelte` - Wallet creation
3. `popup/routes/Unlock.svelte` - Unlock screen
4. `popup/routes/Home.svelte` - Main view
5. `popup/routes/Send.svelte` - Send transaction
6. `popup/routes/Settings.svelte` - RPC settings

### Styles & HTML (2)
1. `popup/index.html` - Popup HTML
2. `popup/styles.css` - TUI styles

## Security Implementation

### Encryption
- **Algorithm**: AES-GCM (256-bit)
- **Key Derivation**: PBKDF2 with SHA-256
- **Iterations**: 100,000
- **Salt**: Random 16 bytes
- **IV**: Random 12 bytes per encryption

### Key Management
- **Storage**: Chrome local storage (encrypted)
- **Memory**: Keys cleared on lock
- **Derivation**: BIP44 standard paths
- **Never Transmitted**: Keys never leave extension

### Access Control
- **Password Required**: For all sensitive operations
- **Auto-lock**: After 15 minutes inactivity
- **Manual Lock**: Available from UI

## Message Types Implemented

### Wallet Management
- `GET_WALLET_STATE` - Current state
- `CREATE_WALLET` - New wallet
- `IMPORT_WALLET` - Import from mnemonic
- `UNLOCK_WALLET` - Decrypt vault
- `LOCK_WALLET` - Lock wallet

### Web3 Operations
- `CONNECT` - Connect to dApp
- `DISCONNECT` - Disconnect from dApp
- `GET_ADDRESS` - Get chain address
- `GET_BALANCE` - Fetch balance
- `SIGN_MESSAGE` - Sign arbitrary message
- `SIGN_TRANSACTION` - Sign transaction
- `SIGN_AND_SEND_TRANSACTION` - Sign + broadcast

### RPC Management
- `GET_RPC_ENDPOINTS` - List endpoints
- `SET_ACTIVE_RPC` - Switch endpoint
- `ADD_CUSTOM_RPC` - Add custom endpoint
- `REMOVE_CUSTOM_RPC` - Remove endpoint

## Design System

### Colors
- **Cyber Cyan** (#00ff9f) - Solana, primary actions
- **Cyber Purple** (#9d4edd) - Starknet, secondary actions
- **Cyber Yellow** (#ffd60a) - Zcash, warnings
- **Cyber BG** (#0a0e14) - Background
- **Cyber Border** (#30363d) - Borders

### Typography
- **Font**: Courier New, Consolas, monospace
- **Style**: Uppercase for labels
- **Format**: [BRACKETS] for emphasis

### Components
- **Buttons**: Bordered with glow on hover
- **Inputs**: Dark background, cyan focus
- **Cards**: Bordered containers with hover effects
- **Tabs**: Chain selector with color coding

## Build Configuration

### Vite Setup
```typescript
- Svelte plugin
- CRXJS plugin for extension
- Buffer polyfill
- Crypto polyfill
- Stream polyfill
```

### TypeScript Config
```typescript
- Target: ESNext
- Module: ESNext
- Strict mode enabled
- Chrome types included
```

### Dependencies
```json
Runtime:
- svelte ^5.0.0
- @solana/web3.js ^1.95.0
- starknet ^6.8.0
- bip39 ^3.1.0
- ed25519-hd-key ^1.3.0
- tweetnacl ^1.0.3
- bs58 ^5.0.0
- buffer ^6.0.3

Development:
- @sveltejs/vite-plugin-svelte ^3.0.0
- @crxjs/vite-plugin ^2.0.0-beta.23
- vite ^5.0.0
- typescript ^5.3.0
- @types/chrome ^0.0.260
- @types/node ^20.10.0
- crypto-browserify ^3.12.0
- stream-browserify ^3.0.0
```

## Testing Strategies

### Manual Testing
1. Create wallet and verify mnemonic display
2. Lock/unlock with password
3. Switch between chains
4. Check balance for each chain
5. Send test transaction
6. Add custom RPC endpoint
7. Test auto-lock after inactivity

### Integration Testing
1. Test provider injection
2. Test dApp connection flow
3. Test transaction signing flow
4. Test multi-tab behavior
5. Test extension update flow

### Security Testing
1. Verify vault encryption
2. Test password validation
3. Test auto-lock functionality
4. Verify key clearing on lock
5. Test against common attack vectors

## Development Workflow

### Adding a New Chain

1. **Create Chain Adapter**

```typescript
// lib/chains/newchain.ts
import { ChainAdapter, KeyPair, TxParams } from './types';

export class NewChainAdapter implements ChainAdapter {
  chainId = 'newchain';
  name = 'NewChain';
  symbol = 'NEW';

  async getAddress(keypair: KeyPair): Promise<string> {
    // Implement address derivation
  }

  async getBalance(address: string): Promise<string> {
    // Implement RPC balance query
  }

  async buildTransaction(params: TxParams): Promise<Transaction> {
    // Implement transaction building
  }

  async signTransaction(tx: Transaction, keypair: KeyPair): Promise<SignedTx> {
    // Implement transaction signing
  }

  async sendTransaction(signedTx: SignedTx): Promise<string> {
    // Implement transaction broadcasting
  }
}
```

2. **Register Adapter**

```typescript
// lib/chains/index.ts
import { NewChainAdapter } from './newchain';

export const CHAIN_ADAPTERS = {
  solana: new SolanaAdapter(),
  starknet: new StarknetAdapter(),
  zcash: new ZcashAdapter(),
  newchain: new NewChainAdapter() // Add here
};
```

3. **Update Types**

```typescript
// lib/chains/types.ts
export type SupportedChain = 'solana' | 'starknet' | 'zcash' | 'newchain';
```

4. **Add Derivation Path**

```typescript
// lib/crypto/keyring.ts
const DERIVATION_PATHS = {
  solana: "m/44'/501'/0'/0'",
  starknet: "m/44'/9004'/0'/0/0",
  zcash: "m/44'/133'/0'",
  newchain: "m/44'/12345'/0'/0'" // Add chain-specific path
};
```

5. **Add UI Components**

- Add tab in `popup/routes/Home.svelte`
- Add chain color in `popup/styles.css`
- Add chain icon if needed

### Adding a New Message Type

1. **Define Message Type**

```typescript
// lib/messaging/types.ts
export type ExtensionMessage =
  | { type: 'GET_WALLET_STATE' }
  | { type: 'NEW_MESSAGE_TYPE', payload: NewPayload }
  // ... other types
```

2. **Implement Handler**

```typescript
// lib/messaging/handlers.ts
export async function handleMessage(
  message: ExtensionMessage
): Promise<MessageResponse> {
  switch (message.type) {
    case 'NEW_MESSAGE_TYPE':
      return handleNewMessage(message.payload);
    // ... other handlers
  }
}
```

3. **Use in Components**

```typescript
// popup/routes/SomeComponent.svelte
async function sendNewMessage() {
  const response = await chrome.runtime.sendMessage({
    type: 'NEW_MESSAGE_TYPE',
    payload: { /* data */ }
  });
}
```

## Debugging

### Popup Logs
1. Right-click extension icon
2. Select "Inspect popup"
3. Check Console tab

### Background Logs
1. Open `chrome://extensions`
2. Find ZyberLink extension
3. Click "background.html" or "service worker"
4. Check Console tab

### Content Script Logs
1. Open web page
2. Press F12
3. Check Console tab
4. Filter by "content script" or extension ID

## Known Limitations

1. **Icons**: Placeholder files need replacement with actual PNGs
2. **Starknet**: Basic implementation, needs full signing logic
3. **Zcash**: Placeholder RPC endpoint, needs real endpoint
4. **Message Signing**: Basic implementation for some chains
5. **Transaction History**: Not implemented yet
6. **Import Wallet**: UI flow exists but needs activation

## Deployment Checklist

- [ ] Replace placeholder icons
- [ ] Update RPC endpoints (especially Zcash)
- [ ] Test on Chrome stable
- [ ] Test on Chrome dev/canary
- [ ] Test wallet creation flow
- [ ] Test import flow
- [ ] Test all chain transactions
- [ ] Security audit
- [ ] Performance testing
- [ ] Browser compatibility testing
- [ ] Package for Chrome Web Store
- [ ] Prepare store listing
- [ ] Submit for review

## Resources

- [Chrome Extension API Reference](https://developer.chrome.com/docs/extensions/reference/)
- [Manifest V3 Migration Guide](https://developer.chrome.com/docs/extensions/mv3/intro/)
- [Svelte 5 Documentation](https://svelte.dev/docs)
- [Solana Web3.js Docs](https://solana-labs.github.io/solana-web3.js/)
- [Starknet.js Documentation](https://www.starknetjs.com/)
- [BIP39 Specification](https://github.com/bitcoin/bips/blob/master/bip-0039.mediawiki)
- [BIP44 Specification](https://github.com/bitcoin/bips/blob/master/bip-0044.mediawiki)

## Support

For issues and questions:
1. Check the [Troubleshooting](#debugging) section
2. Review browser console logs
3. Check extension background logs
4. Report issues to the main repository

## Next Steps

### Immediate
1. Generate actual PNG icons (16x16, 48x48, 128x128)
2. Run `npm install` to install dependencies
3. Run `npm run dev` to test build
4. Load in Chrome and test wallet creation

### Short-term
1. Implement import wallet UI
2. Add transaction history
3. Improve Starknet/Zcash adapters
4. Add error boundaries
5. Add loading states

### Medium-term
1. Token support (SPL, ERC20)
2. NFT gallery
3. Address book
4. Multi-account support
5. Network health monitoring

### Long-term
1. Hardware wallet integration
2. Advanced fee management
3. Batch transactions
4. dApp permission system
5. Mobile version
