<script lang="ts">
  import { createEventDispatcher, onMount } from 'svelte';
  import type { SupportedChain } from '../../lib/chains/types';

  export let activeChain: SupportedChain;
  export let address: string;

  const dispatch = createEventDispatcher();

  type Step = 'form' | 'confirm' | 'sending' | 'success' | 'error';

  let step: Step = 'form';
  let sendingStep = 0; // 0=idle, 1=building, 2=signing, 3=broadcasting, 4=confirming
  let recipient = '';
  let amount = '';
  let memo = '';
  let gasOption: 'slow' | 'normal' | 'fast' = 'normal';
  let loading = false;
  let error = '';
  let txSignature = '';
  let balance = '0';

  const chainConfig: Record<SupportedChain, { symbol: string; icon: string; color: string; name: string }> = {
    solana: { symbol: 'SOL', icon: '◉', color: 'var(--cyber-cyan)', name: 'Solana' },
    starknet: { symbol: 'STRK', icon: '▲', color: 'var(--cyber-purple)', name: 'Starknet' },
    zcash: { symbol: 'ZEC', icon: 'Z', color: 'var(--cyber-yellow)', name: 'Zcash' }
  };

  const gasOptions = {
    slow: { label: 'SLOW', fee: '0.000003', time: '~60s' },
    normal: { label: 'NORMAL', fee: '0.000005', time: '~30s' },
    fast: { label: 'FAST', fee: '0.000010', time: '~10s' }
  };

  // Mock USD price
  const mockPrices: Record<SupportedChain, number> = {
    solana: 95.50,
    starknet: 0.85,
    zcash: 35.20
  };

  $: config = chainConfig[activeChain];
  $: amountNum = parseFloat(amount) || 0;
  $: amountUSD = amountNum * mockPrices[activeChain];
  $: gasFee = parseFloat(gasOptions[gasOption].fee);
  $: totalAmount = amountNum + gasFee;
  $: totalUSD = totalAmount * mockPrices[activeChain];
  $: isValidAmount = amountNum > 0 && amountNum <= parseFloat(balance);
  $: isValidRecipient = recipient.trim().length > 20;

  async function loadBalance() {
    try {
      const response = await chrome.runtime.sendMessage({
        type: 'GET_BALANCE',
        chain: activeChain,
        address
      });
      if (response.success) {
        balance = response.data.balance;
      }
    } catch (err) {
      console.error('[Send] Failed to load balance:', err);
    }
  }

  function handleMax() {
    const maxAmount = Math.max(0, parseFloat(balance) - gasFee);
    amount = maxAmount.toFixed(6);
  }

  function handleReview() {
    if (!recipient || !amount) {
      error = 'Recipient and amount are required';
      return;
    }
    if (!isValidAmount) {
      error = 'Insufficient balance';
      return;
    }
    error = '';
    step = 'confirm';
  }

  async function handleConfirmSend() {
    step = 'sending';
    sendingStep = 0;
    error = '';

    try {
      // Update progress UI
      sendingStep = 1; // Building
      await new Promise(r => setTimeout(r, 300));

      sendingStep = 2; // Signing
      await new Promise(r => setTimeout(r, 300));

      // Send transaction (builds, signs, and sends in one call)
      const response = await chrome.runtime.sendMessage({
        type: 'SEND_TRANSACTION',
        chain: activeChain,
        to: recipient,
        amount,
        memo: memo || undefined
      });

      sendingStep = 3; // Broadcasting

      if (response.success) {
        sendingStep = 4; // Confirming
        await new Promise(r => setTimeout(r, 500));

        txSignature = response.data?.signature || 'tx_confirmed';

        // Save transaction to history
        await chrome.runtime.sendMessage({
          type: 'SAVE_TRANSACTION',
          transaction: {
            type: 'send',
            chain: activeChain,
            amount,
            symbol: config.symbol,
            to: recipient,
            status: 'confirmed',
            signature: txSignature
          }
        });

        step = 'success';
      } else {
        throw new Error(response.error || 'Transaction failed');
      }
    } catch (err) {
      error = err instanceof Error ? err.message : 'Unknown error';
      step = 'error';
      console.error('[Send] Transaction error:', err);
    }
  }

  function handleBack() {
    if (step === 'confirm') {
      step = 'form';
    } else {
      dispatch('back');
    }
  }

  function handleDone() {
    dispatch('success');
  }

  function handleRetry() {
    step = 'form';
    error = '';
  }

  function truncateAddress(addr: string): string {
    if (!addr) return '';
    return `${addr.slice(0, 8)}...${addr.slice(-6)}`;
  }

  onMount(() => {
    loadBalance();
  });
