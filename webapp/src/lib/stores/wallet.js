import { writable } from 'svelte/store';

// Wallet store for managing Solana wallet connection
export const walletStore = writable({
  connected: false,
  publicKey: null,
  provider: null,
  name: null
});

// Helper to disconnect wallet
export function disconnectWallet() {
  walletStore.set({
    connected: false,
    publicKey: null,
    provider: null,
    name: null
  });
}
