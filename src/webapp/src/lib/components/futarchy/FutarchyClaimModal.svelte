<script>
  import { createEventDispatcher } from 'svelte';
  import TerminalBox from './terminal/TerminalBox.svelte';
  import TerminalButton from './terminal/TerminalButton.svelte';
  import FutarchyTxStatus from './FutarchyTxStatus.svelte';

  const dispatch = createEventDispatcher();

  export let open = false;
  export let position = null;
  export let claimAmount = '';
  export let marketTitle = '';

  let txState = 'idle';
  let txMessage = '';
  let txError = '';
  let busy = false;

  function resetState() {
    txState = 'idle';
    txMessage = '';
    txError = '';
    busy = false;
  }

  function handleClose() {
    resetState();
    dispatch('close');
  }

  async function handleClaim() {
    if (busy) return;
    busy = true;
    txState = 'signing';
    txMessage = 'Submitting claim...';

    try {
      dispatch('claim', { position });
      txState = 'success';
      txMessage = 'Claim submitted';
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
