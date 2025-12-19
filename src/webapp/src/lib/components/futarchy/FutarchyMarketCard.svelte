<script>
  import { createEventDispatcher } from 'svelte';
  import TerminalBox from './terminal/TerminalBox.svelte';
  import TerminalButton from './terminal/TerminalButton.svelte';
  import TerminalProgress from './terminal/TerminalProgress.svelte';

  const dispatch = createEventDispatcher();

  export let market = {
    id: '',
    status: 'active',
    expiresIn: '',
    settledAt: '',
    title: '',
    oracle: '',
    maxBet: '',
    outcome: '',
    payoutRatio: '',
    yes: {
      percent: 0,
      pool: '',
      payout: ''
    },
    no: {
      percent: 0,
      pool: '',
      payout: ''
    }
  };

  export let actionLabel = '';
  export let detailsLabel = '';

  const statusMap = {
    active: { label: '[*] ACTIVE', tone: 'cyan' },
    settled: { label: '[✓] SETTLED', tone: 'success' },
    failed: { label: '[✗] FAILED', tone: 'error' }
  };

  $: status = statusMap[market.status] || statusMap.active;
  $: resolvedActionLabel = actionLabel || (market.status === 'active' ? 'PLACE BET >' : 'VIEW DETAILS >');

  function handleAction() {
    dispatch('action', { market });
  }
</script>

<TerminalBox tone={status.tone} dense>
  <div class="market-header">
    <div class="status text-mono">{status.label}</div>
    <div class="meta text-mono text-xs text-muted">
      <span>ID: {market.id}</span>
      {#if market.status === 'active' && market.expiresIn}
        <span>EXPIRES: {market.expiresIn}</span>
      {:else if market.status !== 'active' && market.settledAt}
        <span>SETTLED: {market.settledAt}</span>
      {/if}
    </div>
  </div>

  <div class="divider text-mono text-muted">
    ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  </div>

  <div class="market-title text-mono">&gt; {market.title}</div>

  {#if market.status === 'active'}
    <div class="market-sides">
      <div class="side">
        <div class="side-header text-mono">
          <span class="label">YES</span>
          <span class="muted">pool: {market.yes.pool}</span>
        </div>
        <TerminalProgress value={market.yes.percent} length={22} tone="cyan" showPercent />
        <div class="side-footer text-mono text-xs">
          <span class="muted">if_win:</span>
          <span class="value">{market.yes.payout}</span>
        </div>
      </div>

      <div class="side">
        <div class="side-header text-mono">
          <span class="label">NO</span>
          <span class="muted">pool: {market.no.pool}</span>
        </div>
        <TerminalProgress value={market.no.percent} length={22} tone="violet" showPercent />
        <div class="side-footer text-mono text-xs">
          <span class="muted">if_win:</span>
          <span class="value">{market.no.payout}</span>
        </div>
      </div>
    </div>

    <div class="market-footer text-mono text-xs">
      <span class="muted">oracle: {market.oracle}</span>
      <span class="muted">max_bet: {market.maxBet}</span>
      <TerminalButton label={resolvedActionLabel} tone="cyan" size="sm" on:click={handleAction} />
    </div>
  {:else}
    <div class="settled-body text-mono text-sm">
      <div class="settled-line">
        <span class="muted">outcome:</span>
        <span class="value">{market.outcome}</span>
      </div>
      <div class="settled-line">
        <span class="muted">payout_ratio:</span>
        <span class="value">{market.payoutRatio}</span>
      </div>
    </div>

    <div class="market-footer text-mono text-xs">
      <span class="muted">final_pool: YES {market.yes.pool} vs NO {market.no.pool}</span>
      <TerminalButton label={detailsLabel || resolvedActionLabel} tone="violet" size="sm" on:click={handleAction} />
    </div>
  {/if}
</TerminalBox>

<style>
  .market-header {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    gap: var(--space-3);
    flex-wrap: wrap;
  }

  .meta {
    display: flex;
    gap: var(--space-4);
    flex-wrap: wrap;
  }

  .divider {
    font-size: 8px;
    opacity: 0.3;
  }

  .market-title {
    font-size: var(--text-sm);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .market-sides {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
    gap: var(--space-4);
  }

  .side {
    background: rgba(0, 0, 0, 0.35);
    border: 1px solid var(--zyber-border-muted);
    padding: var(--space-3);
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  .side-header {
    display: flex;
    justify-content: space-between;
    font-size: var(--text-xs);
    text-transform: uppercase;
  }

  .side-header .label {
    color: var(--zyber-text-primary);
  }

  .side-header .muted {
    color: var(--zyber-text-tertiary);
  }

  .side-footer {
    display: flex;
    gap: var(--space-2);
  }

  .market-footer {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: var(--space-3);
    flex-wrap: wrap;
  }

  .muted {
    color: var(--zyber-text-tertiary);
  }

  .settled-body {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .settled-line {
    display: flex;
    gap: var(--space-2);
  }
</style>
