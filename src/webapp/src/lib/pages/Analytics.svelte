<script>
  import { walletStore } from '../stores/wallet';
  import { navigateTo } from '../stores/router';
  import { toastStore } from '../stores/toast';
  import WitnessUploader from '../components/WitnessUploader.svelte';
  import WalletConnect from '../components/WalletConnect.svelte';
  import Loading from '../components/Loading.svelte';
  import PriceSlider from '../components/PriceSlider.svelte';
  import { createFheJobFromWitness, pollJobStatus, getJobResult } from '../utils/job_creator.js';

  // API config
  const API_BASE = import.meta.env.VITE_API_URL || '';
  const RPC_URL = import.meta.env.VITE_SOLANA_RPC_URL || 'http://localhost:8899';

  // State
  let currentStep = 1;
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

  // Price configuration
  let priceLamports = 5000000; // Default, will be updated dynamically
  let priceRecommendation = null;
  let isFetchingPrice = false;
  const requiredProvers = 3;

  // Fetch dynamic price recommendation
  async function fetchPriceRecommendation() {
    isFetchingPrice = true;
    try {
      const response = await fetch(`${API_BASE}/api/price-recommendation`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          operation: selectedOperation,
          operation_value: 0,
          expected_count: 10,
          required_provers: requiredProvers
        })
      });

      if (response.ok) {
        priceRecommendation = await response.json();
        if (!priceLamports || priceLamports < priceRecommendation.recommended_price_lamports) {
          priceLamports = priceRecommendation.recommended_price_lamports;
        }
        console.log('Price recommendation:', priceRecommendation);
      }
    } catch (err) {
      console.error('Failed to get price recommendation:', err);
      // Fallback values
      priceRecommendation = {
        min_price_lamports: 3000000,
        recommended_price_lamports: 5400000,
        max_suggested_lamports: 10800000,
        slider_step: 100000
      };
      priceLamports = priceRecommendation.recommended_price_lamports;
    } finally {
      isFetchingPrice = false;
    }
  }

  // Handle price change from slider
  function handlePriceChange(event) {
    priceLamports = event.detail.price;
  }

  // Fetch price when entering step 2
  $: if (currentStep === 2 && !priceRecommendation) {
    fetchPriceRecommendation();
  }

  // Computed price displays
  $: totalCost = (priceLamports / 1000000000).toFixed(5);
  $: platformFee = (priceLamports * 0.01 / 1000000000).toFixed(5);
  $: totalWithFee = ((priceLamports * 1.01) / 1000000000).toFixed(5);

  // Reactive wallet state
  $: isWalletConnected = $walletStore?.connected === true;
  $: walletAddress = $walletStore?.addresses?.solana || '';

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
      // Fetch dynamic price recommendation first
      processingStep = 'pricing';
      processingMessage = 'Getting price recommendation...';
      await fetchPriceRecommendation();

      const result = await createFheJobFromWitness({
        operation: selectedOperation,
        operationValue: 0,
        serverKeyBytes: witnessData.serverKeyBytes,  // Use raw bytes for pre-upload
        encryptedData: witnessData.encryptedData,
        wallet: $walletStore,
        apiBaseUrl: API_BASE,
        rpcUrl: RPC_URL,
        priceLamports: priceLamports,  // Use dynamic price
        requiredProvers: requiredProvers,
        consensusThreshold: 2,
        onProgress: (progress) => {
          processingStep = progress.step;
          processingMessage = progress.message;
        }
      });

      jobId = result.jobId;
      toastStore.add(`Job created: ${jobId}`, 'success');

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

      processingStep = 'fetching_result';
      processingMessage = 'Fetching encrypted result...';

      jobResult = await getJobResult(jobId, API_BASE);
      currentStep = 4;

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
    error = null;
    currentStep = 1;
  }

  function handleCancel() {
    navigateTo('dashboard');
  }

  $: canProceedStep1 = witnessData !== null;
  $: canProceedStep2 = selectedOperation !== null;
  $: selectedOperationInfo = operations.find(op => op.value === selectedOperation);
</script>

