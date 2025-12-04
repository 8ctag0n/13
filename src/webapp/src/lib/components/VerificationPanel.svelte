<script>
  import { onMount } from 'svelte';

  // Props
  export let jobId = null;
  export let witnessHash = null;
  export let resultHash = null;
  export let provers = [];
  export let consensusThreshold = 2;
  export let txSignature = null;
  export let compact = false;

  // Computed
  $: matchingProvers = provers.filter(p => p.commitmentHash === resultHash);
  $: consensusReached = matchingProvers.length >= consensusThreshold;
  $: consensusPercent = provers.length > 0 ? Math.round((matchingProvers.length / provers.length) * 100) : 0;

  // Explorer URL (Solana)
  function getExplorerUrl(signature) {
    // Use devnet for demo, change to mainnet-beta for production
    return `https://explorer.solana.com/tx/${signature}?cluster=devnet`;
  }

  // Truncate hash for display
  function truncateHash(hash, chars = 8) {
    if (!hash) return '---';
    if (hash.length <= chars * 2) return hash;
    return `${hash.slice(0, chars)}...${hash.slice(-chars)}`;
  }

  // Copy to clipboard
  async function copyToClipboard(text) {
    try {
      await navigator.clipboard.writeText(text);
    } catch (err) {
      console.error('Failed to copy:', err);
    }
  }
</script>

