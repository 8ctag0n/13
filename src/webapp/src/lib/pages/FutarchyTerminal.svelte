<script>
  import { onDestroy, onMount } from 'svelte';
  import { walletStore } from '../stores/wallet';
  import { navigateTo } from '../stores/router';
  import Logo from '../components/Logo.svelte';
  import FutarchyLogo from '../components/futarchy/FutarchyLogo.svelte';
  import FutarchyMarketList from '../components/futarchy/FutarchyMarketList.svelte';
  import FutarchyPositionsList from '../components/futarchy/FutarchyPositionsList.svelte';
  import FutarchyPlaceBetModal from '../components/futarchy/FutarchyPlaceBetModal.svelte';
  import TerminalBox from '../components/futarchy/terminal/TerminalBox.svelte';
  import TerminalButton from '../components/futarchy/terminal/TerminalButton.svelte';
  import TerminalInput from '../components/futarchy/terminal/TerminalInput.svelte';
  import TerminalRadio from '../components/futarchy/terminal/TerminalRadio.svelte';

  const API_BASE = import.meta.env.VITE_API_URL || '';

  let activeTab = 'markets';
  let statusFilter = 'all';
  let commandInput = '';
  let commandLog = [];
  let suggestionIndex = -1;
  let history = [];
  let historyIndex = -1;
  const historyStorageKey = 'zyber_futarchy_history';
  const historyLimit = 30;
  let placeBetOpen = false;
  let selectedMarket = null;
  let showHint = true;
  let hintTimer;
  $: promptPrefix = `zyber@futarchy${$walletStore.connected ? '(✓)' : '( )'}:~$`;

  $: walletAddress = $walletStore.addresses?.solana || $walletStore.publicKey?.toString() || '';
  $: walletLabel = walletAddress
    ? `${walletAddress.slice(0, 4)}...${walletAddress.slice(-4)}`
    : 'WALLET DISCONNECTED';

  const commandMap = {
    '/futarchy': () => navigateTo('futarchy'),
    '/pbtcfi': () => pushLog('PBTCFI is coming soon.'),
    '/ploans': () => pushLog('PLOANS is coming soon.'),
    '/job': () => navigateTo('dashboard'),
    '/jobs': () => navigateTo('dashboard'),
    '/create-job': () => navigateTo('create-job'),
    '/submit-job': () => navigateTo('create-job'),
    '/markets': () => (activeTab = 'markets'),
    '/positions': () => (activeTab = 'positions'),
    '/create': () => (activeTab = 'create'),
    '/help': () => {
      pushLogBatch([
        'Available commands:',
        '/markets    → open markets tab',
        '/positions  → open positions tab',
        '/create     → create market (soon)',
        '/job        → open job marketplace',
        '/create-job → open job creation flow',
        '/submit-job → open job creation flow',
        '/connect    → connect Solana wallet',
        '/disconnect → disconnect wallet',
        '/pbtcfi     → coming soon',
        '/ploans     → coming soon'
      ]);
    },
    '/connect': async () => {
      if ($walletStore.connected) {
        pushLog('Wallet already connected.');
        return;
      }
      try {
        const result = await walletStore.connect('solana');
        pushLog(`Wallet connected: ${result.publicKey.slice(0, 4)}...${result.publicKey.slice(-4)}`);
      } catch (error) {
        pushLog(`Wallet connection failed: ${error.message || 'unknown error'}`);
      }
    },
    '/disconnect': () => {
      if ($walletStore.connected) {
        walletStore.disconnect();
        pushLog('Wallet disconnected.');
      } else {
        pushLog('No wallet connected.');
      }
    }
  };

  const availableCommands = [
    { command: '/markets', description: 'Open markets tab' },
    { command: '/positions', description: 'Open positions tab' },
    { command: '/create', description: 'Create market (soon)' },
    { command: '/job', description: 'Open job marketplace' },
    { command: '/create-job', description: 'Create a new job' },
    { command: '/submit-job', description: 'Submit a job' },
    { command: '/connect', description: 'Connect Solana wallet' },
    { command: '/disconnect', description: 'Disconnect wallet' },
    { command: '/pbtcfi', description: 'Coming soon' },
    { command: '/ploans', description: 'Coming soon' },
    { command: '/help', description: 'Show help' }
  ];

  function pushLog(message) {
    commandLog = [{ message, at: new Date().toISOString() }, ...commandLog].slice(0, 10);
  }

  function pushLogBatch(lines) {
    const entries = lines.map((message) => ({
      message,
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

    const handler = commandMap[command];
    if (handler) {
      await handler();
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

  function handleMarketAction(event) {
    selectedMarket = event.detail.market;
    placeBetOpen = true;
  }

  function closeBetModal() {
    placeBetOpen = false;
    selectedMarket = null;
  }

  onMount(() => {
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

    pushLog('Welcome to Futarchy Terminal. Type /futarchy to stay here.');
    pushLog('Commands: /markets /positions /create /pbtcfi /ploans');
    showHint = true;
    hintTimer = setTimeout(() => {
      showHint = false;
    }, 4500);
  });

  onDestroy(() => {
    if (hintTimer) clearTimeout(hintTimer);
  });
</script>

<div class="futarchy-terminal">
  <header class="terminal-header tui-box">
    <div class="header-logos">
      <Logo />
      <FutarchyLogo />
    </div>
    <div class="wallet-status text-mono">
      [wallet] {walletLabel} {walletAddress ? '[✓]' : '[ ]'}
    </div>
  </header>

  {#if showHint}
    <TerminalBox tone="cyan" dense>
      <div class="hint text-mono text-xs">TIP: /markets /positions /create /job /connect. Use /help. Tab completes, ↑/↓ cycles.</div>
    </TerminalBox>
  {/if}

  <section class="terminal-nav tui-box">
    <div class="tabs text-mono">
      <button class:active={activeTab === 'markets'} on:click={() => (activeTab = 'markets')}>&gt; MARKETS</button>
      <button class:active={activeTab === 'positions'} on:click={() => (activeTab = 'positions')}>MY_POSITIONS</button>
      <button class:active={activeTab === 'create'} on:click={() => (activeTab = 'create')}>CREATE_MARKET</button>
    </div>

    <div class="filters">
      <div class="filter-label text-mono text-xs text-muted">filter:</div>
      <div class="filter-options">
        <TerminalRadio
          name="market-filter"
          value="all"
          label="ALL"
          checked={statusFilter === 'all'}
          on:change={() => (statusFilter = 'all')}
        />
        <TerminalRadio
          name="market-filter"
          value="active"
          label="ACTIVE"
          checked={statusFilter === 'active'}
          on:change={() => (statusFilter = 'active')}
        />
        <TerminalRadio
          name="market-filter"
          value="settled"
          label="SETTLED"
          checked={statusFilter === 'settled'}
          on:change={() => (statusFilter = 'settled')}
        />
      </div>
    </div>
  </section>

  <main class="terminal-main">
    {#if activeTab === 'markets'}
      <FutarchyMarketList
        autoFetch
        apiBaseUrl={API_BASE}
        status={statusFilter === 'all' ? '' : statusFilter}
        on:marketAction={handleMarketAction}
      />
    {:else if activeTab === 'positions'}
      <FutarchyPositionsList
        autoFetch
        apiBaseUrl={API_BASE}
        bettor={walletAddress}
        emptyLabel={walletAddress ? 'NO POSITIONS FOUND' : 'CONNECT WALLET TO VIEW POSITIONS'}
      />
    {:else}
      <TerminalBox tone="muted" dense>
        <div class="text-mono text-sm">CREATE_MARKET</div>
        <div class="text-mono text-xs text-muted">Coming soon in this terminal view.</div>
      </TerminalBox>
    {/if}
  </main>

  <section class="terminal-command">
    <TerminalBox tone="violet" dense>
      <div class="command-bar">
        <TerminalInput
          label="command"
          bind:value={commandInput}
          placeholder="/markets | /positions | /job"
          prefix={promptPrefix}
          on:input={handleCommandInput}
          on:keydown={handleCommandKeydown}
        />
        <TerminalButton label="EXECUTE" tone="cyan" size="sm" on:click={handleCommandSubmit} />
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
  </section>
</div>

<FutarchyPlaceBetModal
  open={placeBetOpen}
  market={selectedMarket}
  apiBaseUrl={API_BASE}
  on:close={closeBetModal}
  on:success={() => pushLog('Bet submitted successfully.')}
  on:error={(event) => pushLog(`Bet failed: ${event.detail.error?.message || 'Unknown error'}`)}
/>

<style>
  .futarchy-terminal {
    display: flex;
    flex-direction: column;
    gap: var(--space-6);
    padding: var(--space-6);
    position: relative;
    z-index: 1;
  }

  .terminal-header {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    align-items: center;
    text-align: center;
  }

  .header-logos {
    display: grid;
    gap: var(--space-4);
  }

  .wallet-status {
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }

  .terminal-nav {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }

  .tabs {
    display: flex;
    gap: var(--space-3);
    flex-wrap: wrap;
  }

  .tabs button {
    background: transparent;
    border: 1px solid var(--zyber-border-secondary);
    color: var(--zyber-text-primary);
    padding: var(--space-2) var(--space-3);
    font-family: var(--font-mono);
    cursor: pointer;
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }

  .tabs button.active {
    border-color: var(--zyber-cyber-cyan);
    color: var(--zyber-cyber-cyan);
    box-shadow: var(--zyber-glow-cyan);
  }

  .filters {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  .filter-options {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
    gap: var(--space-3);
  }

  .terminal-main {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }

  .terminal-command {
    margin-top: var(--space-2);
  }

  .command-bar {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-3);
    align-items: flex-end;
  }

  .command-log {
    margin-top: var(--space-3);
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .log-line {
    color: var(--zyber-text-secondary);
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

  .hint {
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }

  @media (min-width: 960px) {
    .header-logos {
      grid-template-columns: 1fr 1fr;
      align-items: center;
    }
  }

  @media (max-width: 720px) {
    .futarchy-terminal {
      padding: var(--space-4);
    }
  }
</style>
