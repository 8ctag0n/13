<script>
  export let label = '';
  export let tone = 'cyan';
  export let variant = 'solid';
  export let size = 'md';
  export let type = 'button';
  export let disabled = false;
  export let fullWidth = false;

  const toneClasses = {
    violet: 'tone-violet',
    cyan: 'tone-cyan',
    success: 'tone-success',
    warning: 'tone-warning',
    error: 'tone-error',
    muted: 'tone-muted'
  };

  $: toneClass = toneClasses[tone] || toneClasses.cyan;
</script>

<button
  class="terminal-button {toneClass} {variant} {size} {fullWidth ? 'full' : ''}"
  type={type}
  disabled={disabled}
  aria-label={label}
>
  <span class="label text-mono">[ {label} ]</span>
  {#if $$slots.default}
    <span class="slot"><slot /></span>
  {/if}
</button>

<style>
  .terminal-button {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: var(--space-2);
    border: 1px solid transparent;
    background: rgba(0, 0, 0, 0.35);
    color: var(--zyber-text-primary);
    font-family: var(--font-mono);
    letter-spacing: 0.08em;
    text-transform: uppercase;
    cursor: pointer;
    transition: all var(--transition-base);
    padding: var(--space-2) var(--space-4);
  }

  .terminal-button.full {
    width: 100%;
  }

  .terminal-button.sm {
    padding: var(--space-1) var(--space-3);
    font-size: var(--text-xs);
  }

  .terminal-button.lg {
    padding: var(--space-3) var(--space-6);
    font-size: var(--text-sm);
  }

  .terminal-button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .terminal-button.solid {
    border-color: currentColor;
  }

  .terminal-button.ghost {
    background: transparent;
    border-style: dashed;
  }

  .terminal-button:hover:not(:disabled) {
    transform: translateY(-1px);
    box-shadow: var(--zyber-glow-cyan);
  }

  .terminal-button.tone-violet {
    color: var(--zyber-quantum-violet);
    border-color: rgba(139, 92, 246, 0.5);
  }

  .terminal-button.tone-cyan {
    color: var(--zyber-cyber-cyan);
    border-color: rgba(6, 182, 212, 0.5);
  }

  .terminal-button.tone-success {
    color: var(--zyber-success);
    border-color: rgba(16, 185, 129, 0.6);
    box-shadow: var(--zyber-glow-success);
  }

  .terminal-button.tone-warning {
    color: var(--zyber-warning);
    border-color: rgba(245, 158, 11, 0.6);
  }

  .terminal-button.tone-error {
    color: var(--zyber-error);
    border-color: rgba(239, 68, 68, 0.6);
  }

  .terminal-button.tone-muted {
    color: var(--zyber-text-tertiary);
    border-color: var(--zyber-border-muted);
  }

  .label {
    pointer-events: none;
  }
</style>
