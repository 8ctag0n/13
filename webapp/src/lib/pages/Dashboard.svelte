<script>
  import { onMount } from 'svelte';
  import { walletStore } from '../stores/wallet';
  import { navigateTo } from '../stores/router';
  import JobCard from '../components/JobCard.svelte';
  import Logo from '../components/Logo.svelte';

  let jobs = [];
  let loading = true;
  let refreshing = false;

  // Mock data for demo - In production this would fetch from backend
  const mockJobs = [
    {
      id: 'Ab3f9a2c1d4e5f6g',
      status: 'computing',
      operation: 'Multiply by 5',
      progress: 68,
      eta: '00:00:42',
      proversCount: 3,
      provers: [
        { progress: 72 },
        { progress: 65 },
        { progress: 67 }
      ],
      createdAt: Date.now() - 154000 // ~2.5 min ago
    },
    {
      id: '5e8bc71f2a9d3b4c',
      status: 'completed',
      operation: 'Add 10',
      consensus: '3/3',
      cost: '0.00202',
      createdAt: Date.now() - 495000 // ~8 min ago
    },
    {
      id: '2f4a89bd6e7c8f1a',
      status: 'failed',
      consensusRequired: '2-OF-3',
      cost: '0.00202',
      createdAt: Date.now() - 3600000 // 1 hour ago
    }
  ];

  onMount(async () => {
    await loadJobs();
  });

  async function loadJobs() {
    loading = true;
    // Simulate API call
    await new Promise(resolve => setTimeout(resolve, 800));
    jobs = mockJobs;
    loading = false;
  }

  async function handleRefresh() {
    refreshing = true;
    await loadJobs();
    refreshing = false;
  }

  function handleNewJob() {
    navigateTo('create-job');
  }

  $: walletInfo = $walletStore;
</script>

