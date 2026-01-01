<script>
  export let title = '';
  export let subtitle = '';
  export let tone = 'violet';
  export let dense = false;
  export let bordered = true;

  const toneClasses = {
    violet: 'tone-violet',
    cyan: 'tone-cyan',
    success: 'tone-success',
    warning: 'tone-warning',
    error: 'tone-error',
    muted: 'tone-muted'
  };

  $: toneClass = toneClasses[tone] || toneClasses.violet;
</script>

<div class="terminal-box tui-box tui-box-ascii {toneClass} {dense ? 'dense' : ''} {bordered ? 'bordered' : 'borderless'}">
  {#if title || subtitle || $$slots.header}
    <div class="box-header">
      {#if title}
        <div class="box-title text-mono">{title}</div>
      {/if}
      {#if subtitle}
        <div class="box-subtitle text-mono text-muted">{subtitle}</div>
      {/if}
      <slot name="header" />
    </div>
  {/if}

  <div class="box-body text-mono">
    <slot />
  </div>

  {#if $$slots.footer}
    <div class="box-footer text-mono">
      <slot name="footer" />
    </div>
  {/if}
</div>

<style>
  .terminal-box {
    font-family: var(--font-mono);
    position: relative;
  }

  .terminal-box.borderless {
    border: 1px solid transparent;
    box-shadow: none;
  }

  .terminal-box.dense {
    padding: var(--space-4);
  }

  .box-header {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    margin-bottom: var(--space-4);
  }

  .box-title {
    text-transform: uppercase;
    letter-spacing: 0.08em;
    font-size: var(--text-sm);
  }

  .box-subtitle {
    font-size: var(--text-xs);
  }

  .box-body {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  .box-footer {
    margin-top: var(--space-4);
    padding-top: var(--space-3);
    border-top: 1px solid var(--zyber-border-muted);
    font-size: var(--text-sm);
  }

  .tone-violet {
    border-color: var(--zyber-border-primary);
    box-shadow: var(--zyber-glow-violet);
  }

  .tone-cyan {
    border-color: var(--zyber-border-secondary);
    box-shadow: var(--zyber-glow-cyan);
  }

  .tone-success {
    border-color: rgba(16, 185, 129, 0.4);
    box-shadow: var(--zyber-glow-success);
  }

  .tone-warning {
    border-color: rgba(245, 158, 11, 0.4);
    box-shadow: 0 0 24px rgba(245, 158, 11, 0.4);
  }

  .tone-error {
    border-color: rgba(239, 68, 68, 0.4);
    box-shadow: 0 0 24px rgba(239, 68, 68, 0.45);
  }

  .tone-muted {
    border-color: var(--zyber-border-muted);
    box-shadow: none;
  }
</style>