<div class="verification-panel" class:compact>
  <!-- Header -->
  <div class="panel-header">
    <h4 class="text-mono text-uppercase">
      <span class="header-icon">[#]</span> VERIFICATION_CONSENSUS
    </h4>
    {#if consensusReached}
      <span class="consensus-badge success text-mono">
        [{matchingProvers.length}-of-{provers.length} CONSENSUS]
      </span>
    {:else}
      <span class="consensus-badge pending text-mono">
        [PENDING {matchingProvers.length}/{consensusThreshold}]
      </span>
    {/if}
  </div>

  <!-- Witness Hash -->
  {#if witnessHash}
    <div class="hash-row">
      <span class="hash-label text-mono text-muted">WITNESS_HASH:</span>
      <div class="hash-value-wrapper">
        <span class="hash-value text-mono text-cyan">{truncateHash(witnessHash, 12)}</span>
        <button class="btn-copy" on:click={() => copyToClipboard(witnessHash)} title="Copy full hash">
          [C]
        </button>
      </div>
    </div>
  {/if}

  <!-- Result Hash -->
  {#if resultHash}
    <div class="hash-row">
      <span class="hash-label text-mono text-muted">RESULT_HASH:</span>
      <div class="hash-value-wrapper">
        <span class="hash-value text-mono text-success">{truncateHash(resultHash, 12)}</span>
        <button class="btn-copy" on:click={() => copyToClipboard(resultHash)} title="Copy full hash">
          [C]
        </button>
      </div>
    </div>
  {/if}

  <!-- Provers Table -->
  {#if provers.length > 0 && !compact}
    <div class="provers-section">
      <div class="provers-header text-mono text-sm text-muted">
        PROVER_COMMITMENTS:
      </div>
      <div class="provers-table">
        <div class="table-header text-mono text-xs">
          <span class="col-prover">PROVER</span>
          <span class="col-hash">COMMITMENT</span>
          <span class="col-status">STATUS</span>
        </div>
        {#each provers as prover, i}
          <div class="table-row" class:matching={prover.commitmentHash === resultHash}>
            <span class="col-prover text-mono">
              {prover.id || `prover-${i + 1}`}
            </span>
            <span class="col-hash text-mono text-xs">
              {truncateHash(prover.commitmentHash, 6)}
            </span>
            <span class="col-status text-mono">
              {#if prover.commitmentHash === resultHash}
                <span class="status-match">[OK]</span>
              {:else}
                <span class="status-mismatch">[X]</span>
              {/if}
            </span>
          </div>
        {/each}
      </div>
    </div>
  {/if}

  <!-- Consensus Progress Bar -->
  <div class="consensus-progress">
    <div class="progress-bar">
      <div
        class="progress-fill"
        class:success={consensusReached}
        style="width: {consensusPercent}%"
      ></div>
    </div>
    <div class="progress-label text-mono text-xs">
      {matchingProvers.length}/{provers.length} provers agree ({consensusPercent}%)
    </div>
  </div>

  <!-- Explorer Link -->
  {#if txSignature}
    <div class="explorer-link">
      <a
        href={getExplorerUrl(txSignature)}
        target="_blank"
        rel="noopener noreferrer"
        class="text-mono text-sm"
      >
        [VIEW_ON_SOLANA_EXPLORER {'>'}{'>'}]
      </a>
    </div>
  {/if}

  <!-- Info Tooltip -->
  <div class="info-footer">
    <details class="info-details">
      <summary class="text-mono text-xs text-muted">[?] How verification works</summary>
      <div class="info-content text-mono text-xs">
        <p>Each prover independently computes the FHE operation and generates a <strong>deterministic commitment hash</strong> (Blake2s256) of the encrypted result.</p>
        <p>When {consensusThreshold}+ provers produce the <strong>same hash</strong>, consensus is reached, proving the computation was performed correctly without revealing the actual data.</p>
        <p>The witness hash ensures data integrity - provers worked on the same input.</p>
      </div>
    </details>
  </div>
</div>

<style>
  .verification-panel {
    background: rgba(15, 23, 42, 0.6);
    border: 1px solid var(--zyber-border-muted);
    border-radius: var(--radius-md);
    padding: var(--space-4);
  }

  .verification-panel.compact {
    padding: var(--space-3);
  }

  .panel-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    flex-wrap: wrap;
    gap: var(--space-2);
    margin-bottom: var(--space-4);
    padding-bottom: var(--space-3);
    border-bottom: 1px solid var(--zyber-border-muted);
  }

  .panel-header h4 {
    margin: 0;
    font-size: var(--text-sm);
    color: var(--zyber-text-primary);
  }

  .header-icon {
    color: var(--zyber-cyber-cyan);
  }

  .consensus-badge {
    padding: var(--space-1) var(--space-2);
    border-radius: var(--radius-sm);
    font-size: var(--text-xs);
  }

  .consensus-badge.success {
    background: rgba(16, 185, 129, 0.2);
    border: 1px solid var(--zyber-success);
    color: var(--zyber-success);
  }

  .consensus-badge.pending {
    background: rgba(245, 158, 11, 0.2);
    border: 1px solid var(--zyber-warning);
    color: var(--zyber-warning);
  }

  .hash-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: var(--space-2) 0;
    border-bottom: 1px dashed var(--zyber-border-muted);
  }

  .hash-label {
    font-size: var(--text-xs);
  }

  .hash-value-wrapper {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .hash-value {
    font-size: var(--text-sm);
    padding: var(--space-1) var(--space-2);
    background: rgba(0, 0, 0, 0.3);
    border-radius: var(--radius-sm);
  }

  .btn-copy {
    background: transparent;
    border: 1px solid var(--zyber-border-muted);
    color: var(--zyber-text-muted);
    padding: 2px 6px;
    font-size: var(--text-xs);
    font-family: var(--font-mono);
    cursor: pointer;
    border-radius: var(--radius-sm);
    transition: all var(--transition-fast);
  }

  .btn-copy:hover {
    border-color: var(--zyber-cyber-cyan);
    color: var(--zyber-cyber-cyan);
  }

  .provers-section {
    margin-top: var(--space-4);
  }

  .provers-header {
    margin-bottom: var(--space-2);
  }

  .provers-table {
    background: rgba(0, 0, 0, 0.2);
    border-radius: var(--radius-sm);
    overflow: hidden;
  }

  .table-header,
  .table-row {
    display: grid;
    grid-template-columns: 1fr 2fr auto;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
  }

  .table-header {
    background: rgba(0, 0, 0, 0.3);
    border-bottom: 1px solid var(--zyber-border-muted);
    color: var(--zyber-text-muted);
  }

  .table-row {
    border-bottom: 1px solid rgba(100, 116, 139, 0.1);
  }

  .table-row:last-child {
    border-bottom: none;
  }

  .table-row.matching {
    background: rgba(16, 185, 129, 0.05);
  }

  .status-match {
    color: var(--zyber-success);
  }

  .status-mismatch {
    color: var(--zyber-error);
  }

  .consensus-progress {
    margin-top: var(--space-4);
  }

  .progress-bar {
    height: 6px;
    background: rgba(0, 0, 0, 0.3);
    border-radius: 3px;
    overflow: hidden;
  }

  .progress-fill {
    height: 100%;
    background: var(--zyber-warning);
    transition: width 0.3s ease, background 0.3s ease;
  }

  .progress-fill.success {
    background: var(--zyber-success);
  }

  .progress-label {
    margin-top: var(--space-2);
    text-align: center;
    color: var(--zyber-text-muted);
  }

  .explorer-link {
    margin-top: var(--space-4);
    text-align: center;
  }

  .explorer-link a {
    color: var(--zyber-cyber-cyan);
    text-decoration: none;
    transition: all var(--transition-fast);
  }

  .explorer-link a:hover {
    text-shadow: 0 0 10px var(--zyber-cyber-cyan);
  }

  .info-footer {
    margin-top: var(--space-4);
    padding-top: var(--space-3);
    border-top: 1px solid var(--zyber-border-muted);
  }

  .info-details summary {
    cursor: pointer;
    user-select: none;
  }

  .info-details summary:hover {
    color: var(--zyber-cyber-cyan);
  }

  .info-content {
    margin-top: var(--space-3);
    padding: var(--space-3);
    background: rgba(6, 182, 212, 0.05);
    border-left: 2px solid var(--zyber-cyber-cyan);
    border-radius: var(--radius-sm);
    line-height: 1.6;
  }

  .info-content p {
    margin: 0 0 var(--space-2) 0;
  }

  .info-content p:last-child {
    margin-bottom: 0;
  }

  /* Utilities */
  .text-cyan { color: var(--zyber-cyber-cyan); }
  .text-success { color: var(--zyber-success); }
  .text-muted { color: var(--zyber-text-muted); }
</style>
