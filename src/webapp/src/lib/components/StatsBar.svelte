<script>
  import { onMount } from 'svelte';

  let stats = [
    { label: 'ACTIVE_PROVERS', value: 0, target: 47, suffix: '', color: 'cyan', icon: '[>]' },
    { label: 'JOBS_PROCESSED', value: 0, target: 1247, suffix: '', color: 'violet', icon: '[▓]' },
    { label: 'DATA_ENCRYPTED', value: 0, target: 3.8, suffix: 'TB', color: 'success', icon: '[█]' },
    { label: 'NETWORK_UPTIME', value: 0, target: 99.97, suffix: '%', color: 'cyan', icon: '[●]' }
  ];

  let mounted = false;

  onMount(() => {
    mounted = true;

    // Animate each stat
    stats.forEach((stat, index) => {
      const duration = 2000; // 2 seconds
      const steps = 60;
      const increment = stat.target / steps;
      const stepDuration = duration / steps;

      let currentStep = 0;

      const interval = setInterval(() => {
        if (currentStep < steps) {
          stats[index].value = Math.min(
            stat.target,
            stats[index].value + increment
          );
          currentStep++;
        } else {
          stats[index].value = stat.target;
          clearInterval(interval);
        }
      }, stepDuration);
    });
  });

  function formatValue(value, target, suffix) {
    if (suffix === 'TB' || suffix === '%') {
      return value.toFixed(2);
    }
    return Math.floor(value).toString();
  }
</script>

<section class="stats-section">
  <!-- Network Node Map Background -->
  <div class="network-map-bg">
    <div class="node-map text-mono">
      <pre>  ⬡──⬡──⬡
 /  ╱ ╲  ╲
