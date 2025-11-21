<script>
  import { currentRoute } from './lib/stores/router';
  import { walletStore } from './lib/stores/wallet';
  import Landing from './lib/pages/Landing.svelte';
  import Dashboard from './lib/pages/Dashboard.svelte';
  import CreateJob from './lib/pages/CreateJob.svelte';
  import Toast from './lib/components/Toast.svelte';
  import { toastStore } from './lib/stores/toast';
  import './styles/tui-system.css';
  import './app.css';

  // Auto-navigate to dashboard when wallet connects (optional - user can skip)
  // $: if ($walletStore.connected && $currentRoute === 'landing') {
  //   currentRoute.set('dashboard');
  // }
</script>

<main>
  {#if $currentRoute === 'landing'}
    <Landing />
  {:else if $currentRoute === 'dashboard'}
    <Dashboard />
  {:else if $currentRoute === 'create-job'}
    <CreateJob />
  {/if}
</main>

<!-- Global Toast Notifications -->
<div class="toast-container">
  {#each $toastStore as toast (toast.id)}
    <Toast
      message={toast.message}
      type={toast.type}
      duration={toast.duration}
      onClose={() => toastStore.remove(toast.id)}
    />
  {/each}
</div>

<style>
  .toast-container {
    position: fixed;
    top: var(--space-6);
    right: var(--space-6);
    z-index: 9999;
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    pointer-events: none;
  }

  .toast-container :global(.toast) {
    pointer-events: auto;
  }

  @media (max-width: 768px) {
    .toast-container {
      top: var(--space-4);
      right: var(--space-4);
      left: var(--space-4);
    }
  }
</style>
