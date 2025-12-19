<script>
  import { createEventDispatcher } from 'svelte';
  import FutarchyMarketCard from './FutarchyMarketCard.svelte';
  import TerminalBox from './terminal/TerminalBox.svelte';
  import TerminalButton from './terminal/TerminalButton.svelte';
  import { listMarkets } from '../../utils/futarchy_api';

  const dispatch = createEventDispatcher();

  export let markets = [];
  export let title = 'MARKETS';
  export let subtitle = '';
  export let emptyLabel = 'NO MARKETS FOUND';
  export let showHeader = true;
  export let ctaLabel = '';
  export let autoFetch = false;
  export let apiBaseUrl = '';
  export let status = '';
  export let creator = '';
  export let limit = 50;
  export let offset = 0;

  let loading = false;
  let error = '';
  let internalMarkets = markets;
  let lastSignature = '';

  const lamportsPerSol = 1_000_000_000;

  function formatSol(lamports) {
    if (lamports === null || lamports === undefined) return '--';
    const value = Number(lamports) / lamportsPerSol;
    return `${value.toFixed(2)} SOL`;
  }

  function mapMarket(raw) {
    const yes = Number(raw.yes_pool_lamports || 0);
    const no = Number(raw.no_pool_lamports || 0);
    const total = yes + no || 1;
    const yesPercent = Math.round((yes / total) * 100);
    const noPercent = Math.max(0, 100 - yesPercent);

    return {
      id: raw.id,
      status: raw.status,
      expiresIn: raw.ends_at || '',
      settledAt: raw.settled_at || '',
      title: raw.question,
      oracle: raw.oracle,
      maxBet: '--',
      outcome: raw.outcome === null || raw.outcome === undefined ? '' : raw.outcome ? 'YES' : 'NO',
      payoutRatio: '--',
      yes: {
        percent: yesPercent,
        pool: formatSol(yes),
        payout: '--'
      },
      no: {
        percent: noPercent,
        pool: formatSol(no),
        payout: '--'
      }
    };
  }

  async function fetchMarkets() {
    if (!autoFetch) return;
    loading = true;
    error = '';

    try {
      const response = await listMarkets({
        apiBaseUrl: apiBaseUrl || undefined,
        status: status || undefined,
        creator: creator || undefined,
        limit,
        offset
      });
      internalMarkets = (response.markets || []).map(mapMarket);
      dispatch('loaded', { markets: internalMarkets });
    } catch (err) {
      error = err?.message || 'Failed to load markets';
      dispatch('error', { error: err });
    } finally {
      loading = false;
    }
  }

  export async function reload() {
    await fetchMarkets();
  }

  function handleCta() {
    dispatch('cta');
  }

  function handleMarketAction(event) {
    const { market } = event.detail;
    dispatch('marketAction', { market });
  }

  $: if (!autoFetch) {
    internalMarkets = markets;
  }

  $: if (autoFetch) {
    const signature = JSON.stringify({
      apiBaseUrl,
      status,
      creator,
      limit,
      offset
    });
    if (signature !== lastSignature) {
      lastSignature = signature;
      fetchMarkets();
    }
  }
</script>

<div class="market-list">
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
        <div class="empty text-mono">LOADING MARKETS...</div>
      </TerminalBox>
    {:else if error}
      <TerminalBox tone="warning" dense>
        <div class="empty text-mono">[!] {error}</div>
      </TerminalBox>
    {:else if internalMarkets.length === 0}
      <TerminalBox tone="muted" dense>
        <div class="empty text-mono">{emptyLabel}</div>
      </TerminalBox>
    {:else}
      {#each internalMarkets as market}
        <FutarchyMarketCard {market} on:action={handleMarketAction} />
      {/each}
    {/if}
  </div>
</div>

<style>
  .market-list {
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

  .empty {
    text-align: center;
    letter-spacing: 0.08em;
  }
</style>
