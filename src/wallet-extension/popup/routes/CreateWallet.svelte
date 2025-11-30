<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { generatePrivateKey, bytesToHex, createKeypair, getExpandedPrivateKey } from '../../lib/crypto/keyring';
  import bs58 from 'bs58';

  const dispatch = createEventDispatcher();

  type Step = 'create' | 'privatekey' | 'password';

  let step: Step = 'create';
  let password = '';
  let confirmPassword = '';
  let privateKeyBytes: Uint8Array | null = null;
  let confirmed = false;
  let error = '';
  let loading = false;
  let copied = false;
  let showAs: 'hex' | 'base58' = 'hex';

  // Computed values
  $: privateKeyHex = privateKeyBytes ? bytesToHex(privateKeyBytes) : '';
  $: expandedKey = privateKeyBytes ? getExpandedPrivateKey(createKeypair(privateKeyBytes)) : null;
  $: expandedKeyHex = expandedKey ? bytesToHex(expandedKey) : '';
  $: expandedKeyBase58 = expandedKey ? bs58.encode(expandedKey) : '';
  $: displayKey = showAs === 'hex' ? expandedKeyHex : expandedKeyBase58;

  function handleCreateNew() {
    privateKeyBytes = generatePrivateKey();
    step = 'privatekey';
  }

  function handleConfirmPrivateKey() {
    if (!confirmed) {
      error = 'Please confirm you have saved your private key';
      return;
    }
    step = 'password';
  }

  function handleBack() {
    if (step === 'privatekey') {
      step = 'create';
      privateKeyBytes = null;
      confirmed = false;
    } else if (step === 'password') {
      step = 'privatekey';
      password = '';
      confirmPassword = '';
    } else if (step === 'create') {
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
        type: 'CREATE_WALLET',
        password,
        privateKey: expandedKeyHex
      });

      if (response.success) {
        dispatch('created', response.data);
      } else {
        error = response.error || 'Failed to create wallet';
      }
    } catch (err) {
      error = err instanceof Error ? err.message : 'Unknown error';
    } finally {
      loading = false;
    }
  }

  function copyPrivateKey() {
    navigator.clipboard.writeText(displayKey);
    copied = true;
    setTimeout(() => copied = false, 2000);
  }

  function toggleFormat() {
    showAs = showAs === 'hex' ? 'base58' : 'hex';
  }
</script>

<div class="header">
  <div class="logo">[ZYBERLINK]</div>
  <div class="subtitle">Multi-Chain Wallet</div>
</div>

{#if step === 'create'}
  <div class="card">
    <h2 style="color: var(--cyber-cyan); margin-bottom: 20px; text-align: center;">
      [CREATE NEW WALLET]
    </h2>

    <p style="color: var(--cyber-text-dim); font-size: 12px; text-align: center; margin-bottom: 20px;">
      Generate a new private key for your multi-chain wallet.
      This key will work across Solana, Starknet, and Zcash.
    </p>

    <button class="button" on:click={handleCreateNew}>
      [GENERATE PRIVATE KEY]
    </button>

    <button class="button secondary" on:click={handleBack} style="margin-top: 10px;">
      [← BACK]
    </button>
  </div>

{:else if step === 'privatekey'}
  <div class="card">
    <h2 style="color: var(--cyber-yellow); margin-bottom: 15px; text-align: center;">
      [PRIVATE KEY]
    </h2>

    <div class="warning">
      <strong>[CRITICAL]</strong> Save this private key securely.
      Never share it with anyone. This is the ONLY way to recover your wallet.
    </div>

    <div class="info-box">
      <strong>[WHY PRIVATE KEY?]</strong>
      Unlike seed phrases, a raw private key is universally compatible
      with Solana, Starknet, Zcash, and our FHE system. Same key everywhere.
    </div>

    <div class="format-toggle">
      <button
        class="toggle-btn"
        class:active={showAs === 'hex'}
        on:click={() => showAs = 'hex'}
      >
        [HEX]
      </button>
      <button
        class="toggle-btn"
        class:active={showAs === 'base58'}
        on:click={() => showAs = 'base58'}
      >
        [BASE58]
      </button>
    </div>

    <div class="privatekey-display">
      <div class="key-label">[EXPANDED PRIVATE KEY - 64 BYTES]</div>
      <div class="key-value">{displayKey}</div>
    </div>

    <button class="button secondary" on:click={copyPrivateKey}>
      {copied ? '[COPIED!]' : '[COPY TO CLIPBOARD]'}
    </button>

    <label style="display: flex; align-items: center; margin: 20px 0; cursor: pointer;">
      <input type="checkbox" bind:checked={confirmed} style="margin-right: 10px;">
      <span style="font-size: 12px;">I have saved my private key securely</span>
    </label>

    {#if error}
      <div class="error">[ERROR] {error}</div>
    {/if}

    <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 10px;">
      <button class="button secondary" on:click={handleBack}>
        [BACK]
      </button>
      <button class="button" on:click={handleConfirmPrivateKey} disabled={!confirmed}>
        [CONTINUE]
      </button>
    </div>
  </div>

{:else if step === 'password'}
  <div class="card">
    <h2 style="color: var(--cyber-cyan); margin-bottom: 15px; text-align: center;">
      [SET PASSWORD]
    </h2>

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
        {loading ? '[CREATING...]' : '[CREATE WALLET]'}
      </button>
    </div>
  </div>
{/if}
