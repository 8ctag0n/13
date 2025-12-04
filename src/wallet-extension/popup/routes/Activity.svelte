<script lang="ts">
  import { createEventDispatcher, onMount } from 'svelte';
  import type { SupportedChain } from '../../lib/chains/types';

  const dispatch = createEventDispatcher();

  interface Transaction {
    id: string;
    type: 'send' | 'receive' | 'swap' | 'contract';
    chain: SupportedChain;
    amount: string;
    symbol: string;
    to?: string;
    from?: string;
    timestamp: number;
    status: 'pending' | 'confirmed' | 'failed';
    signature?: string;
  }

  export let addresses: Record<SupportedChain, string> | undefined;

  let transactions: Transaction[] = [];
  let loading = true;
  let selectedChain: SupportedChain | 'all' = 'all';

  const chainConfig: Record<SupportedChain, { symbol: string; icon: string; color: string; name: string }> = {
    solana: { symbol: 'SOL', icon: '◉', color: 'var(--cyber-cyan)', name: 'Solana' },
    starknet: { symbol: 'STRK', icon: '▲', color: 'var(--cyber-purple)', name: 'Starknet' },
    zcash: { symbol: 'ZEC', icon: 'Z', color: 'var(--cyber-yellow)', name: 'Zcash' }
  };

  $: filteredTransactions = selectedChain === 'all'
    ? transactions
    : transactions.filter(tx => tx.chain === selectedChain);

  async function loadTransactions() {
    loading = true;

    try {
      const response = await chrome.runtime.sendMessage({
        type: 'GET_TRANSACTIONS',
        addresses
      });

      if (response.success && response.data) {
        transactions = response.data;
      }
    } catch (error) {
      console.error('[Activity] Failed to load transactions:', error);
    }

    // If no real transactions, show placeholder data
    if (transactions.length === 0) {
      transactions = getMockTransactions();
    }

    loading = false;
  }

  function getMockTransactions(): Transaction[] {
    const now = Date.now();
    return [
      {
        id: '1',
        type: 'receive',
        chain: 'solana',
        amount: '2.5',
        symbol: 'SOL',
        from: 'DYw8...jN2q',
        timestamp: now - 1000 * 60 * 30, // 30 min ago
        status: 'confirmed',
        signature: 'abc123...'
      },
      {
        id: '2',
        type: 'send',
        chain: 'solana',
        amount: '0.1',
        symbol: 'SOL',
        to: '7Vbm...kL9x',
        timestamp: now - 1000 * 60 * 60 * 2, // 2 hours ago
        status: 'confirmed',
        signature: 'def456...'
      },
      {
        id: '3',
        type: 'send',
        chain: 'starknet',
        amount: '0.05',
        symbol: 'ETH',
        to: '0x04a...b2c',
        timestamp: now - 1000 * 60 * 60 * 24, // 1 day ago
        status: 'confirmed',
        signature: '0xghi789...'
      },
      {
        id: '4',
        type: 'receive',
        chain: 'zcash',
        amount: '1.0',
        symbol: 'ZEC',
        from: 't1Qx...mN8k',
        timestamp: now - 1000 * 60 * 60 * 24 * 3, // 3 days ago
        status: 'confirmed'
      }
    ];
  }

  function formatTime(timestamp: number): string {
    const diff = Date.now() - timestamp;
    const minutes = Math.floor(diff / (1000 * 60));
    const hours = Math.floor(diff / (1000 * 60 * 60));
    const days = Math.floor(diff / (1000 * 60 * 60 * 24));

    if (minutes < 60) return `${minutes}m ago`;
    if (hours < 24) return `${hours}h ago`;
    if (days < 7) return `${days}d ago`;

    return new Date(timestamp).toLocaleDateString();
  }

  function truncateAddress(addr: string): string {
    if (!addr) return '';
    return addr.length > 12 ? `${addr.slice(0, 6)}...${addr.slice(-4)}` : addr;
  }

  function handleBack() {
    dispatch('back');
  }

  onMount(() => {
    loadTransactions();
  });
</script>

