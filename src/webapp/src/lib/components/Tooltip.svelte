<script>
  import { fade } from 'svelte/transition';

  export let text = '';
  export let position = 'top'; // 'top' | 'bottom' | 'left' | 'right'
  export let maxWidth = '200px';

  let showTooltip = false;
  let tooltipElement;
  let triggerElement;

  function handleMouseEnter() {
    showTooltip = true;
  }

  function handleMouseLeave() {
    showTooltip = false;
  }

  function handleFocus() {
    showTooltip = true;
  }

  function handleBlur() {
    showTooltip = false;
  }
</script>

<div
  class="tooltip-wrapper"
  bind:this={triggerElement}
  on:mouseenter={handleMouseEnter}
  on:mouseleave={handleMouseLeave}
  on:focus={handleFocus}
  on:blur={handleBlur}
>
  <slot />

  {#if showTooltip && text}
    <div
      bind:this={tooltipElement}
      class="tooltip tooltip-{position}"
      style="max-width: {maxWidth}"
      transition:fade={{ duration: 150 }}
      role="tooltip"
    >
      <div class="tooltip-content text-mono text-xs">
        {text}
      </div>
      <div class="tooltip-arrow"></div>
    </div>
  {/if}
</div>

<style>
  .tooltip-wrapper {
    position: relative;
    display: inline-block;
  }

  .tooltip {
    position: absolute;
    z-index: var(--z-tooltip);
    padding: var(--space-2) var(--space-3);
    background: var(--zyber-bg-elevated);
    border: 1px solid var(--zyber-border-secondary);
    border-radius: var(--radius-md);
    box-shadow:
      var(--zyber-glow-cyan),
      0 4px 16px rgba(0, 0, 0, 0.5);
    pointer-events: none;
    white-space: normal;
  }

  .tooltip-content {
    line-height: 1.4;
    color: var(--zyber-text-secondary);
  }

  .tooltip-arrow {
    position: absolute;
    width: 8px;
    height: 8px;
    background: var(--zyber-bg-elevated);
    border: 1px solid var(--zyber-border-secondary);
    transform: rotate(45deg);
  }

  /* Position: Top */
  .tooltip-top {
    bottom: calc(100% + 8px);
    left: 50%;
    transform: translateX(-50%);
  }

  .tooltip-top .tooltip-arrow {
    bottom: -5px;
    left: 50%;
    transform: translateX(-50%) rotate(45deg);
    border-top: none;
    border-left: none;
  }

  /* Position: Bottom */
  .tooltip-bottom {
    top: calc(100% + 8px);
    left: 50%;
    transform: translateX(-50%);
  }

  .tooltip-bottom .tooltip-arrow {
    top: -5px;
    left: 50%;
    transform: translateX(-50%) rotate(45deg);
    border-bottom: none;
    border-right: none;
  }

  /* Position: Left */
  .tooltip-left {
    right: calc(100% + 8px);
    top: 50%;
    transform: translateY(-50%);
  }

  .tooltip-left .tooltip-arrow {
    right: -5px;
    top: 50%;
    transform: translateY(-50%) rotate(45deg);
    border-left: none;
    border-bottom: none;
  }

  /* Position: Right */
  .tooltip-right {
    left: calc(100% + 8px);
    top: 50%;
    transform: translateY(-50%);
  }

  .tooltip-right .tooltip-arrow {
    left: -5px;
    top: 50%;
    transform: translateY(-50%) rotate(45deg);
    border-right: none;
    border-top: none;
  }
</style>
