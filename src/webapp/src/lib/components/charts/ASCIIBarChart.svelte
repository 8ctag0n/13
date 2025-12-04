<script>
  export let data = [];
  export let maxWidth = 16;
  export let showPercentage = true;
  export let color = 'cyan';

  function getBar(value, max) {
    const percentage = max > 0 ? (value / max) * 100 : 0;
    const filledWidth = Math.round((percentage / 100) * maxWidth);
    const emptyWidth = maxWidth - filledWidth;
    return '█'.repeat(filledWidth) + '░'.repeat(emptyWidth);
  }

  function getPercentage(value, max) {
    return max > 0 ? Math.round((value / max) * 100) : 0;
  }
</script>

<div class="ascii-bar-chart text-mono">
  {#each data as item}
    <div class="chart-row">
      <span class="label">{item.label}:</span>
      <span
        class="bar"
        class:text-cyan={color === 'cyan'}
        class:text-violet={color === 'violet'}
      >
        {getBar(item.value, item.max)}
      </span>
      {#if showPercentage}
        <span class="percentage text-muted">{getPercentage(item.value, item.max)}%</span>
      {/if}
    </div>
  {/each}
</div>

<style>
  .ascii-bar-chart {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    font-size: var(--text-sm);
  }

  .chart-row {
    display: grid;
    grid-template-columns: 90px 1fr auto;
    gap: var(--space-2);
    align-items: center;
  }

  .label {
    text-align: right;
    color: var(--zyber-text-secondary);
    text-transform: uppercase;
    font-size: var(--text-xs);
  }

  .bar {
    letter-spacing: 0;
    font-size: var(--text-sm);
  }

  .percentage {
    min-width: 40px;
    text-align: right;
    font-size: var(--text-xs);
  }

  .text-cyan { color: var(--zyber-cyber-cyan); }
  .text-violet { color: var(--zyber-quantum-violet); }
  .text-muted { color: var(--zyber-text-muted); }
</style>
