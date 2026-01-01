<script>
  import { createEventDispatcher } from 'svelte';
  import TerminalBox from './terminal/TerminalBox.svelte';
  import TerminalButton from './terminal/TerminalButton.svelte';
  import TerminalProgress from './terminal/TerminalProgress.svelte';

  const dispatch = createEventDispatcher();

  export let position = {
    status: 'active',
    claimable: false,
    marketTitle: '',
    id: '',
    outcome: '',
    yourBet: '',
    match: false,
    placedAt: '',
    expiresIn: '',
    settledAt: '',
    payout: '',
    profit: '',
    payoutStatus: '',
    yesPercent: 0,
    noPercent: 0,
    yesPool: '',
    noPool: ''
  };

  export let actionLabel = '';

  const statusMap = {
    active: { label: '[*] ACTIVE', tone: 'cyan' },
    won: { label: '[✓] WON', tone: 'success' },
    lost: { label: '[✗] LOST', tone: 'error' }
  };

  $: status = statusMap[position.status] || statusMap.active;
  $: resolvedAction = actionLabel
    || (position.claimable ? 'CLAIM PAYOUT >' : position.status === 'active' ? 'VIEW MARKET >' : 'VIEW DETAILS >');
  $: settled = position.status !== 'active';
  $: matchLabel = position.match ? '[match]' : '[no match]';
  $: payoutBar = position.claimable ? '████████████████████' : '▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓';

  function handleAction() {
    dispatch('action', { position });
  }
</script>

<TerminalBox tone={status.tone} dense>
  <div class="position-header">
    <div class="status text-mono">{status.label}</div>
    <div class="meta text-mono text-xs text-muted">
      {#if position.claimable}
        <span>CLAIMABLE</span>
      {:else if settled}
        <span>SETTLED</span>
      {:else}
        <span>PENDING</span>
      {/if}
      {#if position.settledAt}
        <span>SETTLED: {position.settledAt}</span>
      {:else if position.expiresIn}
        <span>EXPIRES: {position.expiresIn}</span>
      {/if}
    </div>
  </div>

  <div class="divider text-mono text-muted">
    ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  </div>

  <div class="position-body text-mono text-sm">
    <div class="line">market: {position.marketTitle}</div>
    <div class="line">
      id: {position.id}
      {#if position.outcome}
        <span class="inline">outcome: {position.outcome}</span>
      {/if}
      {#if position.yourBet}
        <span class="inline">your_bet: {position.yourBet} {matchLabel}</span>
      {/if}
    </div>

    {#if position.status === 'active'}
      <div class="line">your_amount: [encrypted] (FHE protected)</div>
      <div class="line pools">
        current_pool:
        <span class="pool">YES {position.yesPercent}%</span>
        <TerminalProgress value={position.yesPercent} length={12} tone="cyan" showPercent={false} />
        <span class="pool">NO {position.noPercent}%</span>
        <TerminalProgress value={position.noPercent} length={12} tone="violet" showPercent={false} />
      </div>
      <div class="status-line">status: ░░░░░░░░░░░░░░░░░░░░ WAITING FOR SETTLEMENT</div>
    {:else}
      {#if position.claimable}
        <div class="payout-box">
          <div class="payout-title">PAYOUT CALCULATION</div>
          <div class="payout-line">your_bet: [encrypted] (FHE protected)</div>
          <div class="payout-line">payout: {position.payout}</div>
          <div class="payout-line">profit: {position.profit}</div>
          <div class="payout-line">status: {payoutBar} READY TO CLAIM</div>
        </div>
      {:else}
        <div class="line">your_bet: [encrypted]</div>
        {#if position.payout}
          <div class="line">payout: {position.payout}</div>
        {/if}
        {#if position.profit}
          <div class="line">loss: {position.profit}</div>
        {/if}
        <div class="status-line">status: {payoutBar} SETTLED - NO ACTION</div>
      {/if}
    {/if}
  </div>

  <div class="position-footer">
    <TerminalButton label={resolvedAction} tone={position.claimable ? 'success' : 'cyan'} size="sm" on:click={handleAction} />
  </div>
</TerminalBox>

<style>
  .position-header {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    gap: var(--space-3);
    flex-wrap: wrap;
  }

  .meta {
    display: flex;
    gap: var(--space-3);
    flex-wrap: wrap;
  }

  .divider {
    font-size: 8px;
    opacity: 0.3;
  }

  .position-body {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .line {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-3);
  }

  .inline {
    color: var(--zyber-text-tertiary);
  }

  .pools {
    align-items: center;
    gap: var(--space-2);
  }

  .pool {
    min-width: 70px;
  }

  .payout-box {
    border: 1px solid var(--zyber-border-muted);
    padding: var(--space-3);
    background: rgba(0, 0, 0, 0.35);
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .payout-title {
    text-transform: uppercase;
    letter-spacing: 0.08em;
    font-size: var(--text-xs);
    color: var(--zyber-text-secondary);
  }

  .payout-line {
    font-size: var(--text-xs);
  }

  .status-line {
    font-size: var(--text-xs);
    color: var(--zyber-text-tertiary);
  }

  .position-footer {
    display: flex;
    justify-content: flex-end;
  }
</style>
