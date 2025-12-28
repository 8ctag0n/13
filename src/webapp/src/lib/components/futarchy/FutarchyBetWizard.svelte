<script>
  import { createEventDispatcher } from 'svelte';
  import { walletStore } from '../../stores/wallet';
  import TerminalBox from './terminal/TerminalBox.svelte';
  import TerminalButton from './terminal/TerminalButton.svelte';
  import TerminalInput from './terminal/TerminalInput.svelte';
  import TerminalRadio from './terminal/TerminalRadio.svelte';
  import WitnessUploader from '../WitnessUploader.svelte';
  import FutarchyTxStatus from './FutarchyTxStatus.svelte';
  import { validatePlaceBet, submitBet } from '../../utils/futarchy_api';
  import { signAndSendUnsignedTx } from '../../utils/futarchy_tx';

  const dispatch = createEventDispatcher();

  export let open = false;
  export let apiBaseUrl = '';
  export let initialMarketId = '';

  let step = 1;
  let marketId = '';
  let side = 'yes';
  let amount = '';
  let witnessData = null;
  let witnessError = '';
  let proof = '';
  let publicInputs = '';
  let circuitType = 50;
  let fheJobId = '';
  let txState = 'idle';
  let txMessage = '';
  let txError = '';
  let txHash = '';
  let confirmations = 0;
  let busy = false;

  const lamportsPerSol = 1_000_000_000;

  $: if (initialMarketId && !marketId) {
    marketId = initialMarketId;
  }

  $: amountValue = Number(amount);
  $: amountLamports = Number.isFinite(amountValue) ? Math.round(amountValue * lamportsPerSol) : 0;
  $: walletAddress = $walletStore.addresses?.solana || $walletStore.publicKey?.toString() || '';

  function resetState() {
    step = 1;
    marketId = initialMarketId || '';
    side = 'yes';
    amount = '';
    witnessData = null;
    witnessError = '';
    proof = '';
    publicInputs = '';
    circuitType = 50;
    fheJobId = '';
    txState = 'idle';
    txMessage = '';
    txError = '';
    txHash = '';
    confirmations = 0;
    busy = false;
  }

  function handleClose() {
    resetState();
    dispatch('close');
  }

  function handleWitnessParsed(event) {
    witnessData = event.detail;
    witnessError = '';
  }

  function handleWitnessError(event) {
    witnessData = null;
    witnessError = event.detail?.error || 'Failed to parse witness';
  }

  function nextStep() {
    if (step === 1 && !isMarketIdValid()) {
      witnessError = 'Enter a valid market id';
      return;
    }
    if (step === 2 && !isBetDetailsValid()) {
      witnessError = 'Enter a valid bet amount';
      return;
    }
    if (step === 3 && !witnessData) {
      witnessError = 'Upload witness.bin to continue';
      return;
    }
    if (step === 4 && !isProofValid()) {
      witnessError = 'Enter proof, public inputs, and fhe_job_id';
      return;
    }
    witnessError = '';
    step = Math.min(step + 1, 5);
  }

  function prevStep() {
    witnessError = '';
    step = Math.max(step - 1, 1);
  }

  function isMarketIdValid() {
    return !!marketId && /^\d+$/.test(marketId);
  }

  function isBetDetailsValid() {
    return Number.isFinite(amountValue) && amountValue > 0;
  }

  function isProofValid() {
    const circuitValue = Number(circuitType);
    return !!proof && !!publicInputs && Number.isFinite(circuitValue) && !!fheJobId;
  }

  async function bytesToBase64Async(bytes, onProgress = () => {}) {
    const chunkSize = 48 * 1024;
    const base64Chunks = [];
    const totalBytes = bytes.length;
    let processed = 0;

    for (let i = 0; i < totalBytes; i += chunkSize) {
      const end = Math.min(i + chunkSize, totalBytes);
      const chunk = bytes.subarray(i, end);
      let binary = '';
      for (let j = 0; j < chunk.length; j += 1) {
        binary += String.fromCharCode(chunk[j]);
      }
      base64Chunks.push(btoa(binary));
      processed = end;
      if (base64Chunks.length % 5 === 0) {
        onProgress(Math.round((processed / totalBytes) * 100));
        await new Promise(r => setTimeout(r, 0));
      }
    }

    onProgress(100);
    return base64Chunks.join('');
  }

  async function handleSubmit() {
    if (busy) return;

    if (!walletAddress) {
      txState = 'error';
      txError = 'Connect wallet to submit bet';
      return;
    }

    if (!isMarketIdValid() || !isBetDetailsValid() || !witnessData || !isProofValid()) {
      txState = 'error';
      txError = 'Complete all steps before submitting';
      return;
    }

    busy = true;
    txState = 'signing';
    txMessage = 'Validating bet...';

    try {
      const payload = {
        bettor: walletAddress,
        side,
        amount_lamports: amountLamports,
        encrypted_amount: witnessData.encryptedData,
        proof,
        public_inputs: publicInputs,
        circuit_type: Number(circuitType),
        fhe_job_id: Number(fheJobId)
      };

      const response = await validatePlaceBet(marketId, payload, { apiBaseUrl: apiBaseUrl || undefined });
      const unsignedTx = response?.unsigned_transaction;

      let signedTxBase64 = '';
      const result = await signAndSendUnsignedTx({
        unsignedTransaction: unsignedTx,
        wallet: $walletStore,
        onStatus: (status) => {
          if (status.step === 'confirming') {
            txState = 'confirming';
            txMessage = status.message;
            txHash = status.signature;
            confirmations = status.confirmations || 0;
          } else {
            txState = 'signing';
            txMessage = status.message;
          }
          signedTxBase64 = status.signedTxBase64 || signedTxBase64;
        }
      });

      txState = 'signing';
      txMessage = 'Encoding server key...';
      const serverKeyBase64 = await bytesToBase64Async(witnessData.serverKeyBytes, (progress) => {
        txMessage = `Encoding server key... ${progress}%`;
      });

      txState = 'signing';
      txMessage = 'Submitting ciphertext...';
      await submitBet({
        signed_tx: signedTxBase64 || result.signedTxBase64,
        ciphertext: witnessData.encryptedData,
        server_key: serverKeyBase64
      }, { apiBaseUrl: apiBaseUrl || undefined });

      txState = 'success';
      txMessage = 'Bet confirmed and stored!';
      dispatch('success', { signature: result.signature, marketId });
    } catch (error) {
      txState = 'error';
      txError = error?.message || 'Bet failed';
      dispatch('error', { error });
    } finally {
      busy = false;
    }
  }

  $: if (!open) {
    resetState();
  }
