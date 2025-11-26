import { PublicKey } from '@solana/web3.js';

/**
 * Mock Wallet for E2E Testing
 *
 * Compatible with @solana/wallet-adapter-base interface.
 * Injects into browser context to simulate Phantom/Solflare wallet.
 */

export interface MockWalletConfig {
  publicKey?: string;
  autoApprove?: boolean;
  simulateErrors?: boolean;
}

export class MockWallet {
  private _publicKey: PublicKey;
  private _connected: boolean = false;
  private _autoApprove: boolean;
  private _simulateErrors: boolean;

  constructor(config: MockWalletConfig = {}) {
    // Use deterministic test wallet
    this._publicKey = config.publicKey
      ? new PublicKey(config.publicKey)
      : new PublicKey('DYw8jCTfwHNRJhhmFcbXvVDTqWMEVFBX6ZKUmG5CNSKK');

    this._autoApprove = config.autoApprove ?? true;
    this._simulateErrors = config.simulateErrors ?? false;
  }

  get publicKey() {
    return this._connected ? this._publicKey : null;
  }

  get isPhantom() {
    return true;
  }

  get isConnected() {
    return this._connected;
  }

  async connect() {
    if (this._simulateErrors) {
      throw new Error('User rejected the connection request');
    }

    this._connected = true;
    return { publicKey: this._publicKey };
  }

  async disconnect() {
    this._connected = false;
  }

  async signMessage(message: Uint8Array): Promise<{ signature: Uint8Array }> {
    if (!this._connected) {
      throw new Error('Wallet not connected');
    }

    if (!this._autoApprove) {
      throw new Error('User rejected the request');
    }

    // Return deterministic signature (64 bytes)
    const signature = new Uint8Array(64);
    signature.fill(0xAB);

    return { signature };
  }

  async signTransaction(transaction: any): Promise<any> {
    if (!this._connected) {
      throw new Error('Wallet not connected');
    }

    if (!this._autoApprove) {
      throw new Error('User rejected the request');
    }

    // Mock: add signature to transaction
    const signature = new Uint8Array(64);
    signature.fill(0xCD);

    transaction.signatures = transaction.signatures || [];
    transaction.signatures.push({ signature });

    return transaction;
  }

  async signAllTransactions(transactions: any[]): Promise<any[]> {
    return Promise.all(transactions.map(tx => this.signTransaction(tx)));
  }
}

/**
 * Generate browser injection script for mock wallet
 */
export function generateMockWalletScript(config: MockWalletConfig = {}): string {
  const wallet = new MockWallet(config);

  return `
    // Mock Phantom Wallet
    window.phantom = {
      solana: {
        isPhantom: true,
        publicKey: ${JSON.stringify(wallet.publicKey?.toBase58())},
        isConnected: ${wallet.isConnected},

        connect: async () => {
          ${config.autoApprove === false ? 'throw new Error("User rejected the connection request");' : ''}
          window.phantom.solana.isConnected = true;
          window.phantom.solana.publicKey = {
            toString: () => '${config.publicKey || 'DYw8jCTfwHNRJhhmFcbXvVDTqWMEVFBX6ZKUmG5CNSKK'}',
            toBase58: () => '${config.publicKey || 'DYw8jCTfwHNRJhhmFcbXvVDTqWMEVFBX6ZKUmG5CNSKK'}'
          };
          return { publicKey: window.phantom.solana.publicKey };
        },

        disconnect: async () => {
          window.phantom.solana.isConnected = false;
          window.phantom.solana.publicKey = null;
        },

        signMessage: async (message) => {
          if (!window.phantom.solana.isConnected) {
            throw new Error('Wallet not connected');
          }
          ${config.autoApprove === false ? 'throw new Error("User rejected the request");' : ''}
          const signature = new Uint8Array(64);
          signature.fill(0xAB);
          return { signature };
        },

        signTransaction: async (transaction) => {
          if (!window.phantom.solana.isConnected) {
            throw new Error('Wallet not connected');
          }
          ${config.autoApprove === false ? 'throw new Error("User rejected the request");' : ''}
          const signature = new Uint8Array(64);
          signature.fill(0xCD);
          transaction.signatures = transaction.signatures || [];
          transaction.signatures.push({ signature });
          return transaction;
        },

        signAllTransactions: async (transactions) => {
          return Promise.all(transactions.map(tx => window.phantom.solana.signTransaction(tx)));
        }
      }
    };

    // Mock Solflare Wallet (similar interface)
    window.solflare = {
      ...window.phantom.solana,
      isSolflare: true
    };

    // Also expose at window.solana for compatibility with WalletConnect component
    window.solana = window.phantom.solana;

    console.log('[MockWallet] Injected - PublicKey:', window.phantom.solana.publicKey);
  `;
}

/**
 * Preset configurations for common test scenarios
 */
export const MockWalletPresets = {
  /** Default connected wallet that auto-approves everything */
  connected: {
    publicKey: 'DYw8jCTfwHNRJhhmFcbXvVDTqWMEVFBX6ZKUmG5CNSKK',
    autoApprove: true,
    simulateErrors: false
  },

  /** Wallet that rejects all requests (for error testing) */
  rejecting: {
    publicKey: 'DYw8jCTfwHNRJhhmFcbXvVDTqWMEVFBX6ZKUmG5CNSKK',
    autoApprove: false,
    simulateErrors: false
  },

  /** Wallet with different address (for multi-wallet testing) */
  alternative: {
    publicKey: 'EX9jGJkDHmZ8sQF7aTRsWn3kHzp4QZVqbN8kMxK1cF3Q',
    autoApprove: true,
    simulateErrors: false
  },

  /** Disconnected wallet (for connection flow testing) */
  disconnected: {
    publicKey: undefined,
    autoApprove: true,
    simulateErrors: false
  }
};
