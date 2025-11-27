<script>
  import { onMount, onDestroy } from 'svelte';

  const API_BASE = import.meta.env.VITE_API_URL || import.meta.env.VITE_BACKEND_URL || 'http://127.0.0.1:8080';

  // Network stats from backend
  let activeProvers = 0;
  let jobsTotal = 0;
  let jobsCompleted = 0;
  let lastUpdate = new Date();

  // Activity data (jobs per hour, last 24h)
  let activityData = Array(24).fill(0);
  let peakJobs = 0;
  let avgJobs = 0;

  // Hexagon animation state
  let hexNodes = [];
  let animationFrame;

  // Initialize hexagon positions
  function initHexagons() {
    const rows = 3;
    const cols = 5;
    hexNodes = [];

    for (let row = 0; row < rows; row++) {
      for (let col = 0; col < cols; col++) {
        const offsetX = row % 2 === 1 ? 20 : 0;
        hexNodes.push({
          x: 30 + col * 40 + offsetX,
          y: 25 + row * 35,
          active: false,
          pulseDelay: Math.random() * 2
        });
      }
    }
  }

  // Animate hexagons based on active provers
  function animateHexagons() {
    const activeCount = Math.min(activeProvers, hexNodes.length);
    hexNodes = hexNodes.map((node, i) => ({
      ...node,
      active: i < activeCount
    }));
  }

  // Fetch network stats
  async function fetchStats() {
    try {
      const response = await fetch(`${API_BASE}/api/stats/network`);
      if (response.ok) {
        const data = await response.json();
        activeProvers = data.active_provers || 0;
        jobsTotal = data.jobs_total || 0;
        jobsCompleted = data.jobs_completed || 0;
        lastUpdate = new Date();
        animateHexagons();
      }
    } catch (error) {
      console.error('Failed to fetch network stats:', error);
    }
  }

  // Generate mock activity data (in production, fetch from backend)
  function generateActivityData() {
    const now = new Date().getHours();
    activityData = Array(24).fill(0).map((_, i) => {
      // Simulate more activity during day hours
      const hour = (now - 23 + i + 24) % 24;
      const baseActivity = hour >= 9 && hour <= 21 ? 15 : 5;
      return Math.floor(Math.random() * 10 + baseActivity);
    });
    peakJobs = Math.max(...activityData);
    avgJobs = Math.floor(activityData.reduce((a, b) => a + b, 0) / 24);
  }

  // Format time since last update
  function timeSinceUpdate() {
    const seconds = Math.floor((new Date() - lastUpdate) / 1000);
    if (seconds < 60) return `${seconds}s ago`;
    return `${Math.floor(seconds / 60)}m ago`;
  }

  let interval;
  let updateTimer;

  onMount(() => {
    initHexagons();
    fetchStats();
    generateActivityData();

    // Refresh stats every 30 seconds
    interval = setInterval(() => {
      fetchStats();
      generateActivityData();
    }, 30000);

    // Update "time ago" every second
    updateTimer = setInterval(() => {
      lastUpdate = lastUpdate; // Force reactivity
    }, 1000);
  });

  onDestroy(() => {
    if (interval) clearInterval(interval);
    if (updateTimer) clearInterval(updateTimer);
  });

  // Reactive: time since update
  $: timeAgo = timeSinceUpdate();
</script>