</script>

{#if open}
  <TerminalBox tone="violet">
    <div class="wizard-header text-mono">
      <div>FUTARCHY BET WIZARD</div>
      <div class="text-xs text-muted">STEP {step}/5</div>
    </div>

    {#if step === 1}
      <div class="wizard-step">
        <div class="step-title text-mono text-xs">STEP 1: MARKET ID</div>
        <TerminalInput label="market_id" bind:value={marketId} placeholder="12345" prefix=">" />
        <div class="text-mono text-xs text-muted">Use /markets to list available markets.</div>
      </div>
    {:else if step === 2}
      <div class="wizard-step">
        <div class="step-title text-mono text-xs">STEP 2: BET DETAILS</div>
        <div class="side-options">
          <TerminalRadio name="side" value="yes" label="YES" checked={side === 'yes'} on:change={() => (side = 'yes')} />
          <TerminalRadio name="side" value="no" label="NO" checked={side === 'no'} on:change={() => (side = 'no')} />
        </div>
        <TerminalInput label="amount_sol" bind:value={amount} placeholder="0.5" prefix=">" suffix="SOL" />
      </div>
    {:else if step === 3}
      <div class="wizard-step">
        <div class="step-title text-mono text-xs">STEP 3: WITNESS.BIN</div>
        <div class="text-mono text-xs text-muted">
          Download this CLI, compile it, and then upload the witness.bin:
          <div class="cli-note text-mono text-xs">
            $ git clone &lt;cli-repo&gt;
            <br />
            $ cargo build --release
            <br />
            $ zyb fhe encrypt -p ./output -v 1
          </div>
          Then upload the generated <strong>witness.bin</strong> file.
        </div>
        <WitnessUploader compact on:witnessParsed={handleWitnessParsed} on:witnessError={handleWitnessError} />
      </div>
    {:else if step === 4}
      <div class="wizard-step">
        <div class="step-title text-mono text-xs">STEP 4: PROOF INPUTS</div>
        <label class="text-mono text-xs text-muted">proof (base64)</label>
        <textarea class="wizard-textarea text-mono" bind:value={proof} placeholder="paste proof base64"></textarea>
        <label class="text-mono text-xs text-muted">public_inputs (base64)</label>
        <textarea class="wizard-textarea text-mono" bind:value={publicInputs} placeholder="paste public inputs base64"></textarea>
        <div class="proof-row">
          <TerminalInput label="circuit_type" bind:value={circuitType} placeholder="50" prefix=">" />
          <TerminalInput label="fhe_job_id" bind:value={fheJobId} placeholder="job id" prefix=">" />
        </div>
      </div>
    {:else}
      <div class="wizard-step">
        <div class="step-title text-mono text-xs">STEP 5: REVIEW & SUBMIT</div>
        <div class="review-grid text-mono text-xs">
          <div>market_id: {marketId}</div>
          <div>side: {side.toUpperCase()}</div>
          <div>amount: {amount} SOL</div>
          <div>fhe_job_id: {fheJobId}</div>
          <div>proof: {proof ? `${proof.slice(0, 12)}...` : '--'}</div>
          <div>public_inputs: {publicInputs ? `${publicInputs.slice(0, 12)}...` : '--'}</div>
          <div>witness: {witnessData?.fileName || '--'}</div>
        </div>
      </div>
    {/if}

    {#if witnessError}
      <div class="wizard-error text-mono text-xs">[!] {witnessError}</div>
    {/if}

    <div class="wizard-actions">
      <TerminalButton label="CANCEL" tone="muted" size="sm" on:click={handleClose} />
      {#if step > 1}
        <TerminalButton label="BACK" tone="muted" size="sm" on:click={prevStep} />
      {/if}
      {#if step < 5}
        <TerminalButton label="NEXT" tone="cyan" size="sm" on:click={nextStep} />
      {:else}
        <TerminalButton label="SUBMIT BET >" tone="cyan" size="sm" on:click={handleSubmit} />
      {/if}
    </div>
  </TerminalBox>

  <div class="status-panel">
    <FutarchyTxStatus
      state={txState}
      message={txState === 'error' ? txError : txMessage}
      txHash={txHash}
      confirmations={confirmations}
      primaryLabel={txState === 'error' ? 'TRY AGAIN' : ''}
      secondaryLabel={txState === 'error' ? 'CLOSE' : ''}
      on:primary={handleSubmit}
      on:secondary={handleClose}
    />
  </div>
{/if}

<style>
  .wizard-header {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    margin-bottom: var(--space-4);
  }

  .wizard-step {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  .step-title {
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }

  .side-options {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
    gap: var(--space-3);
  }

  .cli-note {
    margin-top: var(--space-2);
    padding: var(--space-2);
    border: 1px dashed var(--zyber-border-secondary);
    background: rgba(0, 0, 0, 0.25);
  }

  .wizard-textarea {
    min-height: 120px;
    padding: var(--space-2) var(--space-3);
    background: rgba(0, 0, 0, 0.35);
    border: 1px solid var(--zyber-border-secondary);
    color: var(--zyber-text-primary);
    resize: vertical;
  }

  .proof-row {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
    gap: var(--space-3);
  }

  .review-grid {
    display: grid;
    gap: var(--space-2);
  }

  .wizard-actions {
    margin-top: var(--space-4);
    display: flex;
    flex-wrap: wrap;
    justify-content: flex-end;
    gap: var(--space-2);
  }

  .wizard-error {
    margin-top: var(--space-3);
    color: var(--zyber-error);
  }

  .status-panel {
    margin-top: var(--space-4);
  }
</style>
