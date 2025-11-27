<script>
  import { walletStore } from '../stores/wallet';

  export let job;
  export let isMyJob = false;

  // Determine if this is user's job
  $: isOwner = isMyJob || ($walletStore.connected && $walletStore.publicKey?.toString() === job.creator_pubkey);

  // Expanded state (only for owner's jobs)
  let expanded = false;

  // Mock prover progress data
  $: provers = generateMockProvers(job);

  function generateMockProvers(job) {
    if (!job || job.status === 'pending' || job.status === 'pending_tx') {
      return [];
    }

    const count = job.required_provers || 3;
    const isCompleted = job.status === 'completed';
    const isFailed = job.status === 'failed';

    return Array(count).fill(null).map((_, i) => ({
      address: generateMockAddress(),
      progress: isCompleted ? 100 : isFailed ? Math.random() * 50 : Math.floor(Math.random() * 60 + 30),
      status: isCompleted ? 'verified' : isFailed && i === 0 ? 'failed' : 'computing'
    }));
  }

  function generateMockAddress() {
    const chars = 'ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz123456789';
    let addr = '';
    for (let i = 0; i < 8; i++) {
      addr += chars.charAt(Math.floor(Math.random() * chars.length));
    }
    return addr;
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

  // Toggle expand
  function toggleExpand() {
    if (isOwner) {
      expanded = !expanded;
    }
  }
</script>

<div class="job-card glass-card {getStatusClass(job.status)}" class:expanded class:is-owner={isOwner}>
  <!-- Header -->
  <div class="job-header" on:click={toggleExpand} on:keypress={toggleExpand} role="button" tabindex="0">
    <div class="job-id text-mono">
      <span class="text-muted">{'>'}</span> JOB_#{job.job_id}
    </div>
    <div class="job-status text-mono">
      <span class="status-icon">{getStatusIcon(job.status)}</span>
      <span class="status-text">{job.status.toUpperCase()}</span>
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

  <!-- Expanded Content (only for owner's jobs) -->
  {#if isOwner && expanded}
    <div class="job-expanded">
      <div class="divider"></div>

      <!-- Prover Progress -->
      {#if provers.length > 0}
        <div class="prover-section">
          <div class="section-header text-mono text-xs text-muted">PROVER_PROGRESS:</div>
          <div class="prover-list">
            {#each provers as prover, i}
              <div class="prover-row">
                <div class="prover-info">
                  <span class="prover-icon" class:verified={prover.status === 'verified'} class:failed={prover.status === 'failed'}>
                    {prover.status === 'verified' ? 'ok' : prover.status === 'failed' ? '!!' : '>>'}
                  </span>
                  <span class="prover-address text-mono">Prover_{prover.address}</span>
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

      <!-- TX Info -->
      {#if job.tx_signature}
        <div class="tx-section">
          <div class="section-header text-mono text-xs text-muted">BLOCKCHAIN:</div>
          <div class="tx-info">
            <span class="text-mono text-sm">TX: {job.tx_signature.slice(0, 8)}...{job.tx_signature.slice(-6)}</span>
            <button class="btn-link text-mono text-xs" on:click|stopPropagation={openExplorer}>
              [VIEW_ON_SOLSCAN]
            </button>
          </div>
        </div>
      {/if}

      <!-- Actions -->
      <div class="job-actions">
        {#if job.status === 'completed'}
          <button class="btn btn-sm btn-primary">[DOWNLOAD_RESULT]</button>
        {/if}
        {#if job.status === 'pending_tx' || job.status === 'active'}
          <button class="btn btn-sm btn-ghost text-error">[CANCEL_JOB]</button>
        {/if}
        <button class="btn btn-sm btn-ghost">[REFRESH]</button>
      </div>
    </div>
  {/if}

  <!-- Expand indicator for owner's jobs -->
  {#if isOwner && !expanded}
    <div class="expand-hint text-mono text-xs text-muted">
      [CLICK_TO_EXPAND]
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
  }

  .job-card.is-owner {
    cursor: pointer;
    border-color: var(--zyber-border-primary);
  }

  .job-card.is-owner:hover {
    border-color: var(--zyber-cyber-cyan);
    box-shadow: 0 0 20px rgba(6, 182, 212, 0.2);
  }

  .job-card.expanded {
    background: rgba(15, 23, 42, 0.8);
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

  /* TX Section */
  .tx-section {
    margin-bottom: var(--space-4);
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

  /* Expand hint */
  .expand-hint {
    text-align: center;
    margin-top: var(--space-3);
    padding-top: var(--space-2);
    border-top: 1px dashed var(--zyber-border-muted);
  }

  /* Utilities */
  .text-cyan { color: var(--zyber-cyber-cyan); }
  .text-muted { color: var(--zyber-text-muted); }
  .text-success { color: var(--zyber-success); }
  .text-error { color: var(--zyber-error); }
</style>
