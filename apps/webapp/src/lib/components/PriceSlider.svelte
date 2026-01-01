<script>
  import { createEventDispatcher } from 'svelte';

  const dispatch = createEventDispatcher();

  // Props from parent
  export let minPrice = 1000000;        // Minimum in lamports
  export let recommendedPrice = 1800000; // Recommended in lamports
  export let maxPrice = 3600000;         // Maximum in lamports
  export let currentPrice = 1800000;     // Current selected price
  export let step = 100000;              // Step size in lamports
  export let isLoading = false;

  // Computed values
  $: minSol = (minPrice / 1_000_000_000).toFixed(6);
  $: recommendedSol = (recommendedPrice / 1_000_000_000).toFixed(6);
  $: maxSol = (maxPrice / 1_000_000_000).toFixed(6);
  $: currentSol = (currentPrice / 1_000_000_000).toFixed(6);

  // Position percentages for markers
  $: recommendedPercent = ((recommendedPrice - minPrice) / (maxPrice - minPrice)) * 100;
  $: currentPercent = ((currentPrice - minPrice) / (maxPrice - minPrice)) * 100;

  // Acceptance level based on current price
  $: acceptanceLevel = getAcceptanceLevel(currentPrice);
  $: acceptanceColor = getAcceptanceColor(acceptanceLevel);

  function getAcceptanceLevel(price) {
    if (price < recommendedPrice * 0.8) return 'low';
    if (price < recommendedPrice) return 'medium';
    return 'high';
  }

  function getAcceptanceColor(level) {
    switch (level) {
      case 'low': return 'var(--zyber-error)';
      case 'medium': return 'var(--zyber-warning)';
      case 'high': return 'var(--zyber-success)';
      default: return 'var(--zyber-text-muted)';
    }
  }

  function getAcceptancePercent(level) {
    switch (level) {
      case 'low': return '~20%';
      case 'medium': return '~60%';
      case 'high': return '>95%';
      default: return '?';
    }
  }

  function handleSliderChange(e) {
    currentPrice = parseInt(e.target.value);
    dispatch('change', { price: currentPrice });
  }

  function setToRecommended() {
    currentPrice = recommendedPrice;
    dispatch('change', { price: currentPrice });
  }
</script>

