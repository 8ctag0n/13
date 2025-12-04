<script>
  import { onMount, onDestroy } from 'svelte';

  const API_BASE = import.meta.env.VITE_API_URL || '';

  let stats = [
    { id: 'provers', icon: '[#]', label: 'ACTIVE_PROVERS', value: 0, target: 0, color: 'cyan' },
    { id: 'jobs', icon: '[>]', label: 'JOBS_COMPLETED', value: '0/0', color: 'violet', isString: true },
    { id: 'data', icon: '[=]', label: 'DATA_PROCESSED', value: '0 B', color: 'cyan', isString: true },
    { id: 'uptime', icon: '[*]', label: 'NETWORK_UPTIME', value: 0, target: 99.97, suffix: '%', color: 'success' }
  ];

  // Animate counter
  function animateValue(stat, target, duration = 1000) {
    if (stat.isString) return; // Don't animate string values
    const start = stat.value;
    const change = target - start;
    const startTime = performance.now();

    function update(currentTime) {
      const elapsed = currentTime - startTime;
      const progress = Math.min(elapsed / duration, 1);

      // Easing function
      const eased = 1 - Math.pow(1 - progress, 3);
      stat.value = start + change * eased;

      if (progress < 1) {
        requestAnimationFrame(update);
      } else {
        stat.value = target;
      }
      stats = stats; // Trigger reactivity
    }

    requestAnimationFrame(update);
  }

  // Fetch stats from backend
  async function fetchStats() {
    try {
      const response = await fetch(`${API_BASE}/api/stats/network`);
      if (response.ok) {
        const data = await response.json();

        // Update targets
        stats[0].target = data.active_provers || 0;
        // Jobs: show completed/total for clarity
        const completed = data.jobs_completed || 0;
        const total = data.jobs_total || 0;
        stats[1].value = `${completed}/${total}`;
        // Data uses pre-formatted string from backend (adaptive MB/GB/TB)
        stats[2].value = data.data_encrypted_formatted || '0 B';
        stats[3].target = data.uptime_percent || 99.97;

        // Animate numeric values
        stats.forEach(stat => {
          if (!stat.isString && stat.value !== stat.target) {
            animateValue(stat, stat.target);
          }
        });

        stats = stats; // Trigger reactivity for string values
      }
    } catch (error) {
      console.error('Failed to fetch stats:', error);
    }
  }

  // Format value for display
  function formatValue(stat) {
    if (stat.isString) {
      return stat.value; // Already formatted by backend
    }
    if (stat.id === 'uptime') {
      return stat.value.toFixed(2);
    }
    return Math.floor(stat.value).toLocaleString();
  }

  let interval;

  onMount(() => {
    fetchStats();
    interval = setInterval(fetchStats, 30000);
  });

  onDestroy(() => {
    if (interval) clearInterval(interval);
  });
</script>

<div class="stats-grid">
  {#each stats as stat, i}
    <div class="floating-stat glass-card color-{stat.color}" style="animation-delay: {i * 0.1}s">
      <div class="stat-icon text-{stat.color}">{stat.icon}</div>
      <div class="stat-value text-mono">
        <span class="value text-{stat.color}">{formatValue(stat)}</span>
        {#if stat.suffix}
          <span class="suffix text-muted">{stat.suffix}</span>
        {/if}
      </div>
      <div class="stat-label text-mono text-xs text-muted">{stat.label}</div>
      <div class="pulse-indicator"></div>
    </div>
  {/each}
</div>

<style>
  .stats-grid {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: var(--space-4);
    margin: var(--space-6) 0;
  }

  @media (max-width: 1024px) {
    .stats-grid {
      grid-template-columns: repeat(2, 1fr);
    }
  }

  @media (max-width: 480px) {
    .stats-grid {
      grid-template-columns: 1fr;
    }
  }

  .floating-stat {
    position: relative;
    background: rgba(15, 23, 42, 0.6);
    backdrop-filter: blur(20px);
    border: 1px solid var(--zyber-border-muted);
    border-radius: var(--radius-lg);
    padding: var(--space-4);
    text-align: center;
    transition: all 0.4s cubic-bezier(0.4, 0, 0.2, 1);
    animation: float-in 0.6s ease-out backwards;
    overflow: hidden;
  }

  @keyframes float-in {
    from {
      opacity: 0;
      transform: translateY(20px) scale(0.95);
    }
    to {
      opacity: 1;
      transform: translateY(0) scale(1);
    }
  }

  .floating-stat:hover {
    transform: translateY(-8px) scale(1.02);
    border-color: var(--zyber-border-primary);
  }

  .floating-stat.color-cyan {
    border-color: rgba(6, 182, 212, 0.3);
  }

  .floating-stat.color-cyan:hover {
    border-color: rgba(6, 182, 212, 0.6);
    box-shadow:
      0 20px 40px rgba(0, 0, 0, 0.3),
      0 0 40px rgba(6, 182, 212, 0.3);
  }

  .floating-stat.color-violet {
    border-color: rgba(139, 92, 246, 0.3);
  }

  .floating-stat.color-violet:hover {
    border-color: rgba(139, 92, 246, 0.6);
    box-shadow:
      0 20px 40px rgba(0, 0, 0, 0.3),
      0 0 40px rgba(139, 92, 246, 0.3);
  }

  .floating-stat.color-success {
    border-color: rgba(16, 185, 129, 0.3);
  }

  .floating-stat.color-success:hover {
    border-color: rgba(16, 185, 129, 0.6);
    box-shadow:
      0 20px 40px rgba(0, 0, 0, 0.3),
      0 0 40px rgba(16, 185, 129, 0.3);
  }

  .stat-icon {
    font-family: var(--font-mono);
    font-size: var(--text-2xl);
    margin-bottom: var(--space-2);
  }

  .stat-value {
    font-size: var(--text-3xl);
    font-weight: 600;
    margin-bottom: var(--space-1);
  }

  .stat-value .value {
    font-variant-numeric: tabular-nums;
  }

  .stat-value .suffix {
    font-size: var(--text-lg);
  }

  .stat-label {
    letter-spacing: 0.05em;
  }

  .pulse-indicator {
    position: absolute;
    bottom: 8px;
    left: 50%;
    transform: translateX(-50%);
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--zyber-success);
    animation: pulse-indicator 2s ease-in-out infinite;
  }

  @keyframes pulse-indicator {
    0%, 100% { opacity: 0.5; }
    50% { opacity: 1; }
  }

  .text-cyan { color: var(--zyber-cyber-cyan); }
  .text-violet { color: var(--zyber-quantum-violet); }
  .text-success { color: var(--zyber-success); }
  .text-muted { color: var(--zyber-text-muted); }
</style>
