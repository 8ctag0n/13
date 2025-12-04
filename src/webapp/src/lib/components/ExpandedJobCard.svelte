<script>
  import { walletStore } from '../stores/wallet';
  import { navigateTo } from '../stores/router';
  import VerificationPanel from './VerificationPanel.svelte';

  export let job;
  export let isMyJob = false;

  // Determine if this is user's job
  $: isOwner = isMyJob || ($walletStore.connected && $walletStore.publicKey?.toString() === job.creator_pubkey);

  const API_BASE = import.meta.env.VITE_API_URL || '';

  // Expanded state (still used for quick preview, but click goes to full page)
  let expanded = false;
  let provers = [];
  let proversLoading = false;

  // Fetch real prover data from API when expanded
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
          address: p.prover_pubkey
            ? p.prover_pubkey.slice(0, 8)
            : 'Unknown',
          fullAddress: p.prover_pubkey || 'Unknown',
          progress: 100,
          status: 'verified',
          commitment: p.commitment
        }));
      }
    } catch (e) {
      console.error('Failed to fetch provers:', e);
      provers = [];
    } finally {
      proversLoading = false;
    }
  }

  // Fetch provers when expanded and job has results
  $: if (expanded && job?.job_id && (job.status === 'completed' || job.status === 'active' || job.status === 'claimed')) {
    fetchProvers();
  }

  // Seeded random for consistent hashes
  function seededRandom(seed) {
    const x = Math.sin(seed) * 10000;
    return x - Math.floor(x);
  }

  // Download result
  let downloading = false;
  async function downloadResult(event) {
    event.stopPropagation();
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

  // Status styles
  function getStatusClass(status) {
    switch (status) {
      case 'completed': return 'status-completed';
      case 'computing':
      case 'claimed':
      case 'active': return 'status-computing';
      case 'failed':
      case 'cancelled': return 'status-failed';
      default: return 'status-pending';
    }
  }

  function getStatusIcon(status) {
    switch (status) {
      case 'completed': return '[ok]';
      case 'computing':
      case 'claimed':
      case 'active': return '[>>]';
      case 'failed':
      case 'cancelled': return '[!!]';
      default: return '[..]';
    }
  }

  // Format operation display
  function formatOperation(op, value) {
    const opName = (op || 'unknown').toUpperCase();
    return value ? `${opName}_${value}` : opName;
  }

  // Format cost
  function formatCost(lamports) {
    return (lamports / 1_000_000_000).toFixed(5);
  }

  // Open explorer
  function openExplorer() {
    if (job.tx_signature) {
      const network = 'devnet'; // TODO: detect from RPC URL
      window.open(`https://solscan.io/tx/${job.tx_signature}?cluster=${network}`, '_blank');
    }
  }

  // Format time ago
  function formatTimeAgo(dateStr) {
    if (!dateStr) return 'unknown';
    const date = new Date(dateStr);
    const now = new Date();
    const diff = Math.floor((now - date) / 1000);

    if (diff < 60) return `${diff}s ago`;
    if (diff < 3600) return `${Math.floor(diff / 60)}m ago`;
    if (diff < 86400) return `${Math.floor(diff / 3600)}h ago`;
    return `${Math.floor(diff / 86400)}d ago`;
  }

  // Navigate to job details page
  function goToDetails() {
    navigateTo(`job-${job.job_id}`);
  }

  // Toggle expand for quick preview (optional, secondary action)
  function toggleExpand(event) {
    event.stopPropagation();
    expanded = !expanded;
  }

  // Get explorer URL for transaction
  function getExplorerUrl(signature, type = 'tx') {
    // Use localnet explorer if localhost, otherwise devnet
    const cluster = 'custom&customUrl=http%3A%2F%2Flocalhost%3A8899';
    if (type === 'tx') {
      return `https://explorer.solana.com/tx/${signature}?cluster=${cluster}`;
    }
    return `https://explorer.solana.com/address/${signature}?cluster=${cluster}`;
  }

  // Copy to clipboard
  async function copyToClipboard(text, label) {
    try {
      await navigator.clipboard.writeText(text);
      // Could add a toast notification here
    } catch (err) {
      console.error('Failed to copy:', err);
    }
  }

  // Generate deterministic mock hashes for verification panel
  function generateMockHash(seed, length = 64) {
    const chars = '0123456789abcdef';
    let hash = '';
    for (let i = 0; i < length; i++) {
      const charSeed = (seed * 1000) + i;
      const charIndex = Math.floor(seededRandom(charSeed) * chars.length);
      hash += chars.charAt(charIndex);
    }
    return hash;
  }

  // Generate verification data for completed jobs
  $: verificationData = job.status === 'completed' ? {
    witnessHash: generateMockHash(job.job_id * 7, 64),
    resultHash: generateMockHash(job.job_id * 13, 64),
    provers: provers.map((p, i) => ({
      id: `prover-${i + 1}`,
      address: p.address,
      // All provers have same hash for consensus (completed job)
      commitmentHash: generateMockHash(job.job_id * 13, 64)
    }))
  } : null;
</script>

<div class="job-card glass-card {getStatusClass(job.status)}" class:expanded on:click={goToDetails} on:keypress={goToDetails} role="button" tabindex="0">
  <!-- Header -->
  <div class="job-header">
    <div class="job-id text-mono">
      <span class="text-muted">{'>'}</span> JOB_#{job.job_id}
      {#if isOwner}
        <span class="owner-badge text-xs">[MINE]</span>
      {/if}
    </div>
    <div class="job-status text-mono">
      <span class="status-icon">{getStatusIcon(job.status)}</span>
      <span class="status-text">{job.status.toUpperCase()}</span>
      <button class="expand-btn" on:click={toggleExpand} title="Quick preview">
        {expanded ? '[-]' : '[+]'}
      </button>
    </div>
  </div>

  <!-- Basic Info (always visible) -->
  <div class="job-info">
    <div class="info-row">
      <span class="label text-muted">OPERATION:</span>
      <span class="value text-cyan">{formatOperation(job.operation, job.operation_value)}</span>
    </div>
    <div class="info-row">
      <span class="label text-muted">CONSENSUS:</span>
      <span class="value">{job.consensus_threshold}/{job.required_provers}</span>
    </div>
    <div class="info-row">
      <span class="label text-muted">COST:</span>
      <span class="value text-cyan">{formatCost(job.price_lamports)} SOL</span>
    </div>
    <div class="info-row">
      <span class="label text-muted">CREATED:</span>
      <span class="value">{formatTimeAgo(job.created_at)}</span>
    </div>
  </div>

  <!-- Click hint -->
  <div class="click-hint text-mono text-xs text-muted">
    CLICK_FOR_DETAILS →
  </div>

  <!-- Expanded Content (visible to everyone) -->
  {#if expanded}
    <div class="job-expanded">
      <div class="divider"></div>

      <!-- On-Chain Data Section -->
      <div class="onchain-section">
        <div class="section-header text-mono text-xs text-muted">ON_CHAIN_DATA:</div>
        <div class="data-grid">
          <div class="data-row">
            <span class="data-label text-muted">CREATOR:</span>
            <span class="data-value text-mono">
              {job.creator_pubkey.slice(0, 6)}...{job.creator_pubkey.slice(-4)}
              <button class="btn-copy" on:click|stopPropagation={() => copyToClipboard(job.creator_pubkey)} title="Copy">
                [C]
              </button>
            </span>
          </div>
          <div class="data-row">
            <span class="data-label text-muted">PROVERS:</span>
            <span class="data-value">{job.consensus_threshold}/{job.required_provers} required for consensus</span>
          </div>
          <div class="data-row">
            <span class="data-label text-muted">PAYMENT:</span>
            <span class="data-value text-cyan">{job.payment_method || 'Prepaid'}</span>
          </div>
        </div>
      </div>

      <!-- Prover Progress (for active/computing jobs) -->
      {#if provers.length > 0 && (job.status === 'claimed' || job.status === 'active' || job.status === 'completed')}
        <div class="prover-section">
          <div class="section-header text-mono text-xs text-muted">PROVER_CONSENSUS:</div>
          <div class="prover-list">
            {#each provers as prover, i}
              <div class="prover-row">
                <div class="prover-info">
                  <span class="prover-icon" class:verified={prover.status === 'verified'} class:failed={prover.status === 'failed'}>
                    {prover.status === 'verified' ? 'ok' : prover.status === 'failed' ? '!!' : '>>'}
                  </span>
                  <span class="prover-address text-mono">P{i + 1}_{prover.address}</span>
                </div>
                <div class="prover-progress">
                  <div class="progress-bar">
                    <div
                      class="progress-fill"
                      class:completed={prover.progress === 100}
                      class:failed={prover.status === 'failed'}
                      style="width: {prover.progress}%"
                    ></div>
                  </div>
                  <span class="progress-percent text-mono text-xs">{prover.progress}%</span>
                </div>
                <div class="prover-status text-mono text-xs">
                  <span class:text-success={prover.status === 'verified'}
                        class:text-error={prover.status === 'failed'}
                        class:text-cyan={prover.status === 'computing'}>
                    [{prover.status.toUpperCase()}]
                  </span>
                </div>
              </div>
            {/each}
          </div>
        </div>
      {/if}

      <!-- Verification Panel (for completed jobs) -->
      {#if job.status === 'completed' && verificationData}
        <div class="verification-section">
          <VerificationPanel
            jobId={job.job_id}
            witnessHash={verificationData.witnessHash}
            resultHash={verificationData.resultHash}
            provers={verificationData.provers}
            consensusThreshold={job.consensus_threshold || 2}
            txSignature={job.tx_signature}
            compact={false}
          />
        </div>
      {/if}

      <!-- Blockchain Transactions -->
      <div class="tx-section">
        <div class="section-header text-mono text-xs text-muted">BLOCKCHAIN_TRANSACTIONS:</div>
        <div class="tx-list">
          {#if job.tx_signature}
            <div class="tx-row">
              <span class="tx-label text-muted">CREATE_JOB_TX:</span>
              <a
                href={getExplorerUrl(job.tx_signature)}
                target="_blank"
                rel="noopener noreferrer"
                class="tx-link text-mono text-cyan"
                on:click|stopPropagation
              >
                {job.tx_signature.slice(0, 8)}...{job.tx_signature.slice(-6)} [EXPLORER]
              </a>
            </div>
          {:else}
            <div class="tx-row text-muted">
              <span>No transaction signature available yet</span>
            </div>
          {/if}
        </div>
      </div>

      <!-- Actions -->
      <div class="job-actions">
        {#if isOwner && (job.status === 'completed' || job.has_result)}
          <button class="btn btn-sm btn-primary" on:click={downloadResult} disabled={downloading}>
            {downloading ? '[DOWNLOADING...]' : '[DOWNLOAD_RESULT]'}
          </button>
        {/if}
        {#if isOwner && (job.status === 'pending_tx' || job.status === 'pending')}
          <button class="btn btn-sm btn-ghost text-error">[CANCEL]</button>
        {/if}
        {#if job.tx_signature}
          <a
            href={getExplorerUrl(job.tx_signature)}
            target="_blank"
            rel="noopener noreferrer"
            class="btn btn-sm btn-ghost"
            on:click|stopPropagation
          >
            [VIEW_ON_EXPLORER]
          </a>
        {/if}
      </div>
    </div>
  {/if}
</div>

<style>
  .job-card {
    background: rgba(15, 23, 42, 0.6);
    backdrop-filter: blur(20px);
    border: 1px solid var(--zyber-border-muted);
    border-radius: var(--radius-lg);
    padding: var(--space-4);
    margin-bottom: var(--space-4);
    transition: all 0.3s ease;
    cursor: pointer;
  }

  .job-card:hover {
    border-color: var(--zyber-cyber-cyan);
    box-shadow: 0 0 15px rgba(6, 182, 212, 0.15);
  }

  .job-card.expanded {
    background: rgba(15, 23, 42, 0.85);
    border-color: var(--zyber-cyber-cyan);
  }

  .owner-badge {
    color: var(--zyber-quantum-violet);
    margin-left: var(--space-2);
  }

  .expand-btn {
    background: transparent;
    border: 1px solid var(--zyber-border-muted);
    color: var(--zyber-text-muted);
    font-family: var(--font-mono);
    font-size: var(--text-sm);
    padding: 2px 8px;
    margin-left: var(--space-2);
    cursor: pointer;
    border-radius: var(--radius-sm);
    transition: all 0.2s ease;
  }

  .expand-btn:hover {
    border-color: var(--zyber-cyber-cyan);
    color: var(--zyber-cyber-cyan);
    background: rgba(6, 182, 212, 0.1);
  }

  /* Status colors */
  .job-card.status-completed {
    border-left: 3px solid var(--zyber-success);
  }

  .job-card.status-computing {
    border-left: 3px solid var(--zyber-cyber-cyan);
  }

  .job-card.status-failed {
    border-left: 3px solid var(--zyber-error);
  }

  .job-card.status-pending {
    border-left: 3px solid var(--zyber-warning);
  }

  /* Header */
  .job-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: var(--space-3);
  }

  .job-id {
    font-size: var(--text-lg);
    font-weight: 600;
  }

  .job-status {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .status-icon {
    font-size: var(--text-sm);
  }

  .status-completed .status-icon,
  .status-completed .status-text { color: var(--zyber-success); }

  .status-computing .status-icon,
  .status-computing .status-text { color: var(--zyber-cyber-cyan); }

  .status-failed .status-icon,
  .status-failed .status-text { color: var(--zyber-error); }

  .status-pending .status-icon,
  .status-pending .status-text { color: var(--zyber-warning); }

  /* Info */
  .job-info {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: var(--space-2);
  }

  @media (max-width: 480px) {
    .job-info {
      grid-template-columns: 1fr;
    }
  }

  .info-row {
    display: flex;
    gap: var(--space-2);
    font-size: var(--text-sm);
    font-family: var(--font-mono);
  }

  /* Expanded */
  .job-expanded {
    margin-top: var(--space-4);
  }

  .divider {
    height: 1px;
    background: var(--zyber-border-muted);
    margin-bottom: var(--space-4);
  }

  .section-header {
    margin-bottom: var(--space-2);
    letter-spacing: 0.05em;
  }

  /* Prover Progress */
  .prover-section {
    margin-bottom: var(--space-4);
  }

  .prover-list {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .prover-row {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-2);
    background: rgba(0, 0, 0, 0.2);
    border-radius: var(--radius-sm);
  }

  .prover-info {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-width: 140px;
  }

  .prover-icon {
    font-family: var(--font-mono);
    font-size: var(--text-xs);
    color: var(--zyber-cyber-cyan);
  }

  .prover-icon.verified { color: var(--zyber-success); }
  .prover-icon.failed { color: var(--zyber-error); }

  .prover-address {
    font-size: var(--text-sm);
    color: var(--zyber-text-secondary);
  }

  .prover-progress {
    flex: 1;
    display: flex;
    align-items: center;
    gap: var(--space-2);
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

  .progress-percent {
    min-width: 35px;
    text-align: right;
  }

  .prover-status {
    min-width: 80px;
    text-align: right;
  }

  /* On-Chain Data Section */
  .onchain-section {
    margin-bottom: var(--space-4);
  }

  .data-grid {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .data-row {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    font-size: var(--text-sm);
  }

  .data-label {
    min-width: 80px;
    font-family: var(--font-mono);
  }

  .data-value {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    font-size: var(--text-sm);
  }

  .btn-copy {
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

  .btn-copy:hover {
    border-color: var(--zyber-cyber-cyan);
    color: var(--zyber-cyber-cyan);
    background: rgba(6, 182, 212, 0.1);
  }

  /* TX Section */
  .tx-section {
    margin-bottom: var(--space-4);
  }

  .tx-list {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .tx-row {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    font-size: var(--text-sm);
  }

  .tx-label {
    min-width: 120px;
    font-family: var(--font-mono);
  }

  .tx-link {
    text-decoration: none;
    transition: all 0.2s ease;
  }

  .tx-link:hover {
    color: var(--zyber-text-primary);
    text-shadow: 0 0 10px rgba(6, 182, 212, 0.5);
  }

  .tx-info {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    flex-wrap: wrap;
  }

  .btn-link {
    background: none;
    border: none;
    color: var(--zyber-cyber-cyan);
    cursor: pointer;
    padding: 0;
    text-decoration: underline;
  }

  .btn-link:hover {
    color: var(--zyber-text-primary);
  }

  /* Actions */
  .job-actions {
    display: flex;
    gap: var(--space-3);
    flex-wrap: wrap;
  }

  .btn {
    font-family: var(--font-mono);
    padding: var(--space-2) var(--space-4);
    border-radius: var(--radius-md);
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .btn-sm {
    padding: var(--space-2) var(--space-3);
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
    background: rgba(139, 92, 246, 0.1);
  }

  /* Click hint */
  .click-hint {
    text-align: right;
    margin-top: var(--space-3);
    padding-top: var(--space-2);
    border-top: 1px dashed var(--zyber-border-muted);
    opacity: 0.6;
    transition: opacity 0.2s ease;
  }

  .job-card:hover .click-hint {
    opacity: 1;
    color: var(--zyber-cyber-cyan);
  }

  /* Verification Section */
  .verification-section {
    margin-bottom: var(--space-4);
  }

  /* Utilities */
  .text-cyan { color: var(--zyber-cyber-cyan); }
  .text-muted { color: var(--zyber-text-muted); }
  .text-success { color: var(--zyber-success); }
  .text-error { color: var(--zyber-error); }
</style>
