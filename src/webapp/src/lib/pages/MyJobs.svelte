<script>
  import { onMount, onDestroy } from 'svelte';
  import { navigateTo } from '../stores/router';
  import { walletStore } from '../stores/wallet';

  const API_BASE = import.meta.env.VITE_API_URL || '';

  let jobs = [];
  let loading = true;
  let error = null;
  let lastUpdate = Date.now();
  let refreshInterval;

  // Stats
  let stats = {
    total: 0,
    pending: 0,
    computing: 0,
    completed: 0,
    expired: 0
  };

  onMount(async () => {
    await loadMyJobs();
    refreshInterval = setInterval(loadMyJobs, 15000);
  });

  onDestroy(() => {
    if (refreshInterval) clearInterval(refreshInterval);
  });

  async function loadMyJobs() {
    if (!$walletStore.connected || !$walletStore.publicKey) {
      error = 'Connect wallet to view your jobs';
      loading = false;
      return;
    }

    try {
      const creatorPubkey = $walletStore.publicKey.toString();
      const response = await fetch(`${API_BASE}/api/jobs?creator=${creatorPubkey}&limit=100`);

      if (response.ok) {
        const data = await response.json();
        jobs = data.jobs || [];

        // Calculate stats
        stats = {
          total: jobs.length,
          pending: jobs.filter(j => j.status === 'pending' || j.status === 'pending_tx').length,
          computing: jobs.filter(j => j.status === 'claimed' || j.status === 'active').length,
          completed: jobs.filter(j => j.status === 'completed').length,
          expired: jobs.filter(j => j.status === 'expired').length
        };

        error = null;
      } else {
        error = 'Failed to load jobs';
      }
      lastUpdate = Date.now();
    } catch (e) {
      console.error('Failed to load my jobs:', e);
      error = 'Network error';
    } finally {
      loading = false;
    }
  }

  function formatTime(dateStr) {
    const date = new Date(dateStr);
    const now = new Date();
    const diff = Math.floor((now - date) / 1000);

    if (diff < 60) return `${diff}s ago`;
    if (diff < 3600) return `${Math.floor(diff / 60)}m ago`;
    if (diff < 86400) return `${Math.floor(diff / 3600)}h ago`;
    return `${Math.floor(diff / 86400)}d ago`;
  }

  function formatPrice(lamports) {
    return (lamports / 1_000_000_000).toFixed(5);
  }

  function getStatusClass(status) {
    switch (status) {
      case 'completed': return 'status-success';
      case 'pending': case 'pending_tx': return 'status-pending';
      case 'claimed': case 'active': return 'status-computing';
      case 'expired': case 'failed': return 'status-error';
      default: return 'status-muted';
    }
  }

  function getStatusIcon(status) {
    switch (status) {
      case 'completed': return '[OK]';
      case 'pending': case 'pending_tx': return '[..]';
      case 'claimed': case 'active': return '[>>]';
      case 'expired': return '[XX]';
      case 'failed': return '[!!]';
      default: return '[??]';
    }
  }

  function goBack() {
    navigateTo('dashboard');
  }

  function createNewJob() {
    navigateTo('create-job');
  }
</script>

