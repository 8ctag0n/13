<script>
  import { createEventDispatcher } from 'svelte';
  import { walletStore } from '../../stores/wallet';
  import TerminalBox from './terminal/TerminalBox.svelte';
  import TerminalButton from './terminal/TerminalButton.svelte';
  import FutarchyTxStatus from './FutarchyTxStatus.svelte';
  import { validateClaimPayout } from '../../utils/futarchy_api';
  import { signAndSendUnsignedTx } from '../../utils/futarchy_tx';

  const dispatch = createEventDispatcher();

  export let open = false;
  export let position = null;
  export let claimAmount = '';
  export let marketTitle = '';
  export let apiBaseUrl = '';

  let txState = 'idle';
  let txMessage = '';
  let txError = '';
  let txHash = '';
  let confirmations = 0;
  let busy = false;

  $: walletAddress = $walletStore.addresses?.solana || $walletStore.publicKey?.toString() || '';

  function resetState() {
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

  async function handleClaim() {
    if (busy) return;

    if (!walletAddress) {
      txState = 'error';
      txError = 'Connect wallet to claim';
      return;
    }

    if (!position?.marketId) {
      txState = 'error';
      txError = 'Position data missing';
      return;
    }

    busy = true;
    txState = 'signing';
    txMessage = 'Validating claim...';

    try {
      const payload = {
        bettor: walletAddress,
        bet_commitment: position.betCommitment || position.commitment
      };

      const response = await validateClaimPayout(position.marketId, payload, {
        apiBaseUrl: apiBaseUrl || undefined
      });

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
      txMessage = 'Payout claimed successfully!';
      txHash = result.signature;
      dispatch('success', { signature: result.signature, position });
    } catch (error) {
      txState = 'error';
      txError = error?.message || 'Claim failed';
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
      <TerminalBox tone="success">
        <div class="modal-title text-mono">▓▓ CLAIM PAYOUT ▓▓</div>

        <div class="summary text-mono text-sm">
          <div>market: {marketTitle || position?.marketTitle || 'Unknown market'}</div>
          <div>position_id: {position?.id || '--'}</div>
          <div>payout: {claimAmount || position?.payout || '--'}</div>
        </div>

        <div class="modal-actions">
          <TerminalButton label="CANCEL" tone="muted" size="sm" on:click={handleClose} />
          <TerminalButton label="CLAIM >" tone="success" size="sm" on:click={handleClaim} />
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
          on:primary={handleClaim}
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
    width: min(640px, 95vw);
    max-height: 90vh;
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

  .summary {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    margin-bottom: var(--space-4);
  }

  .modal-actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-2);
  }

  .status-panel {
    margin-bottom: var(--space-6);
  }

  @media (max-width: 600px) {
    .modal-actions {
      flex-direction: column;
      align-items: stretch;
    }
  }
</style>
