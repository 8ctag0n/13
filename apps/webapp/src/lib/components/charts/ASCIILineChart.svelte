<script>
  export let data = [];
  export let height = 7;
  export let width = 50;
  export let xLabels = [];
  export let yLabelFormatter = (val) => Math.round(val).toString();

  function createChart() {
    if (data.length === 0) return { grid: [], min: 0, max: 0, step: 0 };

    const yValues = data.map(d => d.y);
    const min = Math.min(...yValues);
    const max = Math.max(...yValues);
    const range = max - min || 1;
    const step = range / (height - 1);

    const grid = Array(height).fill(null).map(() => Array(width).fill(' '));

    const xStep = (width - 1) / Math.max(data.length - 1, 1);

    data.forEach((point, i) => {
      const x = Math.round(i * xStep);
      const normalizedY = ((point.y - min) / range) * (height - 1);
      const y = height - 1 - Math.round(normalizedY);

      if (x >= 0 && x < width && y >= 0 && y < height) {
        grid[y][x] = '●';

        if (i > 0) {
          const prevX = Math.round((i - 1) * xStep);
          const prevNormalizedY = ((yValues[i - 1] - min) / range) * (height - 1);
          const prevY = height - 1 - Math.round(prevNormalizedY);
          drawLine(grid, prevX, prevY, x, y);
        }
      }
    });

    return { grid, min, max, step };
  }

  function drawLine(grid, x0, y0, x1, y1) {
    const dx = Math.abs(x1 - x0);
    const dy = Math.abs(y1 - y0);
    const sx = x0 < x1 ? 1 : -1;
    const sy = y0 < y1 ? 1 : -1;
    let err = dx - dy;
    let x = x0, y = y0;

    while (true) {
      if (x >= 0 && x < grid[0].length && y >= 0 && y < grid.length) {
        if (grid[y][x] === ' ') {
          const char = dy > dx ? '│' : '─';
          grid[y][x] = char;
        }
      }
      if (x === x1 && y === y1) break;
      const e2 = 2 * err;
      if (e2 > -dy) { err -= dy; x += sx; }
      if (e2 < dx) { err += dx; y += sy; }
    }
  }

  $: chartData = createChart();
</script>

<div class="ascii-line-chart text-mono">
  <div class="chart-container">
    {#each chartData.grid as row, i}
      <div class="chart-row">
        <span class="y-label text-muted">
          {yLabelFormatter(chartData.max - (i * chartData.step))}
        </span>
        <span class="y-axis text-muted">┤</span>
        <span class="chart-line text-cyan">{row.join('')}</span>
      </div>
    {/each}
    <div class="x-axis">
      <span class="y-label"></span>
      <span class="y-axis text-muted">└</span>
      <span class="axis-line text-muted">{'─'.repeat(width)}</span>
    </div>
    {#if xLabels.length > 0}
      <div class="x-labels">
        <span class="y-label"></span>
        <span class="spacer"></span>
        <div class="labels-container">
          {#each xLabels as label}
            <span class="x-label text-muted">{label}</span>
          {/each}
        </div>
      </div>
    {/if}
  </div>
</div>

<style>
  .ascii-line-chart {
    font-size: var(--text-xs);
    overflow-x: auto;
  }

  .chart-container {
    display: inline-block;
    min-width: max-content;
  }

  .chart-row {
    display: flex;
    gap: 2px;
    line-height: 1.1;
  }

  .y-label {
    display: inline-block;
    width: 50px;
    text-align: right;
    font-size: var(--text-xs);
  }

  .y-axis {
    display: inline-block;
    width: 12px;
  }

  .chart-line {
    letter-spacing: 0;
    white-space: pre;
  }

  .x-axis {
    display: flex;
    gap: 2px;
  }

  .axis-line {
    letter-spacing: 0;
  }

  .x-labels {
    display: flex;
    gap: 2px;
    margin-top: var(--space-1);
  }

  .labels-container {
    display: flex;
    justify-content: space-between;
    flex: 1;
    padding-left: 12px;
  }

  .x-label {
    font-size: var(--text-xs);
  }

  .spacer { width: 12px; }
  .text-cyan { color: var(--zyber-cyber-cyan); }
  .text-muted { color: var(--zyber-text-muted); }
</style>
