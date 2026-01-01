<script>
  export let message = 'Loading...';
  export let size = 'md'; // 'sm' | 'md' | 'lg'
  export let inline = false;

  const sizes = {
    sm: '24px',
    md: '40px',
    lg: '64px'
  };
</script>

<div class="loading-container {inline ? 'inline' : ''}">
  <div class="spinner" style="width: {sizes[size]}; height: {sizes[size]}">
    <div class="spinner-ring"></div>
    <div class="spinner-ring"></div>
    <div class="spinner-ring"></div>
  </div>
  {#if message}
    <div class="loading-message text-mono text-sm text-muted">
      {message}<span class="cursor-blink"></span>
    </div>
  {/if}
</div>

<style>
  .loading-container {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--space-4);
    padding: var(--space-6);
  }

  .loading-container.inline {
    display: inline-flex;
    flex-direction: row;
    padding: 0;
  }

  .spinner {
    position: relative;
    display: inline-block;
  }

  .spinner-ring {
    position: absolute;
    inset: 0;
    border: 2px solid transparent;
    border-radius: 50%;
    animation: spin 1.5s cubic-bezier(0.5, 0, 0.5, 1) infinite;
  }

  .spinner-ring:nth-child(1) {
    border-top-color: var(--zyber-quantum-violet);
    animation-delay: -0.45s;
  }

  .spinner-ring:nth-child(2) {
    border-top-color: var(--zyber-cyber-cyan);
    animation-delay: -0.3s;
  }

  .spinner-ring:nth-child(3) {
    border-top-color: var(--zyber-quantum-violet);
    animation-delay: -0.15s;
  }

  @keyframes spin {
    0% {
      transform: rotate(0deg);
    }
    100% {
      transform: rotate(360deg);
    }
  }

  .loading-message {
    text-align: center;
    animation: pulse 2s ease-in-out infinite;
  }

  @keyframes pulse {
    0%, 100% {
      opacity: 1;
    }
    50% {
      opacity: 0.6;
    }
  }
</style>
