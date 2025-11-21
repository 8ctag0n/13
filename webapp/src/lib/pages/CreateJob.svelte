<script>
  import { navigateTo } from '../stores/router';
  import { walletStore } from '../stores/wallet';
  import PaymentMethodSelector from '../components/PaymentMethodSelector.svelte';
  import Loading from '../components/Loading.svelte';
  import { ensureTokenAccount, WZEC_MINT } from '../utils/tokenAccountManager';
  import { Connection, clusterApiUrl } from '@solana/web3.js';
  import { toastStore } from '../stores/toast';

  let currentStep = 1;
  let isProcessing = false;
  let processingMessage = '';
  let jobData = {
    encryptedData: null,
    serverKey: null,
    operation: 'Multiply',
    operationValue: 5,
    consensus: '2-of-3',
    priceLamports: 2000000,
    requiredProvers: 3,
    consensusThreshold: 2,
    paymentMethod: 'sol' // 'sol' | 'wzec'
  };

  // File uploads
  let encryptedDataFile = null;
  let serverKeyFile = null;
  let isDragging = false;

  // Pricing
  $: totalCost = (jobData.priceLamports / 1000000000).toFixed(5);
  $: platformFee = (jobData.priceLamports * 0.01 / 1000000000).toFixed(5);
  $: totalWithFee = ((jobData.priceLamports * 1.01) / 1000000000).toFixed(5);

  function handleDragOver(e) {
    e.preventDefault();
    isDragging = true;
  }

  function handleDragLeave() {
    isDragging = false;
  }

  function handleDrop(e, type) {
    e.preventDefault();
    isDragging = false;

    const files = e.dataTransfer.files;
    if (files.length > 0) {
      handleFileUpload(files[0], type);
    }
  }

  function handleFileInput(e, type) {
    const files = e.target.files;
    if (files.length > 0) {
      handleFileUpload(files[0], type);
    }
  }

  function handleFileUpload(file, type) {
    if (type === 'encrypted_data') {
      encryptedDataFile = file;
      jobData.encryptedData = file.name;
    } else if (type === 'server_key') {
      serverKeyFile = file;
      jobData.serverKey = file.name;
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

  function handleCancel() {
    navigateTo('dashboard');
  }

  async function handleSubmit() {
    if (!$walletStore || !$walletStore.publicKey) {
      toastStore.add('Please connect your wallet first', 'error');
      return;
    }

    isProcessing = true;

    try {
      // If paying with wZEC, ensure token account exists
      if (jobData.paymentMethod === 'wzec') {
        const connection = new Connection(clusterApiUrl('devnet'), 'confirmed');

        processingMessage = 'Checking wZEC token account...';

        const tokenAccount = await ensureTokenAccount(
          connection,
          $walletStore,
          WZEC_MINT,
          (progress) => {
            processingMessage = progress.message;
            console.log(`Token account progress: ${progress.step} - ${progress.message}`);

            // Show toast notifications for key progress steps
            if (progress.step === 'creating') {
              toastStore.add('Creating wZEC token account...', 'info');
            } else if (progress.step === 'success') {
              toastStore.add('wZEC token account ready!', 'success');
            }
          }
        );
      }

      processingMessage = 'Preparing transaction...';

      // TODO: Implement actual transaction signing
      toastStore.add('Transaction signing coming soon!', 'info');
      // After successful submission:
      // navigateTo('dashboard');

    } catch (error) {
      console.error('Error during job creation:', error);
      toastStore.add(error.message || 'Failed to create job', 'error');
    } finally {
      isProcessing = false;
      processingMessage = '';
    }
  }

  function handlePaymentMethodChange(event) {
    jobData.paymentMethod = event.detail.payment_method;
  }

  $: canProceedStep1 = jobData.encryptedData && jobData.serverKey;
  $: canProceedStep2 = jobData.operation && jobData.operationValue;
</script>

<div class="create-job">
  <!-- Header -->
  <header class="wizard-header">
    <div class="container">
      <div class="header-content">
        <h1 class="text-mono text-uppercase">CREATE_FHE_JOB</h1>
        <div class="step-indicator text-mono text-sm text-muted">
          STEP {currentStep} OF 4
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
              Run this command in your terminal:
            </div>
          </div>

          <div class="code-block mb-6">
            <div class="text-mono text-cyan">
              $ cargo run --bin fhe-encrypt
            </div>
            <button class="btn-copy text-mono text-sm" on:click={() => navigator.clipboard.writeText('cargo run --bin fhe-encrypt')}>
              [📋 COPY]
            </button>
          </div>

          <div class="text-mono text-sm text-muted mb-4">
            Once complete, upload the generated files below:
          </div>

          <!-- File Upload: Encrypted Data -->
          <div class="upload-section mb-4">
            <label class="text-mono text-sm mb-2">ENCRYPTED_DATA.JSON:</label>
            <div
              class="upload-zone {isDragging ? 'dragging' : ''} {jobData.encryptedData ? 'has-file' : ''}"
              on:dragover={handleDragOver}
              on:dragleave={handleDragLeave}
              on:drop={(e) => handleDrop(e, 'encrypted_data')}
            >
              {#if jobData.encryptedData}
                <div class="file-uploaded text-mono">
                  <span class="text-success">[✓]</span> {jobData.encryptedData}
                  <span class="text-muted text-xs">({encryptedDataFile?.size ? (encryptedDataFile.size / 1024).toFixed(1) : '0'} KB)</span>
                </div>
              {:else}
                <div class="upload-prompt">
                  <div class="text-mono mb-2">[↓] DRAG_&_DROP_FILE_HERE</div>
                  <div class="text-sm text-muted">or click to browse</div>
                </div>
              {/if}
              <input
                type="file"
                accept=".json"
                on:change={(e) => handleFileInput(e, 'encrypted_data')}
                style="display: none"
              />
            </div>
          </div>

          <!-- File Upload: Server Key -->
          <div class="upload-section mb-6">
            <label class="text-mono text-sm mb-2">SERVER_KEY.BIN:</label>
            <div
              class="upload-zone {isDragging ? 'dragging' : ''} {jobData.serverKey ? 'has-file' : ''}"
              on:dragover={handleDragOver}
              on:dragleave={handleDragLeave}
              on:drop={(e) => handleDrop(e, 'server_key')}
            >
              {#if jobData.serverKey}
                <div class="file-uploaded text-mono">
                  <span class="text-success">[✓]</span> {jobData.serverKey}
                  <span class="text-muted text-xs">({serverKeyFile?.size ? (serverKeyFile.size / (1024 * 1024)).toFixed(1) : '0'} MB)</span>
                </div>
              {:else}
                <div class="upload-prompt">
                  <div class="text-mono mb-2">[↓] DRAG_&_DROP_FILE_HERE</div>
                  <div class="text-sm text-muted">or click to browse</div>
                </div>
              {/if}
              <input
                type="file"
                accept=".bin"
                on:change={(e) => handleFileInput(e, 'server_key')}
                style="display: none"
              />
            </div>
          </div>

          <div class="info-box-cyan">
            <div class="text-mono text-sm">
              💡 WHY_LOCAL_ENCRYPTION?<br/>
              This ensures your data is never exposed to our servers.<br/>
              Only YOU can decrypt the final result.
            </div>
          </div>
        </div>

      {:else if currentStep === 2}
        <!-- Step 2: Configure -->
        <div class="wizard-card tui-box fade-in">
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
            <label class="text-mono mb-2">OPERATION_VALUE:</label>
            <input
              type="number"
              bind:value={jobData.operationValue}
              class="input input-code"
              min="1"
              max="255"
            />
            <div class="text-xs text-muted mt-2">
              Estimated time: ~2-3 seconds
            </div>
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
                  on:change={() => { jobData.requiredProvers = 3; jobData.consensusThreshold = 2; jobData.priceLamports = 2000000; }} />
                <span class="radio-label text-mono">
                  <span class="radio-check">●</span>
                  <span>2_OF_3_PROVERS</span>
                  <span class="badge badge-success">RECOMMENDED</span>
                  <span class="text-cyan">[0.002_SOL]</span>
                </span>
              </label>

              <label class="radio-option">
                <input type="radio" bind:group={jobData.consensus} value="3-of-5"
                  on:change={() => { jobData.requiredProvers = 5; jobData.consensusThreshold = 3; jobData.priceLamports = 4000000; }} />
                <span class="radio-label text-mono">
                  <span class="radio-check">○</span>
                  <span>3_OF_5_PROVERS</span>
                  <span class="text-muted text-sm">More secure</span>
                  <span class="text-cyan">[0.004_SOL]</span>
                </span>
              </label>
            </div>
            <div class="text-xs text-muted mt-3">
              💡 Higher consensus = more reliable results
            </div>
          </div>

          <div class="divider-section"></div>

          <!-- Cost Breakdown -->
          <div class="cost-breakdown tui-box-cyan">
            <div class="text-mono text-uppercase mb-3">TOTAL_COST:</div>
            <div class="cost-lines text-mono text-sm">
              <div class="cost-line">
                <span class="text-muted">Provers ({jobData.requiredProvers}):</span>
                <span class="text-cyan">{totalCost}_SOL</span>
              </div>
              <div class="cost-line">
                <span class="text-muted">Platform Fee (1%):</span>
                <span class="text-cyan">{platformFee}_SOL</span>
              </div>
              <div class="divider-cost text-muted">───────────────────────────────</div>
              <div class="cost-line total">
                <span>Total:</span>
                <span class="text-cyan">{totalWithFee}_SOL</span>
                <span class="text-muted text-xs">~$0.02</span>
              </div>
            </div>
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
                <span class="text-success">[✓]</span> {jobData.encryptedData}
                <span class="text-muted">({encryptedDataFile?.size ? (encryptedDataFile.size / 1024).toFixed(1) : '0'} KB)</span>
              </div>
              <div class="review-line text-mono text-sm">
                <span class="text-success">[✓]</span> {jobData.serverKey}
                <span class="text-muted">({serverKeyFile?.size ? (serverKeyFile.size / (1024 * 1024)).toFixed(1) : '0'} MB)</span>
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
              <div class="mb-2">⚠️  YOUR_WALLET_WILL_BE_CHARGED_IMMEDIATELY</div>
              <div class="text-muted">Provers will be paid automatically upon completion</div>
            </div>
          </div>

          <div class="success-box">
            <div class="text-mono text-sm">
              <div class="mb-2">✅  YOUR_DATA_REMAINS_ENCRYPTED_THROUGHOUT</div>
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
            {#if isProcessing}
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
              <!-- Wallet Signature State -->
              <div class="wallet-icon text-center mb-6">
                <div class="text-6xl">👛</div>
              </div>

              <div class="text-mono text-center mb-4">
                WAITING_FOR_WALLET_SIGNATURE<span class="cursor-blink"></span>
              </div>

              <div class="text-sm text-muted text-center mb-6">
                Check your {$walletStore.name || 'wallet'} extension<br/>
                and approve the transaction
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
        >
          [{currentStep === 1 ? 'CANCEL' : '◀ BACK'}]
        </button>

        {#if currentStep < 4}
          <button
            class="btn btn-primary"
            on:click={nextStep}
            disabled={(currentStep === 1 && !canProceedStep1) || (currentStep === 2 && !canProceedStep2)}
          >
            [NEXT: {currentStep === 1 ? 'CONFIGURE' : currentStep === 2 ? 'REVIEW' : 'SUBMIT'} ▶]
          </button>
        {:else}
          <button
            class="btn btn-primary"
            on:click={handleSubmit}
          >
            [CREATE_JOB_&_SIGN]
          </button>
        {/if}
      </div>
    </div>
  </main>
</div>

<style>
  .create-job {
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

  /* Upload Zone */
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
