<script>
  import { onDestroy, onMount } from 'svelte';
  import { walletStore } from '../stores/wallet';
  import { navigateTo } from '../stores/router';
  import Logo from '../components/Logo.svelte';
  import FutarchyLogo from '../components/futarchy/FutarchyLogo.svelte';
  import FutarchyMarketList from '../components/futarchy/FutarchyMarketList.svelte';
  import FutarchyPositionsList from '../components/futarchy/FutarchyPositionsList.svelte';
  import FutarchyPlaceBetModal from '../components/futarchy/FutarchyPlaceBetModal.svelte';
  import FutarchyBetWizard from '../components/futarchy/FutarchyBetWizard.svelte';
  import TerminalBox from '../components/futarchy/terminal/TerminalBox.svelte';
  import TerminalButton from '../components/futarchy/terminal/TerminalButton.svelte';
  import TerminalInput from '../components/futarchy/terminal/TerminalInput.svelte';
  import TerminalRadio from '../components/futarchy/terminal/TerminalRadio.svelte';
  import { getFutarchyStats } from '../utils/futarchy_api';

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
  let betWizardOpen = false;
  let betWizardMarketId = '';
  let statsOpen = false;
  let statsLoading = false;
  let statsError = '';
  let statsData = null;
  let statsUpdatedAt = '';
  let statsTimer;
  let selectedMarket = null;
  let showHint = true;
  let hintTimer;
  $: promptPrefix = `zyber@futarchy${$walletStore.connected ? '(✓)' : '( )'}:~$`;

  $: walletAddress = $walletStore.addresses?.solana || $walletStore.publicKey?.toString() || '';
  $: walletLabel = walletAddress
    ? `${walletAddress.slice(0, 4)}...${walletAddress.slice(-4)}`
    : 'WALLET DISCONNECTED';

  const lamportsPerSol = 1_000_000_000;

  function formatSol(lamports) {
    const value = Number(lamports || 0) / lamportsPerSol;
    return `${value.toFixed(2)} SOL`;
  }

  function truncate(text, max = 42) {
    if (!text) return '--';
    return text.length > max ? `${text.slice(0, max)}…` : text;
  }

  function buildBar(value, max, length = 20, fillChar = '█') {
    if (!max) return `${'░'.repeat(length)}`;
    const filled = Math.max(0, Math.min(length, Math.round((value / max) * length)));
    return `${fillChar.repeat(filled)}${'░'.repeat(Math.max(0, length - filled))}`;
  }

  $: statsYes = statsData?.pools?.yes_lamports || 0;
  $: statsNo = statsData?.pools?.no_lamports || 0;
  $: statsTotal = statsData?.pools?.total_lamports || 0;
  $: statsYesPct = statsTotal ? Math.round((statsYes / statsTotal) * 100) : 0;
  $: statsNoPct = statsTotal ? Math.max(0, 100 - statsYesPct) : 0;
  $: topVolumeMax = Math.max(1, ...(statsData?.top_markets_by_volume || []).map((m) => m.total_lamports || 0));
  $: topBetsMax = Math.max(1, ...(statsData?.top_markets_24h || []).map((m) => m.bets_24h || 0));

  function logMarketAccountLink(id) {
    if (!id) return;
    const link = `https://explorer.solana.com/address/${id}?cluster=devnet`;
    console.log(`[Futarchy] Market account: ${link}`);
  }

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
    '/bet': (args = []) => {
      const inputId = args[0] || '';
      betWizardMarketId = inputId || selectedMarket?.id || '';
      betWizardOpen = true;
      if (betWizardMarketId) {
        logMarketAccountLink(betWizardMarketId);
        pushLog(`Launching bet wizard for market ${betWizardMarketId}...`);
      } else {
        pushLog('Launching bet wizard...');
      }
    },
    '/stats': async (args = []) => {
      statsOpen = true;
      if (args[0] === 'refresh' || !statsData) {
        await fetchStats();
      }
      startStatsTimer();
      pushLog('Futarchy stats panel opened.');
    },
    '/close-stats': () => {
      statsOpen = false;
      stopStatsTimer();
      pushLog('Futarchy stats panel closed.');
    },
    '/close-bet': () => {
      betWizardOpen = false;
      pushLog('Bet wizard closed.');
    },
    '/help': () => {
      pushLogBatch([
        'Available commands:',
        '/markets    → open markets tab',
        '/positions  → open positions tab',
        '/create     → create market (soon)',
        '/bet        → guided bet wizard',
        '/bet <id>   → guided bet wizard for market id',
        '/stats      → show futarchy stats',
        '/stats refresh → refresh futarchy stats',
        '/close-stats → close stats panel',
        '/close-bet  → close bet wizard',
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
    { command: '/bet', description: 'Guided bet wizard' },
    { command: '/bet <id>', description: 'Guided bet wizard for market id' },
    { command: '/stats', description: 'Show futarchy stats' },
    { command: '/stats refresh', description: 'Refresh futarchy stats' },
    { command: '/close-stats', description: 'Close stats panel' },
    { command: '/close-bet', description: 'Close bet wizard' },
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
    const rawCommand = commandInput.trim();
    if (!rawCommand) return;
    const tokens = rawCommand.split(/\s+/);
    const baseCommand = tokens[0].toLowerCase();
    const args = tokens.slice(1);
    const normalized = [baseCommand, ...args].join(' ');

    history = [normalized, ...history.filter(item => item !== normalized)].slice(0, historyLimit);
    historyIndex = -1;

    pushLog(`${promptPrefix} ${normalized}`);

    const handler = commandMap[baseCommand];
    if (handler) {
      await handler(args);
    } else {
      pushLog(`Unknown command: ${normalized}`);
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
    betWizardMarketId = selectedMarket?.id || '';
    logMarketAccountLink(betWizardMarketId);
    betWizardOpen = true;
  }

  function closeBetModal() {
    placeBetOpen = false;
    selectedMarket = null;
  }

  function closeBetWizard() {
    betWizardOpen = false;
  }

  async function fetchStats() {
    if (statsLoading) return;
    statsLoading = true;
    statsError = '';
    try {
      statsData = await getFutarchyStats({ apiBaseUrl: API_BASE || undefined });
      statsUpdatedAt = new Date().toISOString();
    } catch (error) {
      statsError = error?.message || 'Failed to load futarchy stats';
    } finally {
      statsLoading = false;
    }
  }

  function startStatsTimer() {
    if (statsTimer) return;
    statsTimer = setInterval(() => {
      if (statsOpen) {
        fetchStats();
      }
    }, 15000);
  }

  function stopStatsTimer() {
    if (statsTimer) {
      clearInterval(statsTimer);
      statsTimer = null;
    }
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
    pushLog('Commands: /markets /positions /create /bet /stats /close-bet /pbtcfi /ploans');
    showHint = true;
    hintTimer = setTimeout(() => {
      showHint = false;
    }, 4500);
  });

  onDestroy(() => {
    if (hintTimer) clearTimeout(hintTimer);
    stopStatsTimer();
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
      <div class="hint text-mono text-xs">TIP: /markets /positions /create /bet /stats /job /connect. Use /help. Tab completes, ↑/↓ cycles.</div>
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
        autoRefresh
        refreshInterval={30000}
        apiBaseUrl={API_BASE}
        status={statusFilter === 'all' ? '' : statusFilter}
        compact={betWizardOpen || statsOpen}
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

  {#if statsOpen}
    <section class="terminal-stats">
      <TerminalBox tone="muted" dense>
        <div class="stats-header text-mono text-xs">
          <span>FUTARCHY_STATS</span>
          <span class="text-muted">updated: {statsUpdatedAt ? formatTimestamp(statsUpdatedAt) : '--'}</span>
        </div>

        {#if statsLoading}
          <div class="text-mono text-xs">LOADING STATS...</div>
        {:else if statsError}
          <div class="text-mono text-xs text-muted">[!] {statsError}</div>
        {:else if statsData}
          <div class="stats-block text-mono text-xs">
            MARKETS: total {statsData.markets.total} | active {statsData.markets.active} | resolved {statsData.markets.resolved} | cancelled {statsData.markets.cancelled}
          </div>
          <div class="stats-block text-mono text-xs">
            BETS: total {statsData.bets.total} | last_24h {statsData.bets.last_24h}
          </div>
          <div class="stats-block text-mono text-xs">
            POOLS: total {formatSol(statsTotal)} | YES {formatSol(statsYes)} | NO {formatSol(statsNo)}
          </div>
          <div class="stats-block text-mono text-xs stats-bar">
            YES {statsYesPct}% [{buildBar(statsYes, statsTotal, 18)}] {statsNoPct}% NO
          </div>
          <div class="stats-block text-mono text-xs">
            FHE_JOBS: pending {statsData.fhe_jobs.pending} | processing {statsData.fhe_jobs.processing} | completed {statsData.fhe_jobs.completed} | failed {statsData.fhe_jobs.failed}
          </div>

          <div class="stats-section text-mono text-xs text-muted">TOP_MARKETS_BY_VOLUME</div>
          {#if statsData.top_markets_by_volume.length === 0}
            <div class="text-mono text-xs">--</div>
          {:else}
            {#each statsData.top_markets_by_volume as market, index}
              <div class="stats-row text-mono text-xs">
                {index + 1}. {truncate(market.question, 36)}
                <span class="stats-bar">[{buildBar(market.total_lamports, topVolumeMax, 16)}]</span>
                <span class="text-muted">{formatSol(market.total_lamports)}</span>
              </div>
            {/each}
          {/if}

          <div class="stats-section text-mono text-xs text-muted">TOP_MARKETS_24H</div>
          {#if statsData.top_markets_24h.length === 0}
            <div class="text-mono text-xs">--</div>
          {:else}
            {#each statsData.top_markets_24h as market, index}
              <div class="stats-row text-mono text-xs">
                {index + 1}. {truncate(market.question, 32)}
                <span class="stats-bar">[{buildBar(market.bets_24h, topBetsMax, 12)}]</span>
                <span class="text-muted">bets {market.bets_24h} | vol {formatSol(market.volume_lamports_24h)}</span>
              </div>
            {/each}
          {/if}
        {:else}
          <div class="text-mono text-xs text-muted">No stats loaded. Run /stats refresh.</div>
        {/if}
      </TerminalBox>
    </section>
  {/if}

  {#if betWizardOpen}
    <section class="terminal-wizard">
      <FutarchyBetWizard
        open={betWizardOpen}
  apiBaseUrl={API_BASE}
  initialMarketId={betWizardMarketId}
  on:close={closeBetWizard}
  on:success={(event) => {
    const signature = event.detail?.signature;
          pushLog(signature ? `Bet wizard confirmed: ${signature}` : 'Bet wizard confirmed.');
          betWizardOpen = false;
        }}
        on:error={(event) => pushLog(`Bet wizard failed: ${event.detail.error?.message || 'Unknown error'}`)}
      />
    </section>
  {/if}

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
  on:success={(event) => {
    const signature = event.detail?.signature;
    pushLog(signature ? `Bet confirmed: ${signature}` : 'Bet confirmed on-chain.');
  }}
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

  .terminal-stats {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  .stats-header {
    display: flex;
    justify-content: space-between;
    gap: var(--space-3);
    text-transform: uppercase;
    letter-spacing: 0.08em;
    margin-bottom: var(--space-2);
  }

  .stats-block {
    margin-bottom: var(--space-2);
  }

  .stats-section {
    margin-top: var(--space-3);
    letter-spacing: 0.08em;
  }

  .stats-row {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
    align-items: center;
  }

  .stats-bar {
    letter-spacing: 0.08em;
  }

  .terminal-command {
    margin-top: var(--space-2);
  }

  .terminal-wizard {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
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
