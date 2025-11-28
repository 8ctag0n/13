<script>
  import { onMount, onDestroy } from 'svelte';

  const API_BASE = import.meta.env.VITE_API_URL || import.meta.env.VITE_BACKEND_URL || 'http://127.0.0.1:8080';

  let activeProvers = 0;
  let totalProvers = 0;
  let jobsTotal = 0;
  let jobsCompleted = 0;
  let lastUpdate = new Date();

  // Activity data (jobs per hour, last 24h)
  let activityData = Array(24).fill(0);
  const barChars = [' ', '▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

  async function fetchStats() {
    try {
      const response = await fetch(`${API_BASE}/api/stats/network`);
      if (response.ok) {
        const data = await response.json();
        activeProvers = data.active_provers || 0;
        jobsTotal = data.jobs_total || 0;
        jobsCompleted = data.jobs_completed || 0;
        lastUpdate = new Date();
      }
    } catch (error) {
      console.error('Failed to fetch network stats:', error);
    }
  }

  function generateActivityData() {
    const now = new Date().getHours();
    activityData = Array(24).fill(0).map((_, i) => {
      const hour = (now - 23 + i + 24) % 24;
      const baseActivity = hour >= 9 && hour <= 21 ? 15 : 5;
      return Math.floor(Math.random() * 10 + baseActivity);
    });
  }

  function getBarChar(value, max) {
    if (max === 0) return barChars[0];
    const normalized = Math.min(value / max, 1);
    const index = Math.round(normalized * (barChars.length - 1));
    return barChars[index];
  }

  function getSparkline() {
    const max = Math.max(...activityData, 1);
    return activityData.map(v => getBarChar(v, max)).join('');
  }

  function timeSinceUpdate() {
    const seconds = Math.floor((new Date() - lastUpdate) / 1000);
    if (seconds < 60) return `${seconds}s ago`;
    return `${Math.floor(seconds / 60)}m ago`;
  }

  function getProverGrid() {
    // Create a 3x5 grid for provers (max 15)
    const grid = [];
    const maxProvers = 15;
    for (let row = 0; row < 3; row++) {
      let line = '';
      for (let col = 0; col < 5; col++) {
        const index = row * 5 + col;
        if (index < activeProvers) {
          line += '[●]';
        } else if (index < maxProvers) {
          line += '[○]';
        }
      }
      grid.push(line);
    }
    return grid;
  }

  let interval;
  let updateTimer;

  onMount(() => {
    fetchStats();
    generateActivityData();
    interval = setInterval(() => {
      fetchStats();
      generateActivityData();
    }, 30000);
    updateTimer = setInterval(() => {
      lastUpdate = lastUpdate;
    }, 1000);
  });

  onDestroy(() => {
    if (interval) clearInterval(interval);
    if (updateTimer) clearInterval(updateTimer);
  });

  $: timeAgo = timeSinceUpdate();
  $: proverGrid = getProverGrid();
  $: sparkline = getSparkline();
  $: peakJobs = Math.max(...activityData);
  $: avgJobs = Math.floor(activityData.reduce((a, b) => a + b, 0) / 24);
</script>

<div class="command-panel tui-box">
  <div class="panel-header text-mono">
    <span class="text-muted">╔══ NETWORK_COMMAND_PANEL ══╗</span>
    <span class="live-tag">
      <span class="live-dot">●</span>
      <span class="text-xs">LIVE</span>
      <span class="text-xs text-muted">{timeAgo}</span>
    </span>
  </div>

  <div class="panel-content">
    <!-- Left: Prover Grid -->
    <div class="section">
      <div class="section-title text-mono text-xs text-muted">
        ├─ PROVER_NETWORK
      </div>
      <div class="prover-grid text-mono">
        {#each proverGrid as row}
          <div class="prover-row text-cyan">{row}</div>
        {/each}
      </div>
      <div class="prover-stats text-mono text-sm">
        <div><span class="text-cyan">{activeProvers}</span> <span class="text-muted">ONLINE</span></div>
        <div><span class="text-muted">{jobsTotal - jobsCompleted}</span> <span class="text-muted">COMPUTING</span></div>
      </div>
    </div>

    <!-- Right: Activity Sparkline -->
    <div class="section">
      <div class="section-title text-mono text-xs text-muted">
        ├─ ACTIVITY_24H
      </div>
      <div class="sparkline-container">
        <div class="sparkline text-mono text-cyan">{sparkline}</div>
        <div class="sparkline-labels text-mono text-xs text-muted">
          <span>-24h</span>
          <span>-12h</span>
          <span>now</span>
        </div>
      </div>
      <div class="activity-stats text-mono text-sm">
        <div><span class="text-muted">PEAK:</span> <span class="text-cyan">{peakJobs}</span> <span class="text-muted">jobs/h</span></div>
        <div><span class="text-muted">AVG:</span> <span class="text-cyan">{avgJobs}</span> <span class="text-muted">jobs/h</span></div>
      </div>
    </div>
  </div>

  <div class="panel-footer text-mono text-xs text-muted">
    ╚════════════════════════════════════════════════════════╝
  </div>
</div>

<style>
  .command-panel {
    background: rgba(0, 10, 20, 0.9);
    border: 1px solid var(--zyber-border-primary);
    padding: var(--space-4);
  }

  .panel-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: var(--space-4);
  }

  .live-tag {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .live-dot {
    color: var(--zyber-success);
    animation: pulse 2s ease-in-out infinite;
  }

  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.4; }
  }

  .panel-content {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--space-6);
  }

  @media (max-width: 768px) {
    .panel-content {
      grid-template-columns: 1fr;
    }
  }

  .section {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  .section-title {
    padding-bottom: var(--space-2);
  }

  /* Prover Grid */
  .prover-grid {
    display: flex;
    flex-direction: column;
    gap: 2px;
    font-size: var(--text-base);
    letter-spacing: 2px;
  }

  .prover-row {
    text-shadow: 0 0 10px currentColor;
  }

  .prover-stats {
    display: flex;
    gap: var(--space-4);
    margin-top: var(--space-2);
  }

  /* Sparkline */
  .sparkline-container {
    background: rgba(0, 0, 0, 0.3);
    padding: var(--space-3);
    border-radius: var(--radius-sm);
  }

  .sparkline {
    font-size: var(--text-xl);
    letter-spacing: 1px;
    text-shadow: 0 0 8px currentColor;
    overflow: hidden;
    white-space: nowrap;
  }

  .sparkline-labels {
    display: flex;
    justify-content: space-between;
    margin-top: var(--space-1);
  }

  .activity-stats {
    display: flex;
    justify-content: space-between;
  }

  .panel-footer {
    margin-top: var(--space-4);
    text-align: center;
    opacity: 0.5;
  }

  .text-cyan { color: var(--zyber-cyber-cyan); }
  .text-muted { color: var(--zyber-text-muted); }
</style>
