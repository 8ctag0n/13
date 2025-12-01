<script lang="ts">
  import { createEventDispatcher } from 'svelte';

  const dispatch = createEventDispatcher();

  let password = '';
  let error = '';
  let loading = false;

  async function handleUnlock() {
    if (!password) {
      error = 'Password is required';
      return;
    }

    loading = true;
    error = '';

    try {
      const response = await chrome.runtime.sendMessage({
        type: 'UNLOCK_WALLET',
        password
      });

      if (response.success) {
        dispatch('unlocked', response.data);
      } else {
        error = response.error || 'Failed to unlock wallet';
      }
    } catch (err) {
      error = err instanceof Error ? err.message : 'Unknown error';
    } finally {
      loading = false;
    }
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Enter') {
      handleUnlock();
    }
  }
</script>

<div class="header">
  <div class="logo">[ZYBERLINK]</div>
  <div class="subtitle">Multi-Chain Wallet</div>
</div>

<div class="card">
  <label class="label" for="password">[PASSWORD]</label>
  <input
    id="password"
    type="password"
    class="input"
    bind:value={password}
    on:keydown={handleKeydown}
    placeholder="Enter your password..."
    disabled={loading}
  />

  {#if error}
    <div class="error">[ERROR] {error}</div>
  {/if}

  <button
    class="button"
    on:click={handleUnlock}
    disabled={loading || !password}
  >
    {loading ? '[UNLOCKING...]' : '[UNLOCK WALLET]'}
  </button>
</div>

<div class="warning">
  <strong>[WARNING]</strong> Never share your password or seed phrase with anyone.
</div>
