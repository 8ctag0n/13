<script>
  import { onMount } from 'svelte';
  import Logo from '../components/Logo.svelte';
  import HeroBootSequence from '../components/HeroBootSequence.svelte';
  import FHEFlow from '../components/FHEFlow.svelte';
  import StatsBar from '../components/StatsBar.svelte';
  import Timeline from '../components/Timeline.svelte';
  import UseCases from '../components/UseCases.svelte';
  import Footer from '../components/Footer.svelte';
  import GlobalNavigation from '../components/GlobalNavigation.svelte';
  import { navigateTo } from '../stores/router';

  // API config - use relative path for nginx proxy
  const API_BASE = import.meta.env.VITE_API_URL || '';
  let activeProvers = 0;

  onMount(async () => {
    try {
      const response = await fetch(`${API_BASE}/api/stats/network`);
      if (response.ok) {
        const data = await response.json();
        activeProvers = data.active_provers || 0;
      }
    } catch (error) {
      console.error('Failed to load network stats:', error);
    }
  });

  function skipToDemo() {
    navigateTo('dashboard');
  }

  function exploreDocs() {
    // Placeholder for docs navigation
    console.log('Navigate to docs');
  }
</script>

<div class="landing">
  <!-- Global Navigation (Grok style) -->
  <GlobalNavigation />

  <div class="container">
    <!-- Hero Section -->
    <section id="hero" class="hero fade-in">
      <Logo />

      <div class="tagline text-mono text-cyan">
        <span>&gt;</span> COMPUTE_ON_ENCRYPTED_DATA.ZERO_TRUST<span class="cursor-blink"></span>
      </div>

      <!-- Boot Sequence -->
      <HeroBootSequence />

      <!-- Main Hero Card -->
      <div class="hero-card-wrapper">
        <!-- Lock Cipher Watermark -->
        <div class="lock-cipher-bg text-mono">
          <pre>   ╔══════════╗
╔══╝ ENCRYPTED ╚══╗
║  DATA FLOW    ║
╚══╗ PROTECTED ╔══╝
   ╚══════════╝</pre>
        </div>

        <div class="tui-box hero-card">
        <!-- ASCII Corner Decorations -->
        <div class="corner-tl text-mono text-violet">╔═══</div>
        <div class="corner-tr text-mono text-violet">═══╗</div>
        <div class="corner-bl text-mono text-violet">╚═══</div>
        <div class="corner-br text-mono text-violet">═══╝</div>

        <div class="hero-content">
          <div class="status-bar text-mono mb-6">
            <div class="status-item">
              <span class="text-muted">&gt; STATUS......:</span>
              <span class="text-success">ONLINE</span>
            </div>
            <div class="status-item">
              <span class="text-muted">&gt; ENCRYPTION..:</span>
              <span class="text-cyan">FULLY_HOMOMORPHIC</span>
            </div>
            <div class="status-item">
              <span class="text-muted">&gt; NETWORK.....:</span>
              <span class="text-cyan">SOLANA_MAINNET</span>
            </div>
            <div class="status-item">
              <span class="text-muted">&gt; PROVERS.....:</span>
              <span class="text-cyan">{activeProvers}_ACTIVE</span>
            </div>
          </div>

          <div class="info-box mb-6">
            <div class="text-mono text-uppercase text-center">
              <div class="mb-4">YOUR_DATA_STAYS_ENCRYPTED_END_TO_END</div>
              <div>EVEN_PROVERS_NEVER_SEE_YOUR_PLAINTEXT</div>
            </div>
          </div>

          <div class="features mb-8">
            <h3 class="text-mono text-uppercase mb-4">FEATURES:</h3>
            <div class="feature-list text-mono">
              <div class="feature-item">
                <span class="text-success">[✓]</span> Fully Homomorphic Encryption
              </div>
              <div class="feature-item">
                <span class="text-success">[✓]</span> Decentralized Compute Network
              </div>
              <div class="feature-item">
                <span class="text-success">[✓]</span> On-Chain Settlement (Solana)
              </div>
              <div class="feature-item">
                <span class="text-success">[✓]</span> Zero-Knowledge Proofs
              </div>
            </div>
          </div>

          <!-- CTA Section -->
          <div class="cta-section">
            <div class="cta-buttons">
              <button class="btn btn-secondary" on:click={skipToDemo}>
                <span class="text-mono">[EXPLORE_JOBS →]</span>
              </button>
              <button class="btn btn-ghost" on:click={exploreDocs}>
                <span class="text-mono">[READ_DOCS]</span>
              </button>
            </div>

            <div class="text-xs text-muted text-center mt-4">
              <span class="text-mono">&gt;</span> NO_WALLET_REQUIRED_FOR_EXPLORATION
            </div>
          </div>
        </div>
      </div>
      </div>
    </section>

    <!-- Stats Banner -->
    <div id="stats">
      <StatsBar />
    </div>

    <!-- How It Works -->
    <section id="how-it-works" class="hero fade-in">
      <div class="how-it-works mt-8">
        <div class="divider text-mono text-muted">
          ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
        </div>

        <h3 class="text-mono text-uppercase mt-6 mb-6">HOW_IT_WORKS:</h3>

        <!-- FHE Flow Visualization -->
        <FHEFlow animated={true} />

        <!-- Traditional Steps (kept for reference) -->
        <div class="steps mt-8">
          <div class="step">
            <div class="step-box tui-box-cyan">
              <div class="step-number text-mono text-cyan">STEP_1</div>
              <div class="step-label text-mono">ENCRYPT</div>
              <div class="step-desc text-sm text-muted">LOCAL</div>
            </div>
          </div>

          <div class="step-arrow text-cyan text-mono">=&gt;</div>

          <div class="step">
            <div class="step-box tui-box-cyan">
              <div class="step-number text-mono text-cyan">STEP_2</div>
              <div class="step-label text-mono">COMPUTE</div>
              <div class="step-desc text-sm text-muted">REMOTE</div>
            </div>
          </div>

          <div class="step-arrow text-cyan text-mono">=&gt;</div>

          <div class="step">
            <div class="step-box tui-box-cyan">
              <div class="step-number text-mono text-cyan">STEP_3</div>
              <div class="step-label text-mono">DECRYPT</div>
              <div class="step-desc text-sm text-muted">LOCAL</div>
            </div>
          </div>
        </div>

        <!-- Terminal Prompt -->
        <div class="terminal-prompt text-mono text-muted mt-8">
          &gt; READY_FOR_INPUT<span class="cursor-blink"></span>
        </div>
      </div>
    </section>

    <!-- Use Cases -->
    <div id="use-cases">
      <UseCases />
    </div>

    <!-- Privacy Timeline -->
    <div id="timeline">
      <Timeline />
    </div>

    <!-- Footer -->
    <div id="footer">
      <Footer />
    </div>
  </div>
