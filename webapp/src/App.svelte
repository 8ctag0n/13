<script>
  import { currentRoute } from './lib/stores/router';
  import { walletStore } from './lib/stores/wallet';
  import Landing from './lib/pages/Landing.svelte';
  import Dashboard from './lib/pages/Dashboard.svelte';
  import CreateJob from './lib/pages/CreateJob.svelte';
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
