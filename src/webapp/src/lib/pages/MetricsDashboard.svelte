<script>
  import { onMount, onDestroy } from 'svelte';
  import { navigateTo } from '../stores/router';
  import ASCIISparkline from '../components/charts/ASCIISparkline.svelte';
  import ASCIIBarChart from '../components/charts/ASCIIBarChart.svelte';
  import ASCIILineChart from '../components/charts/ASCIILineChart.svelte';

  const API_BASE = import.meta.env.VITE_API_URL || '';

  let metrics = {
    network: { uptime: 0, uptimeFormatted: '0d 0h 0m', activeProvers: 0, totalProvers: 0, offlineProvers: 0, dataProcessedTB: 0, dataProcessed24hTB: 0 },
    jobs: { completed24h: 0, completedTotal: 0, activeCurrent: 0, pendingCurrent: 0, expiredTotal: 0, avgJobTimeMins: 0, successRate: 0 },
    operations: [],
    timeline: [],
    proversHistory: []
  };

  let loading = true;
  let lastUpdate = Date.now();
  let refreshInterval;

  onMount(async () => {
    await loadMetrics();
    refreshInterval = setInterval(loadMetrics, 30000);
  });

  onDestroy(() => {
    if (refreshInterval) clearInterval(refreshInterval);
  });

  async function loadMetrics() {
    try {
      const response = await fetch(`${API_BASE}/api/metrics`);
      if (response.ok) {
        const data = await response.json();
        // Map snake_case API response to camelCase
        metrics = {
          network: {
            uptime: 99.97, // Fixed for now, could calculate from uptime_seconds
            uptimeFormatted: data.network?.uptime_formatted || '0d 0h 0m',
            activeProvers: data.network?.active_provers || 0,
            totalProvers: data.network?.total_provers || 0,
            offlineProvers: data.network?.offline_provers || 0,
            dataProcessedTB: data.network?.data_processed_tb || 0,
            dataProcessed24hTB: data.network?.data_processed_24h_tb || 0,
          },
          jobs: {
            completed24h: data.jobs?.completed_24h || 0,
            completedTotal: data.jobs?.completed_total || 0,
            activeCurrent: data.jobs?.active_current || 0,
            pendingCurrent: data.jobs?.pending_current || 0,
            expiredTotal: data.jobs?.expired_total || 0,
            avgJobTimeMins: data.jobs?.avg_job_time_mins || 0,
            successRate: data.jobs?.success_rate || 0,
          },
          operations: data.operations || [],
          timeline: data.timeline || [],
          proversHistory: data.provers_history || [],
        };
      }
      lastUpdate = Date.now();
    } catch (error) {
      console.error('Failed to load metrics:', error);
    } finally {
      loading = false;
    }
  }

  function formatNumber(num) {
    if (num >= 1000000) return (num / 1000000).toFixed(1) + 'M';
    if (num >= 1000) return (num / 1000).toFixed(1) + 'K';
    return num.toLocaleString();
  }

  function formatTimeSince(timestamp) {
    const seconds = Math.floor((Date.now() - timestamp) / 1000);
    if (seconds < 60) return `${seconds}s ago`;
    return `${Math.floor(seconds / 60)}m ago`;
  }

  function goBack() {
    navigateTo('dashboard');
  }

  $: operationsData = metrics.operations.map(op => ({
    label: op.operation,
    value: op.count,
    max: Math.max(...metrics.operations.map(o => o.count), 1)
  }));

  $: timelineData = metrics.timeline.map((t, i) => ({ x: i, y: t.count }));
  $: xLabels = ['00:00', '06:00', '12:00', '18:00', '24:00'];
</script>

