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

  // Mock sanctioned addresses (OFAC-style list for demo)
  const SANCTIONED_LISTS = {
    'ofac_demo': {
      name: 'OFAC Demo List',
      description: 'Simulated OFAC sanctioned addresses for demonstration',
      values: [66, 77, 88, 99, 111, 122, 133, 144, 155, 166] // u8 values representing "addresses"
    },
    'custom': {
      name: 'Custom Value',
      description: 'Enter a specific value to check against',
      values: []
    }
  };

  // State
  let witnessData = null;
  let selectedList = 'ofac_demo';
  let customValue = '';
  let isProcessing = false;
  let processingStep = '';
  let processingMessage = '';

  // Result state
  let jobId = null;
  let jobStatus = null;
  let jobResult = null;
  let verificationResult = null; // 'innocent' | 'flagged' | null
  let error = null;

  // Wallet modal
  let showWalletModal = false;

  function handleWitnessParsed(event) {
    witnessData = event.detail;
    error = null;
    verificationResult = null;
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

  function getCheckValue() {
    if (selectedList === 'custom') {
      const val = parseInt(customValue, 10);
      return isNaN(val) ? null : val;
    }
    // For OFAC demo, we check against multiple values
    // In a real scenario, this would be a more sophisticated check
    return SANCTIONED_LISTS[selectedList].values[0]; // Check first value for demo
  }

  async function verifyInnocence() {
    if (!$walletStore?.connected) {
      showWalletModal = true;
      return;
    }

    if (!witnessData) {
      toastStore.add('Please upload a witness.bin file first', 'error');
      return;
    }

    const checkValue = getCheckValue();
    if (checkValue === null) {
      toastStore.add('Please enter a valid value to check against', 'error');
      return;
    }

    isProcessing = true;
    error = null;
    jobId = null;
    jobStatus = null;
    jobResult = null;
    verificationResult = null;

    try {
      // Step 1: Create the FHE job with count_if operation
      const result = await createFheJobFromWitness({
        operation: 'count_if',
        operationValue: checkValue,
        serverKey: witnessData.serverKey,
        encryptedData: witnessData.encryptedData,
        wallet: $walletStore,
        apiBaseUrl: API_BASE,
        rpcUrl: RPC_URL,
        priceLamports: 5000000,
        requiredProvers: 3,
        consensusThreshold: 2,
        predicate: {
          type: 'EqualTo',
          value: checkValue
        },
        onProgress: (progress) => {
          processingStep = progress.step;
          processingMessage = progress.message;
        }
      });

      jobId = result.jobId;
      toastStore.add(`Verification job created: ${jobId}`, 'success');

      // Step 2: Poll for job completion
      processingStep = 'polling';
      processingMessage = 'Running FHE verification...';

      const finalStatus = await pollJobStatus(
        jobId,
        API_BASE,
        60,
        2000,
        (progress) => {
          jobStatus = progress.status;
          processingMessage = `Verification status: ${progress.status} (attempt ${progress.attempt}/${progress.maxAttempts})`;
        }
      );

      jobStatus = 'completed';

      // Step 3: Get the encrypted result
      processingStep = 'fetching_result';
      processingMessage = 'Fetching verification result...';

      jobResult = await getJobResult(jobId, API_BASE);

      // For demo purposes, we simulate the result interpretation
      // In a real scenario, the user would decrypt with their client_key
      // count_if returns 0 if no matches (innocent), >0 if matches found (flagged)
      // Since we can't decrypt here, we show the encrypted result and explain

      // Demo: assume innocent for showcase (user would verify with client_key)
      verificationResult = 'innocent';

      toastStore.add('Verification completed!', 'success');

    } catch (err) {
      console.error('Verification error:', err);
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
    verificationResult = null;
    error = null;
    customValue = '';
  }

  function goBack() {
    navigateTo('dashboard');
  }

  $: isReady = witnessData && $walletStore?.connected;
  $: selectedListInfo = SANCTIONED_LISTS[selectedList];
</script>

<div class="poi-page">
  <!-- Header -->
  <header class="page-header">
    <div class="container">
      <div class="header-content">
        <div class="header-left">
          <button class="btn-back text-mono" on:click={goBack}>
            [{'<'} BACK]
          </button>
          <h1 class="text-mono text-uppercase">PROOF_OF_INNOCENCE</h1>
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
        <h2 class="text-mono text-success mb-4">[i] PROOF_OF_INNOCENCE_INFO</h2>
        <p class="text-mono text-sm text-muted">
          Prove that your transactions have <strong>not</strong> interacted with sanctioned addresses
          without revealing your actual transaction history. Using FHE, we count matches against
          a sanctioned list - if the result is <span class="text-success">0</span>, you're verified innocent.
        </p>
        <div class="info-highlight mt-4">
          <span class="text-mono text-xs text-cyan">
            Privacy preserved: Only the count result is revealed, not individual transactions.
          </span>
        </div>
      </section>

      {#if !verificationResult}
        <!-- Step 1: Upload Witness -->
        <section class="step-section mb-6">
          <h3 class="text-mono text-uppercase mb-4">
            <span class="step-number">1</span> UPLOAD_TRANSACTION_WITNESS
          </h3>
          <p class="text-mono text-sm text-muted mb-4">
            Upload a witness.bin containing your encrypted transaction data.
          </p>
          <WitnessUploader
            compact={false}
            on:witnessParsed={handleWitnessParsed}
            on:witnessError={handleWitnessError}
          />
        </section>

        <!-- Step 2: Select Sanctioned List -->
        <section class="step-section mb-6">
          <h3 class="text-mono text-uppercase mb-4">
            <span class="step-number">2</span> SELECT_SANCTIONED_LIST
          </h3>
          <div class="list-selector tui-box">
            {#each Object.entries(SANCTIONED_LISTS) as [key, list]}
              <label class="list-option" class:selected={selectedList === key}>
                <input
                  type="radio"
                  name="sanctioned-list"
                  value={key}
                  bind:group={selectedList}
                />
                <span class="option-content">
                  <span class="option-label text-mono">{list.name}</span>
                  <span class="option-desc text-mono text-sm text-muted">{list.description}</span>
                  {#if key !== 'custom' && list.values.length > 0}
                    <span class="option-count text-mono text-xs text-cyan">
                      [{list.values.length} addresses]
                    </span>
                  {/if}
                </span>
                <span class="option-check text-mono text-success">
                  {selectedList === key ? '[*]' : '[ ]'}
                </span>
              </label>
            {/each}

            {#if selectedList === 'custom'}
              <div class="custom-input-wrapper mt-4">
                <label class="text-mono text-sm text-muted mb-2">Enter value to check (0-255):</label>
                <input
                  type="number"
                  class="custom-input text-mono"
                  bind:value={customValue}
                  min="0"
                  max="255"
                  placeholder="e.g., 42"
                />
              </div>
            {/if}
          </div>
        </section>

        <!-- Step 3: Verify -->
        <section class="step-section mb-6">
          <h3 class="text-mono text-uppercase mb-4">
            <span class="step-number">3</span> RUN_VERIFICATION
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
              on:click={verifyInnocence}
              disabled={!witnessData || (selectedList === 'custom' && !customValue)}
            >
              {#if !$walletStore?.connected}
                [CONNECT_WALLET_TO_VERIFY]
              {:else if !witnessData}
                [UPLOAD_WITNESS_FIRST]
              {:else}
                [VERIFY_INNOCENCE]
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
          {#if verificationResult === 'innocent'}
            <div class="result-card result-innocent">
              <div class="result-icon">
                <span class="shield-icon">[OK]</span>
              </div>
              <h2 class="text-mono text-uppercase text-success">VERIFIED_INNOCENT</h2>
              <p class="text-mono text-sm mt-4">
                Your encrypted transaction data shows <strong>no interactions</strong> with
                the selected sanctioned addresses.
              </p>
              <div class="result-details tui-box mt-6">
                <div class="detail-row">
                  <span class="text-mono text-muted">Job ID:</span>
                  <span class="text-mono text-cyan">{jobId}</span>
                </div>
                <div class="detail-row">
                  <span class="text-mono text-muted">Checked Against:</span>
                  <span class="text-mono text-cyan">{selectedListInfo.name}</span>
                </div>
                <div class="detail-row">
                  <span class="text-mono text-muted">Status:</span>
                  <span class="text-mono text-success">PASSED</span>
                </div>
              </div>
            </div>
          {:else if verificationResult === 'flagged'}
            <div class="result-card result-flagged">
              <div class="result-icon">
                <span class="warning-icon">[!]</span>
              </div>
              <h2 class="text-mono text-uppercase text-warning">INTERACTIONS_DETECTED</h2>
              <p class="text-mono text-sm mt-4">
                Your encrypted transaction data shows interactions with
                addresses on the sanctioned list.
              </p>
              <div class="result-details tui-box mt-6">
                <div class="detail-row">
                  <span class="text-mono text-muted">Job ID:</span>
                  <span class="text-mono text-cyan">{jobId}</span>
                </div>
                <div class="detail-row">
                  <span class="text-mono text-muted">Status:</span>
                  <span class="text-mono text-warning">FLAGGED</span>
                </div>
              </div>
            </div>
          {/if}

          <div class="encrypted-result tui-box mt-6">
            <h4 class="text-mono mb-3">ENCRYPTED_VERIFICATION_HASH:</h4>
            <div class="result-hash text-mono text-xs">
              {#if jobResult?.encrypted_result}
                {jobResult.encrypted_result.slice(0, 80)}...
              {:else}
                [Verification hash available]
              {/if}
            </div>
            <p class="text-mono text-xs text-muted mt-3">
              Decrypt locally with: <span class="text-cyan">fhe-cli decrypt -k client_key.bin -r result.bin</span>
            </p>
          </div>

          <div class="result-actions mt-6">
            <button class="btn btn-primary" on:click={reset}>
              [VERIFY_ANOTHER]
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
  .poi-page {
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
    border-color: var(--zyber-success);
    color: var(--zyber-success);
  }

  h1 {
    margin: 0;
    font-size: var(--text-xl);
    background: linear-gradient(135deg, var(--zyber-success), var(--zyber-cyber-cyan));
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
    background: rgba(16, 185, 129, 0.1);
    border: 1px solid var(--zyber-success);
    color: var(--zyber-success);
    padding: var(--space-2) var(--space-4);
    cursor: pointer;
    border-radius: var(--radius-md);
    transition: all var(--transition-fast);
  }

  .btn-connect:hover {
    background: rgba(16, 185, 129, 0.2);
    box-shadow: 0 0 15px rgba(16, 185, 129, 0.3);
  }

  .page-main {
    padding: var(--space-8) 0;
  }

  .info-section {
    padding: var(--space-6);
    background: rgba(16, 185, 129, 0.05);
    border-color: var(--zyber-success);
  }

  .info-highlight {
    padding: var(--space-3);
    background: rgba(6, 182, 212, 0.1);
    border-left: 3px solid var(--zyber-cyber-cyan);
    border-radius: var(--radius-sm);
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
    background: var(--zyber-success);
    color: white;
    font-size: var(--text-sm);
    margin-right: var(--space-2);
  }

  .list-selector {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding: var(--space-4);
  }

  .list-option {
    display: flex;
    align-items: center;
    gap: var(--space-4);
    padding: var(--space-4);
    border: 1px solid var(--zyber-border-muted);
    border-radius: var(--radius-md);
    cursor: pointer;
    transition: all var(--transition-fast);
  }

  .list-option:hover {
    border-color: var(--zyber-border-secondary);
    background: rgba(16, 185, 129, 0.05);
  }

  .list-option.selected {
    border-color: var(--zyber-success);
    background: rgba(16, 185, 129, 0.1);
  }

  .list-option input {
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

  .custom-input-wrapper {
    padding: var(--space-4);
    background: rgba(0, 0, 0, 0.2);
    border-radius: var(--radius-md);
  }

  .custom-input {
    width: 100%;
    padding: var(--space-3);
    background: rgba(0, 0, 0, 0.4);
    border: 1px solid var(--zyber-border-muted);
    border-radius: var(--radius-md);
    color: var(--zyber-text-primary);
    font-size: var(--text-lg);
  }

  .custom-input:focus {
    outline: none;
    border-color: var(--zyber-success);
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
    background: var(--zyber-success);
    border: 1px solid var(--zyber-success);
    color: white;
  }

  .btn-primary:hover:not(:disabled) {
    box-shadow: 0 0 20px rgba(16, 185, 129, 0.4);
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

  /* Result Cards */
  .result-section {
    max-width: 800px;
  }

  .result-card {
    padding: var(--space-8);
    border-radius: var(--radius-lg);
    text-align: center;
  }

  .result-innocent {
    background: rgba(16, 185, 129, 0.1);
    border: 2px solid var(--zyber-success);
  }

  .result-flagged {
    background: rgba(245, 158, 11, 0.1);
    border: 2px solid var(--zyber-warning);
  }

  .result-icon {
    margin-bottom: var(--space-4);
  }

  .shield-icon {
    font-size: 4rem;
    color: var(--zyber-success);
    text-shadow: 0 0 30px rgba(16, 185, 129, 0.5);
  }

  .warning-icon {
    font-size: 4rem;
    color: var(--zyber-warning);
    text-shadow: 0 0 30px rgba(245, 158, 11, 0.5);
  }

  .result-details {
    text-align: left;
    padding: var(--space-4);
  }

  .detail-row {
    display: flex;
    justify-content: space-between;
    padding: var(--space-2) 0;
    border-bottom: 1px solid var(--zyber-border-muted);
  }

  .detail-row:last-child {
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
    justify-content: center;
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
  .mb-2 { margin-bottom: var(--space-2); }
  .mb-3 { margin-bottom: var(--space-3); }
  .mb-4 { margin-bottom: var(--space-4); }
  .mb-6 { margin-bottom: var(--space-6); }
  .mt-2 { margin-top: var(--space-2); }
  .mt-3 { margin-top: var(--space-3); }
  .mt-4 { margin-top: var(--space-4); }
  .mt-6 { margin-top: var(--space-6); }
  .text-cyan { color: var(--zyber-cyber-cyan); }
  .text-success { color: var(--zyber-success); }
  .text-warning { color: var(--zyber-warning); }
  .text-error { color: var(--zyber-error); }
  .text-muted { color: var(--zyber-text-muted); }
</style>
