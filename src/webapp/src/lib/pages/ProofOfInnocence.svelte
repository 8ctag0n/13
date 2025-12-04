<script>
  import { walletStore } from '../stores/wallet';
  import { navigateTo } from '../stores/router';
  import { toastStore } from '../stores/toast';
  import WitnessUploader from '../components/WitnessUploader.svelte';
  import WalletConnect from '../components/WalletConnect.svelte';
  import Loading from '../components/Loading.svelte';
  import PriceSlider from '../components/PriceSlider.svelte';
  import { createFheJobFromWitness } from '../utils/job_creator.js';

  // API config
  const API_BASE = import.meta.env.VITE_API_URL || '';
  const RPC_URL = import.meta.env.VITE_SOLANA_RPC_URL || 'http://localhost:8899';

  // Demo sanctioned list (simulated OFAC-style indices)
  // In production, these would be hashes of real sanctioned addresses
  const SANCTIONED_LIST = {
    name: 'OFAC Demo List',
    description: 'Simulated sanctioned indices for demonstration',
    values: [66, 77, 88, 99, 111, 122, 133, 144, 155, 166]
  };

  // State
  let currentStep = 1;
  let witnessData = null;
  let selectedIndex = null;  // The sanctioned index to check against
  let expectedCount = 1;  // Number of encrypted values in witness
  let isProcessing = false;
  let processingStep = '';
  let processingMessage = '';

  // Result state
  let jobId = null;
  let jobStatus = null;
  let txSignature = null;
  let verificationResult = null;
  let error = null;

  // Explorer URL for Solana devnet
  const EXPLORER_URL = 'https://explorer.solana.com';

  // Wallet modal
  let showWalletModal = false;

  // Price configuration
  let priceLamports = 5000000;
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
          operation: 'countif',  // Note: no underscore for price-recommendation endpoint
          operation_value: selectedIndex || 0,
          expected_count: expectedCount,
          required_provers: requiredProvers
        })
      });

      if (response.ok) {
        priceRecommendation = await response.json();
        if (!priceLamports || priceLamports < priceRecommendation.recommended_price_lamports) {
          priceLamports = priceRecommendation.recommended_price_lamports;
        }
      }
    } catch (err) {
      console.error('Failed to get price recommendation:', err);
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

  // Fetch price when entering step 3
  $: if (currentStep === 3 && !priceRecommendation) {
    fetchPriceRecommendation();
  }

  // Computed price displays
  $: totalCost = (priceLamports / 1000000000).toFixed(5);
  $: platformFee = (priceLamports * 0.01 / 1000000000).toFixed(5);
  $: totalWithFee = ((priceLamports * 1.01) / 1000000000).toFixed(5);

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

  function selectSanctionedIndex(index) {
    selectedIndex = index;
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

    if (selectedIndex === null) {
      toastStore.add('Please select a sanctioned index to verify against', 'error');
      return;
    }

    isProcessing = true;
    error = null;
    jobId = null;
    jobStatus = null;
    txSignature = null;
    verificationResult = null;

    try {
      processingStep = 'creating';
      processingMessage = 'Creating verification job...';

      const result = await createFheJobFromWitness({
        operation: 'count_if',
        operationValue: selectedIndex,
        expectedCount: expectedCount,
        serverKeyBytes: witnessData.serverKeyBytes,
        encryptedData: witnessData.encryptedData,
        wallet: $walletStore,
        apiBaseUrl: API_BASE,
        rpcUrl: RPC_URL,
        priceLamports: priceLamports,
        requiredProvers: requiredProvers,
        consensusThreshold: 2,
        // Rust enum format: {"Equals": value} for serde deserialization
        predicate: {
          Equals: selectedIndex
        },
        onProgress: (progress) => {
          processingStep = progress.step;
          processingMessage = progress.message;
        }
      });

      jobId = result.jobId;
      txSignature = result.signature;
      toastStore.add(`Verification job created: ${jobId}`, 'success');

      jobStatus = 'submitted';
      verificationResult = 'pending';
      currentStep = 5;

      toastStore.add('Job submitted! Track progress on Solana Explorer', 'success');

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
    if (currentStep < 4) {
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
    selectedIndex = null;
    jobId = null;
    jobStatus = null;
    txSignature = null;
    verificationResult = null;
    error = null;
    priceRecommendation = null;
    currentStep = 1;
  }

  function handleCancel() {
    navigateTo('dashboard');
  }

  $: canProceedStep1 = witnessData !== null;
  $: canProceedStep2 = selectedIndex !== null;
  $: canProceedStep3 = priceLamports > 0;
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
            STEP {currentStep} OF 4
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

      <div class="stepper-step {currentStep >= 3 ? 'active' : ''} {currentStep > 3 ? 'completed' : ''}">
        <div class="step-circle text-mono">
          {currentStep > 3 ? '✓' : '3'}
        </div>
        <div class="step-label text-mono text-xs">PRICE</div>
      </div>

      <div class="stepper-step {currentStep >= 4 ? 'active' : ''}">
        <div class="step-circle text-mono">4</div>
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
            STEP_1: UPLOAD_TRANSACTION_HISTORY
          </h2>

          <div class="info-box mb-6">
            <div class="text-sm">
              Upload your encrypted transaction history (witness.bin).
              We'll verify it has <strong>NOT</strong> interacted with sanctioned indices.
            </div>
          </div>

          <div class="info-box-success mb-6">
            <div class="text-mono text-sm">
              [i] PRIVACY_PRESERVED<br/>
              Using FHE, we count matches against sanctioned indices - if result is <span class="text-success">0</span>, you're verified innocent.
              Only the count is revealed, not your actual transactions.
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

            <!-- Expected Count Input -->
            <div class="count-input-section mt-6">
              <label class="text-mono text-sm mb-2 block">
                VALUE_COUNT: How many values did you encrypt?
              </label>
              <input
                type="number"
                min="1"
                max="1000"
                bind:value={expectedCount}
                class="input text-mono"
                style="max-width: 200px;"
              />
              <div class="text-xs text-muted mt-2">
                If you used <code>fhe-cli encrypt --values 10,20,30,40,50</code>, enter <strong>5</strong>
              </div>
              <div class="warning-box mt-3">
                <span class="text-warning">[!]</span>
                <span class="text-xs">This must match the actual count. Incorrect values will cause computation to fail.</span>
              </div>
            </div>
          {/if}
        </div>

      {:else if currentStep === 2}
        <!-- Step 2: Select Sanctioned Index -->
        <div class="wizard-card tui-box fade-in">
          <h2 class="text-mono text-uppercase mb-6">
            STEP_2: SELECT_SANCTIONED_INDEX
          </h2>

          <div class="info-box mb-6">
            <div class="text-sm">
              Select which sanctioned index to verify against your transaction history.
              The FHE computation will count if this index appears in your data.
            </div>
          </div>

          <!-- Sanctioned List Grid -->
          <div class="sanctioned-grid mb-6">
            <div class="text-mono text-sm mb-3 text-muted">SANCTIONED_LIST ({SANCTIONED_LIST.name}):</div>
            <div class="index-buttons">
              {#each SANCTIONED_LIST.values as index}
                <button
                  class="index-btn text-mono"
                  class:selected={selectedIndex === index}
                  on:click={() => selectSanctionedIndex(index)}
                >
                  {index}
                </button>
              {/each}
            </div>
          </div>

          {#if selectedIndex !== null}
            <div class="selection-info tui-box-success">
              <div class="text-mono text-sm">
                <span class="text-success">[SELECTED]</span> Index <span class="text-cyan">{selectedIndex}</span><br/>
                <span class="text-muted">Will verify: Does your history contain interactions with index {selectedIndex}?</span>
              </div>
            </div>
          {/if}

          <div class="demo-note mt-6">
            <div class="text-mono text-xs text-muted">
              [!] DEMO_MODE: Currently using simulated indices. In production, these will be real sanctioned wallet addresses.
            </div>
          </div>
        </div>

      {:else if currentStep === 3}
        <!-- Step 3: Configure Price -->
        <div class="wizard-card tui-box fade-in">
          <h2 class="text-mono text-uppercase mb-6">
            STEP_3: CONFIGURE_PAYMENT
          </h2>

          <div class="info-box mb-6">
            <div class="text-sm">
              Set the price you're willing to pay provers for this FHE computation.
              Higher prices attract faster processing.
            </div>
          </div>

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
                Loading price...
              </div>
            {/if}
          </div>

          <!-- Cost Summary -->
          <div class="cost-breakdown tui-box-success">
            <div class="text-mono text-uppercase mb-3">VERIFICATION_COST:</div>
            <div class="cost-lines text-mono text-sm">
              <div class="cost-line">
                <span class="text-muted">Operation:</span>
                <span class="text-success">COUNT_IF</span>
              </div>
              <div class="cost-line">
                <span class="text-muted">Check Index:</span>
                <span class="text-cyan">{selectedIndex}</span>
              </div>
              <div class="cost-line">
                <span class="text-muted">Provers ({requiredProvers}):</span>
                <span class="text-success">{totalCost} SOL</span>
              </div>
              <div class="cost-line">
                <span class="text-muted">Platform Fee (1%):</span>
                <span class="text-success">{platformFee} SOL</span>
              </div>
              <div class="divider-cost text-muted">───────────────────────────────</div>
              <div class="cost-line total">
                <span>Total:</span>
                <span class="text-success total-amount">~{totalWithFee} SOL</span>
              </div>
            </div>
          </div>
        </div>

      {:else if currentStep === 4}
        <!-- Step 4: Verify -->
        <div class="wizard-card tui-box fade-in">
          <h2 class="text-mono text-uppercase mb-6 text-center">
            STEP_4: RUN_VERIFICATION
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
                Click "VERIFY_INNOCENCE" to submit the FHE verification job
              </div>
            {/if}

            {#if error}
              <div class="error-box mt-4">
                <span class="text-error">[ERROR]</span> {error}
              </div>
            {/if}

            <div class="divider-section"></div>

            <div class="tx-details text-mono text-sm">
              <div class="text-muted mb-2">VERIFICATION_SUMMARY:</div>
              <div class="tx-line">• Sanctioned Index: <span class="text-cyan">{selectedIndex}</span></div>
              <div class="tx-line">• Operation: count_if (check for matches)</div>
              <div class="tx-line">• Provers: {requiredProvers}</div>
              <div class="tx-line">• Cost: ~{totalWithFee} SOL</div>
            </div>
          </div>
        </div>

      {:else if currentStep === 5}
        <!-- Result Section -->
        <div class="wizard-card tui-box fade-in">
          <div class="result-header mb-6 text-center">
            <div class="text-4xl text-mono text-success mb-4">[OK]</div>
            <h2 class="text-mono text-uppercase text-success">
              VERIFICATION_SUBMITTED
            </h2>
          </div>

          <div class="result-details tui-box mt-6">
            <div class="detail-row">
              <span class="text-mono text-muted">Job ID:</span>
              <span class="text-mono text-cyan">{jobId}</span>
            </div>
            <div class="detail-row">
              <span class="text-mono text-muted">Checked Index:</span>
              <span class="text-mono text-cyan">{selectedIndex}</span>
            </div>
            <div class="detail-row">
              <span class="text-mono text-muted">Sanctioned List:</span>
              <span class="text-mono text-cyan">{SANCTIONED_LIST.name}</span>
            </div>
            <div class="detail-row">
              <span class="text-mono text-muted">Status:</span>
              <span class="text-mono text-success">SUBMITTED_ON_CHAIN</span>
            </div>
          </div>

          <!-- Explorer Link -->
          <div class="explorer-link-box tui-box mt-6">
            <h4 class="text-mono mb-3">TRACK_PROGRESS:</h4>
            <p class="text-mono text-sm text-muted mb-4">
              Your FHE verification job is now on-chain. Provers will compute the result.
              Track the transaction in real-time:
            </p>
            <a
              href="{EXPLORER_URL}/tx/{txSignature}?cluster=devnet"
              target="_blank"
              rel="noopener noreferrer"
              class="explorer-link text-mono"
            >
              [VIEW_ON_SOLANA_EXPLORER]
            </a>
            <div class="tx-signature text-mono text-xs mt-4">
              TX: {txSignature ? txSignature.slice(0, 20) + '...' + txSignature.slice(-20) : 'N/A'}
            </div>
          </div>

          <div class="info-box-success mt-6">
            <div class="text-mono text-sm">
              [i] INTERPRETING_RESULTS<br/>
              • If count = <span class="text-success">0</span> → Your history has NO interactions with index {selectedIndex} (INNOCENT)<br/>
              • If count > 0 → Your history contains interactions with this sanctioned index
            </div>
          </div>
        </div>
      {/if}

      <!-- Navigation Buttons -->
      <div class="wizard-nav">
        <button
          class="btn btn-ghost"
          on:click={currentStep === 1 ? handleCancel : (currentStep === 5 ? reset : prevStep)}
        >
          [{currentStep === 1 ? 'CANCEL' : currentStep === 5 ? 'VERIFY_ANOTHER' : '◀ BACK'}]
        </button>

        {#if currentStep < 4}
          <button
            class="btn btn-primary"
            on:click={nextStep}
            disabled={(currentStep === 1 && !canProceedStep1) || (currentStep === 2 && !canProceedStep2) || (currentStep === 3 && !canProceedStep3)}
          >
            [NEXT: {currentStep === 1 ? 'SELECT_INDEX' : currentStep === 2 ? 'SET_PRICE' : 'VERIFY'} ▶]
          </button>
        {:else if currentStep === 4}
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
  }

  .warning-box {
    padding: var(--space-2) var(--space-3);
    background: rgba(245, 158, 11, 0.1);
    border: 1px dashed rgba(245, 158, 11, 0.5);
    border-radius: var(--radius-md);
  }

  .count-input-section {
    padding: var(--space-4);
    background: rgba(0, 0, 0, 0.2);
    border: 1px solid var(--zyber-border-muted);
    border-radius: var(--radius-md);
  }

  /* Sanctioned Grid */
  .sanctioned-grid {
    padding: var(--space-4);
    background: rgba(239, 68, 68, 0.05);
    border: 1px solid rgba(239, 68, 68, 0.3);
    border-radius: var(--radius-md);
  }

  .index-buttons {
    display: grid;
    grid-template-columns: repeat(5, 1fr);
    gap: var(--space-3);
  }

  .index-btn {
    padding: var(--space-3) var(--space-4);
    background: rgba(0, 0, 0, 0.4);
    border: 2px solid var(--zyber-border-muted);
    border-radius: var(--radius-md);
    color: var(--zyber-error);
    font-size: var(--text-lg);
    font-weight: 600;
    cursor: pointer;
    transition: all var(--transition-fast);
  }

  .index-btn:hover {
    border-color: var(--zyber-error);
    background: rgba(239, 68, 68, 0.1);
    transform: translateY(-2px);
  }

  .index-btn.selected {
    border-color: var(--zyber-success);
    background: rgba(16, 185, 129, 0.2);
    color: var(--zyber-success);
    box-shadow: 0 0 15px rgba(16, 185, 129, 0.3);
  }

  .selection-info {
    padding: var(--space-4);
  }

  .demo-note {
    padding: var(--space-3);
    background: rgba(245, 158, 11, 0.1);
    border: 1px dashed rgba(245, 158, 11, 0.5);
    border-radius: var(--radius-md);
  }

  /* Config Section */
  .config-section {
    margin-bottom: var(--space-6);
  }

  .config-section label {
    display: block;
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
  .result-header {
    text-align: center;
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

  /* Explorer Link */
  .explorer-link-box {
    padding: var(--space-6);
    text-align: center;
  }

  .explorer-link {
    display: inline-block;
    padding: var(--space-3) var(--space-6);
    background: linear-gradient(135deg, var(--zyber-success), var(--zyber-cyber-cyan));
    border-radius: var(--radius-md);
    color: white;
    text-decoration: none;
    font-weight: 600;
    transition: all var(--transition-fast);
  }

  .explorer-link:hover {
    box-shadow: 0 0 20px rgba(16, 185, 129, 0.4);
    transform: translateY(-2px);
  }

  .tx-signature {
    color: var(--zyber-text-muted);
    word-break: break-all;
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

    .index-buttons {
      grid-template-columns: repeat(3, 1fr);
    }

    .cost-line {
      flex-direction: column;
      align-items: flex-start;
      gap: var(--space-1);
    }
  }

  @media (max-width: 480px) {
    .index-buttons {
      grid-template-columns: repeat(2, 1fr);
    }
  }
</style>
