<script>
  import { createEventDispatcher } from 'svelte';
  import TerminalBox from './terminal/TerminalBox.svelte';
  import TerminalButton from './terminal/TerminalButton.svelte';
  import TerminalProgress from './terminal/TerminalProgress.svelte';

  const dispatch = createEventDispatcher();

  export let state = 'idle';
  export let title = '';
  export let message = '';
  export let txHash = '';
  export let confirmations = 0;
  export let totalConfirmations = 32;
  export let errorCode = '';
  export let primaryLabel = '';
  export let secondaryLabel = '';

  const toneMap = {
    idle: 'muted',
    signing: 'cyan',
    confirming: 'violet',
    success: 'success',
    error: 'error'
  };

  $: tone = toneMap[state] || 'muted';
  $: progressValue = totalConfirmations > 0
    ? Math.round((confirmations / totalConfirmations) * 100)
    : 0;

  function handlePrimary() {
    dispatch('primary');
  }

  function handleSecondary() {
    dispatch('secondary');
  }
</script>

{#if state !== 'idle'}
  <TerminalBox tone={tone} dense>
    <div class="status-header text-mono">
      <span class="status-label">
        {#if state === 'signing'}[*] SIGNING{/if}
        {#if state === 'confirming'}[*] CONFIRMING ON-CHAIN{/if}
        {#if state === 'success'}[✓] CONFIRMED{/if}
        {#if state === 'error'}[!] ERROR{/if}
      </span>
      {#if title}
        <span class="status-title">{title}</span>
      {/if}
    </div>

    {#if state === 'confirming'}
      <TerminalProgress value={progressValue} length={24} tone="cyan" />
    {:else if state === 'signing'}
      <TerminalProgress value={20} length={24} tone="cyan" />
    {:else if state === 'success'}
      <TerminalProgress value={100} length={24} tone="success" />
    {/if}

    {#if message}
      <div class="status-message text-mono text-xs">&gt; {message}</div>
    {/if}

    {#if txHash}
      <div class="status-meta text-mono text-xs text-muted">
        tx_hash: {txHash}
      </div>
    {/if}

    {#if state === 'confirming'}
      <div class="status-meta text-mono text-xs text-muted">
        confirmations: {confirmations}/{totalConfirmations}
      </div>
    {/if}

    {#if state === 'error' && errorCode}
      <div class="status-meta text-mono text-xs text-muted">
        error_code: {errorCode}
      </div>
    {/if}

    {#if primaryLabel || secondaryLabel}
      <div class="status-actions">
        {#if secondaryLabel}
          <TerminalButton label={secondaryLabel} tone="muted" size="sm" on:click={handleSecondary} />
        {/if}
        {#if primaryLabel}
          <TerminalButton label={primaryLabel} tone={state === 'error' ? 'warning' : 'cyan'} size="sm" on:click={handlePrimary} />
        {/if}
      </div>
    {/if}
  </TerminalBox>
{/if}

<style>
  .status-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    font-size: var(--text-xs);
  }

  .status-title {
    color: var(--zyber-text-tertiary);
  }

  .status-message {
    line-height: 1.5;
  }

  .status-actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-2);
    flex-wrap: wrap;
  }
</style>
