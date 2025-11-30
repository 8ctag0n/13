# ZyberLink Wallet Extension - Complete Structure

## Overview

Browser extension implementation of ZyberLink multi-chain wallet with support for Solana, Starknet, and Zcash.

## Directory Structure

```
src/wallet-extension/
├── manifest.json                      # Chrome Extension Manifest V3
├── package.json                       # NPM dependencies
├── tsconfig.json                      # TypeScript configuration
├── vite.config.ts                     # Vite build configuration
├── svelte.config.js                   # Svelte compiler options
├── .gitignore                         # Git ignore rules
├── README.md                          # Main documentation
├── QUICKSTART.md                      # Quick start guide
├── STRUCTURE.md                       # This file
│
├── background/
│   └── service-worker.ts              # Background service worker
│                                      # - Message handling
│                                      # - Wallet state management
│                                      # - RPC initialization
│
├── content/
│   └── inject.ts                      # Content script for provider injection
│                                      # - window.solana provider
│                                      # - window.starknet provider
│                                      # - window.zyberlink unified API
│
├── popup/
│   ├── index.html                     # Popup HTML entry point
│   ├── main.ts                        # Popup TypeScript entry
│   ├── styles.css                     # Global cyberpunk TUI styles
│   ├── utils.ts                       # Utility functions
│   ├── App.svelte                     # Main app router
│   │
│   └── routes/
│       ├── CreateWallet.svelte        # Wallet creation flow
│       │                              # - Generate mnemonic
│       │                              # - Display seed phrase
│       │                              # - Set password
│       │
│       ├── Unlock.svelte              # Unlock screen
│       │                              # - Password input
│       │                              # - Vault decryption
│       │
│       ├── Home.svelte                # Main wallet view
│       │                              # - Chain selector
│       │                              # - Balance display
│       │                              # - Quick actions
│       │
│       ├── Send.svelte                # Send transaction
│       │                              # - Recipient input
│       │                              # - Amount input
│       │                              # - Transaction signing
│       │
│       └── Settings.svelte            # Settings management
│                                      # - RPC configuration
│                                      # - Custom endpoints
│                                      # - Chain selection
│
├── lib/
│   ├── crypto/
│   │   ├── index.ts                   # Exports
│   │   ├── keyring.ts                 # BIP39/44 key management
│   │   │                              # - generateMnemonic()
│   │   │                              # - mnemonicToSeed()
│   │   │                              # - deriveSolanaKeypair()
│   │   │                              # - deriveStarknetKeypair()
│   │   │                              # - deriveZcashKeypair()
│   │   │
│   │   └── encryption.ts              # AES-GCM vault encryption
│   │                                  # - encryptVault()
│   │                                  # - decryptVault()
│   │                                  # - deriveKey() (PBKDF2)
│   │
│   ├── chains/
│   │   ├── index.ts                   # Chain adapter registry
│   │   ├── types.ts                   # ChainAdapter interface
│   │   │                              # - TxParams
│   │   │                              # - RPCConfig
│   │   │                              # - RPCEndpoint
│   │   │
│   │   ├── solana.ts                  # Solana implementation
│   │   │                              # - getAddress()
│   │   │                              # - getBalance()
│   │   │                              # - buildTransaction()
│   │   │                              # - signTransaction()
│   │   │                              # - sendTransaction()
│   │   │
│   │   ├── starknet.ts                # Starknet implementation
│   │   │                              # - JSON-RPC calls
│   │   │                              # - Transaction building
│   │   │
│   │   └── zcash.ts                   # Zcash implementation
│   │                                  # - Shielded address support
│   │                                  # - z_sendmany RPC
│   │
│   ├── storage/
│   │   └── vault.ts                   # Chrome storage wrapper
│   │                                  # - saveEncryptedVault()
│   │                                  # - loadEncryptedVault()
│   │                                  # - saveRPCConfig()
│   │                                  # - loadRPCConfig()
│   │                                  # - saveSettings()
│   │                                  # - loadSettings()
│   │
│   ├── rpc/
│   │   └── manager.ts                 # RPC endpoint management
│   │                                  # - initialize()
│   │                                  # - getActiveEndpoint()
│   │                                  # - setActiveEndpoint()
│   │                                  # - addCustomEndpoint()
│   │                                  # - removeCustomEndpoint()
│   │                                  # - checkHealth()
│   │
│   └── messaging/
│       ├── types.ts                   # Message type definitions
│       │                              # - Message
│       │                              # - MessageResponse
│       │                              # - WalletState
│       │                              # - All message types
│       │
│       └── handlers.ts                # Background message handlers
│                                      # - handleMessage()
│                                      # - WalletStateManager class
│                                      # - Message routing
│
└── assets/
    └── icons/
        ├── icon-16.png.placeholder    # 16x16 extension icon
        ├── icon-48.png.placeholder    # 48x48 extension icon
        └── icon-128.png.placeholder   # 128x128 extension icon
```

