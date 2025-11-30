import { writable, derived } from 'svelte/store';

// Supported chains
export const SUPPORTED_CHAINS = ['solana', 'starknet', 'zcash'];

// Chain configuration
export const CHAIN_CONFIG = {
  solana: {
    name: 'Solana',
    symbol: 'SOL',
    icon: '◉',
    color: '#00ff9f'
  },
  starknet: {
    name: 'Starknet',
    symbol: 'STRK',
    icon: '▲',
    color: '#8847ff'
  },
  zcash: {
    name: 'Zcash',
    symbol: 'ZEC',
    icon: 'Z',
    color: '#f4b728'
  }
};

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

  // Error state
  error: null
};

// Create the store
function createWalletStore() {
  const { subscribe, set, update } = writable(initialState);

  return {
    subscribe,

    // Connect to ZyberLink wallet
    async connect(chain = 'solana') {
      update(s => ({ ...s, connecting: true, error: null }));

      try {
        // Check if ZyberLink is available
        if (!window.zyberlink) {
          throw new Error('ZyberLink wallet not detected. Please install the extension.');
        }

        const provider = window.zyberlink[chain];
        if (!provider) {
          throw new Error(`Chain ${chain} not supported`);
        }

        // Connect to the chain
        const result = await provider.connect();

        update(s => {
          const newAddresses = { ...s.addresses, [chain]: result.publicKey };
          const newConnectedChains = [...new Set([...s.connectedChains, chain])];

          return {
            ...s,
            connected: true,
            connecting: false,
            activeChain: chain,
            addresses: newAddresses,
            connectedChains: newConnectedChains,
            provider: window.zyberlink,
            walletName: 'ZyberLink'
          };
        });

        return result;
      } catch (error) {
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
        if (!window.zyberlink) {
          throw new Error('ZyberLink wallet not detected');
        }

        const results = {};
        const connectedChains = [];

        for (const chain of SUPPORTED_CHAINS) {
          try {
            const provider = window.zyberlink[chain];
            const result = await provider.connect();
            results[chain] = result.publicKey;
            connectedChains.push(chain);
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
          provider: window.zyberlink,
          walletName: 'ZyberLink'
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
        if (chain && window.zyberlink?.[chain]) {
          await window.zyberlink[chain].disconnect();

          update(s => {
            const newAddresses = { ...s.addresses, [chain]: null };
            const newConnectedChains = s.connectedChains.filter(c => c !== chain);

            return {
              ...s,
              addresses: newAddresses,
              connectedChains: newConnectedChains,
              connected: newConnectedChains.length > 0,
              activeChain: newConnectedChains[0] || 'solana'
            };
          });
        } else {
          // Disconnect all
          for (const c of SUPPORTED_CHAINS) {
            if (window.zyberlink?.[c]) {
              await window.zyberlink[c].disconnect().catch(() => {});
            }
          }
          set(initialState);
        }
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
            const provider = state.provider[state.activeChain];
            const result = await provider.signMessage(message);
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
            const provider = state.provider[state.activeChain];
            const result = await provider.signAndSendTransaction(transaction);
            resolve(result);
          } catch (error) {
            reject(error);
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

// Helper to check if ZyberLink is available
export function isZyberLinkAvailable() {
  return typeof window !== 'undefined' && window.zyberlink?.isZyberLink === true;
}

// Helper to wait for ZyberLink to be ready
export function waitForZyberLink(timeout = 3000) {
  return new Promise((resolve, reject) => {
    if (isZyberLinkAvailable()) {
      resolve(window.zyberlink);
      return;
    }

    const handleInit = (event) => {
      window.removeEventListener('zyberlink#initialized', handleInit);
      resolve(window.zyberlink);
    };

    window.addEventListener('zyberlink#initialized', handleInit);

    setTimeout(() => {
      window.removeEventListener('zyberlink#initialized', handleInit);
      if (isZyberLinkAvailable()) {
        resolve(window.zyberlink);
      } else {
        reject(new Error('ZyberLink not found'));
      }
    }, timeout);
  });
}
