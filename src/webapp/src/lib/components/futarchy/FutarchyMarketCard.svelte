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
  export let compact = false;
  export let expanded = false;

  const statusMap = {
    active: { label: '[*] ACTIVE', tone: 'cyan' },
    settled: { label: '[✓] SETTLED', tone: 'success' },
    failed: { label: '[✗] FAILED', tone: 'error' }
  };

  $: status = statusMap[market.status] || statusMap.active;
  $: resolvedActionLabel = actionLabel || (market.status === 'active' ? 'PLACE BET >' : 'VIEW DETAILS >');
  $: pyramidWidth = 12;
  $: yesPercentValue = Number(market?.yes?.percent || 0);
  $: noPercentValue = Number(market?.no?.percent || 0);
  $: yesBlocks = Math.min(
    pyramidWidth,
    Math.max(0, Math.round((yesPercentValue / 100) * pyramidWidth))
  );
  $: noBlocks = Math.min(
    pyramidWidth,
    Math.max(0, Math.round((noPercentValue / 100) * pyramidWidth))
  );
  $: yesBar = '█'.repeat(yesBlocks).padStart(pyramidWidth, ' ');
  $: noBar = '█'.repeat(noBlocks).padEnd(pyramidWidth, ' ');

  function handleAction() {
    dispatch('action', { market });
  }

  function handleToggle() {
    dispatch('toggle', { market });
  }
</script>

<TerminalBox tone={status.tone} dense>
  {#if compact}
    <div class="compact-header">
      <div class="compact-text text-mono">
        <div class="compact-title">&gt; {market.title}</div>
        <div class="compact-meta text-xs text-muted">
          {status.label} │ ID: {market.id} │ P(YES): {market.yes.percent}% │ ENDING: {market.expiresIn || '--'}
        </div>
      </div>
      <div class="compact-actions">
        <TerminalButton label={expanded ? 'HIDE GRAPH' : 'GRAPH >'} tone="muted" size="sm" on:click={handleToggle} />
        <TerminalButton label={resolvedActionLabel} tone="cyan" size="sm" on:click={handleAction} />
      </div>
    </div>

    {#if expanded}
      <div class="compact-details">
        <div class="pyramid text-mono text-xs">
          <span class="pyramid-label pyramid-yes">YES</span>
          <span class="pyramid-bars">
            <span class="pyramid-yes">{yesBar}</span><span class="pyramid-divider">┃</span><span class="pyramid-no">{noBar}</span>
          </span>
          <span class="pyramid-label pyramid-no">NO</span>
        </div>
        <div class="detail-row">
          <span class="text-mono text-xs">YES {market.yes.percent}%</span>
          <TerminalProgress value={market.yes.percent} length={20} tone="cyan" showPercent={false} />
          <span class="text-mono text-xs text-muted">{market.yes.pool}</span>
        </div>
        <div class="detail-row">
          <span class="text-mono text-xs">NO {market.no.percent}%</span>
          <TerminalProgress value={market.no.percent} length={20} tone="violet" showPercent={false} />
          <span class="text-mono text-xs text-muted">{market.no.pool}</span>
        </div>
      </div>
    {/if}
  {:else}
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

  .compact-header {
    display: flex;
    justify-content: space-between;
    gap: var(--space-3);
    flex-wrap: wrap;
  }

  .compact-text {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .compact-title {
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }

  .compact-meta {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
  }

  .compact-actions {
    display: flex;
    gap: var(--space-2);
    flex-wrap: wrap;
  }

  .compact-details {
    margin-top: var(--space-3);
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .detail-row {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    flex-wrap: wrap;
  }

  .pyramid {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .pyramid-label {
    min-width: 32px;
  }

  .pyramid-bars {
    letter-spacing: 0.08em;
  }

  .pyramid-yes {
    color: var(--zyber-cyber-cyan);
  }

  .pyramid-no {
    color: var(--zyber-quantum-violet);
  }

  .pyramid-divider {
    color: var(--zyber-text-muted);
  }
</style>
