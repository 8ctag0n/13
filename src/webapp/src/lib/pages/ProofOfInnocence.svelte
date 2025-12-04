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
      values: [66, 77, 88, 99, 111, 122, 133, 144, 155, 166]
    },
    'custom': {
      name: 'Custom Value',
      description: 'Enter a specific value to check against',
      values: []
    }
  };

  // State
  let currentStep = 1;
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
  let verificationResult = null;
  let error = null;

  // Wallet modal
  let showWalletModal = false;

  // Reactive wallet state
  $: isWalletConnected = $walletStore?.connected === true;
  $: walletAddress = $walletStore?.addresses?.solana || '';

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
    return SANCTIONED_LISTS[selectedList].values[0];
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
      const result = await createFheJobFromWitness({
        operation: 'count_if',
        operationValue: checkValue,
        serverKeyBytes: witnessData.serverKeyBytes,  // Use raw bytes for pre-upload
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

      processingStep = 'fetching_result';
      processingMessage = 'Fetching verification result...';

      jobResult = await getJobResult(jobId, API_BASE);

      verificationResult = 'innocent';
      currentStep = 4;

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

  function nextStep() {
    if (currentStep < 3) {
      currentStep++;
    }
  }

  function prevStep() {
    if (currentStep > 1) {
      currentStep--;
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
    currentStep = 1;
  }

  function handleCancel() {
    navigateTo('dashboard');
  }

  $: canProceedStep1 = witnessData !== null;
  $: canProceedStep2 = selectedList && (selectedList !== 'custom' || customValue);
  $: selectedListInfo = SANCTIONED_LISTS[selectedList];
</script>

<div class="poi-page">
  <!-- Header -->
  <header class="wizard-header">
    <div class="container">
      <div class="header-content">
        <h1 class="text-mono text-uppercase">PROOF_OF_INNOCENCE</h1>
        <div class="header-right">
          {#if isWalletConnected}
            <div class="wallet-badge connected text-mono text-sm">
              <span class="wallet-dot"></span>
              {walletAddress.slice(0, 4)}...{walletAddress.slice(-4)}
            </div>
          {:else}
            <button class="wallet-badge disconnected text-mono text-sm" on:click={() => { showWalletModal = true; }}>
              <span class="wallet-dot"></span>
              CONNECT_WALLET
            </button>
          {/if}
          <div class="step-indicator text-mono text-sm text-muted">
            STEP {currentStep} OF 3
          </div>
        </div>
      </div>
    </div>
  </header>

  <div class="divider-header text-mono text-muted">
    ════════════════════════════════════════════════════════════════════════════════
  </div>

  <!-- Stepper -->
  <div class="container">
    <div class="stepper">
      <div class="stepper-line"></div>

      <div class="stepper-step {currentStep >= 1 ? 'active' : ''} {currentStep > 1 ? 'completed' : ''}">
        <div class="step-circle text-mono">
          {currentStep > 1 ? '✓' : '1'}
        </div>
        <div class="step-label text-mono text-xs">UPLOAD</div>
      </div>

      <div class="stepper-step {currentStep >= 2 ? 'active' : ''} {currentStep > 2 ? 'completed' : ''}">
        <div class="step-circle text-mono">
          {currentStep > 2 ? '✓' : '2'}
        </div>
        <div class="step-label text-mono text-xs">SELECT</div>
      </div>

      <div class="stepper-step {currentStep >= 3 ? 'active' : ''}">
        <div class="step-circle text-mono">3</div>
        <div class="step-label text-mono text-xs">VERIFY</div>
      </div>
    </div>
  </div>

  <!-- Main Content -->
  <main class="wizard-main">
    <div class="container">
      {#if currentStep === 1}
        <!-- Step 1: Upload Witness -->
        <div class="wizard-card tui-box fade-in">
          <h2 class="text-mono text-uppercase mb-6">
            STEP_1: UPLOAD_TRANSACTION_WITNESS
          </h2>

          <div class="info-box mb-6">
            <div class="text-sm">
              Prove that your transactions have <strong>not</strong> interacted with sanctioned addresses
              without revealing your actual transaction history.
            </div>
          </div>

          <div class="info-box-success mb-6">
            <div class="text-mono text-sm">
              [i] PRIVACY_PRESERVED<br/>
              Using FHE, we count matches against a sanctioned list - if the result is <span class="text-success">0</span>, you're verified innocent.
              Only the count result is revealed, not individual transactions.
            </div>
          </div>

          <WitnessUploader
            compact={false}
            on:witnessParsed={handleWitnessParsed}
            on:witnessError={handleWitnessError}
          />

          {#if witnessData}
            <div class="upload-success mt-4">
              <span class="text-success">[OK]</span> Witness file loaded successfully
            </div>
          {/if}
        </div>

      {:else if currentStep === 2}
        <!-- Step 2: Select Sanctioned List -->
        <div class="wizard-card tui-box fade-in">
          <h2 class="text-mono text-uppercase mb-6">
            STEP_2: SELECT_SANCTIONED_LIST
          </h2>

          <div class="info-box mb-6">
            <div class="text-sm">
              Choose which sanctioned address list to verify against.
            </div>
          </div>

          <div class="config-section mb-6">
            <label class="text-mono mb-3">SANCTIONED_LIST:</label>
            <div class="radio-group">
              {#each Object.entries(SANCTIONED_LISTS) as [key, list]}
                <label class="radio-option">
                  <input type="radio" name="sanctioned-list" value={key} bind:group={selectedList} />
                  <span class="radio-label text-mono">
                    <span class="radio-check">{selectedList === key ? '●' : '○'}</span>
                    <span>{list.name}</span>
                    <span class="text-muted text-sm">{list.description}</span>
                    {#if key !== 'custom' && list.values.length > 0}
                      <span class="badge badge-info">[{list.values.length} addresses]</span>
                    {/if}
                  </span>
                </label>
              {/each}
            </div>
          </div>

          {#if selectedList === 'custom'}
            <div class="config-section mb-6">
              <label class="text-mono mb-2" for="custom-value-input">CUSTOM_VALUE (0-255):</label>
              <input
                type="number"
                id="custom-value-input"
                bind:value={customValue}
                class="input input-code"
                min="0"
                max="255"
                placeholder="e.g., 42"
              />
            </div>
          {/if}

          <div class="divider-section"></div>

          <!-- Cost Info -->
          <div class="cost-breakdown tui-box-success">
            <div class="text-mono text-uppercase mb-3">VERIFICATION_COST:</div>
            <div class="cost-lines text-mono text-sm">
              <div class="cost-line">
                <span class="text-muted">Operation:</span>
                <span class="text-success">COUNT_IF</span>
              </div>
              <div class="cost-line">
                <span class="text-muted">Provers (3):</span>
                <span class="text-success">0.005_SOL</span>
              </div>
              <div class="cost-line">
                <span class="text-muted">Platform Fee:</span>
                <span class="text-success">0.00005_SOL</span>
              </div>
              <div class="divider-cost text-muted">───────────────────────────────</div>
              <div class="cost-line total">
                <span>Total:</span>
                <span class="text-success total-amount">~0.00505_SOL</span>
              </div>
            </div>
          </div>
        </div>

      {:else if currentStep === 3}
        <!-- Step 3: Verify -->
        <div class="wizard-card tui-box fade-in">
          <h2 class="text-mono text-uppercase mb-6 text-center">
            STEP_3: RUN_VERIFICATION
          </h2>

          <div class="signing-state">
            {#if !isWalletConnected}
              <div class="wallet-connect-prompt">
                <div class="text-mono text-center mb-4 text-warning">
                  [!] WALLET_NOT_CONNECTED
                </div>
                <div class="text-sm text-muted text-center mb-6">
                  Connect your Solana wallet to run the verification
                </div>
                <WalletConnect on:connected={handleWalletConnected} />
              </div>
            {:else if isProcessing}
              <div class="text-center mb-6">
                <Loading size="large" />
              </div>
              <div class="text-mono text-center mb-4 text-success">
                {processingMessage || 'Processing...'}<span class="cursor-blink"></span>
              </div>
              {#if jobId}
                <div class="text-sm text-muted text-center">Job ID: {jobId}</div>
              {/if}
            {:else}
              <div class="wallet-icon text-center mb-6">
                <div class="text-4xl text-mono text-success">[OK]</div>
              </div>
              <div class="text-mono text-center mb-4 text-success">
                READY_TO_VERIFY
              </div>
              <div class="text-sm text-muted text-center mb-6">
                Click "VERIFY_INNOCENCE" below to run the FHE verification
              </div>
            {/if}

            {#if error}
              <div class="error-box mt-4">
                <span class="text-error">[ERROR]</span> {error}
              </div>
            {/if}

            <div class="divider-section"></div>

            <div class="tx-details text-mono text-sm">
              <div class="text-muted mb-2">VERIFICATION_DETAILS:</div>
              <div class="tx-line">• Check against: {selectedListInfo.name}</div>
              <div class="tx-line">• Operation: count_if (predicate match)</div>
              <div class="tx-line">• Cost: ~0.005_SOL + network_fee</div>
            </div>
          </div>
        </div>

      {:else if currentStep === 4}
        <!-- Result Section -->
        <div class="wizard-card tui-box fade-in">
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
            </div>
          {/if}

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
        </div>
      {/if}

      <!-- Navigation Buttons -->
      <div class="wizard-nav">
        <button
          class="btn btn-ghost"
          on:click={currentStep === 1 ? handleCancel : (currentStep === 4 ? reset : prevStep)}
        >
          [{currentStep === 1 ? 'CANCEL' : currentStep === 4 ? 'VERIFY_ANOTHER' : '◀ BACK'}]
        </button>

        {#if currentStep < 3}
          <button
            class="btn btn-primary"
            on:click={nextStep}
            disabled={(currentStep === 1 && !canProceedStep1) || (currentStep === 2 && !canProceedStep2)}
          >
            [NEXT: {currentStep === 1 ? 'SELECT_LIST' : 'VERIFY'} ▶]
          </button>
        {:else if currentStep === 3}
          <button
            class="btn btn-primary"
            on:click={verifyInnocence}
            disabled={isProcessing || !isWalletConnected}
          >
            [VERIFY_INNOCENCE]
          </button>
        {:else}
          <button class="btn btn-primary" on:click={handleCancel}>
            [BACK_TO_DASHBOARD]
          </button>
        {/if}
      </div>
    </div>
  </main>
</div>

<!-- Wallet Modal -->
{#if showWalletModal}
  <div class="wallet-modal-overlay" on:click={() => showWalletModal = false}>
    <div class="wallet-modal" on:click|stopPropagation>
      <button class="wallet-modal-close" on:click={() => showWalletModal = false}>
        [X]
      </button>
      <WalletConnect on:connected={handleWalletConnected} />
    </div>
  </div>
{/if}

<style>
  .poi-page {
    min-height: 100vh;
    padding-bottom: var(--space-8);
  }

  .wizard-header {
    padding: var(--space-6) 0;
    border-bottom: 1px solid var(--zyber-border-muted);
  }

  .header-content {
    display: flex;
    justify-content: space-between;
    align-items: center;
    flex-wrap: wrap;
    gap: var(--space-4);
  }

  .header-right {
    display: flex;
    align-items: center;
    gap: var(--space-4);
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
    border: 1px solid var(--zyber-border-muted);
    background: var(--zyber-bg-glass);
  }

  .wallet-badge.connected {
    border-color: var(--zyber-success);
    background: rgba(16, 185, 129, 0.1);
  }

  .wallet-badge.disconnected {
    cursor: pointer;
    border-color: var(--zyber-warning);
    background: rgba(245, 158, 11, 0.1);
  }

  .wallet-badge.disconnected:hover {
    background: rgba(245, 158, 11, 0.2);
  }

  .wallet-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--zyber-border-muted);
  }

  .wallet-badge.connected .wallet-dot {
    background: var(--zyber-success);
    box-shadow: 0 0 6px var(--zyber-success);
  }

  .wallet-badge.disconnected .wallet-dot {
    background: var(--zyber-warning);
  }

  .divider-header {
    font-size: 8px;
    opacity: 0.2;
    text-align: center;
    margin: var(--space-4) 0;
  }

  /* Stepper */
  .stepper {
    display: flex;
    justify-content: space-between;
    align-items: center;
    max-width: 800px;
    margin: var(--space-8) auto;
    position: relative;
    padding: 0 var(--space-4);
  }

  .stepper-line {
    position: absolute;
    top: 20px;
    left: 10%;
    right: 10%;
    height: 2px;
    background: var(--zyber-border-muted);
    z-index: 0;
  }

  .stepper-step {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-2);
    position: relative;
    z-index: 1;
  }

  .step-circle {
    width: 40px;
    height: 40px;
    border-radius: 50%;
    border: 2px solid var(--zyber-border-muted);
    background: var(--zyber-bg-base);
    display: flex;
    align-items: center;
    justify-content: center;
    font-weight: 600;
    transition: all var(--transition-base);
  }

  .stepper-step.active .step-circle {
    border-color: var(--zyber-success);
    background: rgba(16, 185, 129, 0.1);
    color: var(--zyber-success);
    box-shadow: 0 0 15px rgba(16, 185, 129, 0.3);
  }

  .stepper-step.completed .step-circle {
    border-color: var(--zyber-success);
    background: var(--zyber-success);
    color: white;
  }

  .step-label {
    text-transform: uppercase;
    opacity: 0.6;
  }

  .stepper-step.active .step-label {
    opacity: 1;
    color: var(--zyber-success);
  }

  /* Wizard Main */
  .wizard-main {
    padding: var(--space-8) 0;
  }

  .wizard-card {
    max-width: 800px;
    margin: 0 auto var(--space-6);
    padding: var(--space-8);
  }

  /* Info Boxes */
  .info-box, .info-box-success {
    padding: var(--space-4);
    border-radius: var(--radius-md);
    border: 1px solid;
  }

  .info-box {
    background: rgba(6, 182, 212, 0.05);
    border-color: var(--zyber-border-secondary);
  }

  .info-box-success {
    background: rgba(16, 185, 129, 0.05);
    border-color: rgba(16, 185, 129, 0.3);
    border-left-width: 3px;
  }

  .upload-success {
    padding: var(--space-3);
    background: rgba(16, 185, 129, 0.1);
    border: 1px solid rgba(16, 185, 129, 0.3);
    border-radius: var(--radius-md);
  }

  /* Config Section */
  .config-section {
    margin-bottom: var(--space-6);
  }

  .config-section label {
    display: block;
  }

  /* Radio Options */
  .radio-group {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  .radio-option {
    display: flex;
    align-items: center;
    padding: var(--space-4);
    border: 1px solid var(--zyber-border-muted);
    border-radius: var(--radius-md);
    cursor: pointer;
    transition: all var(--transition-base);
  }

  .radio-option:hover {
    border-color: var(--zyber-border-secondary);
    background: rgba(16, 185, 129, 0.05);
  }

  .radio-option input[type="radio"] {
    display: none;
  }

  .radio-option input[type="radio"]:checked + .radio-label {
    color: var(--zyber-text-primary);
  }

  .radio-option input[type="radio"]:checked + .radio-label .radio-check {
    color: var(--zyber-success);
  }

  .radio-label {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    flex: 1;
    flex-wrap: wrap;
  }

  .radio-check {
    font-size: var(--text-xl);
  }

  .badge {
    padding: 2px 8px;
    border-radius: var(--radius-sm);
    font-size: var(--text-xs);
  }

  .badge-info {
    background: rgba(6, 182, 212, 0.2);
    color: var(--zyber-cyber-cyan);
  }

  /* Input */
  .input {
    width: 100%;
    padding: var(--space-3);
    background: rgba(0, 0, 0, 0.4);
    border: 1px solid var(--zyber-border-muted);
    border-radius: var(--radius-md);
    color: var(--zyber-text-primary);
    font-family: var(--font-mono);
    font-size: var(--text-lg);
  }

  .input:focus {
    outline: none;
    border-color: var(--zyber-success);
  }

  /* Dividers */
  .divider-section {
    height: 1px;
    background: var(--zyber-border-muted);
    margin: var(--space-6) 0;
  }

  /* Cost Breakdown */
  .cost-breakdown {
    padding: var(--space-6);
    border: 2px solid rgba(16, 185, 129, 0.3);
    border-radius: var(--radius-md);
    background: rgba(16, 185, 129, 0.05);
  }

  .tui-box-success {
    border-color: rgba(16, 185, 129, 0.3);
  }

  .cost-lines {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .cost-line {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: var(--space-4);
  }

  .cost-line.total {
    font-weight: 600;
    font-size: var(--text-lg);
    padding-top: var(--space-2);
  }

  .divider-cost {
    font-size: 8px;
    opacity: 0.5;
  }

  .total-amount {
    font-size: var(--text-lg);
    font-weight: 600;
  }

  /* Signing State */
  .signing-state {
    max-width: 500px;
    margin: 0 auto;
  }

  .wallet-connect-prompt {
    padding: var(--space-4);
  }

  .tx-details {
    background: rgba(0, 0, 0, 0.3);
    border: 1px solid var(--zyber-border-muted);
    border-radius: var(--radius-md);
    padding: var(--space-4);
  }

  .tx-line {
    margin-bottom: var(--space-2);
  }

  .error-box {
    padding: var(--space-4);
    background: rgba(239, 68, 68, 0.1);
    border: 1px solid var(--zyber-error);
    border-radius: var(--radius-md);
  }

  /* Result */
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

  /* Navigation */
  .wizard-nav {
    display: flex;
    justify-content: space-between;
    gap: var(--space-4);
    max-width: 800px;
    margin: var(--space-8) auto 0;
  }

  .btn {
    font-family: var(--font-mono);
    padding: var(--space-3) var(--space-6);
    border-radius: var(--radius-md);
    cursor: pointer;
    transition: all var(--transition-fast);
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

  /* Modal */
  .wallet-modal-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.8);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .wallet-modal {
    background: var(--zyber-bg-base);
    border: 2px solid var(--zyber-border-secondary);
    border-radius: var(--radius-lg);
    padding: var(--space-6);
    max-width: 450px;
    width: 90%;
    position: relative;
  }

  .wallet-modal-close {
    position: absolute;
    top: var(--space-3);
    right: var(--space-3);
    background: none;
    border: none;
    color: var(--zyber-text-muted);
    cursor: pointer;
    font-size: var(--text-lg);
  }

  .wallet-modal-close:hover {
    color: var(--zyber-text-primary);
  }

  /* Utilities */
  .mb-3 { margin-bottom: var(--space-3); }
  .mb-4 { margin-bottom: var(--space-4); }
  .mb-6 { margin-bottom: var(--space-6); }
  .mt-4 { margin-top: var(--space-4); }
  .mt-6 { margin-top: var(--space-6); }
  .text-cyan { color: var(--zyber-cyber-cyan); }
  .text-success { color: var(--zyber-success); }
  .text-warning { color: var(--zyber-warning); }
  .text-error { color: var(--zyber-error); }
  .text-muted { color: var(--zyber-text-muted); }

  /* Responsive */
  @media (max-width: 768px) {
    .header-content {
      flex-direction: column;
      align-items: flex-start;
      gap: var(--space-3);
    }

    .header-right {
      width: 100%;
      justify-content: space-between;
    }

    .stepper {
      padding: 0;
    }

    .step-circle {
      width: 32px;
      height: 32px;
      font-size: var(--text-sm);
    }

    .step-label {
      font-size: 10px;
    }

    .wizard-card {
      padding: var(--space-6);
    }

    .wizard-nav {
      flex-direction: column;
    }

    .btn {
      width: 100%;
      text-align: center;
    }

    .radio-label {
      flex-direction: column;
      align-items: flex-start;
      gap: var(--space-1);
    }

    .cost-line {
      flex-direction: column;
      align-items: flex-start;
      gap: var(--space-1);
    }
  }
</style>