<div class="myjobs-page">
  <!-- Header -->
  <header class="page-header">
    <div class="container">
      <div class="header-content">
        <div class="header-left">
          <div class="logo-small">
            <div class="logo-encryption-small text-mono">
              <span class="char-encrypted">|||</span><span class="char-semi">||</span><span class="char-light">|</span>
            </div>
            <span class="logo-text-small text-mono">ZYBERLINK</span>
          </div>
        </div>

        <nav class="header-nav text-mono">
          <a href="#dashboard" class="nav-link" on:click|preventDefault={() => navigateTo('dashboard')}>[ALL_JOBS]</a>
          <a href="#my-jobs" class="nav-link active">[MY_JOBS]</a>
          <a href="#create-job" class="nav-link nav-create" on:click|preventDefault={createNewJob}>[+ CREATE]</a>
          <a href="#metrics" class="nav-link nav-metrics" on:click|preventDefault={() => navigateTo('metrics')}>[# METRICS]</a>
        </nav>

        <div class="header-right">
          {#if $walletStore.connected}
            <span class="wallet-badge text-mono text-xs">
              <span class="text-muted">WALLET:</span>
              <span class="text-cyan">{$walletStore.publicKey?.toString().slice(0, 6)}...{$walletStore.publicKey?.toString().slice(-4)}</span>
            </span>
          {/if}
        </div>
      </div>
    </div>
  </header>

  <main class="page-main">
    <div class="container">
      <!-- Stats Bar -->
      <div class="stats-bar tui-box">
        <div class="stat-item">
          <span class="stat-value text-cyan">{stats.total}</span>
          <span class="stat-label text-muted">TOTAL</span>
        </div>
        <div class="stat-item">
          <span class="stat-value text-warning">{stats.pending}</span>
          <span class="stat-label text-muted">PENDING</span>
        </div>
        <div class="stat-item">
          <span class="stat-value text-violet">{stats.computing}</span>
          <span class="stat-label text-muted">COMPUTING</span>
        </div>
        <div class="stat-item">
          <span class="stat-value text-success">{stats.completed}</span>
          <span class="stat-label text-muted">COMPLETED</span>
        </div>
        <div class="stat-item">
          <span class="stat-value text-error">{stats.expired}</span>
          <span class="stat-label text-muted">EXPIRED</span>
        </div>
      </div>

      <!-- Jobs List -->
      <div class="jobs-section">
        <div class="section-header text-mono">
          <span class="text-muted">╔══ JOB_LIST ══╗</span>
          <span class="text-xs text-muted">{loading ? 'Loading...' : `Updated ${Math.floor((Date.now() - lastUpdate) / 1000)}s ago`}</span>
        </div>

        {#if !$walletStore.connected}
          <div class="empty-state tui-box">
            <div class="empty-icon text-mono text-2xl text-muted">[!]</div>
            <p class="text-mono text-muted">Connect your wallet to view your jobs</p>
          </div>
        {:else if loading}
          <div class="loading-state tui-box">
            <div class="loading-spinner text-mono text-cyan">[...]</div>
            <p class="text-mono text-muted">Loading jobs...</p>
          </div>
        {:else if error}
          <div class="error-state tui-box">
            <div class="error-icon text-mono text-2xl text-error">[X]</div>
            <p class="text-mono text-error">{error}</p>
          </div>
        {:else if jobs.length === 0}
          <div class="empty-state tui-box">
            <div class="empty-icon text-mono text-2xl text-muted">[0]</div>
            <p class="text-mono text-muted">No jobs found</p>
            <button class="btn btn-primary mt-4" on:click={createNewJob}>[+ CREATE_FIRST_JOB]</button>
          </div>
        {:else}
          <div class="jobs-list">
            {#each jobs as job}
              <div class="job-card tui-box">
                <div class="job-header">
                  <span class="job-id text-mono text-cyan">JOB_#{job.job_id}</span>
                  <span class="job-status text-mono {getStatusClass(job.status)}">
                    {getStatusIcon(job.status)} {job.status.toUpperCase()}
                  </span>
                </div>

                <div class="job-details">
                  <div class="detail-row">
                    <span class="detail-label text-muted">OPERATION:</span>
                    <span class="detail-value text-mono">{job.operation}</span>
                  </div>
                  <div class="detail-row">
                    <span class="detail-label text-muted">PROVERS:</span>
                    <span class="detail-value text-mono">{job.consensus_threshold}/{job.required_provers}</span>
                  </div>
                  <div class="detail-row">
                    <span class="detail-label text-muted">COST:</span>
                    <span class="detail-value text-mono text-cyan">{formatPrice(job.price_lamports)} SOL</span>
                  </div>
                  <div class="detail-row">
                    <span class="detail-label text-muted">CREATED:</span>
                    <span class="detail-value text-mono">{formatTime(job.created_at)}</span>
                  </div>
                </div>

                {#if job.tx_signature}
                  <div class="job-footer">
                    <a
                      href="https://explorer.solana.com/tx/{job.tx_signature}?cluster=devnet"
                      target="_blank"
                      rel="noopener noreferrer"
                      class="tx-link text-mono text-xs text-violet"
                    >
                      [VIEW_TX] {job.tx_signature.slice(0, 8)}...
                    </a>
                  </div>
                {/if}
              </div>
            {/each}
          </div>
        {/if}
      </div>
    </div>
  </main>
</div>

<style>
  .myjobs-page {
    min-height: 100vh;
    padding-bottom: var(--space-8);
  }

  .page-header {
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

  .logo-small {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }

  .logo-encryption-small {
    font-size: var(--text-base);
    letter-spacing: 2px;
  }

  .char-encrypted { color: var(--zyber-quantum-violet); }
  .char-semi { color: var(--zyber-cyber-cyan); opacity: 0.7; }
  .char-light { color: var(--zyber-cyber-cyan); opacity: 0.3; }

  .logo-text-small {
    font-weight: 600;
    font-size: var(--text-base);
    background: linear-gradient(135deg, var(--zyber-quantum-violet), var(--zyber-cyber-cyan));
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
  }

  .header-nav {
    display: flex;
    gap: var(--space-2);
  }

  .nav-link {
    padding: var(--space-2) var(--space-3);
    color: var(--zyber-text-tertiary);
    text-decoration: none;
    font-size: var(--text-sm);
    border-bottom: 2px solid transparent;
    transition: all var(--transition-fast);
  }

  .nav-link:hover,
  .nav-link.active {
    color: var(--zyber-text-primary);
    border-bottom-color: var(--zyber-cyber-cyan);
  }

  .nav-link.active {
    color: var(--zyber-quantum-violet);
    border-bottom-color: var(--zyber-quantum-violet);
  }

  .nav-link.nav-create {
    color: var(--zyber-cyber-cyan);
    border: 1px solid var(--zyber-cyber-cyan);
    border-radius: var(--radius-sm);
    padding: var(--space-1) var(--space-2);
    margin-left: var(--space-2);
  }

  .nav-link.nav-create:hover {
    background: rgba(6, 182, 212, 0.1);
    border-bottom-color: transparent;
  }

  .header-right {
    display: flex;
    align-items: center;
    gap: var(--space-4);
  }

  .wallet-badge {
    padding: var(--space-1) var(--space-3);
    background: rgba(0, 0, 0, 0.3);
    border-radius: var(--radius-sm);
  }

  .page-main {
    padding: var(--space-6) 0;
  }

  /* Stats Bar */
  .stats-bar {
    display: flex;
    justify-content: space-around;
    padding: var(--space-4);
    margin-bottom: var(--space-6);
    background: rgba(0, 10, 20, 0.8);
  }

  .stat-item {
    text-align: center;
  }

  .stat-value {
    display: block;
    font-size: var(--text-2xl);
    font-weight: 600;
    font-family: var(--font-mono);
  }

  .stat-label {
    font-size: var(--text-xs);
    font-family: var(--font-mono);
    letter-spacing: 0.05em;
  }

  /* Section */
  .section-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: var(--space-4);
  }

  /* Empty/Loading/Error States */
  .empty-state, .loading-state, .error-state {
    text-align: center;
    padding: var(--space-12);
    background: rgba(0, 10, 20, 0.5);
  }

  .empty-icon, .error-icon {
    margin-bottom: var(--space-4);
  }

  .loading-spinner {
    animation: pulse 1s ease-in-out infinite;
  }

  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.4; }
  }

  /* Jobs List */
  .jobs-list {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }

  .job-card {
    padding: var(--space-4);
    background: rgba(0, 10, 20, 0.6);
    border: 1px solid var(--zyber-border-muted);
    transition: all 0.2s ease;
  }

  .job-card:hover {
    border-color: var(--zyber-cyber-cyan);
    box-shadow: 0 0 20px rgba(6, 182, 212, 0.1);
  }

  .job-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: var(--space-3);
    padding-bottom: var(--space-3);
    border-bottom: 1px solid var(--zyber-border-muted);
  }

  .job-id {
    font-size: var(--text-base);
    font-weight: 600;
  }

  .job-status {
    font-size: var(--text-sm);
  }

  .status-success { color: var(--zyber-success); }
  .status-pending { color: var(--zyber-warning); }
  .status-computing { color: var(--zyber-quantum-violet); }
  .status-error { color: var(--zyber-error); }
  .status-muted { color: var(--zyber-text-muted); }

  .job-details {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: var(--space-2);
  }

  .detail-row {
    display: flex;
    gap: var(--space-2);
    font-size: var(--text-sm);
  }

  .detail-label {
    font-family: var(--font-mono);
  }

  .detail-value {
    font-weight: 500;
  }

  .job-footer {
    margin-top: var(--space-3);
    padding-top: var(--space-3);
    border-top: 1px solid var(--zyber-border-muted);
  }

  .tx-link {
    text-decoration: none;
    transition: color 0.2s;
  }

  .tx-link:hover {
    color: var(--zyber-cyber-cyan);
  }

  /* Buttons */
  .btn {
    font-family: var(--font-mono);
    padding: var(--space-2) var(--space-4);
    border-radius: var(--radius-md);
    cursor: pointer;
    transition: all 0.2s;
    font-size: var(--text-sm);
    border: 1px solid transparent;
  }

  .btn-ghost {
    background: transparent;
    border-color: var(--zyber-border-muted);
    color: var(--zyber-text-secondary);
  }

  .btn-ghost:hover {
    border-color: var(--zyber-cyber-cyan);
    color: var(--zyber-cyber-cyan);
  }

  .btn-primary {
    background: rgba(6, 182, 212, 0.2);
    border-color: var(--zyber-cyber-cyan);
    color: var(--zyber-cyber-cyan);
  }

  .btn-primary:hover {
    background: rgba(6, 182, 212, 0.3);
    box-shadow: 0 0 20px rgba(6, 182, 212, 0.3);
  }

  /* Colors */
  .text-cyan { color: var(--zyber-cyber-cyan); }
  .text-violet { color: var(--zyber-quantum-violet); }
  .text-success { color: var(--zyber-success); }
  .text-warning { color: var(--zyber-warning); }
  .text-error { color: var(--zyber-error); }
  .text-muted { color: var(--zyber-text-muted); }

  .mt-4 { margin-top: var(--space-4); }

  @media (max-width: 768px) {
    .header-content {
      flex-direction: column;
      align-items: flex-start;
    }

    .stats-bar {
      flex-wrap: wrap;
      gap: var(--space-4);
    }

    .stat-item {
      flex: 1 1 30%;
    }

    .job-details {
      grid-template-columns: 1fr;
    }
  }
</style>
