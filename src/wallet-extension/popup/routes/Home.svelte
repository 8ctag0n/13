<script lang="ts">
  import { createEventDispatcher, onMount } from 'svelte';
  import type { WalletState } from '../../lib/messaging/types';
  import type { SupportedChain } from '../../lib/chains/types';
  import Send from './Send.svelte';
  import Settings from './Settings.svelte';
  import Activity from './Activity.svelte';
  import QRCode from '../components/QRCode.svelte';

  export let walletState: WalletState;

  const dispatch = createEventDispatcher();

  type View = 'home' | 'send' | 'receive' | 'activity' | 'settings';
  type ChainFilter = 'all' | SupportedChain;
  type ZcashAddressType = 'transparent' | 'shielded';

  let view: View = 'home';
  let chainFilter: ChainFilter = 'all';
  let balances: Record<SupportedChain, string> = {
    solana: '0',
    starknet: '0',
    zcash: '0'
  };
  let loading = false;
  let copied = false;

  // Zcash shielded state
  let zcashZAddress: string | null = null;
  let zcashShieldedBalance = '0';
  let zcashTransparentBalance = '0';
  let zcashAddressType: ZcashAddressType = 'transparent';
  let loadingShielded = false;
  let shieldingFunds = false;

  const chainConfig = {
    solana: { symbol: 'SOL', icon: '◉', color: 'var(--cyber-cyan)', name: 'Solana' },
    starknet: { symbol: 'STRK', icon: '▲', color: 'var(--cyber-purple)', name: 'Starknet' },
    zcash: { symbol: 'ZEC', icon: 'Z', color: 'var(--cyber-yellow)', name: 'Zcash' }
  };

  // Mock USD prices (in production would come from API)
  const mockPrices: Record<SupportedChain, number> = {
    solana: 95.50,
    starknet: 0.85,
    zcash: 35.20
  };

  $: totalUSD = Object.entries(balances).reduce((acc, [chain, bal]) => {
    return acc + (parseFloat(bal) * mockPrices[chain as SupportedChain]);
  }, 0);

  $: visibleChains = chainFilter === 'all'
    ? (['solana', 'starknet', 'zcash'] as SupportedChain[])
    : [chainFilter];

  $: selectedChain = chainFilter === 'all' ? 'solana' as SupportedChain : chainFilter;
  $: selectedConfig = chainConfig[selectedChain];

  async function loadAllBalances() {
    if (!walletState.addresses) return;
    loading = true;

    const chains: SupportedChain[] = ['solana', 'starknet', 'zcash'];

    await Promise.all(chains.map(async (chain) => {
      try {
        const response = await chrome.runtime.sendMessage({
          type: 'GET_BALANCE',
          chain,
          address: walletState.addresses![chain]
        });
        if (response.success) {
          balances[chain] = response.data.balance;
        }
      } catch (error) {
        console.error(`[Home] Failed to load ${chain} balance:`, error);
      }
    }));

    balances = { ...balances };
    loading = false;

    // Also load Zcash shielded data
    loadZcashShieldedData();
  }

  async function loadZcashShieldedData() {
    loadingShielded = true;
    try {
      // Get shielded address
      const addrResponse = await chrome.runtime.sendMessage({
        type: 'GET_ZCASH_SHIELDED_ADDRESS'
      });
      if (addrResponse.success && addrResponse.data?.zAddress) {
        zcashZAddress = addrResponse.data.zAddress;
      }

      // Get total balance (transparent + shielded)
      const balResponse = await chrome.runtime.sendMessage({
        type: 'GET_ZCASH_TOTAL_BALANCE'
      });
      if (balResponse.success && balResponse.data) {
        zcashTransparentBalance = balResponse.data.transparent || '0';
        zcashShieldedBalance = balResponse.data.shielded || '0';
        // Update main balance to show total
        balances.zcash = balResponse.data.total || '0';
        balances = { ...balances };
      }
    } catch (error) {
      console.error('[Home] Failed to load Zcash shielded data:', error);
    } finally {
      loadingShielded = false;
    }
  }

  async function handleShieldFunds() {
    if (shieldingFunds) return;
    shieldingFunds = true;

    try {
      const response = await chrome.runtime.sendMessage({
        type: 'SHIELD_ZCASH_FUNDS'
      });

      if (response.success) {
        console.log('[Home] Funds shielded:', response.data);
        // Reload balances
        await loadAllBalances();
      } else {
        console.error('[Home] Shield failed:', response.error);
      }
    } catch (error) {
      console.error('[Home] Shield error:', error);
    } finally {
      shieldingFunds = false;
    }
  }

  async function handleMineBlocks() {
    try {
      const response = await chrome.runtime.sendMessage({
        type: 'MINE_ZCASH_BLOCKS',
        count: 10
      });
      if (response.success) {
        console.log('[Home] Mined blocks:', response.data?.blocks?.length);
        await loadAllBalances();
      }
    } catch (error) {
      console.error('[Home] Mine error:', error);
    }
  }

  function handleLock() {
    dispatch('lock');
  }

  function copyAddress(chain: SupportedChain) {
    if (walletState.addresses) {
      navigator.clipboard.writeText(walletState.addresses[chain]);
      copied = true;
      setTimeout(() => copied = false, 2000);
    }
  }

  function truncateAddress(addr: string): string {
    if (!addr) return '';
    return `${addr.slice(0, 6)}...${addr.slice(-4)}`;
  }

  function formatUSD(amount: number): string {
    return amount.toLocaleString('en-US', { style: 'currency', currency: 'USD' });
  }

  onMount(() => {
    loadAllBalances();
  });
