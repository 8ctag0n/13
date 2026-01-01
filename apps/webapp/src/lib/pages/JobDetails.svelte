<script>
  import { onMount, onDestroy } from 'svelte';
  import { currentRoute, navigateTo } from '../stores/router';
  import { walletStore } from '../stores/wallet';

  const API_BASE = import.meta.env.VITE_API_URL || '';

  export let jobId;

  let job = null;
  let loading = true;
  let error = null;
  let pollInterval;

  // Check if current user is the owner
  $: isOwner = job && $walletStore.connected &&
    $walletStore.publicKey?.toString() === job.creator_pubkey;

  // Fetch job details
  async function fetchJob() {
    try {
      const response = await fetch(`${API_BASE}/api/jobs/${jobId}`);
      if (!response.ok) {
        if (response.status === 404) {
          throw new Error('Job not found');
        }
        throw new Error(`Failed to fetch job: ${response.status}`);
      }
      job = await response.json();
      error = null;
    } catch (err) {
      error = err.message;
      console.error('Error fetching job:', err);
    } finally {
      loading = false;
    }
  }

  // Format timestamp
  function formatDate(dateStr) {
    if (!dateStr) return '---';
    const date = new Date(dateStr);
    return date.toLocaleString('en-US', {
      year: 'numeric',
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit'
    });
  }

  // Format time ago
  function formatTimeAgo(dateStr) {
    if (!dateStr) return '---';
    const date = new Date(dateStr);
    const now = new Date();
    const diff = Math.floor((now - date) / 1000);

    if (diff < 60) return `${diff}s ago`;
    if (diff < 3600) return `${Math.floor(diff / 60)}m ago`;
    if (diff < 86400) return `${Math.floor(diff / 3600)}h ago`;
    return `${Math.floor(diff / 86400)}d ago`;
  }

  // Format SOL amount
  function formatSol(lamports) {
    return (lamports / 1_000_000_000).toFixed(5);
  }

  // Get status color class
  function getStatusClass(status) {
    switch (status) {
      case 'completed': return 'text-success';
      case 'computing':
      case 'claimed':
      case 'active': return 'text-cyan';
      case 'failed':
      case 'cancelled': return 'text-error';
      default: return 'text-warning';
    }
  }

  // Get status icon
  function getStatusIcon(status) {
    switch (status) {
      case 'completed': return '[OK]';
      case 'computing':
      case 'claimed':
      case 'active': return '[>>]';
      case 'failed':
      case 'cancelled': return '[!!]';
      default: return '[..]';
    }
  }

  // Explorer URL
  function getExplorerUrl(signature, type = 'tx') {
    const cluster = 'custom&customUrl=http%3A%2F%2Flocalhost%3A8899';
    if (type === 'tx') {
      return `https://explorer.solana.com/tx/${signature}?cluster=${cluster}`;
    }
    return `https://explorer.solana.com/address/${signature}?cluster=${cluster}`;
  }

  // Copy to clipboard
  async function copyToClipboard(text) {
    try {
      await navigator.clipboard.writeText(text);
    } catch (err) {
      console.error('Failed to copy:', err);
    }
  }

  // Prover data from API
  let provers = [];
  let proversLoading = false;

  // Fetch real prover data from API
  async function fetchProvers() {
    if (!job || job.status === 'pending' || job.status === 'pending_tx') {
      provers = [];
      return;
    }

    proversLoading = true;
    try {
      const response = await fetch(`${API_BASE}/api/jobs/${job.job_id}/provers`);
      if (response.ok) {
        const data = await response.json();
        provers = (data.provers || []).map(p => ({
          address: p.prover_pubkey || 'Unknown',
          shortAddress: p.prover_pubkey
            ? p.prover_pubkey.slice(0, 6) + '...' + p.prover_pubkey.slice(-4)
            : 'Unknown',
          progress: 100,
          status: 'verified',
          commitment: p.commitment ? p.commitment.slice(0, 16) + '...' : null,
          submittedAt: p.submitted_at
        }));
      }
    } catch (e) {
      console.error('Failed to fetch provers:', e);
    } finally {
      proversLoading = false;
    }
  }

  // Fetch provers when job changes
  $: if (job?.job_id) {
    fetchProvers();
  }

  // Use real has_result from backend
  $: hasResult = job?.has_result || false;

  // Download encrypted result
  let downloading = false;
  async function downloadResult() {
    if (!job || downloading) return;

    downloading = true;
    try {
      const response = await fetch(`${API_BASE}/api/jobs/${job.job_id}/result`);
      if (!response.ok) {
        throw new Error('Failed to fetch result');
      }

      const data = await response.json();
      if (!data.encrypted_result) {
        throw new Error('No result available');
      }

      // Decode base64 to binary
      const binaryString = atob(data.encrypted_result);
      const bytes = new Uint8Array(binaryString.length);
      for (let i = 0; i < binaryString.length; i++) {
        bytes[i] = binaryString.charCodeAt(i);
      }

      // Create blob and download
      const blob = new Blob([bytes], { type: 'application/octet-stream' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `fhe_result_${job.job_id}.bin`;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);
    } catch (e) {
      console.error('Download failed:', e);
      alert('Download failed: ' + e.message);
    } finally {
      downloading = false;
    }
  }

  // Go back
  function goBack() {
    navigateTo('dashboard');
  }

  onMount(() => {
    fetchJob();
    // Poll for updates every 3 seconds for active jobs
    pollInterval = setInterval(() => {
      if (job && ['pending', 'pending_tx', 'claimed', 'computing', 'active'].includes(job.status)) {
        fetchJob();
      }
    }, 3000);
  });

  onDestroy(() => {
    if (pollInterval) clearInterval(pollInterval);
  });
</script>

<div class="job-details-page">
  <div class="container">
    <!-- Header -->
    <header class="page-header">
      <button class="btn-back" on:click={goBack}>
        <span class="text-muted">{'<'}</span> BACK
      </button>
      <div class="header-title">
        <h1 class="text-mono">
          <span class="text-muted">{'>'}</span> JOB_#{jobId}
          {#if isOwner}
            <span class="owner-badge">[MINE]</span>
          {/if}
        </h1>
      </div>
    </header>

    {#if loading}
      <div class="loading-state">
        <div class="terminal-box">
          <div class="terminal-line">
            <span class="text-muted">{'>'}</span> FETCHING_JOB_DATA...
          </div>
          <div class="loading-bar">
            <div class="loading-fill"></div>
          </div>
        </div>
      </div>
    {:else if error}
      <div class="error-state">
        <div class="terminal-box error">
          <div class="terminal-line text-error">
            <span>[!!]</span> ERROR: {error}
          </div>
          <button class="btn btn-ghost mt-4" on:click={goBack}>
            [RETURN_TO_DASHBOARD]
          </button>
        </div>
      </div>
    {:else if job}
      <!-- Status Banner -->
      <div class="status-banner {job.status}">
        <div class="status-indicator">
          <span class="status-icon {getStatusClass(job.status)}">{getStatusIcon(job.status)}</span>
          <span class="status-text {getStatusClass(job.status)}">{job.status.toUpperCase()}</span>
        </div>
        <div class="status-time text-muted text-mono text-sm">
          {formatTimeAgo(job.created_at)}
        </div>
      </div>

      <!-- Main Grid -->
      <div class="details-grid">
        <!-- Left Column: Job Details -->
        <section class="detail-section">
          <div class="section-header">
            <span class="text-muted">{'>'}</span> JOB_DETAILS
          </div>
          <div class="terminal-box">
            <div class="data-row">
              <span class="data-label">OPERATION:</span>
              <span class="data-value text-cyan">{(job.operation || 'unknown').toUpperCase()}</span>
            </div>
            {#if job.operation_value}
              <div class="data-row">
                <span class="data-label">VALUE:</span>
                <span class="data-value text-violet">{job.operation_value}</span>
              </div>
            {/if}
            <div class="data-row">
              <span class="data-label">CONSENSUS:</span>
              <span class="data-value">{job.consensus_threshold}/{job.required_provers} provers required</span>
            </div>
            <div class="data-row">
              <span class="data-label">COST:</span>
              <span class="data-value text-cyan">{formatSol(job.price_lamports)} SOL</span>
            </div>
            <div class="data-row">
              <span class="data-label">PAYMENT:</span>
              <span class="data-value">{job.payment_method || 'Prepaid'}</span>
            </div>
            <div class="divider"></div>
            <div class="data-row">
              <span class="data-label">CREATED:</span>
              <span class="data-value text-sm">{formatDate(job.created_at)}</span>
            </div>
            {#if job.claimed_at}
              <div class="data-row">
                <span class="data-label">CLAIMED:</span>
                <span class="data-value text-sm">{formatDate(job.claimed_at)}</span>
              </div>
            {/if}
            {#if job.completed_at}
              <div class="data-row">
                <span class="data-label">COMPLETED:</span>
                <span class="data-value text-sm text-success">{formatDate(job.completed_at)}</span>
              </div>
            {/if}
          </div>
        </section>

        <!-- Right Column: Creator & Chain -->
        <section class="detail-section">
          <div class="section-header">
            <span class="text-muted">{'>'}</span> ON_CHAIN_DATA
          </div>
          <div class="terminal-box">
            <div class="data-row">
              <span class="data-label">CREATOR:</span>
              <span class="data-value text-mono">
                {job.creator_pubkey.slice(0, 8)}...{job.creator_pubkey.slice(-6)}
                <button class="btn-copy" on:click={() => copyToClipboard(job.creator_pubkey)} title="Copy full address">
                  [C]
                </button>
              </span>
            </div>
            <div class="data-row">
              <span class="data-label">NETWORK:</span>
              <span class="data-value">Solana Localnet</span>
            </div>
            <div class="data-row">
              <span class="data-label">PROGRAM:</span>
              <span class="data-value text-mono text-sm">ZyberLink_FHE_v1</span>
            </div>
          </div>
        </section>
      </div>

      <!-- Prover Consensus Section -->
      {#if provers.length > 0}
        <section class="prover-section">
          <div class="section-header">
            <span class="text-muted">{'>'}</span> PROVER_CONSENSUS
            <span class="consensus-badge">
              {provers.filter(p => p.status === 'verified').length}/{job.consensus_threshold} VERIFIED
            </span>
          </div>
          <div class="terminal-box">
            <div class="prover-grid">
              {#each provers as prover, i}
                <div class="prover-card">
                  <div class="prover-header">
                    <span class="prover-id">P{i + 1}</span>
                    <span class="prover-status"
                          class:verified={prover.status === 'verified'}
                          class:failed={prover.status === 'failed'}
                          class:computing={prover.status === 'computing'}>
                      [{prover.status.toUpperCase()}]
                    </span>
                  </div>
                  <div class="prover-address text-mono text-sm">
                    {prover.shortAddress}
                    <button class="btn-copy-sm" on:click={() => copyToClipboard(prover.address)}>
                      [C]
                    </button>
                  </div>
                  <div class="prover-progress">
                    <div class="progress-bar">
                      <div class="progress-fill"
                           class:completed={prover.progress === 100}
                           class:failed={prover.status === 'failed'}
                           style="width: {prover.progress}%"></div>
                    </div>
                    <span class="progress-text">{prover.progress}%</span>
                  </div>
                  {#if prover.commitment}
                    <div class="prover-commitment text-xs text-muted">
                      COMMIT: {prover.commitment.slice(0, 16)}...
                    </div>
                  {/if}
                </div>
              {/each}
            </div>
          </div>
        </section>
      {/if}

      <!-- Transactions Section -->
      <section class="tx-section">
        <div class="section-header">
          <span class="text-muted">{'>'}</span> BLOCKCHAIN_TRANSACTIONS
        </div>
        <div class="terminal-box">
          {#if job.tx_signature}
            <div class="tx-row">
              <div class="tx-type">
                <span class="tx-icon text-success">[TX]</span>
                <span class="tx-label">CREATE_JOB</span>
              </div>
              <div class="tx-hash text-mono">
                {job.tx_signature.slice(0, 12)}...{job.tx_signature.slice(-8)}
              </div>
              <div class="tx-actions">
                <button class="btn-copy-sm" on:click={() => copyToClipboard(job.tx_signature)}>
                  [C]
                </button>
                <a href={getExplorerUrl(job.tx_signature)}
                   target="_blank"
                   rel="noopener noreferrer"
                   class="btn-explorer">
                  [EXPLORER]
                </a>
              </div>
            </div>
          {:else}
            <div class="tx-empty text-muted">
              <span>[..]</span> No transactions recorded yet
            </div>
          {/if}

          {#if job.claim_tx}
            <div class="tx-row">
              <div class="tx-type">
                <span class="tx-icon text-cyan">[TX]</span>
                <span class="tx-label">CLAIM_JOB</span>
              </div>
              <div class="tx-hash text-mono">
                {job.claim_tx.slice(0, 12)}...{job.claim_tx.slice(-8)}
              </div>
              <div class="tx-actions">
                <button class="btn-copy-sm" on:click={() => copyToClipboard(job.claim_tx)}>
                  [C]
                </button>
                <a href={getExplorerUrl(job.claim_tx)}
                   target="_blank"
                   rel="noopener noreferrer"
                   class="btn-explorer">
                  [EXPLORER]
                </a>
              </div>
            </div>
          {/if}

          {#if job.result_tx}
            <div class="tx-row">
              <div class="tx-type">
                <span class="tx-icon text-violet">[TX]</span>
                <span class="tx-label">SUBMIT_RESULT</span>
              </div>
              <div class="tx-hash text-mono">
                {job.result_tx.slice(0, 12)}...{job.result_tx.slice(-8)}
              </div>
              <div class="tx-actions">
                <button class="btn-copy-sm" on:click={() => copyToClipboard(job.result_tx)}>
                  [C]
                </button>
                <a href={getExplorerUrl(job.result_tx)}
                   target="_blank"
                   rel="noopener noreferrer"
                   class="btn-explorer">
                  [EXPLORER]
                </a>
              </div>
            </div>
          {/if}
        </div>
      </section>

      <!-- Result Section (Owner Only, when result exists) -->
      {#if isOwner && hasResult}
        <section class="result-section">
          <div class="section-header">
            <span class="text-muted">{'>'}</span> COMPUTATION_RESULT
            <span class="private-badge">[PRIVATE]</span>
          </div>
          <div class="terminal-box result-box">
            <div class="result-info text-muted text-sm mb-4">
              Only you (the job creator) can decrypt this result with your client key.
            </div>
            <div class="result-placeholder">
              <span class="text-cyan">[ENCRYPTED_RESULT_AVAILABLE]</span>
              <button class="btn btn-primary" on:click={downloadResult} disabled={downloading}>
                {downloading ? '[DOWNLOADING...]' : '[DOWNLOAD_RESULT]'}
              </button>
            </div>
          </div>
        </section>
      {:else if isOwner && job.status === 'completed' && !hasResult}
        <section class="result-section">
          <div class="section-header">
            <span class="text-muted">{'>'}</span> COMPUTATION_RESULT
          </div>
          <div class="terminal-box">
            <div class="text-muted text-center">
              [..] Result not yet available. Provers may still be processing.
            </div>
          </div>
        </section>
      {/if}

      <!-- Actions Footer -->
      <footer class="actions-footer">
        {#if job.tx_signature}
          <a href={getExplorerUrl(job.tx_signature)}
             target="_blank"
             rel="noopener noreferrer"
             class="btn btn-ghost">
            [VIEW_ON_SOLANA_EXPLORER]
          </a>
        {/if}
        {#if isOwner && (job.status === 'pending' || job.status === 'pending_tx')}
          <button class="btn btn-ghost text-error">
            [CANCEL_JOB]
          </button>
        {/if}
        <button class="btn btn-ghost" on:click={goBack}>
          [BACK_TO_DASHBOARD]
        </button>
      </footer>
    {/if}
  </div>
</div>

<style>
  .job-details-page {
    min-height: 100vh;
    background: var(--zyber-bg-primary);
    padding-top: 80px;
  }

  .container {
    max-width: 1000px;
    margin: 0 auto;
    padding: var(--space-6);
  }

  /* Header */
  .page-header {
    display: flex;
    align-items: center;
    gap: var(--space-4);
    margin-bottom: var(--space-6);
  }

  .btn-back {
    background: transparent;
    border: 1px solid var(--zyber-border-muted);
    color: var(--zyber-text-secondary);
    font-family: var(--font-mono);
    padding: var(--space-2) var(--space-4);
    cursor: pointer;
    border-radius: var(--radius-md);
    transition: all 0.2s ease;
  }

  .btn-back:hover {
    border-color: var(--zyber-cyber-cyan);
    color: var(--zyber-cyber-cyan);
  }

  .header-title h1 {
    font-size: var(--text-2xl);
    font-weight: 600;
    margin: 0;
  }

  .owner-badge {
    color: var(--zyber-quantum-violet);
    font-size: var(--text-sm);
    margin-left: var(--space-2);
  }

  /* Status Banner */
  .status-banner {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: var(--space-4) var(--space-6);
    background: rgba(15, 23, 42, 0.8);
    border: 1px solid var(--zyber-border-muted);
    border-radius: var(--radius-lg);
    margin-bottom: var(--space-6);
  }

  .status-banner.completed {
    border-left: 4px solid var(--zyber-success);
  }

  .status-banner.computing,
  .status-banner.claimed,
  .status-banner.active {
    border-left: 4px solid var(--zyber-cyber-cyan);
  }

  .status-banner.failed,
  .status-banner.cancelled {
    border-left: 4px solid var(--zyber-error);
  }

  .status-banner.pending,
  .status-banner.pending_tx {
    border-left: 4px solid var(--zyber-warning);
  }

  .status-indicator {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }

  .status-icon {
    font-family: var(--font-mono);
    font-size: var(--text-lg);
  }

  .status-text {
    font-family: var(--font-mono);
    font-size: var(--text-xl);
    font-weight: 600;
    letter-spacing: 0.05em;
  }

  /* Loading & Error States */
  .loading-state,
  .error-state {
    display: flex;
    justify-content: center;
    align-items: center;
    min-height: 300px;
  }

  .loading-bar {
    width: 200px;
    height: 4px;
    background: rgba(100, 116, 139, 0.2);
    border-radius: 2px;
    overflow: hidden;
    margin-top: var(--space-4);
  }

  .loading-fill {
    height: 100%;
    width: 30%;
    background: linear-gradient(90deg, var(--zyber-quantum-violet), var(--zyber-cyber-cyan));
    animation: loading 1.5s ease-in-out infinite;
  }

  @keyframes loading {
    0% { transform: translateX(-100%); }
    100% { transform: translateX(400%); }
  }

  /* Sections */
  .section-header {
    font-family: var(--font-mono);
    font-size: var(--text-sm);
    color: var(--zyber-text-secondary);
    margin-bottom: var(--space-3);
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }

  .terminal-box {
    background: rgba(15, 23, 42, 0.6);
    border: 1px solid var(--zyber-border-muted);
    border-radius: var(--radius-lg);
    padding: var(--space-4);
  }

  .terminal-box.error {
    border-color: var(--zyber-error);
  }

  .terminal-line {
    font-family: var(--font-mono);
    display: flex;
    gap: var(--space-2);
  }

  /* Grid Layout */
  .details-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--space-6);
    margin-bottom: var(--space-6);
  }

  @media (max-width: 768px) {
    .details-grid {
      grid-template-columns: 1fr;
    }
  }

  .detail-section {
    margin-bottom: var(--space-4);
  }

  /* Data Rows */
  .data-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: var(--space-2) 0;
    border-bottom: 1px solid rgba(100, 116, 139, 0.1);
  }

  .data-row:last-child {
    border-bottom: none;
  }

  .data-label {
    font-family: var(--font-mono);
    font-size: var(--text-sm);
    color: var(--zyber-text-muted);
  }

  .data-value {
    font-family: var(--font-mono);
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .divider {
    height: 1px;
    background: var(--zyber-border-muted);
    margin: var(--space-3) 0;
  }

  /* Copy Buttons */
  .btn-copy,
  .btn-copy-sm {
    background: transparent;
    border: 1px solid var(--zyber-border-muted);
    color: var(--zyber-text-muted);
    font-family: var(--font-mono);
    font-size: var(--text-xs);
    padding: 2px 6px;
    cursor: pointer;
    border-radius: var(--radius-sm);
    transition: all 0.2s ease;
  }

  .btn-copy:hover,
  .btn-copy-sm:hover {
    border-color: var(--zyber-cyber-cyan);
    color: var(--zyber-cyber-cyan);
    background: rgba(6, 182, 212, 0.1);
  }

  /* Prover Section */
  .prover-section {
    margin-bottom: var(--space-6);
  }

  .consensus-badge {
    background: rgba(6, 182, 212, 0.1);
    border: 1px solid var(--zyber-cyber-cyan);
    padding: var(--space-1) var(--space-3);
    border-radius: var(--radius-md);
    font-size: var(--text-xs);
    color: var(--zyber-cyber-cyan);
  }

  .prover-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
    gap: var(--space-4);
  }

  .prover-card {
    background: rgba(0, 0, 0, 0.3);
    border: 1px solid var(--zyber-border-muted);
    border-radius: var(--radius-md);
    padding: var(--space-4);
  }

  .prover-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: var(--space-2);
  }

  .prover-id {
    font-family: var(--font-mono);
    font-weight: 600;
    color: var(--zyber-text-primary);
  }

  .prover-status {
    font-family: var(--font-mono);
    font-size: var(--text-xs);
  }

  .prover-status.verified { color: var(--zyber-success); }
  .prover-status.failed { color: var(--zyber-error); }
  .prover-status.computing { color: var(--zyber-cyber-cyan); }

  .prover-address {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin-bottom: var(--space-3);
    color: var(--zyber-text-secondary);
  }

  .prover-progress {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }

  .progress-bar {
    flex: 1;
    height: 8px;
    background: rgba(100, 116, 139, 0.2);
    border-radius: 4px;
    overflow: hidden;
  }

  .progress-fill {
    height: 100%;
    background: linear-gradient(90deg, var(--zyber-quantum-violet), var(--zyber-cyber-cyan));
    border-radius: 4px;
    transition: width 0.3s ease;
  }

  .progress-fill.completed {
    background: var(--zyber-success);
  }

  .progress-fill.failed {
    background: var(--zyber-error);
  }

  .progress-text {
    font-family: var(--font-mono);
    font-size: var(--text-sm);
    min-width: 40px;
    text-align: right;
  }

  .prover-commitment {
    margin-top: var(--space-2);
    padding-top: var(--space-2);
    border-top: 1px dashed var(--zyber-border-muted);
  }

  /* TX Section */
  .tx-section {
    margin-bottom: var(--space-6);
  }

  .tx-row {
    display: flex;
    align-items: center;
    gap: var(--space-4);
    padding: var(--space-3);
    background: rgba(0, 0, 0, 0.2);
    border-radius: var(--radius-md);
    margin-bottom: var(--space-2);
  }

  .tx-row:last-child {
    margin-bottom: 0;
  }

  .tx-type {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-width: 140px;
  }

  .tx-icon {
    font-family: var(--font-mono);
  }

  .tx-label {
    font-family: var(--font-mono);
    font-size: var(--text-sm);
    color: var(--zyber-text-secondary);
  }

  .tx-hash {
    flex: 1;
    font-size: var(--text-sm);
    color: var(--zyber-text-muted);
  }

  .tx-actions {
    display: flex;
    gap: var(--space-2);
  }

  .btn-explorer {
    font-family: var(--font-mono);
    font-size: var(--text-xs);
    color: var(--zyber-cyber-cyan);
    text-decoration: none;
    padding: 2px 8px;
    border: 1px solid var(--zyber-cyber-cyan);
    border-radius: var(--radius-sm);
    transition: all 0.2s ease;
  }

  .btn-explorer:hover {
    background: rgba(6, 182, 212, 0.1);
    box-shadow: 0 0 10px rgba(6, 182, 212, 0.3);
  }

  .tx-empty {
    font-family: var(--font-mono);
    text-align: center;
    padding: var(--space-4);
  }

  /* Result Section */
  .result-section {
    margin-bottom: var(--space-6);
  }

  .private-badge {
    background: rgba(139, 92, 246, 0.1);
    border: 1px solid var(--zyber-quantum-violet);
    padding: var(--space-1) var(--space-3);
    border-radius: var(--radius-md);
    font-size: var(--text-xs);
    color: var(--zyber-quantum-violet);
  }

  .result-box {
    border-color: var(--zyber-quantum-violet);
  }

  .result-placeholder {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-4);
    background: rgba(139, 92, 246, 0.05);
    border: 1px dashed var(--zyber-quantum-violet);
    border-radius: var(--radius-md);
  }

  /* Actions Footer */
  .actions-footer {
    display: flex;
    gap: var(--space-4);
    flex-wrap: wrap;
    padding-top: var(--space-6);
    border-top: 1px solid var(--zyber-border-muted);
  }

  /* Buttons */
  .btn {
    font-family: var(--font-mono);
    padding: var(--space-3) var(--space-5);
    border-radius: var(--radius-md);
    cursor: pointer;
    transition: all 0.2s ease;
    text-decoration: none;
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
    border-color: var(--zyber-cyber-cyan);
    color: var(--zyber-cyber-cyan);
  }

  /* Utilities */
  .text-cyan { color: var(--zyber-cyber-cyan); }
  .text-violet { color: var(--zyber-quantum-violet); }
  .text-success { color: var(--zyber-success); }
  .text-warning { color: var(--zyber-warning); }
  .text-error { color: var(--zyber-error); }
  .text-muted { color: var(--zyber-text-muted); }
  .text-mono { font-family: var(--font-mono); }
  .text-sm { font-size: var(--text-sm); }
  .text-xs { font-size: var(--text-xs); }
  .mt-4 { margin-top: var(--space-4); }
  .mb-4 { margin-bottom: var(--space-4); }
</style>
