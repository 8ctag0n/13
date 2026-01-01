<script>
  export let value = 0;
  export let length = 20;
  export let label = '';
  export let showPercent = true;
  export let tone = 'cyan';

  const toneClasses = {
    violet: 'tone-violet',
    cyan: 'tone-cyan',
    success: 'tone-success',
    warning: 'tone-warning',
    error: 'tone-error',
    muted: 'tone-muted'
  };

  $: clamped = Math.max(0, Math.min(100, value));
  $: filled = Math.round((clamped / 100) * length);
  $: empty = Math.max(0, length - filled);
  $: bar = `${'█'.repeat(filled)}${'░'.repeat(empty)}`;
  $: toneClass = toneClasses[tone] || toneClasses.cyan;
</script>

<div class="terminal-progress {toneClass}">
  {#if label}
    <div class="progress-label text-mono text-xs text-muted">{label}</div>
  {/if}
  <div class="progress-bar text-mono">
    <span class="bar">{bar}</span>
    {#if showPercent}
      <span class="percent">{clamped}%</span>
    {/if}
  </div>
</div>

<style>
  .terminal-progress {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .progress-bar {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    font-size: var(--text-sm);
    letter-spacing: 0.04em;
  }

  .bar {
    white-space: pre;
  }

  .percent {
    font-size: var(--text-xs);
  }

  .tone-violet .bar {
    color: var(--zyber-quantum-violet);
  }

  .tone-cyan .bar {
    color: var(--zyber-cyber-cyan);
  }

  .tone-success .bar {
    color: var(--zyber-success);
  }

  .tone-warning .bar {
    color: var(--zyber-warning);
  }

  .tone-error .bar {
    color: var(--zyber-error);
  }

  .tone-muted .bar {
    color: var(--zyber-text-tertiary);
  }
</style>
