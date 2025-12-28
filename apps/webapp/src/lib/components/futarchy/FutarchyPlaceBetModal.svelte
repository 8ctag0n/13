<script>
  import { createEventDispatcher } from 'svelte';
  import { walletStore } from '../../stores/wallet';
  import TerminalBox from './terminal/TerminalBox.svelte';
  import TerminalButton from './terminal/TerminalButton.svelte';
  import TerminalInput from './terminal/TerminalInput.svelte';
  import TerminalRadio from './terminal/TerminalRadio.svelte';
  import FutarchyTxStatus from './FutarchyTxStatus.svelte';
  import { validatePlaceBet } from '../../utils/futarchy_api';
  import { signAndSendUnsignedTx } from '../../utils/futarchy_tx';

  const dispatch = createEventDispatcher();

  export let open = false;
  export let market = null;
  export let apiBaseUrl = '';
  export let bettor = '';
  export let walletBalance = '';
  export let maxBet = 0;
  export let minBet = 0.1;
  export let quickAmounts = [1, 5, 10];
  export let currency = 'SOL';
  export let encryptedAmount = '';
  export let ciphertextHash = '';
  export let betCommitment = '';
  export let proof = '';
  export let publicInputs = '';
  export let circuitType = 50;
  export let fheJobId = null;
  export let showPrivacyNote = true;

  let amount = '';
  let selectedSide = 'yes';
  let txState = 'idle';
  let txMessage = '';
  let txError = '';
  let txHash = '';
  let confirmations = 0;
  let busy = false;

  const lamportsPerSol = 1_000_000_000;

  $: activeBettor = bettor || $walletStore.addresses?.solana || $walletStore.publicKey?.toString() || '';
  $: amountValue = Number(amount);
  $: amountLamports = Number.isFinite(amountValue) ? Math.round(amountValue * lamportsPerSol) : 0;
  $: maxBetLamports = maxBet ? Math.round(maxBet * lamportsPerSol) : 0;
  $: hasMarket = !!market;

  function resetState() {
    amount = '';
    selectedSide = 'yes';
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

  function setQuickAmount(value) {
    amount = String(value);
  }

  async function handleSubmit() {
    if (busy) return;

    if (!activeBettor) {
      txState = 'error';
      txError = 'Connect wallet to place a bet';
      return;
    }

    if (!Number.isFinite(amountValue) || amountValue <= 0) {
      txState = 'error';
      txError = 'Enter a valid bet amount';
      return;
    }

    if (maxBetLamports && amountLamports > maxBetLamports) {
      txState = 'error';
      txError = 'Bet exceeds max allowed';
      return;
    }

    if (!market?.id) {
      txState = 'error';
      txError = 'Market not available';
      return;
    }

    if (!proof || !publicInputs) {
      txState = 'error';
      txError = 'Missing proof or public inputs';
      return;
    }

    if (!betCommitment && !ciphertextHash && !encryptedAmount) {
      txState = 'error';
      txError = 'Missing bet commitment or ciphertext hash';
      return;
    }

    const circuitValue = Number(circuitType);
    if (!Number.isFinite(circuitValue)) {
      txState = 'error';
      txError = 'Invalid circuit type';
      return;
    }

    busy = true;
    txState = 'signing';
    txMessage = 'Validating bet...';

    try {
      const payload = {
        bettor: activeBettor,
        side: selectedSide,
        amount_lamports: amountLamports,
        encrypted_amount: encryptedAmount || undefined,
        ciphertext_hash: ciphertextHash || undefined,
        bet_commitment: betCommitment || undefined,
        proof,
        public_inputs: publicInputs,
        circuit_type: circuitValue,
        fhe_job_id: fheJobId ?? undefined
      };

      const response = await validatePlaceBet(market.id, payload, { apiBaseUrl: apiBaseUrl || undefined });
      const unsignedTx = response?.unsigned_transaction;

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
        }
      });

      txState = 'success';
      txMessage = 'Bet confirmed on-chain!';
      txHash = result.signature;
      dispatch('success', { response, signature: result.signature, marketId: market.id });
    } catch (error) {
      txState = 'error';
      txError = error?.message || 'Failed to place bet';
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
  <div class="modal-overlay" on:click={handleClose}>
    <div class="modal" on:click|stopPropagation>
      <TerminalBox tone="violet">
        <div class="modal-title text-mono">▓▓ PLACE BET ▓▓</div>

        <div class="market-summary">
          <div class="summary-title text-mono text-xs">MARKET: {market?.title || market?.question || 'Unknown market'}</div>
          <div class="summary-meta text-mono text-xs text-muted">
            STATUS: [*] active
            <span class="divider">│</span>
            EXPIRES: {market?.expiresIn || market?.ends_at || '--'}
            <span class="divider">│</span>
            ID: {market?.id || '--'}
          </div>
        </div>

        <div class="step-box">
          <div class="step-title text-mono text-xs">STEP 1/2: SELECT SIDE</div>
          <div class="side-options">
            <TerminalRadio
              name="side"
              value="yes"
              label="YES"
              description={`current_pool: ${market?.yes?.pool || '--'} | percentage: ${market?.yes?.percent || 0}%`}
              checked={selectedSide === 'yes'}
              on:change={() => (selectedSide = 'yes')}
            />
            <TerminalRadio
              name="side"
              value="no"
              label="NO"
              description={`current_pool: ${market?.no?.pool || '--'} | percentage: ${market?.no?.percent || 0}%`}
              checked={selectedSide === 'no'}
              on:change={() => (selectedSide = 'no')}
            />
          </div>
        </div>

        <div class="step-box">
          <div class="step-title text-mono text-xs">STEP 2/2: ENTER AMOUNT</div>
          <TerminalInput
            label="bet_amount"
            bind:value={amount}
            placeholder="0.0"
            suffix={currency}
            prefix=">"
          />
          <div class="meta-row text-mono text-xs text-muted">
            wallet_balance: {walletBalance || '--'}
            <span class="divider">│</span>
            max_bet: {maxBet ? `${maxBet} ${currency}` : '--'}
            <span class="divider">│</span>
            min_bet: {minBet} {currency}
          </div>
          <div class="quick-amounts">
            {#each quickAmounts as quick}
              <TerminalButton label={`${quick} ${currency}`} tone="muted" size="sm" on:click={() => setQuickAmount(quick)} />
            {/each}
            <TerminalButton label="MAX" tone="muted" size="sm" on:click={() => setQuickAmount(maxBet || '')} />
          </div>
          {#if showPrivacyNote}
            <div class="privacy-note text-mono text-xs">
              [!] PRIVACY: Your bet amount will be encrypted via FHE
              <div class="text-muted">Only total pool amounts are visible. Individual bets: PRIVATE</div>
            </div>
          {/if}
        </div>

        <div class="step-box">
          <div class="step-title text-mono text-xs">CALCULATION</div>
          <div class="calc-line text-mono text-xs">your_bet: {amount || '--'} {currency} ({selectedSide.toUpperCase()} side)</div>
          <div class="calc-line text-mono text-xs">if_you_win: ~-- {currency} (estimated)</div>
          <div class="calc-line text-mono text-xs">potential_profit: --</div>
          <div class="calc-line text-mono text-xs">network_fee: ~--</div>
          <div class="calc-line text-mono text-xs">total_cost: {amount || '--'} {currency}</div>
        </div>

        <div class="modal-actions">
          <TerminalButton label="CANCEL" tone="muted" size="sm" on:click={handleClose} />
          <TerminalButton label="SIGN & SUBMIT >" tone="cyan" size="sm" on:click={handleSubmit} />
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
    </div>
  </div>
{/if}

<style>
  .modal-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.7);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: var(--z-modal-backdrop);
    padding: var(--space-6);
  }

  .modal {
    width: min(900px, 95vw);
    max-height: 95vh;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }

  .modal-title {
    text-align: center;
    letter-spacing: 0.12em;
    margin-bottom: var(--space-3);
  }

  .market-summary {
    border: 1px solid var(--zyber-border-muted);
    padding: var(--space-3);
    background: rgba(0, 0, 0, 0.35);
    margin-bottom: var(--space-4);
  }

  .summary-meta {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
  }

  .divider {
    opacity: 0.4;
  }

  .step-box {
    border: 1px solid var(--zyber-border-muted);
    padding: var(--space-3);
    background: rgba(0, 0, 0, 0.35);
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
    grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
    gap: var(--space-3);
  }

  .meta-row {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
  }

  .quick-amounts {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
  }

  .privacy-note {
    border: 1px dashed var(--zyber-border-secondary);
    padding: var(--space-2) var(--space-3);
    color: var(--zyber-text-secondary);
  }

  .calc-line {
    display: flex;
    gap: var(--space-2);
  }

  .modal-actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-2);
  }

  .status-panel {
    margin-bottom: var(--space-6);
  }

  @media (max-width: 700px) {
    .modal-actions {
      flex-direction: column;
      align-items: stretch;
    }
  }
</style>
