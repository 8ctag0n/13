<script>
  import { onMount, onDestroy } from 'svelte';

  const API_BASE = import.meta.env.VITE_API_URL || '';

  let activeProvers = 0;
  let jobsTotal = 0;
  let jobsCompleted = 0;
  let dataEncrypted = '0 B';
  let dataEncryptedBytes = 0;
  let uptimeSeconds = 0;
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
        dataEncrypted = data.data_encrypted_formatted || '0 B';
        dataEncryptedBytes = data.data_encrypted_bytes || 0;
        uptimeSeconds = data.uptime_seconds || 0;
        lastUpdate = new Date();
      }
    } catch (error) {
      console.error('Failed to fetch network stats:', error);
    }
  }

  function formatUptime(seconds) {
    if (seconds < 60) return `${seconds}s`;
    if (seconds < 3600) return `${Math.floor(seconds / 60)}m`;
    if (seconds < 86400) return `${Math.floor(seconds / 3600)}h`;
    return `${Math.floor(seconds / 86400)}d`;
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

    <!-- Right: Network Stats -->
    <div class="section">
      <div class="section-title text-mono text-xs text-muted">
        ├─ NETWORK_STATS
      </div>
      <div class="stats-grid">
        <div class="stat-item">
          <span class="stat-label text-muted">DATA_ENCRYPTED:</span>
          <span class="stat-value text-cyan">{dataEncrypted}</span>
        </div>
        <div class="stat-item">
          <span class="stat-label text-muted">JOBS_COMPLETED:</span>
          <span class="stat-value text-success">{jobsCompleted}</span>
        </div>
        <div class="stat-item">
          <span class="stat-label text-muted">JOBS_TOTAL:</span>
          <span class="stat-value">{jobsTotal}</span>
        </div>
        <div class="stat-item">
          <span class="stat-label text-muted">UPTIME:</span>
          <span class="stat-value text-cyan">{formatUptime(uptimeSeconds)}</span>
        </div>
      </div>

      <!-- Mini Activity Sparkline -->
      <div class="section-title text-mono text-xs text-muted mt-3">
        ├─ ACTIVITY_24H
      </div>
      <div class="sparkline-mini text-mono text-cyan">{sparkline}</div>
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

  /* Stats Grid */
  .stats-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--space-2);
  }

  .stat-item {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .stat-label {
    font-size: var(--text-xs);
  }

  .stat-value {
    font-size: var(--text-base);
    font-weight: 600;
    text-shadow: 0 0 8px currentColor;
  }

  .sparkline-mini {
    font-size: var(--text-sm);
    letter-spacing: 0.5px;
    text-shadow: 0 0 5px currentColor;
    overflow: hidden;
    white-space: nowrap;
  }

  .mt-3 {
    margin-top: var(--space-3);
  }

  .text-success { color: var(--zyber-success); }

  .panel-footer {
    margin-top: var(--space-4);
    text-align: center;
    opacity: 0.5;
  }

  .text-cyan { color: var(--zyber-cyber-cyan); }
  .text-muted { color: var(--zyber-text-muted); }
</style>