<div class="dashboard">
  <!-- Header with Logo and Wallet -->
  <header class="dashboard-header">
    <div class="container">
      <div class="header-content">
        <div class="header-left">
          <div class="logo-small">
            <div class="logo-encryption-small text-mono">
              <span class="char-encrypted">▓▓▓</span><span class="char-semi">▒▒</span><span class="char-light">░</span>
            </div>
            <span class="logo-text-small text-mono">ZYBERLINK_TERMINAL</span>
          </div>
        </div>

        <nav class="header-nav text-mono">
          <a href="#" class="nav-link active">[MY_JOBS]</a>
          <a href="#" class="nav-link" on:click|preventDefault={handleNewJob}>[CREATE]</a>
        </nav>

        <div class="header-right">
          {#if walletInfo.connected}
            <div class="wallet-info text-mono">
              <span class="text-muted">WALLET:</span>
              <span class="text-cyan">{walletInfo.publicKey.slice(0, 4)}...{walletInfo.publicKey.slice(-4)}</span>
              <span class="badge badge-success">2.456_SOL</span>
            </div>
          {/if}
        </div>
      </div>
    </div>
  </header>

  <div class="divider-header text-mono text-muted">
    ════════════════════════════════════════════════════════════════════════════════
  </div>

  <!-- Main Content -->
  <main class="dashboard-main">
    <div class="container">
      <!-- Controls Bar -->
      <div class="controls-bar">
        <h2 class="text-mono text-uppercase">
          &gt; MY_JOBS
        </h2>

        <div class="controls-buttons">
          <button
            class="btn btn-ghost btn-sm"
            on:click={handleRefresh}
            disabled={refreshing}
          >
            {refreshing ? '[↻ REFRESHING...]' : '[↻ REFRESH]'}
          </button>
          <button
            class="btn btn-primary btn-sm"
            on:click={handleNewJob}
          >
            [NEW_JOB +]
          </button>
        </div>
      </div>

      <!-- Jobs List -->
      {#if loading}
        <div class="loading-state">
          <div class="tui-box">
            <div class="text-mono text-center">
              <div class="text-cyan mb-4">
                <span class="spin">[⠋]</span> LOADING_JOBS...
              </div>
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
                Create your first FHE compute job to get started
              </div>
              <button class="btn btn-primary" on:click={handleNewJob}>
                [CREATE_YOUR_FIRST_JOB]
              </button>
            </div>
          </div>
        </div>
      {:else}
        <div class="jobs-list">
          {#each jobs as job (job.id)}
            <JobCard {job} />
          {/each}
        </div>
      {/if}

      <!-- Terminal Prompt -->
      <div class="terminal-prompt text-mono text-muted mt-8">
        &gt; READY_FOR_INPUT<span class="cursor-blink"></span>
      </div>
    </div>
  </main>
</div>

<style>
  .dashboard {
    min-height: 100vh;
    padding-bottom: var(--space-8);
  }

  .dashboard-header {
    padding: var(--space-6) 0;
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
    gap: var(--space-6);
    flex-wrap: wrap;
  }

  .header-left {
    display: flex;
    align-items: center;
  }

  .logo-small {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }

  .logo-encryption-small {
    font-size: var(--text-base);
  }

  .char-encrypted {
    color: var(--zyber-quantum-violet);
    text-shadow: var(--zyber-glow-violet);
  }

  .char-semi {
    color: var(--zyber-cyber-cyan);
    opacity: 0.7;
  }

  .char-light {
    color: var(--zyber-cyber-cyan);
    opacity: 0.3;
  }

  .logo-text-small {
    font-weight: 600;
    font-size: var(--text-lg);
    background: linear-gradient(
      135deg,
      var(--zyber-quantum-violet),
      var(--zyber-cyber-cyan)
    );
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
  }

  .header-nav {
    display: flex;
    gap: var(--space-4);
  }

  .nav-link {
    padding: var(--space-2) var(--space-4);
    color: var(--zyber-text-tertiary);
    text-decoration: none;
    border-bottom: 2px solid transparent;
    transition: all var(--transition-fast);
  }

  .nav-link:hover {
    color: var(--zyber-text-primary);
    border-bottom-color: var(--zyber-cyber-cyan);
  }

  .nav-link.active {
    color: var(--zyber-text-primary);
    border-bottom-color: var(--zyber-cyber-cyan);
    text-shadow: var(--zyber-glow-cyan);
  }

  .header-right {
    margin-left: auto;
  }

  .wallet-info {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-2) var(--space-4);
    background: rgba(6, 182, 212, 0.1);
    border: 1px solid var(--zyber-border-secondary);
    border-radius: var(--radius-lg);
    font-size: var(--text-sm);
  }

  .divider-header {
    font-size: 8px;
    opacity: 0.2;
    text-align: center;
    overflow: hidden;
    margin-bottom: var(--space-6);
  }

  .dashboard-main {
    padding: var(--space-8) 0;
  }

  .controls-bar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: var(--space-6);
    flex-wrap: wrap;
    gap: var(--space-4);
  }

  .controls-bar h2 {
    margin: 0;
    font-size: var(--text-2xl);
  }

  .controls-buttons {
    display: flex;
    gap: var(--space-3);
  }

  .loading-state,
  .empty-state {
    display: flex;
    justify-content: center;
    padding: var(--space-12) 0;
  }

  .loading-state .tui-box,
  .empty-state .tui-box {
    padding: var(--space-12);
    max-width: 500px;
  }

  .loading-bar {
    width: 100%;
    height: 8px;
    background: rgba(100, 116, 139, 0.2);
    border-radius: 4px;
    overflow: hidden;
  }

  .loading-fill {
    height: 100%;
    width: 40%;
    background: linear-gradient(90deg, var(--zyber-quantum-violet), var(--zyber-cyber-cyan));
    border-radius: 4px;
    animation: loading-slide 1.5s ease-in-out infinite;
  }

  @keyframes loading-slide {
    0% {
      transform: translateX(-100%);
    }
    100% {
      transform: translateX(350%);
    }
  }

  .jobs-list {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }

  .terminal-prompt {
    font-size: var(--text-sm);
    text-align: center;
  }

  /* Responsive */
  @media (max-width: 768px) {
    .header-content {
      flex-direction: column;
      align-items: flex-start;
    }

    .header-nav {
      width: 100%;
      justify-content: center;
    }

    .header-right {
      margin-left: 0;
      width: 100%;
    }

    .wallet-info {
      width: 100%;
      justify-content: center;
    }

    .controls-bar {
      flex-direction: column;
      align-items: flex-start;
    }

    .controls-buttons {
      width: 100%;
      flex-direction: column;
    }

    .btn {
      width: 100%;
    }

    .divider-header {
      font-size: 6px;
    }
  }
</style>
