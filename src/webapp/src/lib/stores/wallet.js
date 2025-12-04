import { writable, derived } from 'svelte/store';

// Supported chains
export const SUPPORTED_CHAINS = ['solana', 'starknet', 'zcash'];

// Chain configuration
export const CHAIN_CONFIG = {
  solana: {
    name: 'Solana',
    symbol: 'SOL',
    icon: '◉',
    color: '#00ff9f',
    enabled: true
  },
  starknet: {
    name: 'Starknet',
    symbol: 'STRK',
    icon: '▲',
    color: '#8847ff',
    enabled: false  // COMING SOON
  },
  zcash: {
    name: 'Zcash',
    symbol: 'ZEC',
    icon: 'Z',
    color: '#f4b728',
    enabled: false  // COMING SOON
  }
};

// Supported Solana wallets in priority order
// Note: Solflare can inject as window.solflare OR window.solana with isSolflare flag
const SOLANA_WALLETS = [
  { name: 'Phantom', check: () => window.phantom?.solana?.isPhantom, get: () => window.phantom.solana },
  { name: 'Solflare (direct)', check: () => window.solflare?.isSolflare, get: () => window.solflare },
  { name: 'Solflare (solana)', check: () => window.solana?.isSolflare, get: () => window.solana },
  { name: 'ZyberLink', check: () => window.zyberlink?.solana, get: () => window.zyberlink.solana },
  { name: 'Backpack', check: () => window.backpack?.solana, get: () => window.backpack.solana },
  { name: 'Generic Solana', check: () => window.solana && typeof window.solana.connect === 'function', get: () => window.solana }
];

// Debug: Log what wallets are available
function logAvailableWallets() {
  console.log('[Wallet Detection] Checking available wallets:', {
    'window.phantom?.solana': !!window.phantom?.solana,
    'window.phantom?.solana?.isPhantom': window.phantom?.solana?.isPhantom,
    'window.solflare': !!window.solflare,
    'window.solflare?.isSolflare': window.solflare?.isSolflare,
    'window.solana': !!window.solana,
    'window.solana?.isSolflare': window.solana?.isSolflare,
    'window.solana?.isPhantom': window.solana?.isPhantom,
    'window.solana?.connect': typeof window.solana?.connect,
    'window.zyberlink': !!window.zyberlink,
    'window.backpack': !!window.backpack
  });
}

// Initial state
const initialState = {
  // Connection status
  connected: false,
  connecting: false,

  // Active chain for operations
  activeChain: 'solana',

  // Connected addresses per chain
  addresses: {
    solana: null,
    starknet: null,
    zcash: null
  },

  // Which chains are connected
  connectedChains: [],

  // Provider reference
  provider: null,
  walletName: null,

  // Public key object (for Solana transactions)
  publicKey: null,

  // Error state
  error: null
};

// Detect available Solana wallet
function detectSolanaWallet() {
  logAvailableWallets();

  for (const wallet of SOLANA_WALLETS) {
    try {
      if (wallet.check()) {
        console.log(`[Wallet Detection] Found: ${wallet.name}`);
        return { name: wallet.name, provider: wallet.get() };
      }
    } catch (e) {
      console.log(`[Wallet Detection] Error checking ${wallet.name}:`, e);
    }
  }

  // Fallback: check if any wallet injected window.solana
  if (window.solana) {
    console.log('[Wallet Detection] Fallback to window.solana');
    return { name: 'Solana Wallet', provider: window.solana };
  }

  console.log('[Wallet Detection] No wallet found');
  return null;
}

