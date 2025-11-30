<script>
  import { walletStore } from '../stores/wallet';
  import { navigateTo } from '../stores/router';
  import { toastStore } from '../stores/toast';
  import WitnessUploader from '../components/WitnessUploader.svelte';
  import WalletConnect from '../components/WalletConnect.svelte';
  import Loading from '../components/Loading.svelte';
  import { createFheJobFromWitness, pollJobStatus, getJobResult } from '../utils/job_creator.js';

  // API config
  const API_BASE = import.meta.env.VITE_API_URL || '';
  const RPC_URL = import.meta.env.VITE_SOLANA_RPC_URL || 'http://localhost:8899';

  // State
  let witnessData = null;
  let selectedOperation = 'sum';
  let isProcessing = false;
  let processingStep = '';
  let processingMessage = '';

  // Result state
  let jobId = null;
  let jobStatus = null;
  let jobResult = null;
  let error = null;

  // Wallet modal
  let showWalletModal = false;

  // Operations available for analytics
  const operations = [
    { value: 'sum', label: 'SUM', description: 'Calculate total of all encrypted values' },
    { value: 'average', label: 'AVERAGE', description: 'Calculate average of encrypted values' },
  ];

  function handleWitnessParsed(event) {
    witnessData = event.detail;
    error = null;
    toastStore.add('Witness file parsed successfully', 'success');
  }

  function handleWitnessError(event) {
    witnessData = null;
    error = event.detail.error;
    toastStore.add(`Error parsing witness: ${event.detail.error}`, 'error');
  }

  function handleWalletConnected(event) {
    showWalletModal = false;
    toastStore.add('Wallet connected', 'success');
  }

  async function runAnalytics() {
    if (!$walletStore?.connected) {
      showWalletModal = true;
      return;
    }

    if (!witnessData) {
      toastStore.add('Please upload a witness.bin file first', 'error');
      return;
    }

    isProcessing = true;
    error = null;
    jobId = null;
    jobStatus = null;
    jobResult = null;

    try {
      // Step 1: Create the FHE job
      const result = await createFheJobFromWitness({
        operation: selectedOperation,
        operationValue: 0,
        serverKey: witnessData.serverKey,
        encryptedData: witnessData.encryptedData,
        wallet: $walletStore,
        apiBaseUrl: API_BASE,
        rpcUrl: RPC_URL,
        priceLamports: 5000000,
        requiredProvers: 3,
        consensusThreshold: 2,
        onProgress: (progress) => {
          processingStep = progress.step;
          processingMessage = progress.message;
        }
      });

      jobId = result.jobId;
      toastStore.add(`Job created: ${jobId}`, 'success');

      // Step 2: Poll for job completion
      processingStep = 'polling';
      processingMessage = 'Waiting for FHE computation...';

      const finalStatus = await pollJobStatus(
        jobId,
        API_BASE,
        60,
        2000,
        (progress) => {
          jobStatus = progress.status;
          processingMessage = `Job status: ${progress.status} (attempt ${progress.attempt}/${progress.maxAttempts})`;
        }
      );

      jobStatus = 'completed';

      // Step 3: Get the encrypted result
      processingStep = 'fetching_result';
      processingMessage = 'Fetching encrypted result...';

      jobResult = await getJobResult(jobId, API_BASE);

      toastStore.add('Analytics computation completed!', 'success');

    } catch (err) {
      console.error('Analytics error:', err);
      error = err.message;
      toastStore.add(`Error: ${err.message}`, 'error');
    } finally {
      isProcessing = false;
      processingStep = '';
      processingMessage = '';
    }
  }

  function reset() {
    witnessData = null;
    jobId = null;
    jobStatus = null;
    jobResult = null;
    error = null;
  }

  function goBack() {
    navigateTo('dashboard');
  }

  $: isReady = witnessData && $walletStore?.connected;
</script>

