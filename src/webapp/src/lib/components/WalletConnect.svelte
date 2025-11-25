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
    <!-- Connected State -->
    <div class="connected-box tui-box-cyan">
      <div class="text-mono text-uppercase text-sm text-success mb-2">
        [✓] WALLET_CONNECTED
      </div>
      <div class="text-mono text-xs text-muted">
        {publicKey.slice(0, 4)}...{publicKey.slice(-4)}
      </div>
    </div>
  {:else}
    <!-- Connect Buttons -->
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
              class="btn btn-primary"
              on:click={() => connectWallet(wallet)}
            >
              [{wallet.name.toUpperCase()}]
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
