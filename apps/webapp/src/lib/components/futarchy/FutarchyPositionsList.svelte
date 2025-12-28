<script>
  import { createEventDispatcher } from 'svelte';
  import FutarchyPositionCard from './FutarchyPositionCard.svelte';
  import TerminalBox from './terminal/TerminalBox.svelte';
  import TerminalButton from './terminal/TerminalButton.svelte';
  import { getBettorPositions } from '../../utils/futarchy_api';

  const dispatch = createEventDispatcher();

  export let positions = [];
  export let title = 'MY POSITIONS';
  export let subtitle = '';
  export let emptyLabel = 'NO POSITIONS FOUND';
  export let summary = {
    active: 0,
    claimable: 0,
    totalWon: 0,
    totalLost: 0
  };
  export let showHeader = true;
  export let ctaLabel = '';
  export let autoFetch = false;
  export let apiBaseUrl = '';
  export let bettor = '';
  export let autoSummary = true;

  let loading = false;
  let error = '';
  let internalPositions = positions;
  let computedSummary = summary;
  let lastSignature = '';

  function mapPosition(raw) {
    return {
      status: raw.status === 'won' || raw.status === 'lost' ? raw.status : 'active',
      claimable: false,
      marketTitle: `Market ${raw.market_id}`,
      id: raw.market_id,
      outcome: '',
      yourBet: raw.side?.toUpperCase(),
      match: false,
      placedAt: raw.created_at || '',
      expiresIn: '',
      settledAt: '',
      payout: '',
      profit: '',
      yesPercent: 0,
      noPercent: 0,
      yesPool: '',
      noPool: ''
    };
  }

  function buildSummary(items) {
    if (!autoSummary) return summary;
    const result = {
      active: 0,
      claimable: 0,
      totalWon: 0,
      totalLost: 0
    };

    items.forEach((item) => {
      if (item.status === 'active') result.active += 1;
      if (item.claimable) result.claimable += 1;
      if (item.status === 'won') result.totalWon += 1;
      if (item.status === 'lost') result.totalLost += 1;
    });

    return result;
  }

  async function fetchPositions() {
    if (!autoFetch || !bettor) return;
    loading = true;
    error = '';

    try {
      const response = await getBettorPositions(bettor, { apiBaseUrl: apiBaseUrl || undefined });
      internalPositions = (response.positions || []).map(mapPosition);
      computedSummary = buildSummary(internalPositions);
      dispatch('loaded', { positions: internalPositions });
    } catch (err) {
      error = err?.message || 'Failed to load positions';
      dispatch('error', { error: err });
    } finally {
      loading = false;
    }
  }

  function handlePositionAction(event) {
    const { position } = event.detail;
    dispatch('positionAction', { position });
  }

  function handleCta() {
    dispatch('cta');
  }

  export async function reload() {
    await fetchPositions();
  }

  $: if (!autoFetch) {
    internalPositions = positions;
    computedSummary = buildSummary(positions);
  }

  $: if (autoFetch) {
    const signature = JSON.stringify({
      apiBaseUrl,
      bettor
    });
    if (signature !== lastSignature) {
      lastSignature = signature;
      if (bettor) {
        fetchPositions();
      } else {
        internalPositions = [];
        computedSummary = buildSummary([]);
      }
    }
  }
</script>

<div class="positions-list">
  {#if showHeader}
    <TerminalBox tone="violet" dense>
      <div class="list-header">
        <div class="text-mono">
          <div class="title">{title}</div>
          {#if subtitle}
            <div class="subtitle text-muted text-xs">{subtitle}</div>
          {/if}
        </div>
        {#if ctaLabel}
          <TerminalButton label={ctaLabel} tone="cyan" size="sm" on:click={handleCta} />
        {/if}
      </div>
      <slot name="filters" />
    </TerminalBox>
  {/if}

  <div class="list-body">
    {#if loading}
      <TerminalBox tone="muted" dense>
        <div class="empty text-mono">LOADING POSITIONS...</div>
      </TerminalBox>
    {:else if error}
      <TerminalBox tone="warning" dense>
        <div class="empty text-mono">[!] {error}</div>
      </TerminalBox>
    {:else if internalPositions.length === 0}
      <TerminalBox tone="muted" dense>
        <div class="empty text-mono">{emptyLabel}</div>
      </TerminalBox>
    {:else}
      {#each internalPositions as position}
        <FutarchyPositionCard {position} on:action={handlePositionAction} />
      {/each}
    {/if}
  </div>

  <TerminalBox tone="muted" dense>
    <div class="summary text-mono text-xs">
      SUMMARY: active: {computedSummary.active}
      <span class="divider">│</span>
      claimable: {computedSummary.claimable}
      <span class="divider">│</span>
      total_won: {computedSummary.totalWon}
      <span class="divider">│</span>
      total_lost: {computedSummary.totalLost}
    </div>
  </TerminalBox>
</div>

<style>
  .positions-list {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }

  .list-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: var(--space-3);
    flex-wrap: wrap;
  }

  .title {
    text-transform: uppercase;
    letter-spacing: 0.08em;
    font-size: var(--text-sm);
  }

  .list-body {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }

  .summary {
    text-transform: uppercase;
    letter-spacing: 0.08em;
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
    align-items: center;
  }

  .divider {
    opacity: 0.4;
  }

  .empty {
    text-align: center;
    letter-spacing: 0.08em;
  }
</style>
