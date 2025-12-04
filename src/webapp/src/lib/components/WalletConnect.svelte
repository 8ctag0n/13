<script>
  import { onMount, createEventDispatcher } from 'svelte';
  import {
    walletStore,
    isZyberLinkAvailable,
    waitForZyberLink,
    CHAIN_CONFIG,
    SUPPORTED_CHAINS
  } from '../stores/wallet';

  const dispatch = createEventDispatcher();

  // Props
  export let mode = 'single'; // 'single' or 'multi' chain connect
  export let defaultChain = 'solana';

  let detecting = true;
  let zyberLinkAvailable = false;
  let selectedChain = defaultChain;
  let retryCount = 0;

  async function detectWallet() {
    detecting = true;
    try {
      await waitForZyberLink(5000); // 5s timeout for slow wallets like Solflare
      zyberLinkAvailable = true;
    } catch {
      zyberLinkAvailable = false;
    }
    detecting = false;
  }

  async function retryDetection() {
    retryCount++;
    console.log(`[Wallet] Retry detection attempt #${retryCount}`);
    await detectWallet();
  }

  onMount(() => {
    detectWallet();
  });

  async function connectSingleChain(chain) {
    console.log(`[WalletConnect] connectSingleChain called for: ${chain}`);
    try {
      await walletStore.connect(chain);
      console.log(`[WalletConnect] Connection successful, address:`, $walletStore.addresses[chain]);
      dispatch('connected', { chain, address: $walletStore.addresses[chain] });
    } catch (err) {
      console.error('[WalletConnect] Failed to connect:', err);
      dispatch('error', { message: err.message });
    }
  }

  async function connectAllChains() {
    try {
      const addresses = await walletStore.connectAll();
      dispatch('connected', { addresses, chains: $walletStore.connectedChains });
    } catch (err) {
      console.error('Failed to connect all:', err);
      dispatch('error', { message: err.message });
    }
  }

  function handleDisconnect() {
    walletStore.disconnect();
    dispatch('disconnected');
  }

  function switchChain(chain) {
    walletStore.setActiveChain(chain);
    dispatch('chainChanged', { chain });
  }

  function truncateAddress(addr) {
    if (!addr) return '';
    return `${addr.slice(0, 4)}...${addr.slice(-4)}`;
  }

  $: isConnected = $walletStore.connected;
  $: connecting = $walletStore.connecting;
  $: activeChain = $walletStore.activeChain;
  $: activeAddress = $walletStore.addresses[activeChain];
  $: connectedChains = $walletStore.connectedChains;
</script>