</script>

{#if view === 'home'}
  <!-- HEADER -->
  <div class="home-header">
    <button class="icon-btn" on:click={() => view = 'settings'} title="Settings">
      [⚙]
    </button>
    <div class="logo-small">[ZYBERLINK]</div>
    <button class="icon-btn" on:click={handleLock} title="Lock Wallet">
      [🔒]
    </button>
  </div>

  <!-- TOTAL BALANCE -->
  <div class="balance-card">
    <div class="balance-label">[TOTAL BALANCE]</div>
    {#if loading}
      <div class="balance-amount loading-pulse">[LOADING...]</div>
    {:else}
      <div class="balance-amount">{formatUSD(totalUSD)}</div>
    {/if}
  </div>

  <!-- CHAIN FILTER TABS -->
  <div class="chain-tabs">
    <button
      class="chain-tab"
      class:active={chainFilter === 'all'}
      on:click={() => chainFilter = 'all'}
    >
      ALL
    </button>
    {#each Object.entries(chainConfig) as [chain, config]}
      <button
        class="chain-tab {chain}"
        class:active={chainFilter === chain}
        on:click={() => chainFilter = chain}
        style="--chain-color: {config.color}"
      >
        {config.icon}
      </button>
    {/each}
  </div>

  <!-- QUICK ACTIONS -->
  <div class="quick-actions">
    <button class="action-btn" on:click={() => view = 'receive'}>
      <span class="action-icon">↓</span>
      <span class="action-label">RECEIVE</span>
    </button>
    <button class="action-btn primary" on:click={() => view = 'send'}>
      <span class="action-icon">↑</span>
      <span class="action-label">SEND</span>
    </button>
    <button class="action-btn" disabled title="Coming soon">
      <span class="action-icon">⇄</span>
      <span class="action-label">SWAP</span>
    </button>
  </div>

  <!-- ASSETS LIST -->
  <div class="assets-section">
    <div class="section-header">
      <span>[ASSETS]</span>
      <div class="section-actions">
        <button class="activity-btn" on:click={() => view = 'activity'} title="Activity">
          [≡]
        </button>
        <button class="refresh-btn" on:click={loadAllBalances} disabled={loading}>
          {loading ? '[...]' : '[↻]'}
        </button>
      </div>
    </div>

    <div class="assets-list">
      {#each visibleChains as chain}
        {@const config = chainConfig[chain]}
        {@const bal = parseFloat(balances[chain])}
        {@const usd = bal * mockPrices[chain]}
        {#if chain === 'zcash' && chainFilter === 'zcash'}
          <!-- Zcash expanded view with t/z addresses -->
          <div class="zcash-expanded">
            <button class="asset-item" on:click={() => copyAddress(chain)}>
              <div class="asset-icon" style="color: {config.color}">t</div>
              <div class="asset-info">
                <div class="asset-symbol">ZEC <span class="addr-type">(transparent)</span></div>
                <div class="asset-address">
                  {#if walletState.addresses}
                    {truncateAddress(walletState.addresses[chain])}
                  {/if}
                </div>
              </div>
              <div class="asset-balance">
                <div class="asset-amount">{parseFloat(zcashTransparentBalance).toFixed(4)}</div>
                <div class="asset-usd">{formatUSD(parseFloat(zcashTransparentBalance) * mockPrices.zcash)}</div>
              </div>
            </button>

            <button class="asset-item shielded" on:click={() => { if(zcashZAddress) navigator.clipboard.writeText(zcashZAddress); copied = true; setTimeout(() => copied = false, 2000); }}>
              <div class="asset-icon" style="color: {config.color}">z</div>
              <div class="asset-info">
                <div class="asset-symbol">ZEC <span class="addr-type shielded">(shielded)</span></div>
                <div class="asset-address">
                  {#if zcashZAddress}
                    {truncateAddress(zcashZAddress)}
                  {:else if loadingShielded}
                    Loading...
                  {:else}
                    Not available
                  {/if}
                </div>
              </div>
              <div class="asset-balance">
                <div class="asset-amount">{parseFloat(zcashShieldedBalance).toFixed(4)}</div>
                <div class="asset-usd">{formatUSD(parseFloat(zcashShieldedBalance) * mockPrices.zcash)}</div>
              </div>
            </button>

            <!-- Shield/Mine actions for testing -->
            <div class="zcash-actions">
              <button
                class="zcash-action-btn"
                on:click={handleShieldFunds}
                disabled={shieldingFunds || parseFloat(zcashTransparentBalance) <= 0}
              >
                {shieldingFunds ? '[...]' : '[SHIELD t→z]'}
              </button>
              <button class="zcash-action-btn" on:click={handleMineBlocks}>
                [MINE +10]
              </button>
            </div>
          </div>
        {:else}
          <!-- Normal chain view -->
          <button class="asset-item" on:click={() => copyAddress(chain)}>
            <div class="asset-icon" style="color: {config.color}">{config.icon}</div>
            <div class="asset-info">
              <div class="asset-symbol">{config.symbol}</div>
              <div class="asset-address">
                {#if walletState.addresses}
                  {truncateAddress(walletState.addresses[chain])}
                {/if}
              </div>
            </div>
            <div class="asset-balance">
              <div class="asset-amount">{bal.toFixed(4)}</div>
              <div class="asset-usd">{formatUSD(usd)}</div>
            </div>
          </button>
        {/if}
      {/each}
    </div>
  </div>

  <!-- FOOTER -->
  <div class="home-footer">
    {#if copied}
      <span class="footer-status success">[✓] Address copied!</span>
    {:else}
      <span class="footer-status">Click asset to copy address</span>
    {/if}
  </div>

{:else if view === 'send'}
  <Send
    activeChain={chainFilter === 'all' ? 'solana' : chainFilter}
    address={walletState.addresses?.[chainFilter === 'all' ? 'solana' : chainFilter] || ''}
    on:back={() => view = 'home'}
    on:success={() => { view = 'home'; loadAllBalances(); }}
  />

{:else if view === 'receive'}
  <!-- RECEIVE VIEW -->
  <div class="receive-view">
    <div class="view-header">
      <button class="back-btn" on:click={() => view = 'home'}>[← BACK]</button>
      <span class="view-title">[RECEIVE]</span>
      <div style="width: 60px;"></div>
    </div>

    <div class="receive-tabs">
      {#each Object.entries(chainConfig) as [chain, config]}
        <button
          class="receive-tab {chain}"
          class:active={chainFilter === chain || (chainFilter === 'all' && chain === 'solana')}
          on:click={() => chainFilter = chain}
          style="--chain-color: {config.color}"
        >
          {config.icon} {config.symbol}
        </button>
      {/each}
    </div>

    {#if selectedChain === 'zcash'}
      <!-- Zcash receive with t/z address toggle -->
      <div class="zcash-addr-toggle">
        <button
          class="toggle-btn"
          class:active={zcashAddressType === 'transparent'}
          on:click={() => zcashAddressType = 'transparent'}
        >
          [t] Transparent
        </button>
        <button
          class="toggle-btn shielded"
          class:active={zcashAddressType === 'shielded'}
          on:click={() => zcashAddressType = 'shielded'}
        >
          [z] Shielded
        </button>
      </div>

      <div class="receive-card">
        <div class="qr-wrapper">
          <QRCode
            value={zcashAddressType === 'transparent' ? (walletState.addresses?.zcash || '') : (zcashZAddress || '')}
            size={150}
            color={zcashAddressType === 'transparent' ? 'var(--cyber-yellow)' : 'var(--cyber-purple)'}
          />
        </div>

        <div class="receive-address-label" style="color: {zcashAddressType === 'transparent' ? 'var(--cyber-yellow)' : 'var(--cyber-purple)'}">
          [{zcashAddressType === 'transparent' ? 'TRANSPARENT' : 'SHIELDED'} ADDRESS]
        </div>

        <div class="receive-address">
          {#if zcashAddressType === 'transparent'}
            {walletState.addresses?.zcash || ''}
          {:else}
            {#if zcashZAddress}
              {zcashZAddress}
            {:else if loadingShielded}
              Loading...
            {:else}
              Not available (requires node connection)
            {/if}
          {/if}
        </div>

        <button class="button" on:click={() => {
          const addr = zcashAddressType === 'transparent' ? walletState.addresses?.zcash : zcashZAddress;
          if (addr) {
            navigator.clipboard.writeText(addr);
            copied = true;
            setTimeout(() => copied = false, 2000);
          }
        }}>
          {copied ? '[✓ COPIED!]' : '[COPY ADDRESS]'}
        </button>
      </div>

      <div class="receive-warning" style="border-color: {zcashAddressType === 'transparent' ? 'var(--cyber-yellow)' : 'var(--cyber-purple)'}; color: {zcashAddressType === 'transparent' ? 'var(--cyber-yellow)' : 'var(--cyber-purple)'}">
        {#if zcashAddressType === 'transparent'}
          <strong>[!]</strong> Transparent address - transactions are public
        {:else}
          <strong>[!]</strong> Shielded address - transactions are private
        {/if}
      </div>
    {:else}
      <!-- Standard receive for other chains -->
      <div class="receive-card">
        <div class="qr-wrapper">
          <QRCode
            value={walletState.addresses?.[selectedChain] || ''}
            size={150}
            color={selectedConfig.color}
          />
        </div>

        <div class="receive-address-label" style="color: {selectedConfig.color}">
          [{selectedConfig.name.toUpperCase()} ADDRESS]
        </div>

        {#if walletState.addresses}
          <div class="receive-address">
            {walletState.addresses[selectedChain]}
          </div>
        {/if}

        <button class="button" on:click={() => copyAddress(selectedChain)}>
          {copied ? '[✓ COPIED!]' : '[COPY ADDRESS]'}
        </button>
      </div>

      <div class="receive-warning">
        <strong>[!]</strong> Only send {selectedConfig.symbol} to this address
      </div>
    {/if}
  </div>

{:else if view === 'activity'}
  <Activity
    addresses={walletState.addresses}
    on:back={() => view = 'home'}
  />

{:else if view === 'settings'}
  <Settings on:back={() => view = 'home'} />
{/if}

<style>
  /* HOME HEADER */
  .home-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 10px 0;
    border-bottom: 1px solid var(--cyber-border);
    margin-bottom: 15px;
  }

  .logo-small {
    font-size: 14px;
    font-weight: bold;
    color: var(--cyber-cyan);
    letter-spacing: 2px;
  }

  .icon-btn {
    background: transparent;
    border: 1px solid var(--cyber-border);
    color: var(--cyber-text-dim);
    padding: 6px 10px;
    font-family: inherit;
    font-size: 12px;
    cursor: pointer;
    transition: all 0.2s;
  }

  .icon-btn:hover {
    border-color: var(--cyber-cyan);
    color: var(--cyber-cyan);
  }

  /* BALANCE CARD */
  .balance-card {
    text-align: center;
    padding: 20px;
    background: var(--cyber-bg-alt);
    border: 1px solid var(--cyber-border);
    margin-bottom: 15px;
  }

  .balance-label {
    font-size: 10px;
    color: var(--cyber-text-dim);
    letter-spacing: 2px;
    margin-bottom: 8px;
  }

  .balance-amount {
    font-size: 28px;
    font-weight: bold;
    color: var(--cyber-cyan);
    text-shadow: 0 0 20px rgba(0, 255, 159, 0.3);
  }

  .loading-pulse {
    animation: pulse 1s ease-in-out infinite;
  }

  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.5; }
  }

  /* CHAIN TABS */
  .chain-tabs {
    display: flex;
    gap: 5px;
    margin-bottom: 15px;
  }

  .chain-tab {
    flex: 1;
    padding: 8px;
    background: transparent;
    border: 1px solid var(--cyber-border);
    color: var(--cyber-text-dim);
    font-family: inherit;
    font-size: 11px;
    cursor: pointer;
    transition: all 0.2s;
  }

  .chain-tab:hover {
    border-color: var(--chain-color, var(--cyber-cyan));
    color: var(--chain-color, var(--cyber-cyan));
  }

  .chain-tab.active {
    border-color: var(--chain-color, var(--cyber-cyan));
    color: var(--chain-color, var(--cyber-cyan));
    background: rgba(0, 255, 159, 0.1);
  }

  .chain-tab.solana { --chain-color: var(--cyber-cyan); }
  .chain-tab.starknet { --chain-color: var(--cyber-purple); }
  .chain-tab.zcash { --chain-color: var(--cyber-yellow); }

  /* QUICK ACTIONS */
  .quick-actions {
    display: flex;
    gap: 10px;
    margin-bottom: 15px;
  }

  .action-btn {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    padding: 12px 8px;
    background: var(--cyber-bg-alt);
    border: 1px solid var(--cyber-border);
    color: var(--cyber-text);
    font-family: inherit;
    cursor: pointer;
    transition: all 0.2s;
  }

  .action-btn:hover:not(:disabled) {
    border-color: var(--cyber-cyan);
    transform: translateY(-2px);
  }

  .action-btn.primary {
    border-color: var(--cyber-cyan);
    background: rgba(0, 255, 159, 0.1);
  }

  .action-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .action-icon {
    font-size: 18px;
    color: var(--cyber-cyan);
  }

  .action-label {
    font-size: 10px;
    letter-spacing: 1px;
  }

  /* ASSETS SECTION */
  .assets-section {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 0;
  }

  .section-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: 11px;
    color: var(--cyber-text-dim);
    margin-bottom: 10px;
    letter-spacing: 1px;
  }

  .section-actions {
    display: flex;
    gap: 5px;
  }

  .activity-btn,
  .refresh-btn {
    background: transparent;
    border: none;
    color: var(--cyber-cyan);
    font-family: inherit;
    font-size: 12px;
    cursor: pointer;
    padding: 2px 6px;
  }

  .activity-btn:hover,
  .refresh-btn:hover:not(:disabled) {
    color: var(--cyber-text);
  }

  .refresh-btn:disabled {
    opacity: 0.5;
  }

  .assets-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .asset-item {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px;
    background: var(--cyber-bg-alt);
    border: 1px solid var(--cyber-border);
    cursor: pointer;
    transition: all 0.2s;
    text-align: left;
    font-family: inherit;
    width: 100%;
  }

  .asset-item:hover {
    border-color: var(--cyber-cyan);
    background: rgba(0, 255, 159, 0.05);
  }

  .asset-icon {
    font-size: 20px;
    width: 30px;
    text-align: center;
  }

  .asset-info {
    flex: 1;
  }

  .asset-symbol {
    font-size: 14px;
    font-weight: bold;
    color: var(--cyber-text);
  }

  .asset-address {
    font-size: 10px;
    color: var(--cyber-text-dim);
  }

  .asset-balance {
    text-align: right;
  }

  .asset-amount {
    font-size: 14px;
    color: var(--cyber-text);
  }

  .asset-usd {
    font-size: 11px;
    color: var(--cyber-text-dim);
  }

  /* ZCASH EXPANDED VIEW */
  .zcash-expanded {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .asset-item.shielded {
    border-color: var(--cyber-purple);
    background: rgba(136, 71, 255, 0.05);
  }

  .asset-item.shielded:hover {
    border-color: var(--cyber-purple);
    background: rgba(136, 71, 255, 0.1);
  }

  .addr-type {
    font-size: 9px;
    font-weight: normal;
    color: var(--cyber-text-dim);
  }

  .addr-type.shielded {
    color: var(--cyber-purple);
  }

  .zcash-actions {
    display: flex;
    gap: 8px;
    margin-top: 4px;
  }

  .zcash-action-btn {
    flex: 1;
    padding: 6px 8px;
    background: transparent;
    border: 1px solid var(--cyber-border);
    color: var(--cyber-yellow);
    font-family: inherit;
    font-size: 10px;
    cursor: pointer;
    transition: all 0.2s;
  }

  .zcash-action-btn:hover:not(:disabled) {
    background: rgba(255, 214, 10, 0.1);
    border-color: var(--cyber-yellow);
  }

  .zcash-action-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  /* FOOTER */
  .home-footer {
    padding: 10px 0;
    text-align: center;
    border-top: 1px solid var(--cyber-border);
    margin-top: 15px;
  }

  .footer-status {
    font-size: 10px;
    color: var(--cyber-text-dim);
  }

  .footer-status.success {
    color: var(--cyber-cyan);
  }

  /* RECEIVE VIEW */
  .receive-view {
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  .view-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 10px 0;
    border-bottom: 1px solid var(--cyber-border);
    margin-bottom: 15px;
  }

  .back-btn {
    background: transparent;
    border: 1px solid var(--cyber-border);
    color: var(--cyber-text-dim);
    padding: 6px 10px;
    font-family: inherit;
    font-size: 11px;
    cursor: pointer;
  }

  .back-btn:hover {
    border-color: var(--cyber-cyan);
    color: var(--cyber-cyan);
  }

  .view-title {
    font-size: 14px;
    font-weight: bold;
    color: var(--cyber-cyan);
    letter-spacing: 2px;
  }

  .receive-tabs {
    display: flex;
    gap: 5px;
    margin-bottom: 20px;
  }

  .receive-tab {
    flex: 1;
    padding: 10px;
    background: transparent;
    border: 1px solid var(--cyber-border);
    color: var(--cyber-text-dim);
    font-family: inherit;
    font-size: 11px;
    cursor: pointer;
    transition: all 0.2s;
  }

  .receive-tab.active {
    border-color: var(--chain-color);
    color: var(--chain-color);
    background: rgba(0, 255, 159, 0.1);
  }

  .receive-card {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 20px;
    background: var(--cyber-bg-alt);
    border: 1px solid var(--cyber-border);
  }

  .qr-wrapper {
    margin-bottom: 20px;
  }

  .receive-address-label {
    font-size: 10px;
    letter-spacing: 2px;
    margin-bottom: 10px;
  }

  .receive-address {
    font-size: 11px;
    word-break: break-all;
    text-align: center;
    color: var(--cyber-text);
    background: var(--cyber-bg);
    padding: 10px;
    border: 1px solid var(--cyber-border);
    margin-bottom: 15px;
    width: 100%;
  }

  .receive-warning {
    margin-top: 15px;
    padding: 10px;
    font-size: 11px;
    color: var(--cyber-yellow);
    border: 1px solid var(--cyber-yellow);
    background: rgba(255, 214, 10, 0.1);
    text-align: center;
  }

  /* ZCASH ADDRESS TOGGLE */
  .zcash-addr-toggle {
    display: flex;
    gap: 8px;
    margin-bottom: 15px;
  }

  .toggle-btn {
    flex: 1;
    padding: 10px;
    background: transparent;
    border: 1px solid var(--cyber-border);
    color: var(--cyber-text-dim);
    font-family: inherit;
    font-size: 11px;
    cursor: pointer;
    transition: all 0.2s;
  }

  .toggle-btn:hover {
    border-color: var(--cyber-yellow);
    color: var(--cyber-yellow);
  }

  .toggle-btn.active {
    border-color: var(--cyber-yellow);
    color: var(--cyber-yellow);
    background: rgba(255, 214, 10, 0.1);
  }

  .toggle-btn.shielded:hover {
    border-color: var(--cyber-purple);
    color: var(--cyber-purple);
  }

  .toggle-btn.shielded.active {
    border-color: var(--cyber-purple);
    color: var(--cyber-purple);
    background: rgba(136, 71, 255, 0.1);
  }
</style>
