<script>
  import { createEventDispatcher } from 'svelte';
  import { fade, slide } from 'svelte/transition';
  import Tooltip from './Tooltip.svelte';

  const dispatch = createEventDispatcher();

  export let selected = 'sol'; // 'sol' | 'wzec'
  export let disabled = false;

  const paymentMethods = [
    {
      id: 'sol',
      name: 'SOL',
      fullName: 'Solana',
      description: 'Native Solana token - Fast & low fees',
      tooltip: 'Standard Solana payments. No token account needed. Instant processing.',
      icon: '◉',
      color: 'violet',
      recommended: true
    },
    {
      id: 'wzec',
      name: 'wZEC',
      fullName: 'Wrapped Zcash',
      description: 'Private payments with Zcash - SPL Token',
      tooltip: 'Privacy-focused payments using Zcash. Requires wZEC token account (auto-created if needed).',
      icon: '⚡',
      color: 'cyan',
      recommended: false
    }
  ];

  function handleSelect(methodId) {
    if (disabled) return;
    selected = methodId;
    dispatch('change', { payment_method: methodId });
  }

  $: selectedMethod = paymentMethods.find(m => m.id === selected);
</script>

<div class="payment-selector">
  <div class="selector-header">
    <label class="text-mono text-sm text-muted">PAYMENT_METHOD:</label>
    {#if selectedMethod}
      <div class="selected-indicator text-mono text-xs">
        <span class="indicator-icon {selectedMethod.color}">{selectedMethod.icon}</span>
        <span class="text-{selectedMethod.color}">{selectedMethod.name}_SELECTED</span>
      </div>
    {/if}
  </div>

  <div class="methods-grid">
    {#each paymentMethods as method}
      <button
        type="button"
        class="method-card {selected === method.id ? 'selected' : ''} {method.color}"
        class:disabled
        on:click={() => handleSelect(method.id)}
        aria-label="Pay with {method.fullName}"
        role="radio"
        aria-checked={selected === method.id}
      >
        <!-- Radio Indicator -->
        <div class="radio-indicator text-mono">
          {selected === method.id ? '●' : '○'}
        </div>

        <!-- Method Icon & Name -->
        <div class="method-header">
          <div class="method-icon text-mono">{method.icon}</div>
          <div class="method-name">
            <Tooltip text={method.tooltip} position="top" maxWidth="250px">
              <div class="name-main text-mono">{method.name}</div>
            </Tooltip>
            <div class="name-sub text-xs text-muted">{method.fullName}</div>
          </div>
        </div>

        <!-- Description -->
        <div class="method-description text-sm text-muted">
          {method.description}
        </div>

        <!-- Recommended Badge -->
        {#if method.recommended}
          <div class="badge badge-success">RECOMMENDED</div>
        {/if}

        <!-- Selection Border Glow -->
        <div class="selection-glow" aria-hidden="true"></div>
      </button>
    {/each}
  </div>

  <!-- Info Box -->
  {#key selected}
    <div class="info-box" transition:slide={{ duration: 300 }}>
      <div class="text-mono text-xs">
        {#if selected === 'sol'}
          <div class="mb-2">💡 PAYING_WITH_NATIVE_SOL</div>
          <div class="text-muted">
            Most efficient option. Transaction will be created instantly.
          </div>
        {:else if selected === 'wzec'}
          <div class="mb-2">💡 PAYING_WITH_wZEC_TOKEN</div>
          <div class="text-muted">
            Requires wZEC token account. System will check and create if needed.
          </div>
        {/if}
      </div>
    </div>
  {/key}
</div>

<style>
  .payment-selector {
    margin-bottom: var(--space-6);
  }

  .selector-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: var(--space-3);
  }

  .selected-indicator {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-1) var(--space-3);
    background: rgba(0, 0, 0, 0.3);
    border-radius: var(--radius-sm);
    border: 1px solid var(--zyber-border-muted);
  }

  .indicator-icon {
    font-size: var(--text-base);
  }

  .indicator-icon.violet {
    color: var(--zyber-quantum-violet);
  }

  .indicator-icon.cyan {
    color: var(--zyber-cyber-cyan);
  }

  /* Methods Grid */
  .methods-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
    gap: var(--space-4);
    margin-bottom: var(--space-4);
  }

  /* Method Card */
  .method-card {
    position: relative;
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    padding: var(--space-6);
    background: var(--zyber-bg-glass);
    backdrop-filter: blur(20px);
    border: 2px solid var(--zyber-border-muted);
    border-radius: var(--radius-lg);
    cursor: pointer;
    transition: all var(--transition-base);
    text-align: left;
    overflow: hidden;
  }

  .method-card:hover:not(.disabled) {
    border-color: var(--zyber-border-secondary);
    transform: translateY(-2px);
  }

  .method-card.disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  /* Selected State */
  .method-card.selected.violet {
    border-color: var(--zyber-quantum-violet);
    background: rgba(139, 92, 246, 0.05);
    box-shadow: var(--zyber-glow-violet);
  }

  .method-card.selected.cyan {
    border-color: var(--zyber-cyber-cyan);
    background: rgba(6, 182, 212, 0.05);
    box-shadow: var(--zyber-glow-cyan);
  }

  /* Radio Indicator */
  .radio-indicator {
    position: absolute;
    top: var(--space-4);
    right: var(--space-4);
    font-size: var(--text-xl);
    color: var(--zyber-text-muted);
    transition: all var(--transition-base);
  }

  .method-card.selected.violet .radio-indicator {
    color: var(--zyber-quantum-violet);
    text-shadow: var(--zyber-glow-violet);
  }

  .method-card.selected.cyan .radio-indicator {
    color: var(--zyber-cyber-cyan);
    text-shadow: var(--zyber-glow-cyan);
  }

  /* Method Header */
  .method-header {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }

  .method-icon {
    font-size: var(--text-3xl);
    line-height: 1;
    transition: all var(--transition-base);
  }

  .method-card.selected.violet .method-icon {
    color: var(--zyber-quantum-violet);
    filter: drop-shadow(var(--zyber-glow-violet));
  }

  .method-card.selected.cyan .method-icon {
    color: var(--zyber-cyber-cyan);
    filter: drop-shadow(var(--zyber-glow-cyan));
  }

  .method-name {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .name-main {
    font-size: var(--text-lg);
    font-weight: 600;
    color: var(--zyber-text-primary);
  }

  .name-sub {
    opacity: 0.7;
  }

  /* Description */
  .method-description {
    line-height: 1.4;
  }

  /* Selection Glow Effect */
  .selection-glow {
    position: absolute;
    inset: 0;
    opacity: 0;
    background: radial-gradient(
      circle at 50% 50%,
      rgba(139, 92, 246, 0.1) 0%,
      transparent 70%
    );
    transition: opacity var(--transition-base);
    pointer-events: none;
  }

  .method-card.selected .selection-glow {
    opacity: 1;
  }

  .method-card.selected.cyan .selection-glow {
    background: radial-gradient(
      circle at 50% 50%,
      rgba(6, 182, 212, 0.1) 0%,
      transparent 70%
    );
  }

  /* Info Box */
  .info-box {
    padding: var(--space-4);
    border-radius: var(--radius-md);
    background: rgba(6, 182, 212, 0.05);
    border: 1px solid var(--zyber-border-secondary);
    border-left-width: 3px;
  }

  /* Responsive */
  @media (max-width: 640px) {
    .methods-grid {
      grid-template-columns: 1fr;
    }

    .method-card {
      padding: var(--space-4);
    }

    .selector-header {
      flex-direction: column;
      align-items: flex-start;
      gap: var(--space-2);
    }
  }

  /* Accessibility */
  .method-card:focus-visible {
    outline: 2px solid var(--zyber-cyber-cyan);
    outline-offset: 2px;
  }

  /* Reduced motion */
  @media (prefers-reduced-motion: reduce) {
    .method-card,
    .radio-indicator,
    .method-icon,
    .selection-glow {
      transition: none;
    }
  }
</style>
