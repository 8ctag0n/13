<script>
  import { onMount, onDestroy } from 'svelte';
  import { createSolanaRpc, address } from '@solana/kit';
  import { walletStore } from '../stores/wallet';
  import { navigateTo } from '../stores/router';
  // Analytics and PoI now have dedicated pages - see Analytics.svelte and ProofOfInnocence.svelte
  import NetworkCommandPanel from '../components/NetworkCommandPanel.svelte';
  import FloatingStatsCards from '../components/FloatingStatsCards.svelte';
  import ExpandedJobCard from '../components/ExpandedJobCard.svelte';

  // API config - use relative path for nginx proxy, fallback for local dev
  const API_BASE = import.meta.env.VITE_API_URL || '';
  const RPC_URL = import.meta.env.VITE_SOLANA_RPC_URL || 'http://localhost:8899';

  // Jobs state
  let jobs = [];
  let loading = true;
  let refreshing = false;
  let lastUpdate = Date.now();

  // Pagination
  let currentPage = 1;
  let totalJobs = 0;
  let jobsPerPage = 20;

  // Filters
  let filters = {
    status: 'all',
    showMyJobs: false
  };

  // Wallet
  let walletBalance = 0;
  let availableWallets = [];
  let detectingWallets = true;
  let showWalletModal = false;

  // Detect available wallets
  function detectWallets() {
    const wallets = [];
    if (typeof window !== 'undefined') {
      if (window.solana && window.solana.isPhantom) {
        wallets.push({ name: 'Phantom', provider: window.solana, icon: '<◊>' });
      }
      if (window.solflare && window.solflare.isSolflare) {
        wallets.push({ name: 'Solflare', provider: window.solflare, icon: '[*]' });
      }
    }
    availableWallets = wallets;
    detectingWallets = false;
  }

  async function connectWallet(wallet) {
    try {
      const response = await wallet.provider.connect();
      walletStore.set({
        connected: true,
        publicKey: response.publicKey.toString(),
        provider: wallet.provider,
        name: wallet.name
      });
      showWalletModal = false;
    } catch (err) {
      console.error('Failed to connect wallet:', err);
    }
  }

  function disconnectWallet() {
    walletStore.set({
      connected: false,
      publicKey: null,
      provider: null,
      name: null
    });
  }

  function openWalletModal() {
    showWalletModal = true;
  }

  function closeWalletModal() {
    showWalletModal = false;
  }

  // Refresh interval
  let refreshInterval;

  onMount(async () => {
    detectWallets();
    await loadJobs();

    // Live polling every 5 seconds
    refreshInterval = setInterval(async () => {
      await loadJobs(true);
    }, 5000);
  });

  onDestroy(() => {
    if (refreshInterval) clearInterval(refreshInterval);
  });

  async function loadJobs(silent = false) {
    if (!silent) loading = true;

    try {
      // Build query params
      const params = new URLSearchParams({
        page: currentPage.toString(),
        limit: jobsPerPage.toString()
      });

      // Apply status filter
      if (filters.status !== 'all') {
        params.append('status', filters.status);
      }

      // Apply "my jobs" filter
      if (filters.showMyJobs && $walletStore.connected && $walletStore.publicKey) {
        params.append('creator', $walletStore.publicKey.toString());
      }

      const response = await fetch(`${API_BASE}/api/jobs?${params}`);
      if (!response.ok) {
        throw new Error(`HTTP ${response.status}`);
      }

      const data = await response.json();

      // Transform backend data
      jobs = data.jobs.map(job => ({
        job_id: job.job_id,
        creator_pubkey: job.creator_pubkey,
        status: job.status,
        operation: job.operation,
        operation_value: job.operation_value,
        price_lamports: job.price_lamports,
        required_provers: job.required_provers,
        consensus_threshold: job.consensus_threshold,
        payment_method: job.payment_method,
        created_at: job.created_at,
        tx_signature: job.tx_signature || null,
        isNew: isRecent(job.created_at)
      }));

      totalJobs = data.total || data.count;
      lastUpdate = Date.now();
    } catch (error) {
      console.error('Failed to load jobs:', error);
    } finally {
      if (!silent) loading = false;
    }
  }

  function isRecent(createdAt) {
    const created = new Date(createdAt).getTime();
    return (Date.now() - created) < 10000;
  }

  function formatTimeSince(timestamp) {
    const seconds = Math.floor((Date.now() - timestamp) / 1000);
    if (seconds < 60) return `${seconds}s ago`;
    const minutes = Math.floor(seconds / 60);
    if (minutes < 60) return `${minutes}m ago`;
    const hours = Math.floor(minutes / 60);
    return `${hours}h ago`;
  }

  async function handleRefresh() {
    refreshing = true;
    await loadJobs();
    refreshing = false;
  }

  function handleNewJob() {
    navigateTo('create-job');
  }

  // Handle filter changes
  function handleStatusFilter(status) {
    filters.status = status;
    currentPage = 1;
    loadJobs();
  }

  function toggleMyJobs() {
    filters.showMyJobs = !filters.showMyJobs;
    currentPage = 1;
    loadJobs();
  }

  // Pagination handlers
  function nextPage() {
    if (currentPage * jobsPerPage < totalJobs) {
      currentPage++;
      loadJobs();
    }
  }

  function prevPage() {
    if (currentPage > 1) {
      currentPage--;
      loadJobs();
    }
  }

  // Wallet balance
  async function loadWalletBalance() {
    if ($walletStore.connected && $walletStore.publicKey) {
      try {
        const rpc = createSolanaRpc(RPC_URL);
        const walletAddress = address($walletStore.publicKey);
        const result = await rpc.getBalance(walletAddress).send();
        walletBalance = Number(result.value) / 1_000_000_000;
      } catch (error) {
        console.error('Failed to load wallet balance:', error);
      }
    }
  }

  // Note: Analytics and Proof of Innocence now have dedicated pages
  // Navigate to /analytics or /proof-of-innocence for full functionality

  $: walletInfo = $walletStore;
  $: if ($walletStore.connected) loadWalletBalance();
  $: totalPages = Math.ceil(totalJobs / jobsPerPage);

  // Check if job belongs to connected wallet
  function isMyJob(job) {
    return $walletStore.connected &&
           $walletStore.publicKey?.toString() === job.creator_pubkey;
  }