<div class="price-slider-container">
  <div class="slider-header">
    <span class="text-mono text-sm">PRICE_SELECTION:</span>
    {#if isLoading}
      <span class="text-mono text-xs text-muted">[LOADING...]</span>
    {/if}
  </div>

  <!-- Main Slider -->
  <div class="slider-wrapper">
    <!-- Track background with zones -->
    <div class="slider-track">
      <div
        class="zone zone-low"
        style="width: {Math.max(0, recommendedPercent * 0.8)}%"
      ></div>
      <div
        class="zone zone-medium"
        style="left: {recommendedPercent * 0.8}%; width: {recommendedPercent * 0.2}%"
      ></div>
      <div
        class="zone zone-high"
        style="left: {recommendedPercent}%; width: {100 - recommendedPercent}%"
      ></div>
    </div>

    <!-- Recommended marker -->
    <div
      class="recommended-marker"
      style="left: {recommendedPercent}%"
      title="Recommended price for high acceptance"
    >
      <div class="marker-line"></div>
      <div class="marker-label text-mono text-xs">REC</div>
    </div>

    <!-- Input range -->
    <input
      type="range"
      min={minPrice}
      max={maxPrice}
      {step}
      value={currentPrice}
      on:input={handleSliderChange}
      class="slider-input"
      disabled={isLoading}
    />
  </div>

  <!-- Labels -->
  <div class="slider-labels text-mono text-xs">
    <span class="label-min">
      MIN<br/>
      <span class="text-muted">{minSol}</span>
    </span>
    <span class="label-recommended" style="left: {recommendedPercent}%">
      RECOMMENDED<br/>
      <span class="text-cyan">{recommendedSol}</span>
    </span>
    <span class="label-max">
      MAX<br/>
      <span class="text-muted">{maxSol}</span>
    </span>
  </div>

  <!-- Current Value Display -->
  <div class="current-value-box">
    <div class="value-display">
      <span class="text-mono text-sm text-muted">YOUR_PRICE:</span>
      <span class="text-mono text-lg text-cyan">{currentSol}_SOL</span>
      <span class="text-mono text-xs text-muted">({currentPrice.toLocaleString()} lamports)</span>
    </div>

    <div class="acceptance-indicator" style="--accent-color: {acceptanceColor}">
      <span class="text-mono text-sm">PROVER_ACCEPTANCE:</span>
      <span class="acceptance-badge" class:low={acceptanceLevel === 'low'} class:medium={acceptanceLevel === 'medium'} class:high={acceptanceLevel === 'high'}>
        {acceptanceLevel.toUpperCase()} ({getAcceptancePercent(acceptanceLevel)})
      </span>
    </div>

    {#if currentPrice < recommendedPrice}
      <button
        class="btn-set-recommended text-mono text-xs"
        on:click={setToRecommended}
      >
        [SET_TO_RECOMMENDED]
      </button>
    {/if}
  </div>

  <!-- Info tooltip -->
  <div class="slider-info text-mono text-xs text-muted">
    <span class="info-icon">[?]</span>
    Prices in the <span class="text-success">green zone</span> have highest prover acceptance.
    Lower prices may result in slower job pickup.
  </div>
</div>

<style>
  .price-slider-container {
    padding: var(--space-4);
    background: rgba(0, 0, 0, 0.2);
    border: 1px solid var(--zyber-border-muted);
    border-radius: var(--radius-md);
  }

  .slider-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: var(--space-4);
  }

  .slider-wrapper {
    position: relative;
    height: 40px;
    margin: var(--space-6) 0;
  }

  .slider-track {
    position: absolute;
    top: 50%;
    left: 0;
    right: 0;
    height: 8px;
    transform: translateY(-50%);
    border-radius: 4px;
    overflow: hidden;
    background: var(--zyber-border-muted);
  }

  .zone {
    position: absolute;
    top: 0;
    height: 100%;
  }

  .zone-low {
    left: 0;
    background: linear-gradient(90deg, rgba(239, 68, 68, 0.4), rgba(245, 158, 11, 0.3));
  }

  .zone-medium {
    background: rgba(245, 158, 11, 0.4);
  }

  .zone-high {
    background: linear-gradient(90deg, rgba(16, 185, 129, 0.4), rgba(16, 185, 129, 0.6));
  }

  .recommended-marker {
    position: absolute;
    top: 0;
    bottom: 0;
    transform: translateX(-50%);
    display: flex;
    flex-direction: column;
    align-items: center;
    z-index: 2;
    pointer-events: none;
  }

  .marker-line {
    width: 2px;
    height: 100%;
    background: var(--zyber-cyber-cyan);
    box-shadow: 0 0 8px var(--zyber-cyber-cyan);
  }

  .marker-label {
    position: absolute;
    top: -18px;
    color: var(--zyber-cyber-cyan);
    white-space: nowrap;
  }

  .slider-input {
    position: absolute;
    top: 50%;
    left: 0;
    right: 0;
    transform: translateY(-50%);
    width: 100%;
    height: 20px;
    -webkit-appearance: none;
    appearance: none;
    background: transparent;
    cursor: pointer;
    z-index: 3;
  }

  .slider-input::-webkit-slider-thumb {
    -webkit-appearance: none;
    appearance: none;
    width: 20px;
    height: 20px;
    border-radius: 50%;
    background: var(--zyber-cyber-cyan);
    border: 2px solid var(--zyber-bg-base);
    box-shadow: 0 0 10px var(--zyber-cyber-cyan), 0 2px 4px rgba(0,0,0,0.3);
    cursor: grab;
    transition: transform 0.1s ease;
  }

  .slider-input::-webkit-slider-thumb:hover {
    transform: scale(1.1);
  }

  .slider-input::-webkit-slider-thumb:active {
    cursor: grabbing;
    transform: scale(0.95);
  }

  .slider-input::-moz-range-thumb {
    width: 20px;
    height: 20px;
    border-radius: 50%;
    background: var(--zyber-cyber-cyan);
    border: 2px solid var(--zyber-bg-base);
    box-shadow: 0 0 10px var(--zyber-cyber-cyan);
    cursor: grab;
  }

  .slider-labels {
    position: relative;
    display: flex;
    justify-content: space-between;
    margin-bottom: var(--space-4);
    height: 36px;
  }

  .label-min, .label-max {
    text-align: center;
  }

  .label-recommended {
    position: absolute;
    transform: translateX(-50%);
    text-align: center;
  }

  .current-value-box {
    padding: var(--space-4);
    background: rgba(6, 182, 212, 0.05);
    border: 1px solid var(--zyber-border-secondary);
    border-radius: var(--radius-md);
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  .value-display {
    display: flex;
    align-items: baseline;
    gap: var(--space-3);
    flex-wrap: wrap;
  }

  .acceptance-indicator {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }

  .acceptance-badge {
    padding: var(--space-1) var(--space-3);
    border-radius: var(--radius-sm);
    font-weight: 600;
  }

  .acceptance-badge.low {
    background: rgba(239, 68, 68, 0.2);
    color: var(--zyber-error);
    border: 1px solid rgba(239, 68, 68, 0.4);
  }

  .acceptance-badge.medium {
    background: rgba(245, 158, 11, 0.2);
    color: var(--zyber-warning);
    border: 1px solid rgba(245, 158, 11, 0.4);
  }

  .acceptance-badge.high {
    background: rgba(16, 185, 129, 0.2);
    color: var(--zyber-success);
    border: 1px solid rgba(16, 185, 129, 0.4);
  }

  .btn-set-recommended {
    align-self: flex-start;
    padding: var(--space-2) var(--space-3);
    background: transparent;
    border: 1px solid var(--zyber-cyber-cyan);
    color: var(--zyber-cyber-cyan);
    border-radius: var(--radius-sm);
    cursor: pointer;
    transition: all var(--transition-fast);
  }

  .btn-set-recommended:hover {
    background: rgba(6, 182, 212, 0.1);
    box-shadow: 0 0 10px rgba(6, 182, 212, 0.3);
  }

  .slider-info {
    margin-top: var(--space-3);
    padding: var(--space-2);
    display: flex;
    align-items: flex-start;
    gap: var(--space-2);
  }

  .info-icon {
    color: var(--zyber-cyber-cyan);
  }

  /* Disabled state */
  .slider-input:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .slider-input:disabled::-webkit-slider-thumb {
    cursor: not-allowed;
  }
</style>