</script>

<!-- HEADER -->
<div class="send-header">
  <button class="back-btn" on:click={handleBack}>
    [← BACK]
  </button>
  <span class="send-title">[SEND]</span>
  <div class="chain-badge" style="color: {config.color}; border-color: {config.color}">
    {config.icon} {config.symbol}
  </div>
</div>

{#if step === 'form'}
  <!-- FORM VIEW -->
  <div class="send-form">
    <!-- Asset Info -->
    <div class="asset-info-card">
      <div class="asset-row">
        <span class="asset-label">Available:</span>
        <span class="asset-value">{parseFloat(balance).toFixed(6)} {config.symbol}</span>
      </div>
      <div class="asset-row dim">
        <span class="asset-label">From:</span>
        <span class="asset-value">{truncateAddress(address)}</span>
      </div>
    </div>

    <!-- Recipient -->
    <div class="form-group">
      <label class="label" for="recipient">[RECIPIENT ADDRESS]</label>
      <input
        id="recipient"
        type="text"
        class="input"
        bind:value={recipient}
        placeholder="Enter {config.name} address..."
        class:valid={isValidRecipient}
      />
    </div>

    <!-- Amount -->
    <div class="form-group">
      <label class="label" for="amount">[AMOUNT]</label>
      <div class="amount-input-wrapper">
        <input
          id="amount"
          type="text"
          class="input amount-input"
          bind:value={amount}
          placeholder="0.00"
        />
        <button class="max-btn" on:click={handleMax}>[MAX]</button>
      </div>
      {#if amountNum > 0}
        <div class="amount-usd">≈ ${amountUSD.toFixed(2)} USD</div>
      {/if}
    </div>

    <!-- Gas Options -->
    <div class="form-group">
      <label class="label">[NETWORK FEE]</label>
      <div class="gas-options">
        {#each Object.entries(gasOptions) as [key, opt]}
          <button
            class="gas-option"
            class:active={gasOption === key}
            on:click={() => gasOption = key}
          >
            <span class="gas-label">{opt.label}</span>
            <span class="gas-fee">{opt.fee}</span>
            <span class="gas-time">{opt.time}</span>
          </button>
        {/each}
      </div>
    </div>

    <!-- Memo (Zcash only) -->
    {#if activeChain === 'zcash'}
      <div class="form-group">
        <label class="label" for="memo">[MEMO (Optional)]</label>
        <input
          id="memo"
          type="text"
          class="input"
          bind:value={memo}
          placeholder="Private message..."
        />
      </div>
    {/if}

    <!-- Summary -->
    <div class="summary-card">
      <div class="summary-row">
        <span>Amount:</span>
        <span>{amountNum.toFixed(6)} {config.symbol}</span>
      </div>
      <div class="summary-row">
        <span>Network fee:</span>
        <span>{gasFee.toFixed(6)} {config.symbol}</span>
      </div>
      <div class="summary-divider"></div>
      <div class="summary-row total">
        <span>Total:</span>
        <span>{totalAmount.toFixed(6)} {config.symbol}</span>
      </div>
      <div class="summary-row dim">
        <span></span>
        <span>≈ ${totalUSD.toFixed(2)} USD</span>
      </div>
    </div>

    {#if error}
      <div class="error">[ERROR] {error}</div>
    {/if}

    <button
      class="button primary full-width"
      on:click={handleReview}
      disabled={!isValidRecipient || !isValidAmount}
    >
      [REVIEW TRANSACTION →]
    </button>
  </div>

{:else if step === 'confirm'}
  <!-- CONFIRM VIEW -->
  <div class="confirm-view">
    <div class="confirm-header">
      <span class="confirm-icon" style="color: {config.color}">{config.icon}</span>
      <div class="confirm-amount">{amountNum.toFixed(6)} {config.symbol}</div>
      <div class="confirm-usd">≈ ${amountUSD.toFixed(2)} USD</div>
    </div>

    <div class="confirm-details">
      <div class="detail-row">
        <span class="detail-label">[FROM]</span>
        <span class="detail-value">{truncateAddress(address)}</span>
      </div>
      <div class="detail-arrow">↓</div>
      <div class="detail-row">
        <span class="detail-label">[TO]</span>
        <span class="detail-value highlight">{truncateAddress(recipient)}</span>
      </div>
      <div class="detail-divider"></div>
      <div class="detail-row">
        <span class="detail-label">Network:</span>
        <span class="detail-value">{config.name}</span>
      </div>
      <div class="detail-row">
        <span class="detail-label">Fee:</span>
        <span class="detail-value">{gasFee.toFixed(6)} {config.symbol}</span>
      </div>
      <div class="detail-row">
        <span class="detail-label">Speed:</span>
        <span class="detail-value">{gasOptions[gasOption].label} {gasOptions[gasOption].time}</span>
      </div>
      <div class="detail-divider"></div>
      <div class="detail-row total">
        <span class="detail-label">TOTAL:</span>
        <span class="detail-value">{totalAmount.toFixed(6)} {config.symbol}</span>
      </div>
    </div>

    <div class="warning">
      <strong>[!]</strong> This action cannot be undone. Verify all details before confirming.
    </div>

    <div class="confirm-actions">
      <button class="button secondary" on:click={handleBack}>
        [← EDIT]
      </button>
      <button class="button primary" on:click={handleConfirmSend}>
        [CONFIRM & SEND →]
      </button>
    </div>
  </div>

{:else if step === 'sending'}
  <!-- SENDING VIEW -->
  <div class="status-view">
    <div class="status-icon sending">
      <div class="spinner"></div>
    </div>
    <div class="status-title">[SENDING...]</div>
    <div class="status-steps">
      <div class="step" class:active={sendingStep >= 1} class:done={sendingStep > 1}>
        {sendingStep > 1 ? '✓' : '○'} Building transaction
      </div>
      <div class="step" class:active={sendingStep >= 2} class:done={sendingStep > 2}>
        {sendingStep > 2 ? '✓' : '○'} Signing with private key
      </div>
      <div class="step" class:active={sendingStep >= 3} class:done={sendingStep > 3}>
        {sendingStep > 3 ? '✓' : '○'} Broadcasting to network
      </div>
      <div class="step" class:active={sendingStep >= 4}>
        {sendingStep >= 4 ? '✓' : '○'} Confirming
      </div>
    </div>
  </div>

{:else if step === 'success'}
  <!-- SUCCESS VIEW -->
  <div class="status-view">
    <div class="status-icon success">✓</div>
    <div class="status-title success">[TRANSACTION SENT]</div>
    <div class="status-message">
      Successfully sent {amountNum.toFixed(6)} {config.symbol}
    </div>

    {#if txSignature}
      <div class="tx-hash">
        <span class="tx-label">[TX HASH]</span>
        <span class="tx-value">{truncateAddress(txSignature)}</span>
      </div>
    {/if}

    <button class="button primary full-width" on:click={handleDone}>
      [DONE]
    </button>
  </div>

{:else if step === 'error'}
  <!-- ERROR VIEW -->
  <div class="status-view">
    <div class="status-icon error">✗</div>
    <div class="status-title error">[TRANSACTION FAILED]</div>
    <div class="status-message error-msg">
      {error}
    </div>

    <div class="error-actions">
      <button class="button secondary" on:click={handleBack}>
        [← BACK]
      </button>
      <button class="button primary" on:click={handleRetry}>
        [TRY AGAIN]
      </button>
    </div>
  </div>
{/if}

<style>
  /* HEADER */
  .send-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 10px 0;
    border-bottom: 1px solid var(--cyber-border);
    margin-bottom: 15px;
  }

  .back-btn {
    background: transparent;
    border: 1px solid var(--cyber-border);
    color: var(--cyber-text-dim);
    padding: 6px 10px;
    font-family: inherit;
    font-size: 11px;
    cursor: pointer;
  }

  .back-btn:hover {
    border-color: var(--cyber-cyan);
    color: var(--cyber-cyan);
  }

  .send-title {
    font-size: 14px;
    font-weight: bold;
    color: var(--cyber-cyan);
    letter-spacing: 2px;
  }

  .chain-badge {
    font-size: 11px;
    padding: 4px 8px;
    border: 1px solid;
    background: rgba(0, 255, 159, 0.1);
  }

  /* FORM */
  .send-form {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .asset-info-card {
    background: var(--cyber-bg-alt);
    border: 1px solid var(--cyber-border);
    padding: 10px 12px;
  }

  .asset-row {
    display: flex;
    justify-content: space-between;
    font-size: 12px;
  }

  .asset-row.dim {
    color: var(--cyber-text-dim);
    font-size: 11px;
    margin-top: 4px;
  }

  .asset-label {
    color: var(--cyber-text-dim);
  }

  .asset-value {
    color: var(--cyber-cyan);
  }

  .form-group {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .amount-input-wrapper {
    display: flex;
    gap: 8px;
  }

  .amount-input {
    flex: 1;
  }

  .max-btn {
    background: transparent;
    border: 1px solid var(--cyber-cyan);
    color: var(--cyber-cyan);
    padding: 8px 12px;
    font-family: inherit;
    font-size: 11px;
    cursor: pointer;
  }

  .max-btn:hover {
    background: rgba(0, 255, 159, 0.1);
  }

  .amount-usd {
    font-size: 11px;
    color: var(--cyber-text-dim);
    text-align: right;
  }

  .input.valid {
    border-color: var(--cyber-cyan);
  }

  /* GAS OPTIONS */
  .gas-options {
    display: flex;
    gap: 8px;
  }

  .gas-option {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
    padding: 8px 4px;
    background: var(--cyber-bg-alt);
    border: 1px solid var(--cyber-border);
    font-family: inherit;
    cursor: pointer;
    transition: all 0.2s;
  }

  .gas-option:hover {
    border-color: var(--cyber-cyan);
  }

  .gas-option.active {
    border-color: var(--cyber-cyan);
    background: rgba(0, 255, 159, 0.1);
  }

  .gas-label {
    font-size: 10px;
    font-weight: bold;
    color: var(--cyber-text);
  }

  .gas-fee {
    font-size: 9px;
    color: var(--cyber-cyan);
  }

  .gas-time {
    font-size: 9px;
    color: var(--cyber-text-dim);
  }

  /* SUMMARY */
  .summary-card {
    background: var(--cyber-bg-alt);
    border: 1px solid var(--cyber-border);
    padding: 12px;
  }

  .summary-row {
    display: flex;
    justify-content: space-between;
    font-size: 12px;
    color: var(--cyber-text-dim);
    margin-bottom: 4px;
  }

  .summary-row.total {
    color: var(--cyber-text);
    font-weight: bold;
  }

  .summary-row.dim {
    font-size: 11px;
  }

  .summary-divider {
    height: 1px;
    background: var(--cyber-border);
    margin: 8px 0;
  }

  .full-width {
    width: 100%;
    margin-top: 10px;
  }

  /* CONFIRM VIEW */
  .confirm-view {
    display: flex;
    flex-direction: column;
    gap: 15px;
  }

  .confirm-header {
    text-align: center;
    padding: 20px;
    background: var(--cyber-bg-alt);
    border: 1px solid var(--cyber-border);
  }

  .confirm-icon {
    font-size: 32px;
    display: block;
    margin-bottom: 10px;
  }

  .confirm-amount {
    font-size: 24px;
    font-weight: bold;
    color: var(--cyber-text);
  }

  .confirm-usd {
    font-size: 12px;
    color: var(--cyber-text-dim);
  }

  .confirm-details {
    background: var(--cyber-bg-alt);
    border: 1px solid var(--cyber-border);
    padding: 15px;
  }

  .detail-row {
    display: flex;
    justify-content: space-between;
    font-size: 12px;
    margin-bottom: 6px;
  }

  .detail-label {
    color: var(--cyber-text-dim);
  }

  .detail-value {
    color: var(--cyber-text);
  }

  .detail-value.highlight {
    color: var(--cyber-cyan);
  }

  .detail-arrow {
    text-align: center;
    color: var(--cyber-cyan);
    font-size: 18px;
    margin: 8px 0;
  }

  .detail-divider {
    height: 1px;
    background: var(--cyber-border);
    margin: 10px 0;
  }

  .detail-row.total {
    font-weight: bold;
    font-size: 14px;
  }

  .confirm-actions {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px;
  }

  /* STATUS VIEWS */
  .status-view {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 40px 20px;
    text-align: center;
  }

  .status-icon {
    width: 60px;
    height: 60px;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 28px;
    margin-bottom: 20px;
  }

  .status-icon.sending {
    border: 2px solid var(--cyber-cyan);
  }

  .status-icon.success {
    border: 2px solid var(--cyber-cyan);
    color: var(--cyber-cyan);
  }

  .status-icon.error {
    border: 2px solid #ff6b6b;
    color: #ff6b6b;
  }

  .spinner {
    width: 30px;
    height: 30px;
    border: 2px solid var(--cyber-border);
    border-top-color: var(--cyber-cyan);
    border-radius: 50%;
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  .status-title {
    font-size: 16px;
    font-weight: bold;
    color: var(--cyber-text);
    margin-bottom: 15px;
  }

  .status-title.success {
    color: var(--cyber-cyan);
  }

  .status-title.error {
    color: #ff6b6b;
  }

  .status-message {
    font-size: 12px;
    color: var(--cyber-text-dim);
    margin-bottom: 20px;
  }

  .error-msg {
    color: #ff6b6b;
    background: rgba(255, 0, 0, 0.1);
    padding: 10px;
    border: 1px solid #ff6b6b;
    width: 100%;
  }

  .status-steps {
    text-align: left;
    font-size: 12px;
  }

  .step {
    padding: 4px 0;
    color: var(--cyber-text-dim);
    transition: color 0.3s;
  }

  .step.active {
    color: var(--cyber-text);
  }

  .step.done {
    color: var(--cyber-cyan);
  }

  .tx-hash {
    background: var(--cyber-bg-alt);
    border: 1px solid var(--cyber-border);
    padding: 12px;
    width: 100%;
    margin-bottom: 20px;
  }

  .tx-label {
    display: block;
    font-size: 10px;
    color: var(--cyber-text-dim);
    margin-bottom: 4px;
  }

  .tx-value {
    font-size: 12px;
    color: var(--cyber-cyan);
  }

  .error-actions {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px;
    width: 100%;
    margin-top: 20px;
  }

  .button.primary {
    background: rgba(0, 255, 159, 0.1);
    border-color: var(--cyber-cyan);
  }

  .warning {
    background: rgba(255, 214, 10, 0.1);
    border: 1px solid var(--cyber-yellow);
    color: var(--cyber-yellow);
    padding: 10px;
    font-size: 11px;
    text-align: center;
  }
</style>