## File Statistics

- Total TypeScript files: 17
- Total Svelte components: 6
- Total lines of code: ~2,350
- Total configuration files: 5
- Total documentation files: 3

## Key Components

### 1. Background Service Worker
- Persistent wallet state management
- Message handling from popup and content scripts
- RPC communication
- Transaction signing

### 2. Content Script
- Injects Web3 providers into web pages
- Bridges communication between page and extension
- Provides window.solana, window.starknet APIs

### 3. Popup UI
- Svelte 5 reactive components
- Cyberpunk TUI design system
- Multi-chain support with tab navigation
- Secure password management

### 4. Crypto Layer
- BIP39 mnemonic generation
- BIP44 HD key derivation
- AES-GCM encryption for vault
- PBKDF2 key derivation (100k iterations)

### 5. Chain Adapters
- Unified interface for all chains
- Chain-specific transaction building
- Balance fetching
- Transaction signing and broadcasting

### 6. Storage Layer
- Chrome storage API wrapper
- Encrypted vault persistence
- RPC configuration management
- Settings storage

### 7. RPC Manager
- Multi-endpoint support per chain
- Custom RPC endpoints
- Health checking
- Active endpoint switching

## Message Flow

```
Web Page
   ↓ (postMessage)
Content Script
   ↓ (chrome.runtime.sendMessage)
Background Service Worker
   ↓
Message Handler
   ↓
Chain Adapter
   ↓
RPC Endpoint
   ↓
Blockchain
```

## Data Flow

```
User Password
   ↓
PBKDF2 (100k iterations)
   ↓
AES Key
   ↓
Encrypted Vault (Chrome Storage)
   ↓ (on unlock)
Decrypted Mnemonic
   ↓
BIP39 Seed
   ↓
BIP44 Derivation
   ↓
Chain-specific Keypairs
   ↓
Addresses & Signing
```

## Security Architecture

1. **Password Protection**: Required for vault encryption/decryption
2. **Memory Safety**: Keys cleared on lock
3. **Auto-lock**: 15-minute inactivity timeout
4. **No Network Exposure**: Keys never leave extension
5. **Encrypted Storage**: AES-GCM with PBKDF2-derived keys

## Build Pipeline

```
Source Files (.ts, .svelte)
   ↓
TypeScript Compiler
   ↓
Svelte Compiler
   ↓
Vite Bundler
   ↓
CRXJS Plugin (Extension formatting)
   ↓
dist/ (Chrome Extension)
```

## Extension Permissions

- `storage`: Chrome local storage for encrypted vault
- `activeTab`: Access to current tab for provider injection

## Provider APIs

### window.solana
- `connect()` - Connect wallet
- `disconnect()` - Disconnect wallet
- `signMessage(message)` - Sign arbitrary message
- `signTransaction(tx)` - Sign transaction
- `signAndSendTransaction(tx)` - Sign and broadcast

### window.starknet
- `connect()` - Connect wallet
- `disconnect()` - Disconnect wallet
- `signMessage(message)` - Sign arbitrary message
- `signTransaction(tx)` - Sign transaction

### window.zyberlink
- `solana` - Solana provider instance
- `starknet` - Starknet provider instance
- `version` - Extension version

## Default RPC Endpoints

### Solana
- Mainnet: `https://api.mainnet-beta.solana.com`
- Devnet: `https://api.devnet.solana.com`

### Starknet
- Mainnet: `https://starknet-mainnet.public.blastapi.io`
- Testnet: `https://starknet-testnet.public.blastapi.io`

### Zcash
- Mainnet: `https://zcash.example.com` (placeholder)

## Development Workflow

1. `npm install` - Install dependencies
2. `npm run dev` - Start development server
3. Load `dist/` in Chrome extensions
4. Make changes (hot reload enabled)
5. `npm run build` - Production build

## Testing Checklist

- [ ] Wallet creation flow
- [ ] Wallet unlock/lock
- [ ] Balance fetching (all chains)
- [ ] Transaction signing (all chains)
- [ ] Transaction broadcasting (all chains)
- [ ] Custom RPC endpoints
- [ ] Auto-lock functionality
- [ ] Provider injection
- [ ] dApp integration
- [ ] Password validation
- [ ] Mnemonic validation
- [ ] Error handling

## Future Enhancements

- Import wallet from mnemonic
- Transaction history
- Token support (SPL, ERC20)
- NFT gallery
- Address book
- Multi-account support
- Hardware wallet integration
- Network health monitoring
- Advanced fee configuration
- Batch transactions
- Message signing UI
- dApp permission management