<div class="wallet-connect">
  {#if detecting}
    <!-- Detecting wallet -->
    <div class="detecting text-mono text-muted">
      <span class="spin">[⠋]</span> DETECTING_WALLET...
    </div>

  {:else if isConnected}
    <!-- Connected State -->
    <div class="connected-panel">
      <!-- Chain selector tabs -->
      {#if connectedChains.length > 1}
        <div class="chain-tabs">
          {#each connectedChains as chain}
            {@const config = CHAIN_CONFIG[chain]}
            <button
              class="chain-tab"
              class:active={activeChain === chain}
              style="--chain-color: {config.color}"
              on:click={() => switchChain(chain)}
            >
              <span class="chain-icon">{config.icon}</span>
              <span class="chain-name">{config.symbol}</span>
            </button>
          {/each}
        </div>
      {/if}

      <!-- Active wallet info -->
      <button
        class="wallet-button connected"
        on:click={handleDisconnect}
        aria-label="Disconnect wallet"
      >
        <span class="wallet-indicator connected"></span>
        <span class="wallet-info">
          <span class="wallet-chain" style="color: {CHAIN_CONFIG[activeChain].color}">
            {CHAIN_CONFIG[activeChain].icon} {CHAIN_CONFIG[activeChain].name}
          </span>
          <span class="wallet-address text-mono">
            {truncateAddress(activeAddress)}
          </span>
        </span>
        <span class="wallet-action text-mono text-xs text-muted">[DISCONNECT]</span>
      </button>

      <!-- Show all connected addresses -->
      {#if connectedChains.length > 1}
        <div class="all-addresses">
          {#each connectedChains as chain}
            {#if $walletStore.addresses[chain]}
              <div class="address-row" style="--chain-color: {CHAIN_CONFIG[chain].color}">
                <span class="addr-chain">{CHAIN_CONFIG[chain].icon}</span>
                <span class="addr-value text-mono">{truncateAddress($walletStore.addresses[chain])}</span>
              </div>
            {/if}
          {/each}
        </div>
      {/if}
    </div>

  {:else if !zyberLinkAvailable}
    <!-- No wallet detected -->
    <div class="connect-box tui-box">
      <div class="connect-header text-mono text-uppercase mb-4">
        &gt; WALLET_NOT_DETECTED
      </div>
      <div class="no-wallet text-mono text-muted text-sm">
        <div class="mb-4">[⚠] NO_SOLANA_WALLET_FOUND</div>

        <!-- Retry button -->
        <button
          class="retry-button mb-4"
          on:click={retryDetection}
          disabled={detecting}
        >
          {detecting ? '[DETECTING...]' : '[RETRY DETECTION]'}
        </button>

        <div class="install-hint">
          If you have a wallet installed, click retry.<br/>
          Otherwise, install one of these:<br/>
          <a href="https://phantom.app" target="_blank" class="text-cyan">Phantom</a> |
          <a href="https://solflare.com" target="_blank" class="text-cyan">Solflare</a> |
          <a href="https://backpack.app" target="_blank" class="text-cyan">Backpack</a>
        </div>

        {#if retryCount > 0}
          <div class="debug-info mt-4">
            <div class="text-xs text-muted">Retry attempts: {retryCount}</div>
            <div class="text-xs text-muted">Check browser console for debug info</div>
          </div>
        {/if}
      </div>
    </div>

  {:else}
    <!-- Connect options -->
    <div class="connect-box tui-box">
      <div class="connect-header text-mono text-uppercase mb-4">
        &gt; CONNECT_WALLET
      </div>

      {#if mode === 'multi'}
        <!-- Multi-chain connect -->
        <div class="connect-options">
          <button
            class="wallet-button connect-all"
            on:click={connectAllChains}
            disabled={connecting}
          >
            <span class="wallet-indicator"></span>
            <span class="wallet-text text-mono">
              {connecting ? '[CONNECTING...]' : '[CONNECT ALL CHAINS]'}
            </span>
          </button>

          <div class="divider">
            <span>or select chain</span>
          </div>

          <div class="chain-buttons">
            {#each SUPPORTED_CHAINS as chain}
              {@const config = CHAIN_CONFIG[chain]}
              <button
                class="chain-button"
                class:disabled={!config.enabled}
                style="--chain-color: {config.color}"
                on:click={() => config.enabled && connectSingleChain(chain)}
                disabled={connecting || !config.enabled}
                title={config.enabled ? '' : 'COMING SOON'}
              >
                <span class="chain-icon">{config.icon}</span>
                <span class="chain-name">{config.name}</span>
                {#if !config.enabled}
                  <span class="coming-soon-badge">SOON</span>
                {/if}
              </button>
            {/each}
          </div>
        </div>

      {:else}
        <!-- Single chain connect -->
        <div class="connect-options">
          <div class="chain-selector mb-4">
            <label class="text-mono text-muted text-sm">SELECT_CHAIN:</label>
            <div class="chain-buttons">
              {#each SUPPORTED_CHAINS as chain}
                {@const config = CHAIN_CONFIG[chain]}
                <button
                  class="chain-button"
                  class:selected={selectedChain === chain && config.enabled}
                  class:disabled={!config.enabled}
                  style="--chain-color: {config.color}"
                  on:click={() => config.enabled && (selectedChain = chain)}
                  disabled={!config.enabled}
                  title={config.enabled ? '' : 'COMING SOON'}
                >
                  <span class="chain-icon">{config.icon}</span>
                  <span class="chain-name">{config.symbol}</span>
                  {#if !config.enabled}
                    <span class="coming-soon-badge">SOON</span>
                  {/if}
                </button>
              {/each}
            </div>
          </div>

          <button
            class="wallet-button primary"
            on:click={() => connectSingleChain(selectedChain)}
            disabled={connecting}
          >
            <span class="wallet-indicator"></span>
            <span class="wallet-text text-mono">
              {connecting ? '[CONNECTING...]' : `[CONNECT ${CHAIN_CONFIG[selectedChain].name.toUpperCase()}]`}
            </span>
          </button>
        </div>
      {/if}

      {#if $walletStore.error}
        <div class="error-msg text-mono text-sm mt-4">
          [ERROR] {$walletStore.error}
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

  .spin {
    display: inline-block;
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    0% { content: '[⠋]'; }
    12% { content: '[⠙]'; }
    25% { content: '[⠹]'; }
    37% { content: '[⠸]'; }
    50% { content: '[⠼]'; }
    62% { content: '[⠴]'; }
    75% { content: '[⠦]'; }
    87% { content: '[⠧]'; }
    100% { content: '[⠇]'; }
  }

  .connect-box {
    padding: var(--space-8);
    min-width: 400px;
    text-align: center;
  }

  .connect-header {
    font-size: var(--text-lg);
    font-weight: 600;
    color: var(--zyber-cyber-cyan);
  }

  .no-wallet {
    text-align: center;
  }

  .install-hint {
    margin-top: var(--space-4);
    padding: var(--space-4);
    background: rgba(0, 0, 0, 0.3);
    border: 1px solid var(--zyber-border-muted);
  }

  /* Chain buttons */
  .chain-buttons {
    display: flex;
    gap: var(--space-3);
    justify-content: center;
    flex-wrap: wrap;
  }

  .chain-button {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-4);
    background: transparent;
    border: 2px solid var(--zyber-border-muted);
    border-radius: var(--radius-md);
    cursor: pointer;
    transition: all 0.2s;
    min-width: 80px;
  }

  .chain-button:hover:not(.disabled) {
    border-color: var(--chain-color);
    background: rgba(0, 255, 159, 0.05);
  }

  .chain-button.disabled {
    opacity: 0.4;
    cursor: not-allowed;
    filter: grayscale(70%);
    position: relative;
  }

  .chain-button.disabled:hover {
    border-color: var(--zyber-border-muted);
    background: transparent;
  }

  .coming-soon-badge {
    position: absolute;
    top: -8px;
    right: -8px;
    background: var(--zyber-warning);
    color: var(--zyber-bg-primary);
    font-size: 8px;
    font-weight: 700;
    padding: 2px 4px;
    border-radius: 3px;
    font-family: var(--font-mono);
  }

  .chain-button.selected {
    border-color: var(--chain-color);
    background: rgba(0, 255, 159, 0.1);
    box-shadow: 0 0 15px color-mix(in srgb, var(--chain-color) 30%, transparent);
  }

  .chain-icon {
    font-size: var(--text-xl);
    color: var(--chain-color);
  }

  .chain-name {
    font-size: var(--text-xs);
    font-family: var(--font-mono);
    color: var(--zyber-text-secondary);
  }

  /* Wallet button */
  .wallet-button {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-4);
    border: 2px solid var(--zyber-border-muted);
    border-radius: var(--radius-md);
    background: var(--zyber-bg-glass);
    cursor: pointer;
    transition: all 0.2s;
    width: 100%;
    font-family: inherit;
  }

  .wallet-button:hover:not(:disabled) {
    border-color: var(--zyber-cyber-cyan);
    background: rgba(6, 182, 212, 0.1);
  }

  .wallet-button:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .wallet-button.primary {
    border-color: var(--zyber-cyber-cyan);
    background: rgba(6, 182, 212, 0.1);
  }

  .wallet-button.connected {
    border-color: var(--zyber-success);
    background: rgba(16, 185, 129, 0.05);
  }

  .wallet-button.connected:hover {
    background: rgba(239, 68, 68, 0.1);
    border-color: var(--zyber-error);
  }

  .wallet-button.connect-all {
    margin-bottom: var(--space-4);
  }

  .wallet-indicator {
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: var(--zyber-border-muted);
    flex-shrink: 0;
  }

  .wallet-indicator.connected {
    background: var(--zyber-success);
    box-shadow: 0 0 8px var(--zyber-success);
    animation: pulse 2s ease-in-out infinite;
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
    text-align: left;
  }

  .wallet-chain {
    font-size: var(--text-sm);
    font-weight: 600;
  }

  .wallet-address {
    font-size: var(--text-sm);
    color: var(--zyber-text-secondary);
  }

  .wallet-text {
    flex: 1;
    text-align: left;
  }

  .wallet-action {
    font-size: var(--text-xs);
  }

  /* Connected panel */
  .connected-panel {
    width: 100%;
    max-width: 400px;
  }

  .chain-tabs {
    display: flex;
    gap: var(--space-2);
    margin-bottom: var(--space-4);
  }

  .chain-tab {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: var(--space-2);
    padding: var(--space-3);
    background: transparent;
    border: 1px solid var(--zyber-border-muted);
    border-radius: var(--radius-sm);
    cursor: pointer;
    transition: all 0.2s;
    font-family: var(--font-mono);
    font-size: var(--text-sm);
    color: var(--zyber-text-secondary);
  }

  .chain-tab:hover {
    border-color: var(--chain-color);
    color: var(--chain-color);
  }

  .chain-tab.active {
    border-color: var(--chain-color);
    color: var(--chain-color);
    background: rgba(0, 255, 159, 0.1);
  }

  .all-addresses {
    margin-top: var(--space-4);
    padding: var(--space-3);
    background: rgba(0, 0, 0, 0.2);
    border: 1px solid var(--zyber-border-muted);
    border-radius: var(--radius-sm);
  }

  .address-row {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-2) 0;
    font-size: var(--text-xs);
  }

  .addr-chain {
    color: var(--chain-color);
  }

  .addr-value {
    color: var(--zyber-text-muted);
  }

  /* Divider */
  .divider {
    display: flex;
    align-items: center;
    gap: var(--space-4);
    margin: var(--space-4) 0;
    color: var(--zyber-text-muted);
    font-size: var(--text-xs);
    font-family: var(--font-mono);
  }

  .divider::before,
  .divider::after {
    content: '';
    flex: 1;
    height: 1px;
    background: var(--zyber-border-muted);
  }

  /* Error */
  .error-msg {
    color: var(--zyber-error);
    padding: var(--space-3);
    background: rgba(239, 68, 68, 0.1);
    border: 1px solid var(--zyber-error);
    border-radius: var(--radius-sm);
  }

  /* Retry button */
  .retry-button {
    display: block;
    width: 100%;
    padding: var(--space-3) var(--space-4);
    background: rgba(6, 182, 212, 0.1);
    border: 2px solid var(--zyber-cyber-cyan);
    border-radius: var(--radius-md);
    color: var(--zyber-cyber-cyan);
    font-family: var(--font-mono);
    font-size: var(--text-sm);
    cursor: pointer;
    transition: all 0.2s;
  }

  .retry-button:hover:not(:disabled) {
    background: rgba(6, 182, 212, 0.2);
    box-shadow: 0 0 15px rgba(6, 182, 212, 0.3);
  }

  .retry-button:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .debug-info {
    padding: var(--space-3);
    background: rgba(0, 0, 0, 0.3);
    border: 1px dashed var(--zyber-border-muted);
    border-radius: var(--radius-sm);
  }

  /* Responsive */
  @media (max-width: 768px) {
    .connect-box {
      min-width: 0;
      width: 100%;
      padding: var(--space-6);
    }

    .chain-buttons {
      flex-direction: row;
    }

    .chain-button {
      min-width: 70px;
      padding: var(--space-3);
    }
  }
</style>
