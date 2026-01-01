<script>
  export let data = [];
  export let height = 8;
  export let width = 20;
  export let color = 'cyan';

  const chars = [' ', '▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

  function getSparkline() {
    if (data.length === 0) return '';

    const slicedData = data.slice(-width);
    const min = Math.min(...slicedData);
    const max = Math.max(...slicedData);
    const range = max - min || 1;

    return slicedData.map(val => {
      const normalized = ((val - min) / range) * (height - 1);
      const charIndex = Math.min(Math.round(normalized) + 1, chars.length - 1);
      return chars[charIndex];
    }).join('');
  }

  $: sparkline = getSparkline();
</script>

<span
  class="ascii-sparkline text-mono"
  class:text-cyan={color === 'cyan'}
  class:text-violet={color === 'violet'}
  class:text-success={color === 'success'}
  class:text-warning={color === 'warning'}
>
  {sparkline}
</span>

<style>
  .ascii-sparkline {
    letter-spacing: 1px;
    font-size: var(--text-base);
  }

  .text-cyan { color: var(--zyber-cyber-cyan); }
  .text-violet { color: var(--zyber-quantum-violet); }
  .text-success { color: var(--zyber-success); }
  .text-warning { color: var(--zyber-warning); }
</style>
