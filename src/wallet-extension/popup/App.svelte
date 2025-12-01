<script lang="ts">
  import { onMount } from 'svelte';
  import type { WalletState } from '../lib/messaging/types';

  import Welcome from './routes/Welcome.svelte';
  import Unlock from './routes/Unlock.svelte';
  import CreateWallet from './routes/CreateWallet.svelte';
  import ImportWallet from './routes/ImportWallet.svelte';
  import Home from './routes/Home.svelte';

  type AppState = 'loading' | 'welcome' | 'create' | 'import' | 'locked' | 'unlocked';

  let state: AppState = 'loading';
  let walletState: WalletState | null = null;

  async function checkWalletState() {
    try {
      const response = await chrome.runtime.sendMessage({ type: 'GET_WALLET_STATE' });

      if (response.success) {
        walletState = response.data;

        if (!walletState.hasVault) {
          state = 'welcome';
        } else if (walletState.isLocked) {
          state = 'locked';
        } else {
          state = 'unlocked';
        }
      }
    } catch (error) {
      console.error('[App] Failed to check wallet state:', error);
      state = 'welcome';
    }
  }

  function handleNavigate(event: CustomEvent<{ route: string }>) {
    const { route } = event.detail;
    if (route === 'create') {
      state = 'create';
    } else if (route === 'import') {
      state = 'import';
    }
  }

  function handleBack() {
    state = 'welcome';
  }

  function handleWalletCreated(event: CustomEvent) {
    walletState = event.detail;
    state = 'unlocked';
  }

  function handleWalletImported(event: CustomEvent) {
    walletState = event.detail;
    state = 'unlocked';
  }

  function handleWalletUnlocked(event: CustomEvent) {
    walletState = event.detail;
    state = 'unlocked';
  }

  function handleLock() {
    chrome.runtime.sendMessage({ type: 'LOCK_WALLET' });
    state = 'locked';
  }

  onMount(() => {
    checkWalletState();
  });
</script>

<div class="container">
  {#if state === 'loading'}
    <div class="loading">
      [INITIALIZING WALLET]
    </div>
  {:else if state === 'welcome'}
    <Welcome on:navigate={handleNavigate} />
  {:else if state === 'create'}
    <CreateWallet on:created={handleWalletCreated} on:back={handleBack} />
  {:else if state === 'import'}
    <ImportWallet on:imported={handleWalletImported} on:back={handleBack} />
  {:else if state === 'locked'}
    <Unlock on:unlocked={handleWalletUnlocked} />
  {:else if state === 'unlocked' && walletState}
    <Home {walletState} on:lock={handleLock} />
  {/if}
</div>
