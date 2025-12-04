<script>
  export let job;

  // Calculate time ago
  function timeAgo(timestamp) {
    const now = Date.now();
    const diff = now - timestamp;
    const minutes = Math.floor(diff / 60000);
    const hours = Math.floor(minutes / 60);

    if (hours > 0) {
      return `${String(hours).padStart(2, '0')}:${String(minutes % 60).padStart(2, '0')}:00`;
    }
    return `00:${String(minutes).padStart(2, '0')}:${String(Math.floor((diff % 60000) / 1000)).padStart(2, '0')}`;
  }

  $: statusIcon = {
    'computing': '[⏳]',
    'completed': '[✓]',
    'failed': '[⚠]'
  }[job.status] || '[•]';

  $: statusClass = {
    'computing': 'text-cyan',
    'completed': 'text-success',
    'failed': 'text-warning'
  }[job.status] || '';

  $: borderClass = {
    'computing': 'border-cyan',
    'completed': 'border-success',
    'failed': 'border-warning'
  }[job.status] || '';
</script>

<div class="job-card tui-box {borderClass}">
  <!-- Header with Status Icon -->
  <div class="job-header">
    <div class="job-id text-mono">
      <span class="{statusClass}">{statusIcon}</span>
      <span>JOB_ID: #{job.id.slice(0, 8)}</span>
    </div>
    <div class="job-age text-mono text-muted text-sm">
      AGE: {timeAgo(job.createdAt)}
    </div>
  </div>

  <div class="divider-line text-mono text-muted">
    ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  </div>

  <div class="job-body">
    {#if job.status === 'computing'}
      <!-- Computing State -->
      <div class="progress-section mb-4">
        <div class="text-mono text-xs text-muted mb-2">PROGRESS:</div>
        <div class="progress-ascii text-mono text-sm mb-2">
          [{Array(Math.floor(job.progress * 40 / 100)).fill('█').join('')}{Array(40 - Math.floor(job.progress * 40 / 100)).fill('░').join('')}] {job.progress}%
        </div>
      </div>

      <div class="job-info text-mono text-sm mb-4">
        <div class="info-line">
          <span class="text-muted">&gt; STATUS........:</span>
          <span class="text-cyan">COMPUTING</span>
        </div>
        <div class="info-line">
          <span class="text-muted">&gt; OPERATION....:</span>
          <span>{job.operation.toUpperCase().replace(' ', '_')}</span>
        </div>
        <div class="info-line">
          <span class="text-muted">&gt; ETA..........:</span>
          <span class="text-cyan">{job.eta}</span>
        </div>
      </div>

      <div class="timeline text-mono text-xs">
        <div class="text-muted mb-2">TIMELINE:</div>
        <div class="timeline-item">
          <span class="text-success">[✓]</span> {job.proversCount}_PROVERS_CLAIMED
        </div>
        <div class="timeline-item">
          <span class="text-cyan pulse">[⏳]</span> COMPUTING.....
          {#each job.provers as prover, i}
            PROVER_{i + 1}: {prover.progress}%{i < job.provers.length - 1 ? ' | ' : ''}
          {/each}
        </div>
        <div class="timeline-item text-muted">
          <span>[ ]</span> CONSENSUS_PENDING
        </div>
      </div>

    {:else if job.status === 'completed'}
      <!-- Completed State -->
      <div class="status-box completed mb-4">
        <div class="text-mono text-uppercase text-center">
          [COMPLETED_SUCCESSFULLY]
        </div>
      </div>

      <div class="job-info text-mono text-sm mb-4">
        <div class="info-line">
          <span class="text-muted">&gt; OPERATION....:</span>
          <span>{job.operation.toUpperCase().replace(' ', '_')}</span>
        </div>
        <div class="info-line">
          <span class="text-muted">&gt; CONSENSUS....:</span>
          <span class="text-success">{job.consensus}_MATCH</span>
        </div>
        <div class="info-line">
          <span class="text-muted">&gt; COST.........:</span>
          <span class="text-cyan">{job.cost}_SOL</span>
        </div>
      </div>

      <div class="action-buttons">
        <button class="btn btn-secondary btn-sm">
          [DOWNLOAD_RESULT]
        </button>
      </div>

    {:else if job.status === 'failed'}
      <!-- Failed State -->
      <div class="status-box failed mb-4">
        <div class="text-mono text-uppercase text-center">
          [CONSENSUS_FAILED]
        </div>
      </div>

      <div class="job-info text-mono text-sm mb-4">
        <div class="info-line text-muted">
          REASON: PROVERS_COULD_NOT_REACH_{job.consensusRequired}_AGREEMENT
        </div>
        <div class="info-line">
          <span class="text-muted">REFUND:</span>
          <span class="text-success">[✓] {job.cost}_SOL_RETURNED</span>
        </div>
      </div>

      <div class="action-buttons">
        <button class="btn btn-ghost btn-sm">
          [RETRY_JOB]
        </button>
      </div>
    {/if}
  </div>
</div>

<style>
  .job-card {
    margin-bottom: var(--space-4);
    transition: all var(--transition-base);
  }

  .job-card:hover {
    transform: translateY(-2px);
  }

  .border-cyan {
    border-color: var(--zyber-border-secondary);
    box-shadow: var(--zyber-glow-cyan);
  }

  .border-success {
    border-color: rgba(16, 185, 129, 0.3);
    box-shadow: 0 0 30px rgba(16, 185, 129, 0.4);
  }

  .border-warning {
    border-color: rgba(245, 158, 11, 0.3);
    box-shadow: 0 0 30px rgba(245, 158, 11, 0.4);
  }

  .job-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: var(--space-3);
    flex-wrap: wrap;
    gap: var(--space-2);
  }

  .job-id {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    font-weight: 600;
  }

  .divider-line {
    font-size: 8px;
    opacity: 0.3;
    margin-bottom: var(--space-4);
    overflow: hidden;
  }

  .job-body {
    position: relative;
  }

  .progress-ascii {
    color: var(--zyber-cyber-cyan);
    letter-spacing: 0;
  }

  .job-info {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .info-line {
    display: flex;
    gap: var(--space-3);
  }

  .timeline {
    background: rgba(0, 0, 0, 0.3);
    border-left: 2px solid var(--zyber-border-secondary);
    padding: var(--space-3);
    border-radius: var(--radius-sm);
  }

  .timeline-item {
    display: flex;
    gap: var(--space-2);
    margin-bottom: var(--space-2);
  }

  .timeline-item:last-child {
    margin-bottom: 0;
  }

  .status-box {
    padding: var(--space-4);
    border-radius: var(--radius-md);
    font-weight: 600;
  }

  .status-box.completed {
    background: rgba(16, 185, 129, 0.1);
    border: 1px solid rgba(16, 185, 129, 0.3);
    color: var(--zyber-success);
  }

  .status-box.failed {
    background: rgba(245, 158, 11, 0.1);
    border: 1px solid rgba(245, 158, 11, 0.3);
    color: var(--zyber-warning);
  }

  .action-buttons {
    display: flex;
    gap: var(--space-3);
  }

  /* Responsive */
  @media (max-width: 768px) {
    .job-header {
      flex-direction: column;
      align-items: flex-start;
    }

    .divider-line {
      font-size: 6px;
    }

    .progress-ascii {
      font-size: 10px;
    }

    .info-line {
      flex-direction: column;
      gap: var(--space-1);
    }

    .action-buttons {
      flex-direction: column;
      width: 100%;
    }

    .btn {
      width: 100%;
    }
  }
</style>