<div class="metrics-dashboard">
  <!-- Header -->
  <header class="dashboard-header">
    <div class="container">
      <div class="header-content">
        <div class="header-left">
          <button class="btn btn-ghost btn-sm" on:click={goBack}>[&lt; BACK]</button>
          <h1 class="text-mono">
            <span class="text-violet">{'>'}</span> NETWORK_METRICS
          </h1>
        </div>
        <div class="header-right">
          <span class="text-mono text-muted text-sm">Updated: {formatTimeSince(lastUpdate)}</span>
          <button class="btn btn-ghost btn-sm" on:click={loadMetrics} disabled={loading}>
            {loading ? '[...]' : '[REFRESH]'}
          </button>
        </div>
      </div>
    </div>
  </header>

  <main class="dashboard-main">
    <div class="container">
      <!-- Stats Grid Row 1 -->
      <div class="stats-row">
        <!-- Network Status -->
        <div class="tui-box stat-card">
          <h3 class="card-title text-mono text-cyan">NETWORK_STATUS</h3>
          <div class="stat-content">
            <div class="stat-item">
              <span class="stat-label">UPTIME</span>
              <span class="stat-value text-success">{metrics.network.uptimeFormatted}</span>
            </div>
            <div class="progress-bar">
              <div class="progress-fill" style="width: {metrics.network.uptime}%"></div>
            </div>
            <div class="stat-item mt-4">
              <span class="stat-label">PROVERS_ONLINE</span>
              <span class="stat-value text-cyan">{metrics.network.activeProvers}</span>
            </div>
            <div class="sparkline-row">
              <ASCIISparkline data={metrics.proversHistory.length > 0 ? metrics.proversHistory : [3,3,3,3,3]} color="cyan" width={12} />
              <span class="text-muted text-xs">(24h)</span>
            </div>
            <div class="stat-item mt-4">
              <span class="stat-label">DATA_PROCESSED</span>
              <span class="stat-value text-violet">{metrics.network.dataProcessedTB.toFixed(2)} TB</span>
            </div>
            <span class="text-muted text-xs">+{metrics.network.dataProcessed24hTB.toFixed(2)} TB (24h)</span>
          </div>
        </div>

        <!-- Jobs Overview -->
        <div class="tui-box stat-card">
          <h3 class="card-title text-mono text-cyan">JOBS_OVERVIEW</h3>
          <div class="stat-content">
            <div class="comparison-block">
              <span class="comparison-label text-muted text-xs">COMPLETED (24h / Total)</span>
              <div class="comparison-values">
                <span class="text-cyan text-xl">{formatNumber(metrics.jobs.completed24h)}</span>
                <span class="text-muted">/</span>
                <span class="text-violet text-xl">{formatNumber(metrics.jobs.completedTotal)}</span>
              </div>
            </div>
            <div class="stats-grid mt-4">
              <div class="mini-stat">
                <span class="mini-label text-muted">ACTIVE</span>
                <span class="mini-value text-cyan">{metrics.jobs.activeCurrent}</span>
              </div>
              <div class="mini-stat">
                <span class="mini-label text-muted">PENDING</span>
                <span class="mini-value text-warning">{metrics.jobs.pendingCurrent}</span>
              </div>
              <div class="mini-stat">
                <span class="mini-label text-muted">EXPIRED</span>
                <span class="mini-value text-error">{formatNumber(metrics.jobs.expiredTotal)}</span>
              </div>
              <div class="mini-stat">
                <span class="mini-label text-muted">SUCCESS</span>
                <span class="mini-value text-success">{metrics.jobs.successRate.toFixed(1)}%</span>
              </div>
            </div>
          </div>
        </div>

        <!-- Provers Status -->
        <div class="tui-box stat-card">
          <h3 class="card-title text-mono text-cyan">PROVERS</h3>
          <div class="stat-content prover-stats">
            <div class="prover-item">
              <div class="prover-icon text-success">●</div>
              <span class="prover-label">ONLINE</span>
              <span class="prover-value text-xl">{metrics.network.activeProvers}</span>
            </div>
            <div class="prover-item">
              <div class="prover-icon text-muted">○</div>
              <span class="prover-label">OFFLINE</span>
              <span class="prover-value text-xl">{metrics.network.offlineProvers}</span>
            </div>
            <div class="prover-item">
              <div class="prover-icon text-error">✕</div>
              <span class="prover-label">TOTAL</span>
              <span class="prover-value text-xl">{metrics.network.totalProvers}</span>
            </div>
          </div>
        </div>
      </div>

      <!-- Volume Chart -->
      <div class="tui-box chart-card">
        <h3 class="card-title text-mono text-cyan">COMPUTATION_VOLUME (24h)</h3>
        <div class="chart-wrapper">
          {#if timelineData.length > 0}
            <ASCIILineChart
              data={timelineData}
              height={6}
              width={60}
              {xLabels}
              yLabelFormatter={(val) => formatNumber(val)}
            />
          {:else}
            <div class="no-data text-muted text-mono">NO_DATA_AVAILABLE</div>
          {/if}
        </div>
      </div>

      <!-- Operations Breakdown -->
      <div class="stats-row">
        <div class="tui-box stat-card wide">
          <h3 class="card-title text-mono text-cyan">OPERATION_BREAKDOWN</h3>
          <div class="chart-wrapper">
            {#if operationsData.length > 0}
              <ASCIIBarChart data={operationsData} color="cyan" maxWidth={20} />
            {:else}
              <div class="no-data text-muted text-mono">NO_OPERATIONS_YET</div>
            {/if}
          </div>
        </div>
      </div>

      <!-- Historical Table -->
      <div class="tui-box table-card">
        <h3 class="card-title text-mono text-cyan">HISTORICAL_METRICS</h3>
        <div class="table-wrapper">
          <table class="metrics-table text-mono">
            <thead>
              <tr>
                <th>METRIC</th>
                <th>24H</th>
                <th>TOTAL</th>
                <th>TREND</th>
              </tr>
            </thead>
            <tbody>
              <tr>
                <td>Jobs Completed</td>
                <td class="text-cyan">{formatNumber(metrics.jobs.completed24h)}</td>
                <td class="text-violet">{formatNumber(metrics.jobs.completedTotal)}</td>
                <td><ASCIISparkline data={[3,4,5,4,6]} width={5} color="success" /></td>
              </tr>
              <tr>
                <td>Jobs Expired</td>
                <td class="text-warning">-</td>
                <td class="text-error">{formatNumber(metrics.jobs.expiredTotal)}</td>
                <td><ASCIISparkline data={[5,4,3,2,1]} width={5} color="warning" /></td>
              </tr>
              <tr>
                <td>Active Provers</td>
                <td class="text-cyan">{metrics.network.activeProvers}</td>
                <td class="text-violet">{metrics.network.totalProvers} (peak)</td>
                <td><ASCIISparkline data={[3,3,3,3,3]} width={5} color="cyan" /></td>
              </tr>
              <tr>
                <td>Data Processed</td>
                <td class="text-cyan">{metrics.network.dataProcessed24hTB.toFixed(2)} TB</td>
                <td class="text-violet">{metrics.network.dataProcessedTB.toFixed(2)} TB</td>
                <td><ASCIISparkline data={[2,3,4,5,6]} width={5} color="success" /></td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>

      <!-- Terminal Prompt -->
      <div class="terminal-prompt text-mono text-muted">
        {'>'} METRICS_STREAM_ACTIVE<span class="cursor-blink"></span>
      </div>
    </div>
  </main>
</div>

<style>
  .metrics-dashboard {
    min-height: 100vh;
    padding-bottom: var(--space-8);
  }

  .dashboard-header {
    padding: var(--space-4) 0;
    border-bottom: 1px solid var(--zyber-border-muted);
    background: rgba(15, 23, 42, 0.95);
    backdrop-filter: blur(10px);
    position: sticky;
    top: 0;
    z-index: 100;
  }

  .header-content {
    display: flex;
    justify-content: space-between;
    align-items: center;
    flex-wrap: wrap;
    gap: var(--space-4);
  }

  .header-left {
    display: flex;
    align-items: center;
    gap: var(--space-4);
  }

  .header-left h1 {
    font-size: var(--text-lg);
    margin: 0;
  }

  .header-right {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }

  .dashboard-main {
    padding: var(--space-6) 0;
  }

  .stats-row {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: var(--space-4);
    margin-bottom: var(--space-4);
  }

  .stat-card {
    padding: var(--space-4);
  }

  .stat-card.wide {
    grid-column: span 3;
  }

  .card-title {
    font-size: var(--text-xs);
    margin: 0 0 var(--space-4) 0;
    padding-bottom: var(--space-2);
    border-bottom: 1px solid var(--zyber-border-muted);
    letter-spacing: 0.05em;
  }

  .stat-content {
    font-size: var(--text-sm);
  }

  .stat-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: var(--space-2);
  }

  .stat-label {
    font-size: var(--text-xs);
    color: var(--zyber-text-muted);
    font-family: var(--font-mono);
  }

  .stat-value {
    font-size: var(--text-lg);
    font-weight: 600;
    font-family: var(--font-mono);
  }

  .progress-bar {
    height: 4px;
    background: rgba(100, 116, 139, 0.2);
    border-radius: 2px;
    overflow: hidden;
  }

  .progress-fill {
    height: 100%;
    background: linear-gradient(90deg, var(--zyber-cyber-cyan), var(--zyber-quantum-violet));
    border-radius: 2px;
    transition: width 0.3s ease;
  }

  .sparkline-row {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin-bottom: var(--space-2);
  }

  .comparison-block {
    text-align: center;
  }

  .comparison-label {
    display: block;
    margin-bottom: var(--space-2);
  }

  .comparison-values {
    display: flex;
    justify-content: center;
    align-items: baseline;
    gap: var(--space-2);
    font-family: var(--font-mono);
  }

  .stats-grid {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: var(--space-3);
  }

  .mini-stat {
    text-align: center;
    padding: var(--space-2);
    background: rgba(0, 0, 0, 0.2);
    border-radius: var(--radius-sm);
  }

  .mini-label {
    display: block;
    font-size: var(--text-xs);
    font-family: var(--font-mono);
    margin-bottom: var(--space-1);
  }

  .mini-value {
    font-size: var(--text-lg);
    font-weight: 600;
    font-family: var(--font-mono);
  }

  .prover-stats {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  .prover-item {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-2);
    background: rgba(0, 0, 0, 0.2);
    border-radius: var(--radius-sm);
  }

  .prover-icon {
    font-size: var(--text-lg);
  }

  .prover-label {
    flex: 1;
    font-size: var(--text-xs);
    font-family: var(--font-mono);
    color: var(--zyber-text-muted);
  }

  .prover-value {
    font-family: var(--font-mono);
    font-weight: 600;
  }

  .chart-card, .table-card {
    padding: var(--space-4);
    margin-bottom: var(--space-4);
  }

  .chart-wrapper {
    overflow-x: auto;
    padding: var(--space-2) 0;
  }

  .table-wrapper {
    overflow-x: auto;
  }

  .metrics-table {
    width: 100%;
    border-collapse: collapse;
    font-size: var(--text-sm);
  }

  .metrics-table th {
    text-align: left;
    padding: var(--space-2) var(--space-3);
    border-bottom: 2px solid var(--zyber-border-primary);
    color: var(--zyber-text-secondary);
    font-size: var(--text-xs);
    letter-spacing: 0.05em;
  }

  .metrics-table td {
    padding: var(--space-2) var(--space-3);
    border-bottom: 1px solid var(--zyber-border-muted);
  }

  .metrics-table tbody tr:hover {
    background: rgba(139, 92, 246, 0.05);
  }

  .no-data {
    text-align: center;
    padding: var(--space-8);
  }

  .terminal-prompt {
    text-align: center;
    margin-top: var(--space-6);
    font-size: var(--text-sm);
  }

  .mt-4 { margin-top: var(--space-4); }

  .text-xl { font-size: var(--text-xl); }
  .text-cyan { color: var(--zyber-cyber-cyan); }
  .text-violet { color: var(--zyber-quantum-violet); }
  .text-success { color: var(--zyber-success); }
  .text-warning { color: var(--zyber-warning); }
  .text-error { color: var(--zyber-error); }
  .text-muted { color: var(--zyber-text-muted); }

  .btn {
    font-family: var(--font-mono);
    padding: var(--space-1) var(--space-3);
    border-radius: var(--radius-md);
    cursor: pointer;
    transition: all var(--transition-fast);
    font-size: var(--text-sm);
  }

  .btn-ghost {
    background: transparent;
    border: 1px solid var(--zyber-border-muted);
    color: var(--zyber-text-secondary);
  }

  .btn-ghost:hover {
    border-color: var(--zyber-cyber-cyan);
    color: var(--zyber-cyber-cyan);
  }

  @media (max-width: 1024px) {
    .stats-row {
      grid-template-columns: repeat(2, 1fr);
    }
    .stat-card.wide {
      grid-column: span 2;
    }
  }

  @media (max-width: 768px) {
    .stats-row {
      grid-template-columns: 1fr;
    }
    .stat-card.wide {
      grid-column: span 1;
    }
    .header-content {
      flex-direction: column;
      align-items: flex-start;
    }
  }
</style>
