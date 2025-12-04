<script>
  import { navigateTo } from '../stores/router';
  import { walletStore } from '../stores/wallet';
  import PaymentMethodSelector from '../components/PaymentMethodSelector.svelte';
  import PriceSlider from '../components/PriceSlider.svelte';
  import Loading from '../components/Loading.svelte';
  import WalletConnect from '../components/WalletConnect.svelte';
  import { ensureTokenAccount, WZEC_MINT } from '../utils/tokenAccountManager';
  import { createSolanaRpc } from '@solana/kit';
  import { toastStore } from '../stores/toast';
  import { createFheJobFromWitness } from '../utils/job_creator';

  // Wallet connection state - use store directly for reactivity
  let showWalletModal = false;

  // Reactive wallet state
  $: isWalletConnected = $walletStore?.connected === true;
  $: walletAddress = $walletStore?.addresses?.solana || '';

  // Handle wallet connection event
  function handleWalletConnected(event) {
    console.log('Wallet connected event:', event.detail);
    showWalletModal = false;
    // Force reactivity update
    isWalletConnected = true;
    walletAddress = event.detail.address || $walletStore?.addresses?.solana || '';
  }

  // API URLs - use relative path for nginx proxy, fallback for local dev
  const API_BASE = import.meta.env.VITE_API_URL || '';
  const RPC_URL = import.meta.env.VITE_SOLANA_RPC_URL || 'http://localhost:8899';

  let currentStep = 1;
  let isProcessing = false;
  let processingMessage = '';
  let jobData = {
    encryptedData: null,
    serverKey: null,
    witness: null,
    operation: 'Multiply',
    operationValue: 5,
    consensus: '2-of-3',
    priceLamports: 2000000,
    requiredProvers: 3,
    consensusThreshold: 2,
    paymentMethod: 'sol' // 'sol' | 'wzec'
  };

  // File uploads - simplified to witness.bin only
  let witnessFile = null;
  let witnessParseError = null;
  let isDragging = false;
  let serverKeyBytes = null;  // Raw bytes for pre-upload

  // Dynamic pricing estimation
  let estimatedCost = null;
  let isEstimating = false;

  // Price recommendation for slider
  let priceRecommendation = null;
  let isFetchingPrice = false;

  // Form validation errors
  let operationValueError = '';

  // Pricing (reactive based on estimated cost)
  $: totalCost = (jobData.priceLamports / 1000000000).toFixed(5);
  $: platformFee = (jobData.priceLamports * 0.01 / 1000000000).toFixed(5);
  $: totalWithFee = ((jobData.priceLamports * 1.01) / 1000000000).toFixed(5);

  // Track previous values to prevent infinite loops
  let lastEstimateKey = '';
  let lastPriceKey = '';

  // Fetch estimates only when entering step 2 or when relevant params change
  function maybeEstimateCost() {
    const key = `${jobData.operation}-${jobData.operationValue}-${jobData.requiredProvers}`;
    if (key === lastEstimateKey || isEstimating) return;
    lastEstimateKey = key;
    estimateCost();
  }

  function maybeFetchPriceRecommendation() {
    const key = `${jobData.operation}-${jobData.requiredProvers}`;
    if (key === lastPriceKey || isFetchingPrice) return;
    lastPriceKey = key;
    fetchPriceRecommendation();
  }

  // Fetch cost estimation from backend
  async function estimateCost() {
    isEstimating = true;
    try {
      const response = await fetch(`${API_BASE}/api/estimate-cost`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          operation: jobData.operation.toLowerCase(),
          operation_value: jobData.operationValue,
          expected_count: 100,
          bins: 5,
          required_provers: jobData.requiredProvers
        })
      });

      if (!response.ok) {
        throw new Error(`Failed to estimate cost: ${response.statusText}`);
      }

      estimatedCost = await response.json();
      jobData.priceLamports = estimatedCost.total_min_payment_lamports;
      console.log('Cost estimated:', estimatedCost);
    } catch (error) {
      console.error('Failed to estimate cost:', error);
      jobData.priceLamports = jobData.requiredProvers === 3 ? 3000000 : 5000000;
    } finally {
      isEstimating = false;
    }
  }

  // Fetch price recommendation for slider
  async function fetchPriceRecommendation() {
    isFetchingPrice = true;
    try {
      const response = await fetch(`${API_BASE}/api/price-recommendation`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          operation: jobData.operation.toLowerCase(),
          operation_value: jobData.operationValue,
          expected_count: 10,
          bins: 5,
          required_provers: jobData.requiredProvers
        })
      });

      if (!response.ok) {
        throw new Error(`Failed to get price recommendation: ${response.statusText}`);
      }

      priceRecommendation = await response.json();

      if (!jobData.priceLamports || jobData.priceLamports < priceRecommendation.recommended_price_lamports) {
        jobData.priceLamports = priceRecommendation.recommended_price_lamports;
      }

      console.log('Price recommendation:', priceRecommendation);
    } catch (error) {
      console.error('Failed to get price recommendation:', error);
      priceRecommendation = {
        min_price_lamports: 3000000,
        recommended_price_lamports: 5400000,
        max_suggested_lamports: 10800000,
        slider_step: 100000
      };
    } finally {
      isFetchingPrice = false;
    }
  }

  // Trigger estimates when entering step 2
  $: if (currentStep === 2) {
    maybeEstimateCost();
    maybeFetchPriceRecommendation();
  }

  function handlePriceChange(event) {
    jobData.priceLamports = event.detail.price;
  }

  function handleDragOver(e) {
    e.preventDefault();
    isDragging = true;
  }

  function handleDragLeave() {
    isDragging = false;
  }

  function handleDrop(e) {
    e.preventDefault();
    isDragging = false;

    const files = e.dataTransfer.files;
    if (files.length > 0) {
      handleWitnessUpload(files[0]);
    }
  }

  function handleFileInput(e) {
    const files = e.target.files;
    if (files.length > 0) {
      handleWitnessUpload(files[0]);
    }
  }

  /**
   * Convert Uint8Array to base64 using chunks (memory efficient)
   */
  function arrayBufferToBase64(bytes) {
    const chunkSize = 0x8000; // 32KB chunks
    let result = '';
    for (let i = 0; i < bytes.length; i += chunkSize) {
      const chunk = bytes.subarray(i, i + chunkSize);
      result += String.fromCharCode.apply(null, chunk);
    }
    return btoa(result);
  }

  // Processing state for witness parsing
  let isParsingWitness = false;

  /**
   * Parse witness.bin format:
   * [server_key_len (8 bytes LE)][server_key bytes][encrypted_data bytes]
   */
  async function handleWitnessUpload(file) {
    witnessFile = file;
    witnessParseError = null;
    jobData.witness = file.name;
    isParsingWitness = true;

    try {
      // Use setTimeout to let UI update before heavy processing
      await new Promise(r => setTimeout(r, 50));

      const buffer = await file.arrayBuffer();
      const view = new DataView(buffer);

      // Read server_key length (8 bytes, little-endian u64)
      const serverKeyLen = Number(view.getBigUint64(0, true));

      if (serverKeyLen <= 0 || serverKeyLen > buffer.byteLength - 8) {
        throw new Error(`Invalid server_key length: ${serverKeyLen}`);
      }

      // Extract server_key bytes - keep raw for pre-upload
      const skBytes = new Uint8Array(buffer, 8, serverKeyLen);
      // Copy to avoid issues with buffer views
      serverKeyBytes = new Uint8Array(skBytes);

      // Extract encrypted_data bytes (rest of the file)
      const encryptedDataStart = 8 + serverKeyLen;
      const encryptedDataBytes = new Uint8Array(buffer, encryptedDataStart);

      if (encryptedDataBytes.length === 0) {
        throw new Error('No encrypted data found in witness');
      }

      // Only encode encrypted_data to base64 (small, ~KB)
      // Server key will be uploaded separately via pre-upload
      jobData.serverKey = null;  // No longer used - using serverKeyBytes instead

      // Small delay to keep UI responsive
      await new Promise(r => setTimeout(r, 10));

      jobData.encryptedData = arrayBufferToBase64(encryptedDataBytes);

      console.log(`Parsed witness: server_key=${(serverKeyLen / 1024 / 1024).toFixed(1)}MB (raw bytes), encrypted_data=${(encryptedDataBytes.length / 1024).toFixed(1)}KB`);

    } catch (error) {
      console.error('Failed to parse witness file:', error);
      witnessParseError = error.message || 'Invalid witness file format';
      serverKeyBytes = null;
      jobData.serverKey = null;
      jobData.encryptedData = null;
    } finally {
      isParsingWitness = false;
    }
  }

  function validateOperationValue() {
    const value = jobData.operationValue;

    if (!value && value !== 0) {
      operationValueError = 'Operation value is required';
      return false;
    }

    if (value < 1 || value > 255) {
      operationValueError = 'Value must be between 1 and 255';
      return false;
    }

    operationValueError = '';
    return true;
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

  function handleCancel() {
    navigateTo('dashboard');
  }

  // Track job result for showing explorer link
  let createdJobId = null;
  let txSignature = null;

  async function handleSubmit() {
    if (!$walletStore || !$walletStore.publicKey) {
      toastStore.add('Please connect your wallet first', 'error');
      return;
    }

    if (!witnessFile || !serverKeyBytes || !jobData.encryptedData) {
      toastStore.add('Please upload a valid witness.bin file', 'error');
      return;
    }

    if (witnessParseError) {
      toastStore.add(`Invalid witness file: ${witnessParseError}`, 'error');
      return;
    }

    isProcessing = true;
    createdJobId = null;
    txSignature = null;

    try {
      // If paying with wZEC, ensure token account exists
      if (jobData.paymentMethod === 'wzec') {
        processingMessage = 'Checking wZEC token account...';

        const tokenResult = await ensureTokenAccount(
          RPC_URL,
          $walletStore,
          WZEC_MINT,
          (progress) => {
            processingMessage = progress.message;
            console.log(`Token account progress: ${progress.step} - ${progress.message}`);

            if (progress.step === 'creating') {
              toastStore.add('Preparing wZEC token account...', 'info');
            } else if (progress.step === 'ready') {
              toastStore.add('wZEC token account ready!', 'success');
            }
          }
        );

        if (tokenResult.needsCreation) {
          console.log('Token account needs creation, instruction prepared');
        }
      }

      // Use createFheJobFromWitness which handles:
      // 1. Pre-upload of server key (streaming with progress)
      // 2. Signature generation
      // 3. Transaction signing and sending
      // 4. Backend confirmation
      const result = await createFheJobFromWitness({
        operation: jobData.operation,
        operationValue: jobData.operationValue,
        expectedCount: 5,  // Default for custom jobs
        serverKeyBytes: serverKeyBytes,  // Raw bytes for pre-upload
        encryptedData: jobData.encryptedData,
        wallet: $walletStore,
        apiBaseUrl: API_BASE,
        rpcUrl: RPC_URL,
        priceLamports: jobData.priceLamports,
        requiredProvers: jobData.requiredProvers,
        consensusThreshold: jobData.consensusThreshold,
        paymentMethod: jobData.paymentMethod.toUpperCase(),
        onProgress: (progress) => {
          processingMessage = progress.message;
          // Show upload progress details
          if (progress.uploadDetails) {
            const { loadedMB, totalMB, speedKBps, etaFormatted } = progress.uploadDetails;
            processingMessage = `Uploading: ${loadedMB}/${totalMB} MB @ ${speedKBps} KB/s (ETA: ${etaFormatted})`;
          }
        }
      });

      createdJobId = result.jobId;
      txSignature = result.signature;

      // Success - show explorer link
      toastStore.add(`Job created! ID: ${createdJobId}`, 'success');

      // Move to step 4 (success) instead of navigating away
      currentStep = 4;

    } catch (error) {
      console.error('Error during job creation:', error);
      toastStore.add(error.message || 'Failed to create job', 'error');
    } finally {
      isProcessing = false;
      processingMessage = '';
    }
  }

  // Get explorer URL for transaction
  function getExplorerUrl(signature) {
    return `https://explorer.solana.com/tx/${signature}?cluster=devnet`;
  }

  function handlePaymentMethodChange(event) {
    jobData.paymentMethod = event.detail.payment_method;
  }

  $: canProceedStep1 = jobData.witness && serverKeyBytes && jobData.encryptedData && !witnessParseError;
  $: canProceedStep2 = jobData.operation && jobData.operationValue;
</script>

<div class="create-job">
  <!-- Header -->
  <header class="wizard-header">
    <div class="container">
      <div class="header-content">
        <h1 class="text-mono text-uppercase">CREATE_FHE_JOB</h1>
        <div class="header-right">
          <!-- Wallet Status -->
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
        <div class="step-label text-mono text-xs">PREPARE</div>
      </div>

      <div class="stepper-step {currentStep >= 2 ? 'active' : ''} {currentStep > 2 ? 'completed' : ''}">
        <div class="step-circle text-mono">
          {currentStep > 2 ? '✓' : '2'}
        </div>
        <div class="step-label text-mono text-xs">CONFIGURE</div>
      </div>

      <div class="stepper-step {currentStep >= 3 ? 'active' : ''} {currentStep > 3 ? 'completed' : ''}">
        <div class="step-circle text-mono">
          {currentStep > 3 ? '✓' : '3'}
        </div>
        <div class="step-label text-mono text-xs">CONFIRM</div>
      </div>

      <div class="stepper-step {currentStep >= 4 ? 'active' : ''}">
        <div class="step-circle text-mono">4</div>
        <div class="step-label text-mono text-xs">SUBMIT</div>
      </div>
    </div>
  </div>

  <!-- Main Content -->
  <main class="wizard-main">
    <div class="container">
      {#if currentStep === 1}
        <!-- Step 1: Prepare Data -->
        <div class="wizard-card tui-box fade-in">
          <h2 class="text-mono text-uppercase mb-6">
            STEP_1: ENCRYPT_YOUR_DATA_LOCALLY
          </h2>

          <div class="info-box mb-6">
            <div class="text-sm">
              For privacy, encryption happens on <strong>YOUR</strong> computer.
              Download the <span class="text-cyan">fhe-cli</span> tool and run:
            </div>
          </div>

          <div class="code-block mb-6">
            <div class="text-mono text-cyan">
              $ fhe-cli encrypt -p ./my-job -v 42
            </div>
            <button class="btn-copy text-mono text-sm" on:click={() => navigator.clipboard.writeText('fhe-cli encrypt -p ./my-job -v 42')}>
              [COPY]
            </button>
          </div>

          <!-- Simplified File Upload Section - Only witness.bin -->
          <div class="upload-section-unified mb-6">
            <div class="upload-help-header text-mono text-sm mb-4">
              <span class="text-cyan">[REQUIRED]</span> Upload the <strong>witness.bin</strong> file generated by fhe-cli:
            </div>

            <!-- Single File: witness.bin -->
            <div class="file-input-wrapper mb-4" data-testid="witness-upload-wrapper">
              <label class="file-label" for="witness-input">
                <span class="label-header">
                  <span class="label-title text-mono">WITNESS.BIN</span>
                  <span class="label-format text-xs text-muted">Contains server key + encrypted data</span>
                </span>
                <input
                  type="file"
                  id="witness-input"
                  accept=".bin"
                  on:change={handleFileInput}
                  data-testid="witness-input"
                  class="file-input"
                  aria-label="Upload witness.bin file"
                />
                <span
                  class="file-status text-mono text-sm"
                  class:file-selected={jobData.witness && !witnessParseError && !isParsingWitness}
                  class:file-pending={!jobData.witness}
                  class:file-processing={isParsingWitness}
                  class:file-error={witnessParseError}
                  data-testid="witness-status"
                  aria-live="polite"
                >
                  {#if isParsingWitness}
                    <span class="text-cyan">[...]</span> Processing witness file...
                  {:else if witnessParseError}
                    <span class="text-error">[!]</span> {witnessParseError}
                  {:else if jobData.witness && jobData.serverKey}
                    <span class="text-success">[OK]</span> {jobData.witness}
                    <span class="text-muted text-xs">({witnessFile?.size ? (witnessFile.size / (1024 * 1024)).toFixed(1) : '0'} MB)</span>
                  {:else}
                    <span class="text-warning">[...]</span> Click or drop witness.bin here
                  {/if}
                </span>
              </label>
            </div>

            <!-- Upload Status Indicator -->
            <div class="upload-status-container" data-testid="upload-status">
              {#if isParsingWitness}
                <div class="upload-processing-indicator text-mono text-sm text-cyan">
                  <span>[~]</span> Processing ~50MB file, please wait...
                </div>
              {:else if witnessParseError}
                <div class="upload-error-indicator text-mono text-sm text-error">
                  <span>[X]</span> Invalid witness file. Generate it with: fhe-cli encrypt -p ./output
                </div>
              {:else if jobData.witness && jobData.serverKey && jobData.encryptedData}
                <div class="upload-complete-indicator text-mono text-sm text-success">
                  <span>[OK]</span> Witness parsed successfully. Ready to proceed.
                </div>
              {:else}
                <div class="upload-pending-indicator text-mono text-sm text-muted">
                  <span>[...]</span> Upload witness.bin to continue
                </div>
              {/if}
            </div>
          </div>

          <div class="info-box-cyan">
            <div class="text-mono text-sm">
              [i] WHY_LOCAL_ENCRYPTION?<br/>
              This ensures your data is never exposed to our servers.<br/>
              Only YOU can decrypt the final result with your client_key.bin.
            </div>
          </div>
        </div>

      {:else if currentStep === 2}
        <!-- Step 2: Configure -->
        <div class="wizard-card tui-box fade-in scrollable">
          <h2 class="text-mono text-uppercase mb-6">
            STEP_2: CONFIGURE_COMPUTATION
          </h2>

          <!-- Operation Selection -->
          <div class="config-section mb-6">
            <label class="text-mono mb-3">OPERATION_TYPE:</label>
            <div class="radio-group">
              <label class="radio-option">
                <input type="radio" bind:group={jobData.operation} value="Add" />
                <span class="radio-label text-mono">
                  <span class="radio-check">○</span>
                  <span>ADD</span>
                  <span class="text-muted text-sm">Combine two encrypted numbers</span>
                </span>
              </label>

              <label class="radio-option">
                <input type="radio" bind:group={jobData.operation} value="Multiply" />
                <span class="radio-label text-mono">
                  <span class="radio-check">●</span>
                  <span>MULTIPLY</span>
                  <span class="text-muted text-sm">Multiply encrypted numbers</span>
                </span>
              </label>

              <label class="radio-option">
                <input type="radio" bind:group={jobData.operation} value="Subtract" />
                <span class="radio-label text-mono">
                  <span class="radio-check">○</span>
                  <span>SUBTRACT</span>
                  <span class="text-muted text-sm">Subtract encrypted numbers</span>
                </span>
              </label>
            </div>
          </div>

          <!-- Operation Value -->
          <div class="config-section mb-6">
            <label class="text-mono mb-2" for="operation-value-input">OPERATION_VALUE:</label>
            <input
              type="number"
              id="operation-value-input"
              bind:value={jobData.operationValue}
              on:blur={validateOperationValue}
              on:input={() => { if (operationValueError) validateOperationValue(); }}
              class="input input-code"
              class:input-error={operationValueError}
              min="1"
              max="255"
              data-testid="operation-value-input"
              aria-invalid={!!operationValueError}
              aria-describedby={operationValueError ? 'operation-value-error' : null}
            />
            {#if operationValueError}
              <div class="error-message" id="operation-value-error" role="alert" data-testid="operation-value-error">
                <span class="text-error">[!]</span> {operationValueError}
              </div>
            {:else}
              <div class="text-xs text-muted mt-2">
                Estimated time: ~2-3 seconds
              </div>
            {/if}
          </div>

          <div class="divider-section"></div>

          <!-- Payment Method -->
          <PaymentMethodSelector
            selected={jobData.paymentMethod}
            on:change={handlePaymentMethodChange}
          />

          <div class="divider-section"></div>

          <!-- Consensus -->
          <div class="config-section mb-6">
            <label class="text-mono mb-3">PROVER_CONSENSUS:</label>
            <div class="radio-group">
              <label class="radio-option">
                <input type="radio" bind:group={jobData.consensus} value="2-of-3"
                  on:change={() => { jobData.requiredProvers = 3; jobData.consensusThreshold = 2; }} />
                <span class="radio-label text-mono">
                  <span class="radio-check">●</span>
                  <span>2_OF_3_PROVERS</span>
                  <span class="badge badge-success">RECOMMENDED</span>
                  <span class="text-cyan">[0.002_SOL]</span>
                </span>
              </label>

              <label class="radio-option">
                <input type="radio" bind:group={jobData.consensus} value="3-of-5"
                  on:change={() => { jobData.requiredProvers = 5; jobData.consensusThreshold = 3; }} />
                <span class="radio-label text-mono">
                  <span class="radio-check">○</span>
                  <span>3_OF_5_PROVERS</span>
                  <span class="text-muted text-sm">More secure</span>
                  <span class="text-cyan">[0.004_SOL]</span>
                </span>
              </label>
            </div>
            <div class="text-xs text-muted mt-3">
              Higher consensus = more reliable results
            </div>
          </div>

          <div class="divider-section"></div>

          <!-- Price Selection Slider -->
          {#if priceRecommendation}
            <div class="config-section mb-6">
              <PriceSlider
                minPrice={priceRecommendation.min_price_lamports}
                recommendedPrice={priceRecommendation.recommended_price_lamports}
                maxPrice={priceRecommendation.max_suggested_lamports}
                currentPrice={jobData.priceLamports}
                step={priceRecommendation.slider_step}
                isLoading={isFetchingPrice}
                on:change={handlePriceChange}
              />
            </div>

            <div class="divider-section"></div>
          {/if}

          <!-- Cost Breakdown - Enhanced Visibility -->
          <div class="cost-breakdown tui-box-cyan" data-testid="cost-breakdown">
            <div class="text-mono text-uppercase mb-3">
              {#if isEstimating}
                <span class="text-cyan">[ESTIMATING_COST...]</span>
              {:else}
                ESTIMATED_COST:
              {/if}
            </div>
            <div class="cost-lines text-mono text-sm">
              {#if estimatedCost}
                <div class="cost-line">
                  <span class="text-muted">Operation:</span>
                  <span class="text-cyan">{estimatedCost.operation.toUpperCase()}</span>
                  <span class="badge badge-info" data-testid="complexity-tier">TIER_{estimatedCost.complexity_tier}</span>
                </div>
                <div class="cost-line">
                  <span class="text-muted">Complexity:</span>
                  <span class="complexity-bar">
                    {#each Array(10) as _, i}
                      <span class:filled={i < estimatedCost.complexity_tier}>█</span>
                    {/each}
                  </span>
                </div>
                <div class="cost-line">
                  <span class="text-muted">Base Cost/Prover:</span>
                  <span class="text-cyan" data-testid="base-cost">{estimatedCost.min_payment_sol.toFixed(6)}_SOL</span>
                </div>
                <div class="cost-line">
                  <span class="text-muted">Provers ({jobData.requiredProvers}):</span>
                  <span class="text-cyan" data-testid="provers-cost">{totalCost}_SOL</span>
                </div>
                <div class="cost-line">
                  <span class="text-muted">Max Duration:</span>
                  <span class="text-cyan">{estimatedCost.timeout_seconds}s</span>
                </div>
              {:else}
                <div class="cost-line">
                  <span class="text-muted">Provers ({jobData.requiredProvers}):</span>
                  <span class="text-cyan" data-testid="provers-cost">{totalCost}_SOL</span>
                </div>
              {/if}
              <div class="cost-line">
                <span class="text-muted">Platform Fee (1%):</span>
                <span class="text-cyan" data-testid="platform-fee">{platformFee}_SOL</span>
              </div>
              <div class="divider-cost text-muted">───────────────────────────────</div>
              <div class="cost-line total">
                <span>Total:</span>
                <span class="text-cyan total-amount" data-testid="total-cost">{totalWithFee}_SOL</span>
                <span class="text-muted text-xs" data-testid="total-usd">~${(parseFloat(totalWithFee) * 20).toFixed(2)}</span>
              </div>
            </div>

            {#if estimatedCost}
              <div class="cost-explanation text-xs text-muted mt-4" data-testid="cost-explanation">
                {#if estimatedCost.complexity_tier <= 2}
                  Low complexity operations use basic FHE circuits - fastest and cheapest.
                {:else if estimatedCost.complexity_tier <= 4}
                  Moderate computation requires more complex circuits - standard pricing.
                {:else}
                  Advanced operations involve intensive computation - premium pricing.
                {/if}
              </div>
            {/if}
          </div>
        </div>

      {:else if currentStep === 3}
        <!-- Step 3: Review -->
        <div class="wizard-card tui-box fade-in">
          <h2 class="text-mono text-uppercase mb-6">
            STEP_3: REVIEW_YOUR_JOB
          </h2>

          <!-- Data Files -->
          <div class="review-section mb-4">
            <div class="text-mono text-sm text-muted mb-2">DATA_UPLOADED:</div>
            <div class="review-box">
              <div class="review-line text-mono text-sm">
                <span class="text-success">[OK]</span> {jobData.witness}
                <span class="text-muted">({witnessFile?.size ? (witnessFile.size / (1024 * 1024)).toFixed(1) : '0'} MB)</span>
              </div>
            </div>
          </div>

          <!-- Configuration -->
          <div class="review-section mb-4">
            <div class="text-mono text-sm text-muted mb-2">CONFIGURATION:</div>
            <div class="review-box">
              <div class="review-line text-mono text-sm">
                <span class="text-muted">Operation:</span>
                <span class="text-cyan">{jobData.operation.toUpperCase()}_BY_{jobData.operationValue}</span>
              </div>
              <div class="review-line text-mono text-sm">
                <span class="text-muted">Consensus:</span>
                <span class="text-cyan">{jobData.consensusThreshold}_OF_{jobData.requiredProvers}_PROVERS</span>
              </div>
              <div class="review-line text-mono text-sm">
                <span class="text-muted">Payment Method:</span>
                <span class="text-cyan">{jobData.paymentMethod.toUpperCase()}</span>
                {#if jobData.paymentMethod === 'wzec'}
                  <span class="badge badge-info">SPL_TOKEN</span>
                {/if}
              </div>
              <div class="review-line text-mono text-sm">
                <span class="text-muted">Cost:</span>
                <span class="text-cyan">{totalWithFee}_SOL</span>
              </div>
            </div>
          </div>

          <!-- Warnings -->
          <div class="warning-box mb-4">
            <div class="text-mono text-sm">
              <div class="mb-2">[!] YOUR_WALLET_WILL_BE_CHARGED_IMMEDIATELY</div>
              <div class="text-muted">Provers will be paid automatically upon completion</div>
            </div>
          </div>

          <div class="success-box">
            <div class="text-mono text-sm">
              <div class="mb-2">[ok] YOUR_DATA_REMAINS_ENCRYPTED_THROUGHOUT</div>
              <div class="text-muted">No one can see your plaintext values</div>
            </div>
          </div>
        </div>

      {:else if currentStep === 4}
        <!-- Step 4: Sign Transaction -->
        <div class="wizard-card tui-box fade-in">
          <h2 class="text-mono text-uppercase mb-6 text-center">
            STEP_4: SIGN_TRANSACTION
          </h2>

          <div class="signing-state">
            {#if !isWalletConnected}
              <!-- Wallet Not Connected - Show Connect UI -->
              <div class="wallet-connect-prompt">
                <div class="text-mono text-center mb-4 text-warning">
                  [!] WALLET_NOT_CONNECTED
                </div>
                <div class="text-sm text-muted text-center mb-6">
                  Connect your Solana wallet to sign and submit the transaction
                </div>
                <WalletConnect on:connected={handleWalletConnected} />
              </div>
            {:else if isProcessing}
              <!-- Processing State -->
              <div class="text-center mb-6">
                <Loading size="large" />
              </div>

              <div class="text-mono text-center mb-4 text-cyan">
                {processingMessage || 'Processing...'}<span class="cursor-blink"></span>
              </div>

              <div class="text-sm text-muted text-center mb-6">
                Please wait while we prepare your transaction
              </div>
            {:else}
              <!-- Ready to Sign -->
              <div class="wallet-icon text-center mb-6">
                <div class="text-4xl text-mono text-success">[OK]</div>
              </div>

              <div class="text-mono text-center mb-4 text-success">
                WALLET_CONNECTED
              </div>

              <div class="text-sm text-muted text-center mb-6">
                Click "CREATE_JOB_&_SIGN" below to submit your transaction
              </div>
            {/if}

            <div class="divider-section"></div>

            <div class="tx-details text-mono text-sm">
              <div class="text-muted mb-2">TRANSACTION_DETAILS:</div>
              <div class="tx-line">• Create FHE Job</div>
              <div class="tx-line">• Cost: {totalWithFee}_SOL + network_fee (~0.000005_SOL)</div>
              {#if jobData.paymentMethod === 'wzec'}
                <div class="tx-line text-cyan">• Payment via wZEC token</div>
              {/if}
            </div>
          </div>
        </div>
      {/if}

      <!-- Navigation Buttons -->
      <div class="wizard-nav">
        <button
          class="btn btn-ghost"
          on:click={currentStep === 1 ? handleCancel : prevStep}
          data-testid="wizard-back"
          aria-label={currentStep === 1 ? 'Cancel job creation' : 'Go back to previous step'}
        >
          [{currentStep === 1 ? 'CANCEL' : '◀ BACK'}]
        </button>

        {#if currentStep < 4}
          <button
            class="btn btn-primary"
            on:click={nextStep}
            disabled={(currentStep === 1 && !canProceedStep1) || (currentStep === 2 && !canProceedStep2)}
            data-testid="wizard-next"
            aria-label="Proceed to next step"
            aria-disabled={(currentStep === 1 && !canProceedStep1) || (currentStep === 2 && !canProceedStep2)}
          >
            [NEXT: {currentStep === 1 ? 'CONFIGURE' : currentStep === 2 ? 'REVIEW' : 'SUBMIT'} ▶]
          </button>
        {:else}
          <button
            class="btn btn-primary"
            on:click={handleSubmit}
            data-testid="wizard-submit"
            aria-label="Create job and sign transaction"
          >
            [CREATE_JOB_&_SIGN]
          </button>
        {/if}
      </div>
    </div>
  </main>
</div>

<!-- Wallet Connect Modal -->
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
  .create-job {
    min-height: 100vh;
    padding-bottom: var(--space-8);
  }

  .wizard-header {
    padding: var(--space-6) 0;
    border-bottom: 1px solid var(--zyber-border-muted);
  }

  .header-right {
    display: flex;
    align-items: center;
    gap: var(--space-4);
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

  .wallet-connect-prompt {
    padding: var(--space-4);
  }

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

  .header-content {
    display: flex;
    justify-content: space-between;
    align-items: center;
    flex-wrap: wrap;
    gap: var(--space-4);
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
    border-color: var(--zyber-cyber-cyan);
    background: rgba(6, 182, 212, 0.1);
    color: var(--zyber-cyber-cyan);
    box-shadow: var(--zyber-glow-cyan);
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
    color: var(--zyber-cyber-cyan);
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

  .wizard-card.scrollable {
    max-height: calc(100vh - 280px);
    overflow-y: auto;
  }

  /* Unified Upload Section */
  .upload-section-unified {
    background: rgba(0, 0, 0, 0.2);
    border: 1px solid var(--zyber-border-muted);
    border-radius: var(--radius-md);
    padding: var(--space-6);
  }

  .upload-help-header {
    margin-bottom: var(--space-4);
    padding-bottom: var(--space-3);
    border-bottom: 1px solid var(--zyber-border-muted);
  }

  .file-input-wrapper {
    position: relative;
  }

  .file-label {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    padding: var(--space-4);
    background: rgba(100, 116, 139, 0.05);
    border: 2px solid var(--zyber-border-primary);
    border-radius: var(--radius-md);
    cursor: pointer;
    transition: all var(--transition-base);
  }

  .file-label:hover {
    border-color: var(--zyber-cyber-cyan);
    background: rgba(6, 182, 212, 0.1);
    box-shadow: 0 0 20px rgba(6, 182, 212, 0.2);
  }

  .label-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    flex-wrap: wrap;
    gap: var(--space-2);
  }

  .label-title {
    font-weight: 600;
    color: var(--zyber-text-primary);
  }

  .label-format {
    padding: 2px 8px;
    background: rgba(100, 116, 139, 0.2);
    border-radius: var(--radius-sm);
  }

  .file-input {
    position: absolute;
    opacity: 0;
    pointer-events: none;
  }

  .file-status {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2);
    border-radius: var(--radius-sm);
    background: rgba(0, 0, 0, 0.3);
    min-height: 32px;
  }

  .file-status.file-selected {
    background: rgba(16, 185, 129, 0.1);
    border: 1px solid rgba(16, 185, 129, 0.3);
  }

  .upload-complete-indicator {
    padding: var(--space-3);
    background: rgba(16, 185, 129, 0.1);
    border: 1px solid rgba(16, 185, 129, 0.3);
    border-radius: var(--radius-md);
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .upload-pending-indicator {
    padding: var(--space-3);
    background: rgba(245, 158, 11, 0.05);
    border: 1px solid rgba(245, 158, 11, 0.2);
    border-radius: var(--radius-md);
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .upload-partial-indicator {
    padding: var(--space-3);
    background: rgba(245, 158, 11, 0.1);
    border: 1px solid rgba(245, 158, 11, 0.4);
    border-radius: var(--radius-md);
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .file-status.file-pending {
    background: rgba(245, 158, 11, 0.05);
    border: 1px solid rgba(245, 158, 11, 0.2);
  }

  .file-status.file-error {
    background: rgba(239, 68, 68, 0.1);
    border: 1px solid rgba(239, 68, 68, 0.4);
  }

  .file-status.file-processing {
    background: rgba(6, 182, 212, 0.1);
    border: 1px solid rgba(6, 182, 212, 0.4);
    animation: pulse 1.5s ease-in-out infinite;
  }

  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.6; }
  }

  .upload-error-indicator {
    padding: var(--space-3);
    background: rgba(239, 68, 68, 0.1);
    border: 1px solid rgba(239, 68, 68, 0.4);
    border-radius: var(--radius-md);
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .upload-processing-indicator {
    padding: var(--space-3);
    background: rgba(6, 182, 212, 0.1);
    border: 1px solid rgba(6, 182, 212, 0.4);
    border-radius: var(--radius-md);
    display: flex;
    align-items: center;
    gap: var(--space-2);
    animation: pulse 1.5s ease-in-out infinite;
  }

  /* Form validation styles */
  .input-error {
    border-color: var(--zyber-error);
    box-shadow: 0 0 0 2px rgba(239, 68, 68, 0.2);
  }

  .error-message {
    color: var(--zyber-error);
    font-size: var(--text-sm);
    margin-top: var(--space-2);
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2);
    background: rgba(239, 68, 68, 0.05);
    border: 1px solid rgba(239, 68, 68, 0.2);
    border-radius: var(--radius-sm);
  }

  /* Legacy upload zone styles (mantener por compatibilidad si es necesario) */
  .upload-section {
    margin-bottom: var(--space-4);
  }

  .upload-zone {
    border: 2px dashed var(--zyber-border-primary);
    border-radius: var(--radius-md);
    padding: var(--space-8);
    text-align: center;
    cursor: pointer;
    transition: all var(--transition-base);
    background: rgba(100, 116, 139, 0.05);
  }

  .upload-zone:hover {
    border-color: var(--zyber-cyber-cyan);
    background: rgba(6, 182, 212, 0.1);
  }

  .upload-zone.dragging {
    border-color: var(--zyber-quantum-violet);
    background: rgba(139, 92, 246, 0.1);
    box-shadow: var(--zyber-glow-violet);
  }

  .upload-zone.has-file {
    border-style: solid;
    border-color: var(--zyber-success);
    background: rgba(16, 185, 129, 0.05);
  }

  .file-uploaded {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: var(--space-2);
    font-size: var(--text-sm);
  }

  /* Code Block */
  .code-block {
    background: rgba(0, 0, 0, 0.5);
    border: 1px solid var(--zyber-border-secondary);
    border-radius: var(--radius-md);
    padding: var(--space-4);
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .btn-copy {
    background: none;
    border: 1px solid var(--zyber-border-secondary);
    color: var(--zyber-cyber-cyan);
    padding: var(--space-2) var(--space-3);
    border-radius: var(--radius-sm);
    cursor: pointer;
    transition: all var(--transition-fast);
  }

  .btn-copy:hover {
    background: rgba(6, 182, 212, 0.1);
    box-shadow: var(--zyber-glow-cyan);
  }

  /* Info Boxes */
  .info-box, .info-box-cyan, .warning-box, .success-box {
    padding: var(--space-4);
    border-radius: var(--radius-md);
    border: 1px solid;
  }

  .info-box {
    background: rgba(6, 182, 212, 0.05);
    border-color: var(--zyber-border-secondary);
  }

  .info-box-cyan {
    background: rgba(6, 182, 212, 0.05);
    border-color: var(--zyber-border-secondary);
    border-left-width: 3px;
  }

  .warning-box {
    background: rgba(245, 158, 11, 0.05);
    border-color: rgba(245, 158, 11, 0.3);
    border-left-width: 3px;
  }

  .success-box {
    background: rgba(16, 185, 129, 0.05);
    border-color: rgba(16, 185, 129, 0.3);
    border-left-width: 3px;
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
    background: rgba(6, 182, 212, 0.05);
  }

  .radio-option input[type="radio"] {
    display: none;
  }

  .radio-option input[type="radio"]:checked + .radio-label {
    color: var(--zyber-text-primary);
  }

  .radio-option input[type="radio"]:checked + .radio-label .radio-check {
    color: var(--zyber-cyber-cyan);
  }

  .radio-label {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    flex: 1;
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
    border: 2px solid var(--zyber-border-secondary);
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

  .complexity-bar {
    display: inline-flex;
    gap: 2px;
    font-family: var(--font-mono);
    font-size: 10px;
  }

  .complexity-bar span {
    color: var(--zyber-border-muted);
  }

  .complexity-bar span.filled {
    color: var(--zyber-cyber-cyan);
  }

  .total-amount {
    font-size: var(--text-lg);
    font-weight: 600;
  }

  .cost-explanation {
    padding: var(--space-3);
    background: rgba(6, 182, 212, 0.05);
    border-radius: var(--radius-sm);
    border-left: 2px solid var(--zyber-cyber-cyan);
  }

  /* Review Boxes */
  .review-section {
    margin-bottom: var(--space-4);
  }

  .review-box {
    background: rgba(0, 0, 0, 0.3);
    border: 1px solid var(--zyber-border-muted);
    border-radius: var(--radius-md);
    padding: var(--space-4);
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .review-line {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }

  /* Signing State */
  .signing-state {
    max-width: 500px;
    margin: 0 auto;
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

  /* Navigation */
  .wizard-nav {
    display: flex;
    justify-content: space-between;
    gap: var(--space-4);
    max-width: 800px;
    margin: var(--space-8) auto 0;
  }

  /* Responsive */
  @media (max-width: 768px) {
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
    }

    .code-block {
      flex-direction: column;
      gap: var(--space-3);
      align-items: stretch;
    }

    .cost-line {
      flex-direction: column;
      align-items: flex-start;
      gap: var(--space-1);
    }
  }
</style>