</script>

<div class="dashboard">
  <!-- Header -->
  <header class="dashboard-header">
    <div class="container">
      <div class="header-content">
        <div class="header-left">
          <div class="logo-small">
            <div class="logo-encryption-small text-mono">
              <span class="char-encrypted">|||</span><span class="char-semi">||</span><span class="char-light">|</span>
            </div>
            <span class="logo-text-small text-mono">ZYBERLINK_COMMAND_CENTER</span>
          </div>
        </div>

        <nav class="header-nav text-mono">
          <div class="nav-select-wrapper">
            <select
              class="nav-select text-mono"
              bind:value={filters.status}
              on:change={() => { filters.showMyJobs = false; loadJobs(); }}
            >
              <option value="all">ALL_JOBS</option>
              <option value="pending_tx">PENDING</option>
              <option value="claimed">COMPUTING</option>
              <option value="completed">COMPLETED</option>
              <option value="expired">EXPIRED</option>
            </select>
          </div>
          <a href="#my-jobs" class="nav-link nav-myjobs" on:click|preventDefault={() => navigateTo('my-jobs')}>[MY_JOBS]</a>
          <a href="#" class="nav-link nav-create" on:click|preventDefault={handleNewJob}>[+ CREATE]</a>
          <a href="#metrics" class="nav-link nav-metrics" on:click|preventDefault={() => navigateTo('metrics')}>[# METRICS]</a>
        </nav>

        <div class="header-right">
          {#if walletInfo.connected}
            <div class="wallet-connected text-mono">
              <span class="wallet-dot connected"></span>
              <span class="text-cyan">{walletInfo.publicKey.slice(0, 4)}...{walletInfo.publicKey.slice(-4)}</span>
              <span class="badge badge-success">{walletBalance.toFixed(3)}_SOL</span>
              <button class="btn-disconnect" on:click={disconnectWallet} title="Disconnect wallet">
                [X]
              </button>
            </div>
          {:else}
            <button class="btn-wallet-connect text-mono" on:click={openWalletModal}>
              [CONNECT_WALLET]
            </button>
          {/if}
        </div>
      </div>
    </div>
  </header>

  <!-- Main Content -->
  <main class="dashboard-main">
    <div class="container">
      <!-- Network Overview -->
      <section class="network-overview">
        <NetworkCommandPanel />
        <FloatingStatsCards />
      </section>

      <!-- Quick Actions Section -->
      <section class="quick-actions-section">
        <div class="quick-actions-grid">
          <!-- Private Analytics Card -->
          <button class="action-card tui-box" on:click={() => navigateTo('analytics')}>
            <div class="action-icon text-mono text-cyan">[SUM]</div>
            <h3 class="text-mono text-uppercase">Private Analytics</h3>
            <p class="text-mono text-sm text-muted">
              Run aggregate computations (Sum, Average) on encrypted data without revealing values.
            </p>
            <div class="action-cta text-mono text-cyan">[LAUNCH {'>'}{'>'}]</div>
          </button>

          <!-- Proof of Innocence Card -->
          <button class="action-card tui-box" on:click={() => navigateTo('proof-of-innocence')}>
            <div class="action-icon text-mono text-success">[OK]</div>
            <h3 class="text-mono text-uppercase">Proof of Innocence</h3>
            <p class="text-mono text-sm text-muted">
              Verify your transactions have not interacted with sanctioned addresses.
            </p>
            <div class="action-cta text-mono text-cyan">[VERIFY {'>'}{'>'}]</div>
          </button>

          <!-- Create Custom Job Card -->
          <button class="action-card tui-box" on:click={() => navigateTo('create-job')}>
            <div class="action-icon text-mono text-violet">[+]</div>
            <h3 class="text-mono text-uppercase">Custom FHE Job</h3>
            <p class="text-mono text-sm text-muted">
              Create a custom computation with full control over operation and parameters.
            </p>
            <div class="action-cta text-mono text-cyan">[CREATE {'>'}{'>'}]</div>
          </button>
        </div>
      </section>

      <!-- Jobs Section -->
      <section class="jobs-section">
        <!-- Controls Bar -->
        <div class="controls-bar">
          <div class="title-with-status">
            <h2 class="text-mono text-uppercase">
              {'>'} {filters.showMyJobs ? 'MY_JOBS' : 'ALL_JOBS'}
            </h2>
            <div class="live-indicator">
              <span class="pulse-dot"></span>
              <span class="text-mono text-sm">LIVE | {totalJobs} JOBS | {formatTimeSince(lastUpdate)}</span>
            </div>
          </div>

          <div class="controls-buttons">
            <button class="btn btn-ghost btn-sm" on:click={handleRefresh} disabled={refreshing}>
              {refreshing ? '[REFRESHING...]' : '[REFRESH]'}
            </button>
            <button class="btn btn-primary btn-sm" on:click={handleNewJob}>
              [NEW_JOB +]
            </button>
          </div>
        </div>

        <!-- Filters Bar -->
        <div class="filters-bar">
          <div class="filter-group">
            <span class="filter-label text-mono text-xs text-muted">STATUS:</span>
            <div class="filter-buttons">
              <button
                class="filter-btn text-mono"
                class:active={filters.status === 'all'}
                on:click={() => handleStatusFilter('all')}
              >[ALL]</button>
              <button
                class="filter-btn text-mono"
                class:active={filters.status === 'pending_tx'}
                on:click={() => handleStatusFilter('pending_tx')}
              >[PENDING]</button>
              <button
                class="filter-btn text-mono"
                class:active={filters.status === 'claimed' || filters.status === 'active'}
                on:click={() => handleStatusFilter('claimed')}
              >[COMPUTING]</button>
              <button
                class="filter-btn text-mono"
                class:active={filters.status === 'completed'}
                on:click={() => handleStatusFilter('completed')}
              >[COMPLETED]</button>
              <button
                class="filter-btn text-mono"
                class:active={filters.status === 'failed'}
                on:click={() => handleStatusFilter('failed')}
              >[FAILED]</button>
            </div>
          </div>

          {#if $walletStore.connected}
            <div class="filter-group">
              <button
                class="filter-btn toggle text-mono"
                class:active={filters.showMyJobs}
                on:click={toggleMyJobs}
              >
                {filters.showMyJobs ? '[*] MY_JOBS_ONLY' : '[ ] MY_JOBS_ONLY'}
              </button>
            </div>
          {/if}
        </div>

        <!-- Jobs List -->
        {#if loading}
          <div class="loading-state">
            <div class="tui-box">
              <div class="text-mono text-center">
                <div class="text-cyan mb-4">[>>] LOADING_JOBS...</div>
                <div class="loading-bar">
                  <div class="loading-fill"></div>
                </div>
              </div>
            </div>
          </div>
        {:else if jobs.length === 0}
          <div class="empty-state">
            <div class="tui-box">
              <div class="text-mono text-center">
                <div class="text-muted mb-4 text-xl">[ ]</div>
                <div class="mb-4">NO_JOBS_FOUND</div>
                <div class="text-sm text-muted mb-6">
                  {filters.showMyJobs ? 'You have no jobs yet.' : 'No jobs match the current filters.'}
                </div>
                <button class="btn btn-primary" on:click={handleNewJob}>
                  [CREATE_YOUR_FIRST_JOB]
                </button>
              </div>
            </div>
          </div>
        {:else}
          <div class="jobs-list">
            {#each jobs as job (job.job_id)}
              <ExpandedJobCard {job} isMyJob={isMyJob(job)} />
            {/each}
          </div>

          <!-- Pagination -->
          {#if totalPages > 1}
            <div class="pagination">
              <button
                class="btn btn-ghost btn-sm"
                disabled={currentPage === 1}
                on:click={prevPage}
              >
                [PREV]
              </button>
              <span class="text-mono text-sm">
                PAGE {currentPage} / {totalPages}
              </span>
              <button
                class="btn btn-ghost btn-sm"
                disabled={currentPage >= totalPages}
                on:click={nextPage}
              >
                [NEXT]
              </button>
            </div>
          {/if}
        {/if}
      </section>

      <!-- Terminal Prompt -->
      <div class="terminal-prompt text-mono text-muted mt-8">
        {'>'} READY_FOR_INPUT<span class="cursor-blink"></span>
      </div>
    </div>
  </main>
</div>

<!-- Wallet Connect Modal -->
{#if showWalletModal}
  <div class="modal-overlay" on:click={closeWalletModal}>
    <div class="modal-content" on:click|stopPropagation>
      <div class="modal-header">
        <h3 class="text-mono">{'>'} SELECT_WALLET</h3>
        <button class="modal-close" on:click={closeWalletModal}>[X]</button>
      </div>

      <div class="modal-body">
        {#if detectingWallets}
          <div class="modal-loading text-mono text-muted">
            [..] DETECTING_WALLETS...
          </div>
        {:else if availableWallets.length > 0}
          <div class="wallet-options">
            {#each availableWallets as wallet}
              <button
                class="wallet-option"
                on:click={() => connectWallet(wallet)}
              >
                <span class="wallet-option-icon">{wallet.icon}</span>
                <span class="wallet-option-name text-mono">{wallet.name.toUpperCase()}</span>
                <span class="wallet-option-arrow text-mono text-muted">{'>'}</span>
              </button>
            {/each}
          </div>
        {:else}
          <div class="no-wallets-found">
            <div class="text-mono text-muted mb-4">[!] NO_WALLETS_DETECTED</div>
            <div class="wallet-install-links">
              <a href="https://phantom.app" target="_blank" rel="noopener" class="install-link">
                <span class="install-icon text-mono text-cyan">&lt;◊&gt;</span>
                <span class="text-mono">INSTALL_PHANTOM</span>
              </a>
              <a href="https://solflare.com" target="_blank" rel="noopener" class="install-link">
                <span class="install-icon text-mono text-cyan">[*]</span>
                <span class="text-mono">INSTALL_SOLFLARE</span>
              </a>
            </div>
          </div>
        {/if}
      </div>

      <div class="modal-footer text-mono text-xs text-muted">
        CONNECT_TO_INTERACT_WITH_SOLANA_NETWORK
      </div>
    </div>
  </div>
{/if}

<style>
  .dashboard {
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
    z-index: var(--z-sticky);
  }

  .header-content {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-4);
    flex-wrap: wrap;
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
    align-items: center;
    gap: var(--space-3);
  }

  .nav-select-wrapper {
    position: relative;
  }

  .nav-select {
    appearance: none;
    background: rgba(6, 182, 212, 0.1);
    border: 1px solid var(--zyber-cyber-cyan);
    color: var(--zyber-cyber-cyan);
    padding: var(--space-2) var(--space-4);
    padding-right: var(--space-8);
    font-size: var(--text-sm);
    cursor: pointer;
    transition: all var(--transition-fast);
  }

  .nav-select:hover {
    background: rgba(6, 182, 212, 0.2);
    box-shadow: 0 0 10px rgba(6, 182, 212, 0.3);
  }

  .nav-select:focus {
    outline: none;
    box-shadow: 0 0 15px rgba(6, 182, 212, 0.4);
  }

  .nav-select option {
    background: var(--zyber-bg-primary);
    color: var(--zyber-text-primary);
  }

  .nav-select-wrapper::after {
    content: '▼';
    position: absolute;
    right: var(--space-2);
    top: 50%;
    transform: translateY(-50%);
    color: var(--zyber-cyber-cyan);
    font-size: var(--text-xs);
    pointer-events: none;
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

  .nav-link.nav-myjobs.active {
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

  .wallet-connected {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
    background: rgba(16, 185, 129, 0.1);
    border: 1px solid var(--zyber-success);
    border-radius: var(--radius-md);
    font-size: var(--text-sm);
  }

  .wallet-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--zyber-success);
    box-shadow: 0 0 8px var(--zyber-success);
    animation: pulse 2s ease-in-out infinite;
  }

  .btn-disconnect {
    background: transparent;
    border: none;
    color: var(--zyber-text-muted);
    cursor: pointer;
    font-family: var(--font-mono);
    font-size: var(--text-xs);
    padding: 2px 4px;
    transition: color 0.2s ease;
  }

  .btn-disconnect:hover {
    color: var(--zyber-error);
  }

  .wallet-connect-bar {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .btn-connect {
    background: rgba(6, 182, 212, 0.1);
    border: 1px solid var(--zyber-cyber-cyan);
    color: var(--zyber-cyber-cyan);
    padding: var(--space-2) var(--space-3);
    font-size: var(--text-sm);
    cursor: pointer;
    border-radius: var(--radius-md);
    transition: all 0.2s ease;
  }

  .btn-connect:hover {
    background: rgba(6, 182, 212, 0.2);
    box-shadow: 0 0 15px rgba(6, 182, 212, 0.3);
  }

  .btn-install {
    color: var(--zyber-quantum-violet);
    text-decoration: none;
    padding: var(--space-2) var(--space-3);
    border: 1px solid var(--zyber-quantum-violet);
    border-radius: var(--radius-md);
    font-size: var(--text-sm);
    transition: all 0.2s ease;
  }

  .btn-wallet-connect {
    background: rgba(6, 182, 212, 0.1);
    border: 1px solid var(--zyber-cyber-cyan);
    color: var(--zyber-cyber-cyan);
    padding: var(--space-2) var(--space-4);
    font-size: var(--text-sm);
    cursor: pointer;
    border-radius: var(--radius-md);
    transition: all 0.2s ease;
  }

  .btn-wallet-connect:hover {
    background: rgba(6, 182, 212, 0.2);
    box-shadow: 0 0 20px rgba(6, 182, 212, 0.4);
    transform: translateY(-1px);
  }

  /* Modal Styles */
  .modal-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.8);
    backdrop-filter: blur(4px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 9999;
    animation: fadeIn 0.2s ease;
  }

  @keyframes fadeIn {
    from { opacity: 0; }
    to { opacity: 1; }
  }

  .modal-content {
    background: rgba(15, 23, 42, 0.98);
    border: 1px solid var(--zyber-cyber-cyan);
    border-radius: var(--radius-lg);
    width: 90%;
    max-width: 400px;
    box-shadow:
      0 0 30px rgba(6, 182, 212, 0.3),
      0 25px 50px rgba(0, 0, 0, 0.5);
    animation: slideUp 0.3s ease;
  }

  @keyframes slideUp {
    from {
      opacity: 0;
      transform: translateY(20px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  .modal-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: var(--space-4) var(--space-5);
    border-bottom: 1px solid var(--zyber-border-muted);
  }

  .modal-header h3 {
    margin: 0;
    font-size: var(--text-lg);
    color: var(--zyber-cyber-cyan);
  }

  .modal-close {
    background: transparent;
    border: none;
    color: var(--zyber-text-muted);
    font-family: var(--font-mono);
    font-size: var(--text-sm);
    cursor: pointer;
    padding: var(--space-1) var(--space-2);
    transition: color 0.2s ease;
  }

  .modal-close:hover {
    color: var(--zyber-error);
  }

  .modal-body {
    padding: var(--space-5);
  }

  .modal-loading {
    text-align: center;
    padding: var(--space-6);
  }

  .wallet-options {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  .wallet-option {
    display: flex;
    align-items: center;
    gap: var(--space-4);
    padding: var(--space-4);
    background: rgba(6, 182, 212, 0.05);
    border: 1px solid var(--zyber-border-muted);
    border-radius: var(--radius-md);
    cursor: pointer;
    transition: all 0.2s ease;
    width: 100%;
    text-align: left;
  }

  .wallet-option:hover {
    background: rgba(6, 182, 212, 0.15);
    border-color: var(--zyber-cyber-cyan);
    box-shadow: 0 0 15px rgba(6, 182, 212, 0.2);
    transform: translateX(4px);
  }

  .wallet-option-icon {
    font-size: var(--text-2xl);
    width: 40px;
    text-align: center;
  }

  .wallet-option-name {
    flex: 1;
    font-size: var(--text-base);
    color: var(--zyber-text-primary);
  }

  .wallet-option-arrow {
    font-size: var(--text-lg);
    transition: transform 0.2s ease;
  }

  .wallet-option:hover .wallet-option-arrow {
    transform: translateX(4px);
    color: var(--zyber-cyber-cyan);
  }

  .no-wallets-found {
    text-align: center;
    padding: var(--space-4);
  }

  .wallet-install-links {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  .install-link {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-3) var(--space-4);
    background: rgba(139, 92, 246, 0.1);
    border: 1px solid var(--zyber-quantum-violet);
    border-radius: var(--radius-md);
    color: var(--zyber-quantum-violet);
    text-decoration: none;
    transition: all 0.2s ease;
  }

  .install-link:hover {
    background: rgba(139, 92, 246, 0.2);
    box-shadow: 0 0 15px rgba(139, 92, 246, 0.3);
  }

  .install-icon {
    font-size: var(--text-xl);
  }

  .modal-footer {
    padding: var(--space-3) var(--space-5);
    border-top: 1px solid var(--zyber-border-muted);
    text-align: center;
  }

  .dashboard-main {
    padding: var(--space-6) 0;
  }

  .network-overview {
    margin-bottom: var(--space-8);
  }

  .private-analytics-section {
    margin-bottom: var(--space-8);
  }

  .private-analytics-section .tui-box {
    padding: var(--space-6);
  }

  .analytics-result {
    padding: var(--space-3);
    border: 1px solid var(--zyber-success);
    background: rgba(16, 185, 129, 0.1);
    color: var(--zyber-success);
  }

  /* Quick Actions Section */
  .quick-actions-section {
    margin-bottom: var(--space-8);
  }

  .quick-actions-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
    gap: var(--space-4);
  }

  .action-card {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: var(--space-3);
    padding: var(--space-6);
    background: rgba(15, 23, 42, 0.6);
    border: 1px solid var(--zyber-border-muted);
    border-radius: var(--radius-lg);
    cursor: pointer;
    transition: all var(--transition-base);
    text-align: left;
    width: 100%;
  }

  .action-card:hover {
    border-color: var(--zyber-cyber-cyan);
    background: rgba(6, 182, 212, 0.05);
    transform: translateY(-2px);
    box-shadow: 0 4px 20px rgba(6, 182, 212, 0.2);
  }

  .action-card h3 {
    margin: 0;
    font-size: var(--text-lg);
  }

  .action-card p {
    margin: 0;
    line-height: 1.5;
  }

  .action-icon {
    font-size: var(--text-2xl);
    font-weight: bold;
  }

  .action-cta {
    margin-top: auto;
    padding-top: var(--space-3);
    font-size: var(--text-sm);
  }

  .text-violet {
    color: var(--zyber-quantum-violet);
  }

  .jobs-section {
    background: rgba(15, 23, 42, 0.4);
    border: 1px solid var(--zyber-border-muted);
    border-radius: var(--radius-lg);
    padding: var(--space-6);
  }

  .controls-bar {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    margin-bottom: var(--space-4);
    flex-wrap: wrap;
    gap: var(--space-4);
  }

  .title-with-status h2 {
    margin: 0 0 var(--space-2) 0;
    font-size: var(--text-xl);
  }

  .live-indicator {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    color: var(--zyber-cyber-cyan);
  }

  .pulse-dot {
    width: 8px;
    height: 8px;
    background: var(--zyber-cyber-cyan);
    border-radius: 50%;
    animation: pulse 2s ease-in-out infinite;
  }

  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.5; }
  }

  .controls-buttons {
    display: flex;
    gap: var(--space-2);
  }

  /* Filters */
  .filters-bar {
    display: flex;
    align-items: center;
    gap: var(--space-6);
    padding: var(--space-3);
    background: rgba(0, 0, 0, 0.2);
    border-radius: var(--radius-md);
    margin-bottom: var(--space-6);
    flex-wrap: wrap;
  }

  .filter-group {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .filter-label {
    margin-right: var(--space-2);
  }

  .filter-buttons {
    display: flex;
    gap: var(--space-1);
  }

  .filter-btn {
    padding: var(--space-1) var(--space-2);
    background: transparent;
    border: 1px solid var(--zyber-border-muted);
    color: var(--zyber-text-muted);
    border-radius: var(--radius-sm);
    cursor: pointer;
    font-size: var(--text-xs);
    transition: all var(--transition-fast);
  }

  .filter-btn:hover {
    border-color: var(--zyber-border-primary);
    color: var(--zyber-text-secondary);
  }

  .filter-btn.active {
    background: rgba(139, 92, 246, 0.2);
    border-color: var(--zyber-quantum-violet);
    color: var(--zyber-text-primary);
  }

  .filter-btn.toggle.active {
    background: rgba(6, 182, 212, 0.2);
    border-color: var(--zyber-cyber-cyan);
  }

  /* Loading & Empty states */
  .loading-state,
  .empty-state {
    display: flex;
    justify-content: center;
    padding: var(--space-8) 0;
  }

  .loading-state .tui-box,
  .empty-state .tui-box {
    padding: var(--space-8);
    max-width: 400px;
  }

  .loading-bar {
    width: 100%;
    height: 6px;
    background: rgba(100, 116, 139, 0.2);
    border-radius: 3px;
    overflow: hidden;
  }

  .loading-fill {
    height: 100%;
    width: 30%;
    background: linear-gradient(90deg, var(--zyber-quantum-violet), var(--zyber-cyber-cyan));
    border-radius: 3px;
    animation: loading-slide 1.2s ease-in-out infinite;
  }

  @keyframes loading-slide {
    0% { transform: translateX(-100%); }
    100% { transform: translateX(400%); }
  }

  .jobs-list {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  /* Pagination */
  .pagination {
    display: flex;
    justify-content: center;
    align-items: center;
    gap: var(--space-4);
    margin-top: var(--space-6);
    padding-top: var(--space-4);
    border-top: 1px solid var(--zyber-border-muted);
  }

  .terminal-prompt {
    font-size: var(--text-sm);
    text-align: center;
  }

  /* Buttons */
  .btn {
    font-family: var(--font-mono);
    padding: var(--space-2) var(--space-4);
    border-radius: var(--radius-md);
    cursor: pointer;
    transition: all var(--transition-fast);
  }

  .btn-sm {
    padding: var(--space-1) var(--space-3);
    font-size: var(--text-sm);
  }

  .btn-primary {
    background: var(--zyber-quantum-violet);
    border: 1px solid var(--zyber-quantum-violet);
    color: white;
  }

  .btn-primary:hover {
    box-shadow: var(--zyber-glow-violet);
  }

  .btn-ghost {
    background: transparent;
    border: 1px solid var(--zyber-border-muted);
    color: var(--zyber-text-secondary);
  }

  .btn-ghost:hover {
    border-color: var(--zyber-border-primary);
  }

  .btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  /* Utilities */
  .text-cyan { color: var(--zyber-cyber-cyan); }
  .text-muted { color: var(--zyber-text-muted); }
  .mb-4 { margin-bottom: var(--space-4); }
  .mb-6 { margin-bottom: var(--space-6); }
  .mt-8 { margin-top: var(--space-8); }

  /* Responsive */
  @media (max-width: 1024px) {
    .header-nav {
      flex-wrap: wrap;
      gap: var(--space-1);
    }

    .nav-link {
      font-size: var(--text-xs);
      padding: var(--space-1) var(--space-2);
    }
  }

  @media (max-width: 768px) {
    .header-content {
      flex-direction: column;
      align-items: stretch;
    }

    .header-nav {
      justify-content: center;
      order: 2;
    }

    .header-left {
      justify-content: center;
    }

    .header-right {
      order: 1;
    }

    .wallet-info {
      justify-content: center;
    }

    .controls-bar {
      flex-direction: column;
    }

    .filters-bar {
      flex-direction: column;
      align-items: stretch;
    }

    .filter-buttons {
      flex-wrap: wrap;
    }
  }
</style>
