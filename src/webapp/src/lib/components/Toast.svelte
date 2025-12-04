<script>
  import { onMount } from 'svelte';
  import { fade, fly } from 'svelte/transition';

  export let message = '';
  export let type = 'info'; // 'success' | 'error' | 'warning' | 'info'
  export let duration = 3000;
  export let onClose = null;

  let visible = true;

  onMount(() => {
    if (duration > 0) {
      const timer = setTimeout(() => {
        visible = false;
        if (onClose) onClose();
      }, duration);

      return () => clearTimeout(timer);
    }
  });

  function handleClose() {
    visible = false;
    if (onClose) onClose();
  }

  const icons = {
    success: '✓',
    error: '✗',
    warning: '⚠',
    info: 'ℹ'
  };
</script>

{#if visible}
  <div
    class="toast toast-{type}"
    transition:fly={{ y: -20, duration: 300 }}
    role="alert"
    aria-live="polite"
  >
    <div class="toast-icon text-mono">
      {icons[type]}
    </div>
    <div class="toast-message text-mono text-sm">
      {message}
    </div>
    <button
      class="toast-close text-mono"
      on:click={handleClose}
      aria-label="Close notification"
    >
      ✕
    </button>
  </div>
{/if}

<style>
  .toast {
    position: fixed;
    top: var(--space-6);
    right: var(--space-6);
    z-index: var(--z-tooltip);
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-4) var(--space-6);
    background: var(--zyber-bg-elevated);
    backdrop-filter: blur(20px);
    border: 2px solid;
    border-radius: var(--radius-lg);
    min-width: 300px;
    max-width: 500px;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5);
  }

  .toast-success {
    border-color: var(--zyber-success);
    box-shadow:
      0 0 20px rgba(16, 185, 129, 0.3),
      0 8px 32px rgba(0, 0, 0, 0.5);
  }

  .toast-error {
    border-color: var(--zyber-error);
    box-shadow:
      0 0 20px rgba(239, 68, 68, 0.3),
      0 8px 32px rgba(0, 0, 0, 0.5);
  }

  .toast-warning {
    border-color: var(--zyber-warning);
    box-shadow:
      0 0 20px rgba(245, 158, 11, 0.3),
      0 8px 32px rgba(0, 0, 0, 0.5);
  }

  .toast-info {
    border-color: var(--zyber-cyber-cyan);
    box-shadow:
      var(--zyber-glow-cyan),
      0 8px 32px rgba(0, 0, 0, 0.5);
  }

  .toast-icon {
    font-size: var(--text-xl);
    line-height: 1;
  }

  .toast-success .toast-icon {
    color: var(--zyber-success);
  }

  .toast-error .toast-icon {
    color: var(--zyber-error);
  }

  .toast-warning .toast-icon {
    color: var(--zyber-warning);
  }

  .toast-info .toast-icon {
    color: var(--zyber-cyber-cyan);
  }

  .toast-message {
    flex: 1;
    line-height: 1.4;
  }

  .toast-close {
    background: none;
    border: none;
    color: var(--zyber-text-muted);
    cursor: pointer;
    font-size: var(--text-lg);
    padding: 0;
    line-height: 1;
    transition: color var(--transition-fast);
  }

  .toast-close:hover {
    color: var(--zyber-text-primary);
  }

  @media (max-width: 640px) {
    .toast {
      top: var(--space-4);
      right: var(--space-4);
      left: var(--space-4);
      min-width: auto;
    }
  }
</style>