⬡──⬡──⬡──⬡
 ╲  ╲ ╱  ╱
  ⬡──⬡──⬡</pre>
    </div>
  </div>

  <div class="stats-container">
    <!-- Section Header -->
    <div class="stats-header text-center mb-8">
      <h3 class="text-mono text-uppercase">
        <span class="text-cyan">&gt;</span> NETWORK_STATISTICS
      </h3>
      <div class="text-xs text-muted text-mono mt-2">
        REAL_TIME_METRICS_FROM_DECENTRALIZED_NETWORK
      </div>
    </div>

    <!-- Stats Grid -->
    <div class="stats-grid">
      {#each stats as stat, index}
        <div class="stat-card tui-box" class:mounted>
          <!-- ASCII Corners -->
          <div class="corner-tl text-mono text-{stat.color}">┌─</div>
          <div class="corner-tr text-mono text-{stat.color}">─┐</div>
          <div class="corner-bl text-mono text-{stat.color}">└─</div>
          <div class="corner-br text-mono text-{stat.color}">─┘</div>

          <div class="stat-content">
            <div class="stat-icon text-2xl mb-3">{stat.icon}</div>

            <div class="stat-value text-mono">
              <span class="value-number text-{stat.color}">
                {formatValue(stat.value, stat.target, stat.suffix)}
              </span>
              {#if stat.suffix}
                <span class="value-suffix text-muted">{stat.suffix}</span>
              {/if}
            </div>

            <div class="stat-label text-mono text-xs text-muted mt-2">
              {stat.label}
            </div>

            <!-- Pulse Indicator -->
            <div class="pulse-indicator">
              <div class="pulse-dot bg-{stat.color}"></div>
              <div class="pulse-ring bg-{stat.color}"></div>
            </div>
          </div>
        </div>
      {/each}
    </div>

    <!-- Bottom Info -->
    <div class="stats-footer text-center mt-8">
      <div class="text-xs text-muted text-mono">
        <span class="text-success">[✓]</span> ALL_SYSTEMS_OPERATIONAL
        <span class="mx-3">|</span>
        LAST_UPDATE: <span class="text-cyan">{new Date().toLocaleTimeString()}</span>
      </div>
    </div>
  </div>
</section>

<style>
  .stats-section {
    width: 100%;
    padding: var(--space-12) 0;
    background: linear-gradient(180deg, rgba(139, 92, 246, 0.03) 0%, transparent 100%);
    position: relative;
    overflow: hidden;
  }

  .network-map-bg {
    position: absolute;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    opacity: 0.08;
    z-index: 0;
    pointer-events: none;
  }

  .node-map {
    font-size: var(--text-4xl);
    color: var(--zyber-cyan);
    text-shadow: 0 0 20px var(--zyber-cyan);
    line-height: 1.2;
  }

  .stats-container {
    max-width: 1200px;
    margin: 0 auto;
    padding: 0 var(--space-6);
    position: relative;
    z-index: 1;
  }

  .stats-header h3 {
    font-size: var(--text-xl);
    font-weight: 600;
  }

  .stats-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
    gap: var(--space-6);
  }

  .stat-card {
    position: relative;
    padding: var(--space-8) var(--space-6);
    text-align: center;
    background: rgba(0, 0, 0, 0.3);
    border: 1px solid var(--zyber-border-secondary);
    transition: all var(--transition-base);
    opacity: 0;
    transform: translateY(20px);
  }

  .stat-card.mounted {
    opacity: 1;
    transform: translateY(0);
    animation: fadeInUp 0.6s ease forwards;
  }

  .stat-card:nth-child(1) { animation-delay: 0s; }
  .stat-card:nth-child(2) { animation-delay: 0.1s; }
  .stat-card:nth-child(3) { animation-delay: 0.2s; }
  .stat-card:nth-child(4) { animation-delay: 0.3s; }

  .stat-card:hover {
    transform: translateY(-4px) scale(1.02);
    box-shadow: 0 8px 30px rgba(6, 182, 212, 0.2);
  }

  /* ASCII Corners */
  .corner-tl, .corner-tr, .corner-bl, .corner-br {
    position: absolute;
    font-size: var(--text-sm);
    opacity: 0.7;
    transition: opacity var(--transition-base);
  }

  .stat-card:hover .corner-tl,
  .stat-card:hover .corner-tr,
  .stat-card:hover .corner-bl,
  .stat-card:hover .corner-br {
    opacity: 1;
  }

  .corner-tl { top: 4px; left: 4px; }
  .corner-tr { top: 4px; right: 4px; }
  .corner-bl { bottom: 4px; left: 4px; }
  .corner-br { bottom: 4px; right: 4px; }

  .stat-content {
    position: relative;
    z-index: 1;
  }

  .stat-icon {
    font-family: 'Courier New', monospace;
    font-weight: 600;
    letter-spacing: -0.05em;
  }

  .stat-value {
    font-size: var(--text-3xl);
    font-weight: 600;
    line-height: 1;
  }

  .value-number {
    display: inline-block;
    min-width: 120px;
    text-align: center;
  }

  .value-suffix {
    font-size: var(--text-lg);
    margin-left: var(--space-2);
  }

  .stat-label {
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  /* Pulse Indicator */
  .pulse-indicator {
    position: absolute;
    bottom: var(--space-4);
    right: var(--space-4);
    width: 8px;
    height: 8px;
  }

  .pulse-dot {
    position: absolute;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    z-index: 2;
  }

  .pulse-ring {
    position: absolute;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    opacity: 0.6;
    animation: pulse-ring 2s ease-out infinite;
  }

  /* Color Variants */
  .text-cyan { color: var(--zyber-cyan); }
  .text-violet { color: var(--zyber-violet); }
  .text-success { color: var(--zyber-success); }

  .bg-cyan { background: var(--zyber-cyan); }
  .bg-violet { background: var(--zyber-violet); }
  .bg-success { background: var(--zyber-success); }

  /* Animations */
  @keyframes fadeInUp {
    from {
      opacity: 0;
      transform: translateY(20px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  @keyframes pulse-ring {
    0% {
      transform: scale(1);
      opacity: 0.6;
    }
    100% {
      transform: scale(3);
      opacity: 0;
    }
  }

  /* Responsive */
  @media (max-width: 768px) {
    .stats-grid {
      grid-template-columns: 1fr;
      gap: var(--space-4);
    }

    .stat-value {
      font-size: var(--text-2xl);
    }
  }
</style>
