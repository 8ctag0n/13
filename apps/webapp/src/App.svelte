<script>
  import { currentRoute, isPatternRoute, getRouteParams } from './lib/stores/router';
  import { walletStore } from './lib/stores/wallet';
  import Landing from './lib/pages/Landing.svelte';
  import Dashboard from './lib/pages/Dashboard.svelte';
  import CreateJob from './lib/pages/CreateJob.svelte';
  import MetricsDashboard from './lib/pages/MetricsDashboard.svelte';
  import MyJobs from './lib/pages/MyJobs.svelte';
  import JobDetails from './lib/pages/JobDetails.svelte';
  import Analytics from './lib/pages/Analytics.svelte';
  import ProofOfInnocence from './lib/pages/ProofOfInnocence.svelte';
  import FutarchyTerminal from './lib/pages/FutarchyTerminal.svelte';
  import Toast from './lib/components/Toast.svelte';
  import { toastStore } from './lib/stores/toast';
  import './styles/tui-system.css';
  import './app.css';

  // Get job ID for job-details route
  $: jobParams = getRouteParams($currentRoute);
</script>

<main>
  {#if $currentRoute === 'landing'}
    <Landing />
  {:else if $currentRoute === 'dashboard'}
    <Dashboard />
  {:else if $currentRoute === 'create-job'}
    <CreateJob />
  {:else if $currentRoute === 'metrics'}
    <MetricsDashboard />
  {:else if $currentRoute === 'my-jobs'}
    <MyJobs />
  {:else if $currentRoute === 'analytics'}
    <Analytics />
  {:else if $currentRoute === 'proof-of-innocence'}
    <ProofOfInnocence />
  {:else if $currentRoute === 'futarchy'}
    <FutarchyTerminal />
  {:else if isPatternRoute($currentRoute, 'job-details')}
    <JobDetails jobId={jobParams.jobId} />
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