<div class="analytics-page">
  <!-- Header -->
  <header class="page-header">
    <div class="container">
      <div class="header-content">
        <div class="header-left">
          <button class="btn-back text-mono" on:click={goBack}>
            [{'<'} BACK]
          </button>
          <h1 class="text-mono text-uppercase">PRIVATE_ANALYTICS</h1>
        </div>
        <div class="header-right">
          {#if $walletStore?.connected}
            <div class="wallet-badge connected text-mono text-sm">
              <span class="wallet-dot"></span>
              {$walletStore.publicKey.slice(0, 4)}...{$walletStore.publicKey.slice(-4)}
            </div>
          {:else}
            <button class="btn-connect text-mono text-sm" on:click={() => showWalletModal = true}>
              [CONNECT_WALLET]
            </button>
          {/if}
        </div>
      </div>
    </div>
  </header>

  <main class="page-main">
    <div class="container">
      <!-- Info Section -->
      <section class="info-section tui-box mb-6">
        <h2 class="text-mono text-cyan mb-4">[i] PRIVATE_ANALYTICS_INFO</h2>
        <p class="text-mono text-sm text-muted">
          Run aggregate computations on your encrypted data without revealing individual values.
          Upload a witness.bin file generated with <span class="text-cyan">fhe-cli encrypt</span>,
          select an operation, and get the encrypted result.
        </p>
        <div class="code-hint text-mono text-xs mt-4">
          $ fhe-cli encrypt -p ./my-data -v 10,20,30,40,50
        </div>
      </section>

      {#if !jobResult}
        <!-- Step 1: Upload Witness -->
        <section class="step-section mb-6">
          <h3 class="text-mono text-uppercase mb-4">
            <span class="step-number">1</span> UPLOAD_WITNESS
          </h3>
          <WitnessUploader
            compact={false}
            on:witnessParsed={handleWitnessParsed}
            on:witnessError={handleWitnessError}
          />
        </section>

        <!-- Step 2: Select Operation -->
        <section class="step-section mb-6">
          <h3 class="text-mono text-uppercase mb-4">
            <span class="step-number">2</span> SELECT_OPERATION
          </h3>
          <div class="operation-selector tui-box">
            {#each operations as op}
              <label class="operation-option" class:selected={selectedOperation === op.value}>
                <input
                  type="radio"
                  name="operation"
                  value={op.value}
                  bind:group={selectedOperation}
                />
                <span class="option-content">
                  <span class="option-label text-mono">{op.label}</span>
                  <span class="option-desc text-mono text-sm text-muted">{op.description}</span>
                </span>
                <span class="option-check text-mono text-cyan">
                  {selectedOperation === op.value ? '[*]' : '[ ]'}
                </span>
              </label>
            {/each}
          </div>
        </section>

        <!-- Step 3: Run -->
        <section class="step-section mb-6">
          <h3 class="text-mono text-uppercase mb-4">
            <span class="step-number">3</span> RUN_COMPUTATION
          </h3>

          {#if isProcessing}
            <div class="processing-state tui-box">
              <Loading size="medium" />
              <div class="processing-info mt-4">
                <div class="text-mono text-cyan">{processingMessage}</div>
                {#if jobId}
                  <div class="text-mono text-sm text-muted mt-2">Job ID: {jobId}</div>
                {/if}
              </div>
            </div>
          {:else}
            <button
              class="btn btn-primary btn-lg"
              on:click={runAnalytics}
              disabled={!witnessData}
            >
              {#if !$walletStore?.connected}
                [CONNECT_WALLET_TO_RUN]
              {:else if !witnessData}
                [UPLOAD_WITNESS_FIRST]
              {:else}
                [RUN_{selectedOperation.toUpperCase()}_COMPUTATION]
              {/if}
            </button>

            {#if error}
              <div class="error-box mt-4">
                <span class="text-error">[ERROR]</span> {error}
              </div>
            {/if}
          {/if}
        </section>

      {:else}
        <!-- Result Section -->
        <section class="result-section">
          <div class="result-header mb-6">
            <h3 class="text-mono text-uppercase text-success">
              [OK] COMPUTATION_COMPLETE
            </h3>
          </div>

          <div class="result-card tui-box-success mb-6">
            <div class="result-row">
              <span class="result-label text-mono text-muted">Job ID:</span>
              <span class="result-value text-mono text-cyan">{jobId}</span>
            </div>
            <div class="result-row">
              <span class="result-label text-mono text-muted">Operation:</span>
              <span class="result-value text-mono text-cyan">{selectedOperation.toUpperCase()}</span>
            </div>
            <div class="result-row">
              <span class="result-label text-mono text-muted">Status:</span>
              <span class="result-value text-mono text-success">{jobStatus}</span>
            </div>
          </div>

          <div class="encrypted-result tui-box mb-6">
            <h4 class="text-mono mb-3">ENCRYPTED_RESULT:</h4>
            <div class="result-hash text-mono text-xs">
              {#if jobResult?.encrypted_result}
                {jobResult.encrypted_result.slice(0, 100)}...
              {:else}
                [Result available for download]
              {/if}
            </div>
            <p class="text-mono text-xs text-muted mt-3">
              To decrypt this result, use: <span class="text-cyan">fhe-cli decrypt -k client_key.bin -r result.bin</span>
            </p>
          </div>

          <div class="result-actions">
            <button class="btn btn-primary" on:click={reset}>
              [RUN_ANOTHER_COMPUTATION]
            </button>
            <button class="btn btn-ghost" on:click={goBack}>
              [BACK_TO_DASHBOARD]
            </button>
          </div>
        </section>
      {/if}
    </div>
  </main>
</div>

<!-- Wallet Modal -->
{#if showWalletModal}
  <div class="modal-overlay" on:click={() => showWalletModal = false}>
    <div class="modal-content" on:click|stopPropagation>
      <button class="modal-close" on:click={() => showWalletModal = false}>[X]</button>
      <WalletConnect on:connected={handleWalletConnected} />
    </div>
  </div>
{/if}

<style>
  .analytics-page {
    min-height: 100vh;
    padding-bottom: var(--space-8);
  }

  .page-header {
    padding: var(--space-6) 0;
    border-bottom: 1px solid var(--zyber-border-muted);
    background: rgba(15, 23, 42, 0.95);
    backdrop-filter: blur(10px);
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

  .btn-back {
    background: transparent;
    border: 1px solid var(--zyber-border-muted);
    color: var(--zyber-text-muted);
    padding: var(--space-2) var(--space-3);
    cursor: pointer;
    transition: all var(--transition-fast);
  }

  .btn-back:hover {
    border-color: var(--zyber-cyber-cyan);
    color: var(--zyber-cyber-cyan);
  }

  h1 {
    margin: 0;
    font-size: var(--text-xl);
    background: linear-gradient(135deg, var(--zyber-quantum-violet), var(--zyber-cyber-cyan));
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
  }

  .wallet-badge {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
    border-radius: var(--radius-md);
    border: 1px solid var(--zyber-success);
    background: rgba(16, 185, 129, 0.1);
  }

  .wallet-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--zyber-success);
    box-shadow: 0 0 8px var(--zyber-success);
  }

  .btn-connect {
    background: rgba(6, 182, 212, 0.1);
    border: 1px solid var(--zyber-cyber-cyan);
    color: var(--zyber-cyber-cyan);
    padding: var(--space-2) var(--space-4);
    cursor: pointer;
    border-radius: var(--radius-md);
    transition: all var(--transition-fast);
  }

  .btn-connect:hover {
    background: rgba(6, 182, 212, 0.2);
    box-shadow: 0 0 15px rgba(6, 182, 212, 0.3);
  }

  .page-main {
    padding: var(--space-8) 0;
  }

  .info-section {
    padding: var(--space-6);
    background: rgba(6, 182, 212, 0.05);
    border-color: var(--zyber-cyber-cyan);
  }

  .code-hint {
    padding: var(--space-3);
    background: rgba(0, 0, 0, 0.4);
    border-radius: var(--radius-sm);
    color: var(--zyber-cyber-cyan);
  }

  .step-section {
    max-width: 800px;
  }

  .step-number {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    border-radius: 50%;
    background: var(--zyber-quantum-violet);
    color: white;
    font-size: var(--text-sm);
    margin-right: var(--space-2);
  }

  .operation-selector {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding: var(--space-4);
  }

  .operation-option {
    display: flex;
    align-items: center;
    gap: var(--space-4);
    padding: var(--space-4);
    border: 1px solid var(--zyber-border-muted);
    border-radius: var(--radius-md);
    cursor: pointer;
    transition: all var(--transition-fast);
  }

  .operation-option:hover {
    border-color: var(--zyber-border-secondary);
    background: rgba(6, 182, 212, 0.05);
  }

  .operation-option.selected {
    border-color: var(--zyber-cyber-cyan);
    background: rgba(6, 182, 212, 0.1);
  }

  .operation-option input {
    display: none;
  }

  .option-content {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .option-label {
    font-weight: 600;
  }

  .btn {
    font-family: var(--font-mono);
    padding: var(--space-3) var(--space-6);
    border-radius: var(--radius-md);
    cursor: pointer;
    transition: all var(--transition-fast);
  }

  .btn-lg {
    padding: var(--space-4) var(--space-8);
    font-size: var(--text-lg);
  }

  .btn-primary {
    background: var(--zyber-quantum-violet);
    border: 1px solid var(--zyber-quantum-violet);
    color: white;
  }

  .btn-primary:hover:not(:disabled) {
    box-shadow: var(--zyber-glow-violet);
  }

  .btn-primary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .btn-ghost {
    background: transparent;
    border: 1px solid var(--zyber-border-muted);
    color: var(--zyber-text-secondary);
  }

  .btn-ghost:hover {
    border-color: var(--zyber-border-secondary);
  }

  .processing-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: var(--space-8);
    text-align: center;
  }

  .error-box {
    padding: var(--space-4);
    background: rgba(239, 68, 68, 0.1);
    border: 1px solid var(--zyber-error);
    border-radius: var(--radius-md);
  }

  .result-section {
    max-width: 800px;
  }

  .result-card {
    padding: var(--space-6);
  }

  .tui-box-success {
    background: rgba(16, 185, 129, 0.05);
    border: 1px solid var(--zyber-success);
    border-radius: var(--radius-md);
  }

  .result-row {
    display: flex;
    justify-content: space-between;
    padding: var(--space-2) 0;
    border-bottom: 1px solid var(--zyber-border-muted);
  }

  .result-row:last-child {
    border-bottom: none;
  }

  .encrypted-result {
    padding: var(--space-6);
  }

  .result-hash {
    padding: var(--space-4);
    background: rgba(0, 0, 0, 0.4);
    border-radius: var(--radius-sm);
    word-break: break-all;
    color: var(--zyber-cyber-cyan);
  }

  .result-actions {
    display: flex;
    gap: var(--space-4);
    flex-wrap: wrap;
  }

  /* Modal */
  .modal-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.8);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .modal-content {
    background: var(--zyber-bg-base);
    border: 2px solid var(--zyber-border-secondary);
    border-radius: var(--radius-lg);
    padding: var(--space-6);
    max-width: 450px;
    width: 90%;
    position: relative;
  }

  .modal-close {
    position: absolute;
    top: var(--space-3);
    right: var(--space-3);
    background: none;
    border: none;
    color: var(--zyber-text-muted);
    cursor: pointer;
    font-family: var(--font-mono);
  }

  .modal-close:hover {
    color: var(--zyber-error);
  }

  /* Utilities */
  .mb-3 { margin-bottom: var(--space-3); }
  .mb-4 { margin-bottom: var(--space-4); }
  .mb-6 { margin-bottom: var(--space-6); }
  .mt-2 { margin-top: var(--space-2); }
  .mt-3 { margin-top: var(--space-3); }
  .mt-4 { margin-top: var(--space-4); }
  .text-cyan { color: var(--zyber-cyber-cyan); }
  .text-success { color: var(--zyber-success); }
  .text-error { color: var(--zyber-error); }
  .text-muted { color: var(--zyber-text-muted); }
</style>