<div class="command-panel glass-card">
  <div class="panel-header">
    <span class="text-mono text-sm text-muted">NETWORK_COMMAND_PANEL</span>
    <span class="live-indicator">
      <span class="live-dot"></span>
      <span class="text-mono text-xs">LIVE</span>
      <span class="text-mono text-xs text-muted">{timeAgo}</span>
    </span>
  </div>

  <div class="panel-grid">
    <!-- Left: Network Pulse -->
    <div class="network-pulse">
      <div class="pulse-header text-mono text-xs text-muted mb-2">NETWORK_PULSE</div>

      <svg viewBox="0 0 240 130" class="hex-network">
        <!-- Connection lines -->
        {#each hexNodes as node, i}
          {#if i < hexNodes.length - 1 && (i + 1) % 5 !== 0}
            <line
              x1={node.x}
              y1={node.y}
              x2={hexNodes[i + 1].x}
              y2={hexNodes[i + 1].y}
              class="connection-line"
              class:active={node.active && hexNodes[i + 1].active}
            />
          {/if}
          {#if i + 5 < hexNodes.length}
            <line
              x1={node.x}
              y1={node.y}
              x2={hexNodes[i + 5].x}
              y2={hexNodes[i + 5].y}
              class="connection-line"
              class:active={node.active && hexNodes[i + 5].active}
            />
          {/if}
        {/each}

        <!-- Hexagon nodes -->
        {#each hexNodes as node, i}
          <g transform="translate({node.x}, {node.y})">
            <polygon
              points="-10,0 -5,-9 5,-9 10,0 5,9 -5,9"
              class="hex-node"
              class:active={node.active}
              style="animation-delay: {node.pulseDelay}s"
            />
            {#if node.active}
              <text x="0" y="3" class="hex-label">{i + 1}</text>
            {/if}
          </g>
        {/each}
      </svg>

      <div class="network-stats text-mono text-sm">
        <div class="stat-line">
          <span class="text-cyan">[{activeProvers}]</span> ACTIVE_PROVERS
        </div>
        <div class="stat-line">
          <span class="text-violet">[{jobsTotal - jobsCompleted}]</span> JOBS_COMPUTING
        </div>
      </div>
    </div>

    <!-- Right: Activity Graph -->
    <div class="activity-graph">
      <div class="graph-header text-mono text-xs text-muted mb-2">ACTIVITY_24H</div>

      <div class="ascii-chart">
        {#each activityData as count, i}
          <div class="bar-container">
            <div
              class="bar"
              style="height: {Math.max(5, (count / Math.max(peakJobs, 1)) * 100)}%"
              class:peak={count === peakJobs}
            ></div>
            {#if i % 6 === 0}
              <span class="bar-label text-mono text-xs">{(new Date().getHours() - 23 + i + 24) % 24}h</span>
            {/if}
          </div>
        {/each}
      </div>

      <div class="graph-stats text-mono text-sm mt-3">
        <div class="stat-line">
          <span class="text-muted">PEAK:</span>
          <span class="text-cyan">{peakJobs} jobs/h</span>
        </div>
        <div class="stat-line">
          <span class="text-muted">AVG:</span>
          <span class="text-cyan">{avgJobs} jobs/h</span>
        </div>
      </div>
    </div>
  </div>
</div>

<style>
  .command-panel {
    background: rgba(15, 23, 42, 0.7);
    backdrop-filter: blur(20px);
    border: 1px solid var(--zyber-border-primary);
    border-radius: var(--radius-lg);
    padding: var(--space-4);
    box-shadow: var(--zyber-glow-violet);
  }

  .panel-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding-bottom: var(--space-3);
    margin-bottom: var(--space-4);
    border-bottom: 1px solid var(--zyber-border-muted);
  }

  .live-indicator {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .live-dot {
    width: 8px;
    height: 8px;
    background: var(--zyber-success);
    border-radius: 50%;
    animation: pulse-dot 2s ease-in-out infinite;
  }

  @keyframes pulse-dot {
    0%, 100% { opacity: 1; box-shadow: 0 0 0 0 rgba(16, 185, 129, 0.7); }
    50% { opacity: 0.7; box-shadow: 0 0 0 6px rgba(16, 185, 129, 0); }
  }

  .panel-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--space-6);
  }

  @media (max-width: 768px) {
    .panel-grid {
      grid-template-columns: 1fr;
    }
  }

  /* Network Pulse */
  .network-pulse {
    display: flex;
    flex-direction: column;
  }

  .hex-network {
    width: 100%;
    max-width: 240px;
    height: auto;
    margin: 0 auto var(--space-3);
  }

  .connection-line {
    stroke: var(--zyber-border-muted);
    stroke-width: 1;
    opacity: 0.3;
  }

  .connection-line.active {
    stroke: var(--zyber-cyber-cyan);
    opacity: 0.6;
    stroke-dasharray: 4 2;
    animation: dash-flow 1s linear infinite;
  }

  @keyframes dash-flow {
    to { stroke-dashoffset: -12; }
  }

  .hex-node {
    fill: var(--zyber-bg-elevated);
    stroke: var(--zyber-border-muted);
    stroke-width: 1.5;
    transition: all 0.3s ease;
  }

  .hex-node.active {
    fill: rgba(6, 182, 212, 0.2);
    stroke: var(--zyber-cyber-cyan);
    animation: hex-pulse 2s ease-in-out infinite;
  }

  @keyframes hex-pulse {
    0%, 100% {
      filter: drop-shadow(0 0 3px rgba(6, 182, 212, 0.5));
    }
    50% {
      filter: drop-shadow(0 0 10px rgba(6, 182, 212, 0.8));
    }
  }

  .hex-label {
    font-family: var(--font-mono);
    font-size: 8px;
    fill: var(--zyber-cyber-cyan);
    text-anchor: middle;
  }

  .network-stats {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  /* Activity Graph */
  .activity-graph {
    display: flex;
    flex-direction: column;
  }

  .ascii-chart {
    display: flex;
    align-items: flex-end;
    gap: 2px;
    height: 80px;
    padding: var(--space-2);
    background: rgba(0, 0, 0, 0.3);
    border-radius: var(--radius-sm);
  }

  .bar-container {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    height: 100%;
    justify-content: flex-end;
  }

  .bar {
    width: 100%;
    min-height: 4px;
    background: linear-gradient(to top, var(--zyber-quantum-violet), var(--zyber-cyber-cyan));
    border-radius: 2px 2px 0 0;
    transition: height 0.3s ease;
  }

  .bar.peak {
    background: var(--zyber-cyber-cyan);
    box-shadow: 0 0 10px rgba(6, 182, 212, 0.6);
  }

  .bar-label {
    color: var(--zyber-text-muted);
    margin-top: 4px;
    font-size: 9px;
  }

  .graph-stats {
    display: flex;
    justify-content: space-between;
  }

  .stat-line {
    display: flex;
    gap: var(--space-2);
  }

  .text-cyan {
    color: var(--zyber-cyber-cyan);
  }

  .text-violet {
    color: var(--zyber-quantum-violet);
  }

  .text-muted {
    color: var(--zyber-text-muted);
  }

  .mb-2 { margin-bottom: var(--space-2); }
  .mb-3 { margin-bottom: var(--space-3); }
  .mt-3 { margin-top: var(--space-3); }
</style>
