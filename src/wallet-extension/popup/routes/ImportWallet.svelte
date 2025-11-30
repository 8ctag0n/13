<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { validatePrivateKey } from '../../lib/crypto/keyring';

  const dispatch = createEventDispatcher();

  type Step = 'import' | 'password';

  let step: Step = 'import';
  let privateKeyInput = '';
  let password = '';
  let confirmPassword = '';
  let error = '';
  let loading = false;

  $: isValidKey = privateKeyInput.trim().length > 0 && validatePrivateKey(privateKeyInput);

  function handleContinue() {
    if (!isValidKey) {
      error = 'Invalid private key format';
      return;
    }
    error = '';
    step = 'password';
  }

  function handleBack() {
    if (step === 'password') {
      step = 'import';
      password = '';
      confirmPassword = '';
    } else {
      dispatch('back');
    }
  }

  async function handleFinish() {
    if (!password || !confirmPassword) {
      error = 'Password is required';
      return;
    }

    if (password !== confirmPassword) {
      error = 'Passwords do not match';
      return;
    }

    if (password.length < 8) {
      error = 'Password must be at least 8 characters';
      return;
    }

    loading = true;
    error = '';

    try {
      const response = await chrome.runtime.sendMessage({
        type: 'IMPORT_WALLET',
        password,
        privateKey: privateKeyInput.trim()
      });

      if (response.success) {
        dispatch('imported', response.data);
      } else {
        error = response.error || 'Failed to import wallet';
      }
    } catch (err) {
      error = err instanceof Error ? err.message : 'Unknown error';
    } finally {
      loading = false;
    }
  }

  function handlePaste() {
    navigator.clipboard.readText().then(text => {
      privateKeyInput = text.trim();
    });
  }
</script>

<div class="header">
  <div class="logo">[ZYBERLINK]</div>
  <div class="subtitle">Import Wallet</div>
</div>

{#if step === 'import'}
  <div class="card">
    <h2 style="color: var(--cyber-purple); margin-bottom: 15px; text-align: center;">
      [IMPORT PRIVATE KEY]
    </h2>

    <div class="info-box">
      <strong>[SUPPORTED FORMATS]</strong>
      • HEX: 64 chars (32 bytes) or 128 chars (64 bytes expanded)
      • BASE58: ~44 chars (32 bytes) or ~88 chars (64 bytes)
    </div>

    <label class="label" for="private-key">[PRIVATE KEY]</label>
    <textarea
      id="private-key"
      class="input textarea"
      bind:value={privateKeyInput}
      placeholder="Paste your private key here..."
      rows="4"
    ></textarea>

    <button class="button secondary small" on:click={handlePaste}>
      [PASTE FROM CLIPBOARD]
    </button>

    {#if privateKeyInput.trim().length > 0}
      <div class="validation-status" class:valid={isValidKey} class:invalid={!isValidKey}>
        {isValidKey ? '[✓] VALID FORMAT' : '[✗] INVALID FORMAT'}
      </div>
    {/if}

    {#if error}
      <div class="error">[ERROR] {error}</div>
    {/if}

    <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 10px; margin-top: 20px;">
      <button class="button secondary" on:click={handleBack}>
        [BACK]
      </button>
      <button class="button" on:click={handleContinue} disabled={!isValidKey}>
        [CONTINUE]
      </button>
    </div>
  </div>

{:else if step === 'password'}
  <div class="card">
    <h2 style="color: var(--cyber-cyan); margin-bottom: 15px; text-align: center;">
      [SET PASSWORD]
    </h2>

    <div class="info-box">
      <strong>[ENCRYPTION]</strong>
      Your private key will be encrypted with AES-256-GCM
      and stored locally. Only you can decrypt it.
    </div>

    <label class="label" for="new-password">[NEW PASSWORD]</label>
    <input
      id="new-password"
      type="password"
      class="input"
      bind:value={password}
      placeholder="Min. 8 characters..."
      disabled={loading}
    />

    <label class="label" for="confirm-password">[CONFIRM PASSWORD]</label>
    <input
      id="confirm-password"
      type="password"
      class="input"
      bind:value={confirmPassword}
      placeholder="Re-enter password..."
      disabled={loading}
    />

    {#if error}
      <div class="error">[ERROR] {error}</div>
    {/if}

    <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 10px; margin-top: 20px;">
      <button class="button secondary" on:click={handleBack} disabled={loading}>
        [BACK]
      </button>
      <button class="button" on:click={handleFinish} disabled={loading || !password || !confirmPassword}>
        {loading ? '[IMPORTING...]' : '[IMPORT WALLET]'}
      </button>
    </div>
  </div>
{/if}

<style>
  .textarea {
    resize: none;
    font-size: 12px;
    line-height: 1.5;
  }

  .small {
    padding: 8px 12px;
    font-size: 11px;
    margin: 5px 0;
  }

  .validation-status {
    text-align: center;
    font-size: 12px;
    padding: 8px;
    margin: 10px 0;
    border: 1px solid;
  }

  .validation-status.valid {
    color: var(--cyber-cyan);
    border-color: var(--cyber-cyan);
    background: rgba(0, 255, 159, 0.1);
  }

  .validation-status.invalid {
    color: #ff6b6b;
    border-color: #ff6b6b;
    background: rgba(255, 0, 0, 0.1);
  }
</style>