<div class="analytics-page">
  <!-- Header -->
  <header class="wizard-header">
    <div class="container">
      <div class="header-content">
        <h1 class="text-mono text-uppercase">PRIVATE_ANALYTICS</h1>
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
        <div class="step-label text-mono text-xs">CONFIGURE</div>
      </div>

      <div class="stepper-step {currentStep >= 3 ? 'active' : ''}">
        <div class="step-circle text-mono">3</div>
        <div class="step-label text-mono text-xs">COMPUTE</div>
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
            STEP_1: UPLOAD_DATA_WITNESS
          </h2>

          <div class="info-box mb-6">
            <div class="text-sm">
              Run aggregate computations on your encrypted data without revealing individual values.
            </div>
          </div>

          <div class="info-box-violet mb-6">
            <div class="text-mono text-sm">
              [i] PRIVACY_PRESERVED<br/>
              Upload a witness.bin file generated with <span class="text-cyan">fhe-cli encrypt</span>.
              Only the encrypted result is revealed, not your raw data.
            </div>
          </div>

          <div class="code-hint text-mono text-xs mb-6">
            $ fhe-cli encrypt -p ./my-data -v 10,20,30,40,50
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
        <!-- Step 2: Select Operation -->
        <div class="wizard-card tui-box fade-in">
          <h2 class="text-mono text-uppercase mb-6">
            STEP_2: SELECT_OPERATION
          </h2>

          <div class="info-box mb-6">
            <div class="text-sm">
              Choose which aggregate computation to run on your encrypted data.
            </div>
          </div>

          <div class="config-section mb-6">
            <label class="text-mono mb-3">OPERATION_TYPE:</label>
            <div class="radio-group">
              {#each operations as op}
                <label class="radio-option">
                  <input type="radio" name="operation" value={op.value} bind:group={selectedOperation} />
                  <span class="radio-label text-mono">
                    <span class="radio-check">{selectedOperation === op.value ? '●' : '○'}</span>
                    <span>{op.label}</span>
                    <span class="text-muted text-sm">{op.description}</span>
                  </span>
                </label>
              {/each}
            </div>
          </div>

          <div class="divider-section"></div>

          <!-- Price Slider -->
          <div class="config-section mb-6">
            <label class="text-mono mb-3">PROVER_PRICE:</label>
            {#if isFetchingPrice}
              <div class="loading-price text-mono text-sm text-muted">
                Loading price recommendation<span class="cursor-blink">_</span>
              </div>
            {:else if priceRecommendation}
              <PriceSlider
                minPrice={priceRecommendation.min_price_lamports}
                recommendedPrice={priceRecommendation.recommended_price_lamports}
                maxPrice={priceRecommendation.max_suggested_lamports}
                currentPrice={priceLamports}
                step={priceRecommendation.slider_step}
                isLoading={isFetchingPrice}
                on:change={handlePriceChange}
              />
            {:else}
              <div class="text-mono text-sm text-muted">
                Price will be calculated when you proceed
              </div>
            {/if}
          </div>

          <!-- Cost Summary -->
          <div class="cost-breakdown tui-box-violet">
            <div class="text-mono text-uppercase mb-3">COMPUTATION_COST:</div>
            <div class="cost-lines text-mono text-sm">
              <div class="cost-line">
                <span class="text-muted">Operation:</span>
                <span class="text-violet">{selectedOperation.toUpperCase()}</span>
              </div>
              <div class="cost-line">
                <span class="text-muted">Provers ({requiredProvers}):</span>
                <span class="text-violet">{totalCost}_SOL</span>
              </div>
              <div class="cost-line">
                <span class="text-muted">Platform Fee (1%):</span>
                <span class="text-violet">{platformFee}_SOL</span>
              </div>
              <div class="divider-cost text-muted">───────────────────────────────</div>
              <div class="cost-line total">
                <span>Total:</span>
                <span class="text-violet total-amount">~{totalWithFee}_SOL</span>
              </div>
            </div>
          </div>
        </div>

      {:else if currentStep === 3}
        <!-- Step 3: Run Computation -->
        <div class="wizard-card tui-box fade-in">
          <h2 class="text-mono text-uppercase mb-6 text-center">
            STEP_3: RUN_COMPUTATION
          </h2>

          <div class="signing-state">
            {#if !isWalletConnected}
              <div class="wallet-connect-prompt">
                <div class="text-mono text-center mb-4 text-warning">
                  [!] WALLET_NOT_CONNECTED
                </div>
                <div class="text-sm text-muted text-center mb-6">
                  Connect your Solana wallet to run the computation
                </div>
                <WalletConnect on:connected={handleWalletConnected} />
              </div>
            {:else if isProcessing}
              <div class="text-center mb-6">
                <Loading size="large" />
              </div>
              <div class="text-mono text-center mb-4 text-violet">
                {processingMessage || 'Processing...'}<span class="cursor-blink"></span>
              </div>
              {#if jobId}
                <div class="text-sm text-muted text-center">Job ID: {jobId}</div>
              {/if}
            {:else}
              <div class="wallet-icon text-center mb-6">
                <div class="text-4xl text-mono text-violet">[OK]</div>
              </div>
              <div class="text-mono text-center mb-4 text-violet">
                READY_TO_COMPUTE
              </div>
              <div class="text-sm text-muted text-center mb-6">
                Click "RUN_COMPUTATION" below to execute the FHE operation
              </div>
            {/if}

            {#if error}
              <div class="error-box mt-4">
                <span class="text-error">[ERROR]</span> {error}
              </div>
            {/if}

            <div class="divider-section"></div>

            <div class="tx-details text-mono text-sm">
              <div class="text-muted mb-2">COMPUTATION_DETAILS:</div>
              <div class="tx-line">• Operation: {selectedOperation.toUpperCase()}</div>
              <div class="tx-line">• Provers: {requiredProvers} (2-of-3 consensus)</div>
              <div class="tx-line">• Cost: ~{totalWithFee}_SOL + network_fee</div>
            </div>
          </div>
        </div>

      {:else if currentStep === 4}
        <!-- Result Section -->
        <div class="wizard-card tui-box fade-in">
          <div class="result-header mb-6 text-center">
            <div class="text-4xl text-mono text-violet mb-4">[OK]</div>
            <h2 class="text-mono text-uppercase text-violet">
              COMPUTATION_COMPLETE
            </h2>
          </div>

          <div class="result-card tui-box-violet mb-6">
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
        </div>
      {/if}

      <!-- Navigation Buttons -->
      <div class="wizard-nav">
        <button
          class="btn btn-ghost"
          on:click={currentStep === 1 ? handleCancel : (currentStep === 4 ? reset : prevStep)}
        >
          [{currentStep === 1 ? 'CANCEL' : currentStep === 4 ? 'RUN_ANOTHER' : '◀ BACK'}]
        </button>

        {#if currentStep < 3}
          <button
            class="btn btn-primary"
            on:click={nextStep}
            disabled={(currentStep === 1 && !canProceedStep1) || (currentStep === 2 && !canProceedStep2)}
          >
            [NEXT: {currentStep === 1 ? 'CONFIGURE' : 'COMPUTE'} ▶]
          </button>
        {:else if currentStep === 3}
          <button
            class="btn btn-primary"
            on:click={runAnalytics}
            disabled={isProcessing || !isWalletConnected}
          >
            [RUN_{selectedOperation.toUpperCase()}_COMPUTATION]
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
  .analytics-page {
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
    border-color: var(--zyber-quantum-violet);
    background: rgba(139, 92, 246, 0.1);
    color: var(--zyber-quantum-violet);
    box-shadow: var(--zyber-glow-violet);
  }

  .stepper-step.completed .step-circle {
    border-color: var(--zyber-quantum-violet);
    background: var(--zyber-quantum-violet);
    color: white;
  }

  .step-label {
    text-transform: uppercase;
    opacity: 0.6;
  }

  .stepper-step.active .step-label {
    opacity: 1;
    color: var(--zyber-quantum-violet);
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
  .info-box, .info-box-violet {
    padding: var(--space-4);
    border-radius: var(--radius-md);
    border: 1px solid;
  }

  .info-box {
    background: rgba(6, 182, 212, 0.05);
    border-color: var(--zyber-border-secondary);
  }

  .info-box-violet {
    background: rgba(139, 92, 246, 0.05);
    border-color: rgba(139, 92, 246, 0.3);
    border-left-width: 3px;
  }

  .code-hint {
    padding: var(--space-3);
    background: rgba(0, 0, 0, 0.4);
    border-radius: var(--radius-sm);
    color: var(--zyber-cyber-cyan);
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
    background: rgba(139, 92, 246, 0.05);
  }

  .radio-option input[type="radio"] {
    display: none;
  }

  .radio-option input[type="radio"]:checked + .radio-label {
    color: var(--zyber-text-primary);
  }

  .radio-option input[type="radio"]:checked + .radio-label .radio-check {
    color: var(--zyber-quantum-violet);
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

  /* Dividers */
  .divider-section {
    height: 1px;
    background: var(--zyber-border-muted);
    margin: var(--space-6) 0;
  }

  /* Cost Breakdown */
  .cost-breakdown {
    padding: var(--space-6);
    border: 2px solid rgba(139, 92, 246, 0.3);
    border-radius: var(--radius-md);
    background: rgba(139, 92, 246, 0.05);
  }

  .tui-box-violet {
    border-color: rgba(139, 92, 246, 0.3);
    background: rgba(139, 92, 246, 0.05);
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

  .text-violet {
    color: var(--zyber-quantum-violet);
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
  .result-header {
    text-align: center;
  }

  .result-card {
    padding: var(--space-6);
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
  .mt-3 { margin-top: var(--space-3); }
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

    .result-row {
      flex-direction: column;
      align-items: flex-start;
      gap: var(--space-1);
    }
  }
</style>