// Create the store
function createWalletStore() {
  const { subscribe, set, update } = writable(initialState);

  return {
    subscribe,

    // Connect to any available Solana wallet
    async connect(chain = 'solana') {
      console.log(`[Wallet] connect() called for chain: ${chain}`);
      update(s => ({ ...s, connecting: true, error: null }));

      try {
        if (chain === 'solana') {
          // Detect and connect to any available Solana wallet
          const wallet = detectSolanaWallet();

          if (!wallet) {
            throw new Error('No Solana wallet detected. Install Phantom, Solflare, or another Solana wallet.');
          }

          console.log(`[Wallet] Connecting to ${wallet.name}...`);
          console.log(`[Wallet] Provider:`, wallet.provider);
          console.log(`[Wallet] Provider.connect:`, typeof wallet.provider.connect);

          // Connect to the wallet - this should trigger the popup
          console.log(`[Wallet] Calling provider.connect()...`);
          const response = await wallet.provider.connect();
          console.log(`[Wallet] connect() response:`, response);

          const publicKey = response?.publicKey || wallet.provider.publicKey;

          if (!publicKey) {
            throw new Error('Failed to get public key from wallet');
          }

          const address = publicKey.toString();
          console.log(`Connected to ${wallet.name}: ${address}`);

          update(s => ({
            ...s,
            connected: true,
            connecting: false,
            activeChain: 'solana',
            addresses: { ...s.addresses, solana: address },
            connectedChains: [...new Set([...s.connectedChains, 'solana'])],
            provider: wallet.provider,
            walletName: wallet.name,
            publicKey: publicKey
          }));

          return { publicKey: address };

        } else if (window.zyberlink?.[chain]) {
          // For other chains, try ZyberLink
          const provider = window.zyberlink[chain];
          const result = await provider.connect();

          update(s => ({
            ...s,
            connected: true,
            connecting: false,
            activeChain: chain,
            addresses: { ...s.addresses, [chain]: result.publicKey },
            connectedChains: [...new Set([...s.connectedChains, chain])],
            provider: window.zyberlink,
            walletName: 'ZyberLink'
          }));

          return result;
        } else {
          throw new Error(`No wallet available for ${chain}`);
        }
      } catch (error) {
        console.error('Wallet connection error:', error);
        update(s => ({
          ...s,
          connecting: false,
          error: error.message
        }));
        throw error;
      }
    },

    // Connect to all chains at once
    async connectAll() {
      update(s => ({ ...s, connecting: true, error: null }));

      try {
        const results = {};
        const connectedChains = [];

        // Connect Solana first with any available wallet
        const solanaWallet = detectSolanaWallet();
        if (solanaWallet) {
          try {
            const response = await solanaWallet.provider.connect();
            const publicKey = response.publicKey || solanaWallet.provider.publicKey;
            results.solana = publicKey.toString();
            connectedChains.push('solana');
          } catch (err) {
            console.warn('Failed to connect Solana:', err);
            results.solana = null;
          }
        }

        // Try other chains via ZyberLink
        for (const chain of ['starknet', 'zcash']) {
          try {
            if (window.zyberlink?.[chain]) {
              const result = await window.zyberlink[chain].connect();
              results[chain] = result.publicKey;
              connectedChains.push(chain);
            }
          } catch (err) {
            console.warn(`Failed to connect ${chain}:`, err);
            results[chain] = null;
          }
        }

        update(s => ({
          ...s,
          connected: connectedChains.length > 0,
          connecting: false,
          addresses: results,
          connectedChains,
          provider: solanaWallet?.provider || window.zyberlink,
          walletName: solanaWallet?.name || 'ZyberLink'
        }));

        return results;
      } catch (error) {
        update(s => ({ ...s, connecting: false, error: error.message }));
        throw error;
      }
    },

    // Disconnect from a specific chain or all
    async disconnect(chain = null) {
      try {
        // For now, just reset the store - most wallets don't need explicit disconnect
        set(initialState);
      } catch (error) {
        console.error('Disconnect error:', error);
      }
    },

    // Switch active chain
    setActiveChain(chain) {
      if (SUPPORTED_CHAINS.includes(chain)) {
        update(s => ({ ...s, activeChain: chain }));
      }
    },

    // Sign a message on active chain
    async signMessage(message) {
      return new Promise((resolve, reject) => {
        const unsubscribe = subscribe(async (state) => {
          unsubscribe();

          if (!state.connected || !state.provider) {
            reject(new Error('Wallet not connected'));
            return;
          }

          try {
            const encodedMessage = new TextEncoder().encode(message);
            const result = await state.provider.signMessage(encodedMessage, 'utf8');
            resolve(result);
          } catch (error) {
            reject(error);
          }
        });
      });
    },

    // Sign and send transaction on active chain
    async signAndSendTransaction(transaction) {
      return new Promise((resolve, reject) => {
        const unsubscribe = subscribe(async (state) => {
          unsubscribe();

          if (!state.connected || !state.provider) {
            reject(new Error('Wallet not connected'));
            return;
          }

          try {
            const result = await state.provider.signAndSendTransaction(transaction);
            resolve(result);
          } catch (error) {
            reject(error);
          }
        });
      });
    },

    // Get the raw provider for direct access
    getProvider() {
      return new Promise((resolve, reject) => {
        const unsubscribe = subscribe((state) => {
          unsubscribe();
          if (state.provider) {
            resolve(state.provider);
          } else {
            reject(new Error('No provider available'));
          }
        });
      });
    },

    // Reset store
    reset() {
      set(initialState);
    }
  };
}

// Export the store
export const walletStore = createWalletStore();

// Derived stores for convenience
export const isConnected = derived(walletStore, $w => $w.connected);
export const activeChain = derived(walletStore, $w => $w.activeChain);
export const activeAddress = derived(walletStore, $w => $w.addresses[$w.activeChain]);
export const allAddresses = derived(walletStore, $w => $w.addresses);

// Helper to check if any Solana wallet is available
export function isSolanaWalletAvailable() {
  return detectSolanaWallet() !== null;
}

// Backwards compatibility - check for ZyberLink or any Solana wallet
export function isZyberLinkAvailable() {
  return detectSolanaWallet() !== null;
}

// Helper to wait for any wallet to be ready
export function waitForZyberLink(timeout = 3000) {
  return new Promise((resolve, reject) => {
    // Check immediately
    const wallet = detectSolanaWallet();
    if (wallet) {
      resolve(wallet.provider);
      return;
    }

    // Wait for wallet initialization
    let resolved = false;

    const checkWallet = () => {
      const wallet = detectSolanaWallet();
      if (wallet && !resolved) {
        resolved = true;
        resolve(wallet.provider);
        return true;
      }
      return false;
    };

    // Listen for common wallet events
    const handlePhantom = () => { if (checkWallet()) window.removeEventListener('phantom#initialized', handlePhantom); };
    const handleSolflare = () => { if (checkWallet()) window.removeEventListener('solflare#initialized', handleSolflare); };
    const handleZyberLink = () => { if (checkWallet()) window.removeEventListener('zyberlink#initialized', handleZyberLink); };

    window.addEventListener('phantom#initialized', handlePhantom);
    window.addEventListener('solflare#initialized', handleSolflare);
    window.addEventListener('zyberlink#initialized', handleZyberLink);

    // Poll a few times in case events are missed
    const pollInterval = setInterval(() => {
      if (checkWallet()) {
        clearInterval(pollInterval);
      }
    }, 100);

    setTimeout(() => {
      clearInterval(pollInterval);
      window.removeEventListener('phantom#initialized', handlePhantom);
      window.removeEventListener('solflare#initialized', handleSolflare);
      window.removeEventListener('zyberlink#initialized', handleZyberLink);

      if (!resolved) {
        const wallet = detectSolanaWallet();
        if (wallet) {
          resolve(wallet.provider);
        } else {
          reject(new Error('No Solana wallet found'));
        }
      }
    }, timeout);
  });
}
