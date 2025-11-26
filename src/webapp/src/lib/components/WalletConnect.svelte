<script>
  import { onMount } from 'svelte';
  import { walletStore } from '../stores/wallet';

  let detecting = true;
  let availableWallets = [];

  onMount(async () => {
    // Detect available Solana wallets
    const wallets = [];

    if (window.solana && window.solana.isPhantom) {
      wallets.push({ name: 'Phantom', provider: window.solana });
    }

    if (window.solflare && window.solflare.isSolflare) {
      wallets.push({ name: 'Solflare', provider: window.solflare });
    }

    availableWallets = wallets;
    detecting = false;
  });

  async function connectWallet(wallet) {
    try {
      const response = await wallet.provider.connect();
      walletStore.set({
        connected: true,
        publicKey: response.publicKey.toString(),
        provider: wallet.provider,
        name: wallet.name
      });
    } catch (err) {
      console.error('Failed to connect wallet:', err);
    }
  }

  $: isConnected = $walletStore.connected;
  $: publicKey = $walletStore.publicKey;
</script>

<div class="wallet-connect">
  {#if detecting}
    <div class="detecting text-mono text-muted">
      <span class="spin">[⠋]</span> DETECTING_WALLETS...
    </div>
  {:else if isConnected}
    <!-- Connected State with Better Visual Feedback -->
    <button
      class="wallet-button connected"
      data-testid="wallet-connected-button"
      on:click={() => {
        walletStore.set({ connected: false, publicKey: null, provider: null, name: null });
      }}
      aria-label="Disconnect wallet - Currently connected as {publicKey.slice(0, 4)}...{publicKey.slice(-4)}"
    >
      <span class="wallet-indicator connected" aria-hidden="true"></span>
      <span class="wallet-info">
        <span class="wallet-status text-mono text-xs text-success" data-testid="wallet-status">CONNECTED</span>
        <span class="wallet-address text-mono text-sm" data-testid="wallet-address">
          {publicKey.slice(0, 4)}...{publicKey.slice(-4)}
        </span>
      </span>
      <span class="wallet-action text-mono text-xs text-muted">Click to disconnect</span>
    </button>
  {:else}
    <!-- Connect Buttons with Clear Visual State -->
    <div class="connect-box tui-box">
      <div class="connect-header text-mono text-uppercase mb-4">
        &gt; CONNECT_WALLET_TO_START
      </div>

      {#if availableWallets.length === 0}
        <div class="no-wallets text-mono text-muted text-sm">
          <div class="mb-4">[⚠] NO_WALLET_DETECTED</div>
          <div>
            <a href="https://phantom.app" target="_blank" class="text-cyan">
              INSTALL_PHANTOM
            </a>
            {' | '}
            <a href="https://solflare.com" target="_blank" class="text-cyan">
              INSTALL_SOLFLARE
            </a>
          </div>
        </div>
      {:else}
        <div class="wallet-buttons">
          {#each availableWallets as wallet}
            <button
              class="wallet-button disconnected"
              data-testid="wallet-connect-button"
              on:click={() => connectWallet(wallet)}
              aria-label="Connect {wallet.name} wallet"
            >
              <span class="wallet-indicator disconnected" aria-hidden="true"></span>
              <span class="wallet-text text-mono">[{wallet.name.toUpperCase()}]</span>
            </button>
          {/each}
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .wallet-connect {
    width: 100%;
    display: flex;
    justify-content: center;
  }

  .detecting {
    padding: var(--space-4);
    font-size: var(--text-sm);
  }

  .connect-box {
    padding: var(--space-8);
    min-width: 400px;
    text-align: center;
  }

  .connected-box {
    padding: var(--space-6);
    text-align: center;
    border: 2px solid var(--zyber-border-secondary);
    background: rgba(6, 182, 212, 0.1);
    border-radius: var(--radius-md);
  }

  .connect-header {
    font-size: var(--text-lg);
    font-weight: 600;
  }

  .wallet-buttons {
    display: flex;
    gap: var(--space-4);
    justify-content: center;
    flex-wrap: wrap;
  }

  .no-wallets {
    text-align: center;
  }

  /* Wallet button states */
  .wallet-button {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-4);
    border: 2px solid var(--zyber-border-muted);
    border-radius: var(--radius-md);
    background: var(--zyber-bg-glass);
    cursor: pointer;
    transition: all var(--transition-base);
    width: 100%;
  }

  .wallet-button.connected {
    border-color: var(--zyber-success);
    background: rgba(16, 185, 129, 0.05);
  }

  .wallet-button.connected:hover {
    background: rgba(16, 185, 129, 0.1);
    box-shadow: 0 0 20px rgba(16, 185, 129, 0.3);
  }

  .wallet-button.disconnected {
    border-color: var(--zyber-cyber-cyan);
  }

  .wallet-button.disconnected:hover {
    background: rgba(6, 182, 212, 0.1);
    box-shadow: 0 0 20px rgba(6, 182, 212, 0.3);
  }

  .wallet-indicator {
    width: 12px;
    height: 12px;
    border-radius: 50%;
    flex-shrink: 0;
  }

  .wallet-indicator.connected {
    background: var(--zyber-success);
    box-shadow: 0 0 8px var(--zyber-success);
    animation: pulse 2s ease-in-out infinite;
  }

  .wallet-indicator.disconnected {
    background: var(--zyber-border-muted);
  }

  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.6; }
  }

  .wallet-info {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    flex: 1;
  }

  .wallet-text {
    flex: 1;
  }

  /* Responsive */
  @media (max-width: 768px) {
    .connect-box {
      min-width: 0;
      width: 100%;
      padding: var(--space-6);
    }

    .wallet-buttons {
      flex-direction: column;
    }

    .btn {
      width: 100%;
    }
  }
</style>