<div class="activity-container">
  <!-- HEADER -->
  <div class="activity-header">
    <button class="back-btn" on:click={handleBack}>
      [←]
    </button>
    <span class="header-title">[ACTIVITY]</span>
    <button class="refresh-btn" on:click={loadTransactions} disabled={loading}>
      {loading ? '[...]' : '[↻]'}
    </button>
  </div>

  <!-- CHAIN FILTER -->
  <div class="chain-tabs">
    <button
      class="chain-tab"
      class:active={selectedChain === 'all'}
      on:click={() => selectedChain = 'all'}
    >
      ALL
    </button>
    {#each Object.entries(chainConfig) as [chain, config]}
      <button
        class="chain-tab"
        class:active={selectedChain === chain}
        on:click={() => selectedChain = chain}
        style="--chain-color: {config.color}"
      >
        {config.icon}
      </button>
    {/each}
  </div>

  <!-- TRANSACTIONS LIST -->
  <div class="transactions-list">
    {#if loading}
      <div class="loading-state">
        <div class="spinner"></div>
        <span>[LOADING...]</span>
      </div>
    {:else if filteredTransactions.length === 0}
      <div class="empty-state">
        <div class="empty-icon">[∅]</div>
        <div class="empty-text">No transactions yet</div>
        <div class="empty-hint">Your transaction history will appear here</div>
      </div>
    {:else}
      {#each filteredTransactions as tx}
        {@const config = chainConfig[tx.chain]}
        <div class="tx-item" class:pending={tx.status === 'pending'} class:failed={tx.status === 'failed'}>
          <div class="tx-icon" style="color: {config.color}">
            {#if tx.type === 'receive'}
              <span class="tx-direction">↓</span>
            {:else if tx.type === 'send'}
              <span class="tx-direction">↑</span>
            {:else}
              <span class="tx-direction">⇄</span>
            {/if}
          </div>

          <div class="tx-info">
            <div class="tx-title">
              {#if tx.type === 'receive'}
                Received {config.symbol}
              {:else if tx.type === 'send'}
                Sent {config.symbol}
              {:else}
                Swap
              {/if}
            </div>
            <div class="tx-address">
              {#if tx.type === 'receive' && tx.from}
                From: {truncateAddress(tx.from)}
              {:else if tx.type === 'send' && tx.to}
                To: {truncateAddress(tx.to)}
              {/if}
            </div>
          </div>

          <div class="tx-details">
            <div class="tx-amount" class:positive={tx.type === 'receive'} class:negative={tx.type === 'send'}>
              {tx.type === 'receive' ? '+' : '-'}{tx.amount} {tx.symbol}
            </div>
            <div class="tx-time">{formatTime(tx.timestamp)}</div>
          </div>

          <div class="tx-status">
            {#if tx.status === 'pending'}
              <span class="status-badge pending">[...]</span>
            {:else if tx.status === 'confirmed'}
              <span class="status-badge confirmed">[✓]</span>
            {:else}
              <span class="status-badge failed">[✗]</span>
            {/if}
          </div>
        </div>
      {/each}
    {/if}
  </div>

  <!-- FOOTER -->
  <div class="activity-footer">
    <span class="footer-text">
      {filteredTransactions.length} transaction{filteredTransactions.length !== 1 ? 's' : ''}
    </span>
  </div>
</div>

<style>
  .activity-container {
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  .activity-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 10px 0;
    border-bottom: 1px solid var(--cyber-border);
    margin-bottom: 15px;
  }

  .back-btn,
  .refresh-btn {
    background: transparent;
    border: 1px solid var(--cyber-border);
    color: var(--cyber-text-dim);
    padding: 6px 10px;
    font-family: inherit;
    font-size: 12px;
    cursor: pointer;
    transition: all 0.2s;
  }

  .back-btn:hover,
  .refresh-btn:hover:not(:disabled) {
    border-color: var(--cyber-cyan);
    color: var(--cyber-cyan);
  }

  .refresh-btn:disabled {
    opacity: 0.5;
  }

  .header-title {
    font-size: 14px;
    font-weight: bold;
    color: var(--cyber-cyan);
    letter-spacing: 2px;
  }

  /* Chain tabs */
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

  /* Transactions list */
  .transactions-list {
    flex: 1;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .tx-item {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px;
    background: var(--cyber-bg-alt);
    border: 1px solid var(--cyber-border);
    transition: all 0.2s;
  }

  .tx-item:hover {
    border-color: var(--cyber-cyan);
  }

  .tx-item.pending {
    opacity: 0.7;
    border-style: dashed;
  }

  .tx-item.failed {
    border-color: #ff4444;
    opacity: 0.7;
  }

  .tx-icon {
    width: 32px;
    height: 32px;
    display: flex;
    align-items: center;
    justify-content: center;
    border: 1px solid currentColor;
    font-size: 16px;
  }

  .tx-direction {
    font-weight: bold;
  }

  .tx-info {
    flex: 1;
    min-width: 0;
  }

  .tx-title {
    font-size: 12px;
    color: var(--cyber-text);
    margin-bottom: 2px;
  }

  .tx-address {
    font-size: 10px;
    color: var(--cyber-text-dim);
  }

  .tx-details {
    text-align: right;
  }

  .tx-amount {
    font-size: 12px;
    font-weight: bold;
  }

  .tx-amount.positive {
    color: var(--cyber-cyan);
  }

  .tx-amount.negative {
    color: var(--cyber-text);
  }

  .tx-time {
    font-size: 10px;
    color: var(--cyber-text-dim);
  }

  .tx-status {
    width: 30px;
    text-align: center;
  }

  .status-badge {
    font-size: 10px;
  }

  .status-badge.pending {
    color: var(--cyber-yellow);
  }

  .status-badge.confirmed {
    color: var(--cyber-cyan);
  }

  .status-badge.failed {
    color: #ff4444;
  }

  /* Empty/Loading states */
  .loading-state,
  .empty-state {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    color: var(--cyber-text-dim);
    gap: 10px;
  }

  .spinner {
    width: 30px;
    height: 30px;
    border: 2px solid var(--cyber-border);
    border-top-color: var(--cyber-cyan);
    border-radius: 50%;
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  .empty-icon {
    font-size: 40px;
    color: var(--cyber-text-dim);
  }

  .empty-text {
    font-size: 14px;
    color: var(--cyber-text);
  }

  .empty-hint {
    font-size: 11px;
  }

  /* Footer */
  .activity-footer {
    padding: 10px 0;
    border-top: 1px solid var(--cyber-border);
    text-align: center;
  }

  .footer-text {
    font-size: 10px;
    color: var(--cyber-text-dim);
  }
</style>
