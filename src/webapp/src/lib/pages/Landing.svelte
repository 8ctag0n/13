<script>
  import { onMount } from 'svelte';
  import Logo from '../components/Logo.svelte';
  import HeroBootSequence from '../components/HeroBootSequence.svelte';
  import FHEFlow from '../components/FHEFlow.svelte';
  import FloatingStatsCards from '../components/FloatingStatsCards.svelte';
  import Timeline from '../components/Timeline.svelte';
  import UseCases from '../components/UseCases.svelte';
  import Footer from '../components/Footer.svelte';
  import GlobalNavigation from '../components/GlobalNavigation.svelte';
  import TerminalBox from '../components/futarchy/terminal/TerminalBox.svelte';
  import TerminalButton from '../components/futarchy/terminal/TerminalButton.svelte';
  import TerminalInput from '../components/futarchy/terminal/TerminalInput.svelte';
  import { walletStore } from '../stores/wallet';
  import { navigateTo } from '../stores/router';

  // API config - use relative path for nginx proxy
  const API_BASE = import.meta.env.VITE_API_URL || '';
  let activeProvers = 0;
  let commandInput = '';
  let commandLog = [];
  let suggestionIndex = -1;
  let history = [];
  let historyIndex = -1;
  const historyStorageKey = 'zyber_terminal_history';
  const historyLimit = 30;

  $: promptPrefix = `zyber@terminal${$walletStore.connected ? '(✓)' : '( )'}:~$`;

  const availableCommands = [
    { command: '/futarchy', description: 'Open Futarchy terminal' },
    { command: '/job', description: 'Open job marketplace' },
    { command: '/create-job', description: 'Create a new job' },
    { command: '/submit-job', description: 'Submit a job' },
    { command: '/connect', description: 'Connect Solana wallet' },
    { command: '/disconnect', description: 'Disconnect wallet' },
    { command: '/help', description: 'Show help' },
    { command: '/pbtcfi', description: 'Coming soon' },
    { command: '/ploans', description: 'Coming soon' }
  ];

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

    if (typeof window !== 'undefined') {
      try {
        const stored = JSON.parse(window.localStorage.getItem(historyStorageKey) || '[]');
        if (Array.isArray(stored)) {
          history = stored.slice(0, historyLimit);
        }
      } catch (error) {
        console.warn('Failed to load command history:', error);
      }
    }
  });

  function skipToDemo() {
    navigateTo('dashboard');
  }

  function openFutarchyTerminal() {
    navigateTo('futarchy');
  }

  function pushLog(message, type = 'info') {
    commandLog = [{ message, type, at: new Date().toISOString() }, ...commandLog].slice(0, 10);
  }

  function pushLogBatch(lines) {
    const entries = lines.map((message) => ({
      message,
      type: 'info',
      at: new Date().toISOString()
    }));
    commandLog = [...entries.reverse(), ...commandLog].slice(0, 10);
  }

  function formatTimestamp(iso) {
    return new Date(iso).toISOString().slice(11, 19);
  }

  async function handleCommandSubmit() {
    const command = commandInput.trim().toLowerCase();
    if (!command) return;

    history = [command, ...history.filter(item => item !== command)].slice(0, historyLimit);
    historyIndex = -1;

    pushLog(`${promptPrefix} ${command}`);

    if (command === '/futarchy') {
      navigateTo('futarchy');
    } else if (command === '/job' || command === '/jobs') {
      navigateTo('dashboard');
    } else if (command === '/create-job' || command === '/submit-job') {
      navigateTo('create-job');
    } else if (command === '/connect') {
      if ($walletStore.connected) {
        pushLog('Wallet already connected.');
      } else {
        try {
          const result = await walletStore.connect('solana');
          pushLog(`Wallet connected: ${result.publicKey.slice(0, 4)}...${result.publicKey.slice(-4)}`);
        } catch (error) {
          pushLog(`Wallet connection failed: ${error.message || 'unknown error'}`, 'error');
        }
      }
    } else if (command === '/disconnect') {
      if ($walletStore.connected) {
        walletStore.disconnect();
        pushLog('Wallet disconnected.');
      } else {
        pushLog('No wallet connected.');
      }
    } else if (command === '/help') {
      pushLogBatch([
        'Available commands:',
        '/futarchy   → open Futarchy terminal',
        '/job        → open job marketplace',
        '/create-job → open job creation flow',
        '/submit-job → open job creation flow',
        '/connect    → connect Solana wallet',
        '/disconnect → disconnect wallet',
        '/pbtcfi     → coming soon',
        '/ploans     → coming soon'
      ]);
    } else if (command === '/pbtcfi') {
      pushLog('PBTCFI is coming soon.');
    } else if (command === '/ploans') {
      pushLog('PLOANS is coming soon.');
    } else {
      pushLog(`Unknown command: ${command}`);
    }

    commandInput = '';
    suggestionIndex = -1;
    historyIndex = -1;
  }

  function handleCommandKeydown(event) {
    const nativeEvent = event.detail || event;
    if (nativeEvent.key === 'Enter') {
      handleCommandSubmit();
    } else if (nativeEvent.key === 'Tab') {
      nativeEvent.preventDefault();
      if (filteredSuggestions.length > 0) {
        const nextIndex = (suggestionIndex + 1) % filteredSuggestions.length;
        const selected = filteredSuggestions[nextIndex];
        commandInput = selected.command;
        suggestionIndex = nextIndex;
      }
    } else if (nativeEvent.key === 'ArrowDown') {
      nativeEvent.preventDefault();
      if (filteredSuggestions.length > 0 && commandInput.trim()) {
        suggestionIndex = (suggestionIndex + 1) % filteredSuggestions.length;
      } else if (history.length > 0) {
        if (historyIndex > 0) {
          historyIndex -= 1;
          commandInput = history[historyIndex];
        } else {
          historyIndex = -1;
          commandInput = '';
        }
      }
    } else if (nativeEvent.key === 'ArrowUp') {
      nativeEvent.preventDefault();
      if (filteredSuggestions.length > 0 && commandInput.trim()) {
        suggestionIndex = suggestionIndex <= 0 ? filteredSuggestions.length - 1 : suggestionIndex - 1;
      } else if (history.length > 0) {
        if (historyIndex < history.length - 1) {
          historyIndex += 1;
          commandInput = history[historyIndex];
        } else if (historyIndex === -1) {
          historyIndex = 0;
          commandInput = history[historyIndex];
        }
      }
    } else if (nativeEvent.key === 'Escape') {
      suggestionIndex = -1;
    }
  }

  function handleCommandInput() {
    historyIndex = -1;
    suggestionIndex = -1;
  }

  function selectSuggestion(index) {
    const selected = filteredSuggestions[index];
    if (selected) {
      commandInput = selected.command;
      suggestionIndex = index;
    }
  }

  $: filteredSuggestions = availableCommands.filter((item) =>
    item.command.includes(commandInput.trim().toLowerCase())
  );

  $: if (!commandInput) {
    suggestionIndex = -1;
  }

  $: if (typeof window !== 'undefined') {
    try {
      window.localStorage.setItem(historyStorageKey, JSON.stringify(history.slice(0, historyLimit)));
    } catch (error) {
      console.warn('Failed to save command history:', error);
    }
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
              <span class="text-cyan">SOLANA_DEVNET</span>
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
              <button class="btn btn-primary" on:click={openFutarchyTerminal}>
                <span class="text-mono">[ENTER_FUTARCHY_TERMINAL]</span>
              </button>
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

            <TerminalBox tone="muted" dense>
              <div class="command-bar">
                <TerminalInput
                  label="command"
                  bind:value={commandInput}
                  placeholder="/futarchy | /job | /connect"
                  prefix={promptPrefix}
                  on:input={handleCommandInput}
                  on:keydown={handleCommandKeydown}
                />
                <TerminalButton label="EXECUTE" tone="cyan" size="sm" on:click={handleCommandSubmit} />
              </div>
              <div class="command-hint text-mono text-xs text-muted">
                Tip: /help for full list. Tab completes. ↑/↓ cycles suggestions or history.
              </div>
              {#if commandInput && filteredSuggestions.length}
                <div class="command-suggestions">
                  {#each filteredSuggestions as suggestion, index}
                    <button
                      type="button"
                      class="suggestion {index === suggestionIndex ? 'active' : ''}"
                      on:click={() => selectSuggestion(index)}
                    >
                      <span class="text-mono">{suggestion.command}</span>
                      <span class="text-mono text-xs text-muted">{suggestion.description}</span>
                    </button>
                  {/each}
                </div>
              {/if}
              {#if commandLog.length}
                <div class="command-log">
                  {#each commandLog as entry}
                    <div class="log-line text-mono text-xs">
                      [{formatTimestamp(entry.at)}] {entry.message}
                    </div>
                  {/each}
                </div>
              {/if}
            </TerminalBox>
          </div>
        </div>
      </div>
      </div>
    </section>

    <!-- Stats Banner -->
    <section id="stats" class="stats-section">
      <div class="stats-header text-center mb-6">
        <h3 class="text-mono text-uppercase">
          <span class="text-cyan">&gt;</span> NETWORK_STATISTICS
        </h3>
        <div class="text-xs text-muted text-mono mt-2">
          REAL_TIME_METRICS_FROM_DECENTRALIZED_NETWORK
        </div>
      </div>
      <FloatingStatsCards />
    </section>

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

  .stats-section {
    width: 100%;
    max-width: 900px;
    margin: var(--space-8) auto;
    padding: var(--space-8) var(--space-6);
    background: linear-gradient(180deg, rgba(139, 92, 246, 0.03) 0%, transparent 100%);
  }

  .stats-header h3 {
    font-size: var(--text-xl);
    font-weight: 600;
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

  .command-bar {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-3);
    align-items: flex-end;
  }

  .command-suggestions {
    margin-top: var(--space-3);
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .suggestion {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-2) var(--space-3);
    background: rgba(0, 0, 0, 0.35);
    border: 1px solid var(--zyber-border-muted);
    cursor: pointer;
    text-align: left;
  }

  .suggestion.active {
    border-color: var(--zyber-cyber-cyan);
    box-shadow: var(--zyber-glow-cyan);
  }

  .command-log {
    margin-top: var(--space-3);
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .command-hint {
    margin-top: var(--space-2);
    letter-spacing: 0.04em;
  }

  .log-line {
    color: var(--zyber-text-secondary);
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