</div>

<style>
  .landing {
    width: 100%;
    min-height: 100vh;
    padding: var(--space-8) 0;
  }

  .hero {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-6);
  }

  .tagline {
    font-size: var(--text-lg);
    margin-bottom: var(--space-4);
  }

  .hero-card-wrapper {
    width: 100%;
    max-width: 900px;
    position: relative;
  }

  .lock-cipher-bg {
    position: absolute;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    opacity: 0.05;
    z-index: 0;
    pointer-events: none;
    font-size: var(--text-xl);
    color: var(--zyber-cyan);
    text-shadow: 0 0 30px var(--zyber-cyan);
    line-height: 1.4;
  }

  .hero-card {
    width: 100%;
    position: relative;
    padding: var(--space-12);
    z-index: 1;
  }

  /* ASCII Corner Decorations */
  .corner-tl, .corner-tr, .corner-bl, .corner-br {
    position: absolute;
    font-size: var(--text-lg);
    text-shadow: var(--zyber-glow-violet);
    z-index: 10;
  }

  .corner-tl {
    top: -2px;
    left: -2px;
  }

  .corner-tr {
    top: -2px;
    right: -2px;
  }

  .corner-bl {
    bottom: -2px;
    left: -2px;
  }

  .corner-br {
    bottom: -2px;
    right: -2px;
  }

  .hero-content {
    position: relative;
    z-index: 1;
  }

  .status-bar {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    font-size: var(--text-sm);
  }

  .status-item {
    display: flex;
    gap: var(--space-4);
  }

  .info-box {
    background: rgba(6, 182, 212, 0.05);
    border: 1px solid var(--zyber-border-secondary);
    border-radius: var(--radius-md);
    padding: var(--space-6);
  }

  .features {
    text-align: left;
  }

  .feature-list {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  .feature-item {
    font-size: var(--text-sm);
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }

  .cta-section {
    width: 100%;
  }

  .cta-buttons {
    display: flex;
    gap: var(--space-4);
    justify-content: center;
    align-items: center;
    flex-wrap: wrap;
  }

  .cta-buttons .btn {
    flex: 1;
    min-width: 180px;
    max-width: 220px;
  }

  .divider {
    overflow: hidden;
    text-align: center;
    font-size: var(--text-xs);
    opacity: 0.3;
  }

  .how-it-works {
    width: 100%;
    max-width: 900px;
  }

  .steps {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: var(--space-4);
    flex-wrap: wrap;
  }

  .step {
    flex: 1;
    min-width: 150px;
    max-width: 200px;
  }

  .step-box {
    padding: var(--space-6);
    text-align: center;
    border: 2px solid var(--zyber-border-secondary);
    border-radius: var(--radius-md);
    background: rgba(6, 182, 212, 0.05);
    transition: all var(--transition-base);
  }

  .step-box:hover {
    transform: translateY(-4px);
    box-shadow: var(--zyber-glow-cyan);
  }

  .step-number {
    font-weight: 600;
    margin-bottom: var(--space-2);
  }

  .step-label {
    font-size: var(--text-lg);
    font-weight: 600;
    margin-bottom: var(--space-1);
  }

  .step-desc {
    text-transform: uppercase;
  }

  .step-arrow {
    font-size: var(--text-2xl);
    flex-shrink: 0;
    animation: pulse 2s ease-in-out infinite;
  }

  .terminal-prompt {
    font-size: var(--text-sm);
  }

  /* Responsive */
  @media (max-width: 768px) {
    .hero-card {
      padding: var(--space-6);
    }

    .status-bar {
      font-size: var(--text-xs);
    }

    .steps {
      flex-direction: column;
    }

    .step {
      max-width: 100%;
      width: 100%;
    }

    .step-arrow {
      transform: rotate(90deg);
    }
  }
</style>
