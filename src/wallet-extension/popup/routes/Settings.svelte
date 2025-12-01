<script lang="ts">
  import { createEventDispatcher, onMount } from 'svelte';
  import type { RPCEndpoint, SupportedChain } from '../../lib/chains/types';

  const dispatch = createEventDispatcher();

  type SettingsView = 'main' | 'networks' | 'security' | 'export';

  let view: SettingsView = 'main';
  let selectedChain: SupportedChain = 'solana';
  let endpoints: RPCEndpoint[] = [];
  let activeEndpoint = '';
  let showAddCustom = false;
  let customUrl = '';
  let customName = '';
  let error = '';
  let success = '';

  // Export key state
  let exportPassword = '';
  let exportedKey = '';
  let exportFormat: 'hex' | 'base58' = 'hex';
  let showExportedKey = false;

  // Security state
  let autoLockMinutes = 5;

  const chainConfig = {
    solana: { symbol: 'SOL', icon: '◉', color: 'var(--cyber-cyan)', name: 'Solana' },
    starknet: { symbol: 'STRK', icon: '▲', color: 'var(--cyber-purple)', name: 'Starknet' },
    zcash: { symbol: 'ZEC', icon: 'Z', color: 'var(--cyber-yellow)', name: 'Zcash' }
  };

  async function loadEndpoints() {
    try {
      const response = await chrome.runtime.sendMessage({
        type: 'GET_RPC_ENDPOINTS',
        chain: selectedChain
      });

      if (response.success) {
        endpoints = response.data.endpoints;
        activeEndpoint = response.data.activeEndpoint;
      }
    } catch (err) {
      console.error('[Settings] Failed to load endpoints:', err);
    }
  }

  async function handleSetActive(url: string) {
    error = '';
    try {
      const response = await chrome.runtime.sendMessage({
        type: 'SET_ACTIVE_RPC',
        chain: selectedChain,
        url
      });

      if (response.success) {
        activeEndpoint = url;
        success = 'RPC endpoint updated';
        setTimeout(() => success = '', 2000);
      } else {
        error = response.error || 'Failed to set active endpoint';
      }
    } catch (err) {
      error = err instanceof Error ? err.message : 'Unknown error';
    }
  }

  async function handleAddCustom() {
    error = '';
    if (!customUrl || !customName) {
      error = 'URL and name are required';
      return;
    }

    try {
      const response = await chrome.runtime.sendMessage({
        type: 'ADD_CUSTOM_RPC',
        chain: selectedChain,
        endpoint: {
          url: customUrl,
          name: customName,
          custom: true
        }
      });

      if (response.success) {
        customUrl = '';
        customName = '';
        showAddCustom = false;
        await loadEndpoints();
        success = 'Custom RPC added';
        setTimeout(() => success = '', 2000);
      } else {
        error = response.error || 'Failed to add endpoint';
      }
    } catch (err) {
      error = err instanceof Error ? err.message : 'Unknown error';
    }
  }

  async function handleRemoveCustom(url: string) {
    try {
      const response = await chrome.runtime.sendMessage({
        type: 'REMOVE_CUSTOM_RPC',
        chain: selectedChain,
        url
      });

      if (response.success) {
        await loadEndpoints();
      } else {
        error = response.error || 'Failed to remove endpoint';
      }
    } catch (err) {
      error = err instanceof Error ? err.message : 'Unknown error';
    }
  }

  async function handleExportKey() {
    error = '';
    if (!exportPassword) {
      error = 'Password required';
      return;
    }

    try {
      const response = await chrome.runtime.sendMessage({
        type: 'EXPORT_PRIVATE_KEY',
        password: exportPassword
      });

      if (response.success) {
        exportedKey = response.data.privateKey;
        showExportedKey = true;
        exportPassword = '';
      } else {
        error = response.error || 'Invalid password';
      }
    } catch (err) {
      error = err instanceof Error ? err.message : 'Unknown error';
    }
  }

  function copyExportedKey() {
    navigator.clipboard.writeText(exportedKey);
    success = 'Key copied to clipboard';
    setTimeout(() => success = '', 2000);
  }

  function handleLock() {
    chrome.runtime.sendMessage({ type: 'LOCK' });
    dispatch('back');
  }

  function handleBack() {
    dispatch('back');
  }

  function goBack() {
    if (view === 'main') {
      handleBack();
    } else {
      view = 'main';
      showExportedKey = false;
      exportedKey = '';
      error = '';
    }
  }

  onMount(() => {
    loadEndpoints();
  });

  $: if (selectedChain) {
    loadEndpoints();
  }
</script>

<div class="settings-container">
  <!-- HEADER -->
  <div class="settings-header">
    <button class="back-btn" on:click={goBack}>
      [←]
    </button>
    <span class="header-title">
      {#if view === 'main'}[SETTINGS]
      {:else if view === 'networks'}[NETWORKS]
      {:else if view === 'security'}[SECURITY]
      {:else if view === 'export'}[EXPORT KEY]
      {/if}
    </span>
    <div style="width: 30px;"></div>
  </div>

  {#if success}
    <div class="toast success">[✓] {success}</div>
  {/if}

  {#if error && view !== 'networks'}
    <div class="toast error">[!] {error}</div>
  {/if}

  <!-- MAIN VIEW -->
  {#if view === 'main'}
    <div class="settings-content">
      <!-- GENERAL SECTION -->
      <div class="section">
        <div class="section-title">[GENERAL]</div>

        <button class="menu-item" on:click={() => view = 'networks'}>
          <span class="menu-icon">⛓</span>
          <span class="menu-text">Networks & RPC</span>
          <span class="menu-arrow">→</span>
        </button>

        <button class="menu-item" on:click={() => view = 'security'}>
          <span class="menu-icon">🛡</span>
          <span class="menu-text">Security</span>
          <span class="menu-arrow">→</span>
        </button>
      </div>

      <!-- WALLET SECTION -->
      <div class="section">
        <div class="section-title">[WALLET]</div>

        <button class="menu-item" on:click={() => view = 'export'}>
          <span class="menu-icon">🔑</span>
          <span class="menu-text">Export Private Key</span>
          <span class="menu-arrow">→</span>
        </button>

        <button class="menu-item danger" on:click={handleLock}>
          <span class="menu-icon">🔒</span>
          <span class="menu-text">Lock Wallet</span>
          <span class="menu-arrow">→</span>
        </button>
      </div>

      <!-- ABOUT SECTION -->
      <div class="section">
        <div class="section-title">[ABOUT]</div>

        <div class="about-info">
          <div class="about-row">
            <span class="about-label">Version</span>
            <span class="about-value">0.2.0-alpha</span>
          </div>
          <div class="about-row">
            <span class="about-label">Build</span>
            <span class="about-value">2024.11.30</span>
          </div>
        </div>

        <div class="chains-display">
          <div class="chains-label">[SUPPORTED CHAINS]</div>
          <div class="chains-icons">
            {#each Object.entries(chainConfig) as [_, config]}
              <span class="chain-badge" style="color: {config.color}">
                {config.icon} {config.symbol}
              </span>
            {/each}
          </div>
        </div>
      </div>
    </div>

  <!-- NETWORKS VIEW -->
  {:else if view === 'networks'}
    <div class="settings-content">
      <div class="chain-tabs">
        {#each Object.entries(chainConfig) as [chain, config]}
          <button
            class="chain-tab"
            class:active={selectedChain === chain}
            on:click={() => selectedChain = chain}
            style="--chain-color: {config.color}"
          >
            {config.icon} {config.symbol}
          </button>
        {/each}
      </div>

      <div class="section">
        <div class="section-title">[RPC ENDPOINTS]</div>

        {#if error}
          <div class="inline-error">[!] {error}</div>
        {/if}

        <div class="endpoints-list">
          {#each endpoints as endpoint}
            <div class="endpoint-item" class:active={activeEndpoint === endpoint.url}>
              <div class="endpoint-info">
                <div class="endpoint-name">{endpoint.name}</div>
                <div class="endpoint-url">{endpoint.url}</div>
              </div>
              <div class="endpoint-actions">
                {#if activeEndpoint === endpoint.url}
                  <span class="active-badge">[✓]</span>
                {:else}
                  <button class="action-btn" on:click={() => handleSetActive(endpoint.url)}>
                    [USE]
                  </button>
                {/if}
                {#if endpoint.custom}
                  <button class="action-btn danger" on:click={() => handleRemoveCustom(endpoint.url)}>
                    [✕]
                  </button>
                {/if}
              </div>
            </div>
          {/each}
        </div>

        {#if showAddCustom}
          <div class="add-custom-form">
            <div class="form-group">
              <label class="label" for="custom-name">[NAME]</label>
              <input
                id="custom-name"
                type="text"
                class="input"
                bind:value={customName}
                placeholder="My Custom RPC..."
              />
            </div>

            <div class="form-group">
              <label class="label" for="custom-url">[URL]</label>
              <input
                id="custom-url"
                type="url"
                class="input"
                bind:value={customUrl}
                placeholder="https://..."
              />
            </div>

            <div class="form-actions">
              <button class="button secondary" on:click={() => { showAddCustom = false; error = ''; }}>
                [CANCEL]
              </button>
              <button class="button" on:click={handleAddCustom}>
                [ADD]
              </button>
            </div>
          </div>
        {:else}
          <button class="add-custom-btn" on:click={() => showAddCustom = true}>
            [+ ADD CUSTOM RPC]
          </button>
        {/if}
      </div>
    </div>

  <!-- SECURITY VIEW -->
  {:else if view === 'security'}
    <div class="settings-content">
      <div class="section">
        <div class="section-title">[AUTO-LOCK]</div>

        <div class="setting-row">
          <span class="setting-label">Lock after inactivity</span>
          <select class="select" bind:value={autoLockMinutes}>
            <option value={1}>1 minute</option>
            <option value={5}>5 minutes</option>
            <option value={15}>15 minutes</option>
            <option value={30}>30 minutes</option>
            <option value={60}>1 hour</option>
            <option value={0}>Never</option>
          </select>
        </div>
      </div>

      <div class="section">
        <div class="section-title">[PERMISSIONS]</div>

        <div class="setting-row">
          <span class="setting-label">Require password for transactions</span>
          <div class="toggle active">[ON]</div>
        </div>

        <div class="setting-row">
          <span class="setting-label">Allow DApp connections</span>
          <div class="toggle active">[ON]</div>
        </div>
      </div>

      <div class="warning-box">
        <strong>[!] WARNING</strong>
        <p>Changing security settings may affect wallet protection. Ensure you understand the implications.</p>
      </div>
    </div>

  <!-- EXPORT KEY VIEW -->
  {:else if view === 'export'}
    <div class="settings-content">
      {#if !showExportedKey}
        <div class="warning-box danger">
          <strong>[!!] DANGER ZONE</strong>
          <p>Your private key gives full access to your wallet. Never share it with anyone. Store it securely offline.</p>
        </div>

        <div class="section">
          <div class="section-title">[VERIFY PASSWORD]</div>

          <div class="form-group">
            <label class="label" for="export-password">[PASSWORD]</label>
            <input
              id="export-password"
              type="password"
              class="input"
              bind:value={exportPassword}
              placeholder="Enter wallet password..."
            />
          </div>

          <button class="button danger" on:click={handleExportKey}>
            [EXPORT PRIVATE KEY]
          </button>
        </div>
      {:else}
        <div class="section">
          <div class="section-title">[YOUR PRIVATE KEY]</div>

          <div class="format-toggle">
            <button
              class="toggle-btn"
              class:active={exportFormat === 'hex'}
              on:click={() => exportFormat = 'hex'}
            >
              HEX
            </button>
            <button
              class="toggle-btn"
              class:active={exportFormat === 'base58'}
              on:click={() => exportFormat = 'base58'}
            >
              BASE58
            </button>
          </div>

          <div class="key-display">
            <code class="key-value">{exportedKey}</code>
          </div>

          <button class="button" on:click={copyExportedKey}>
            [COPY TO CLIPBOARD]
          </button>

          <button class="button secondary" on:click={() => { showExportedKey = false; exportedKey = ''; }}>
            [HIDE KEY]
          </button>
        </div>

        <div class="reminder-box">
          <strong>[!] REMINDER</strong>
          <ul>
            <li>Store this key securely offline</li>
            <li>Never share with anyone</li>
            <li>Clear clipboard after pasting</li>
          </ul>
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .settings-container {
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  .settings-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 10px 0;
    border-bottom: 1px solid var(--cyber-border);
    margin-bottom: 15px;
  }

  .back-btn {
    background: transparent;
    border: 1px solid var(--cyber-border);
    color: var(--cyber-text-dim);
    padding: 6px 10px;
    font-family: inherit;
    font-size: 12px;
    cursor: pointer;
    transition: all 0.2s;
  }

  .back-btn:hover {
    border-color: var(--cyber-cyan);
    color: var(--cyber-cyan);
  }

  .header-title {
    font-size: 14px;
    font-weight: bold;
    color: var(--cyber-cyan);
    letter-spacing: 2px;
  }

  .settings-content {
    flex: 1;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 15px;
  }

  /* Toast messages */
  .toast {
    padding: 8px 12px;
    font-size: 11px;
    text-align: center;
    margin-bottom: 10px;
  }

  .toast.success {
    background: rgba(0, 255, 159, 0.1);
    border: 1px solid var(--cyber-cyan);
    color: var(--cyber-cyan);
  }

  .toast.error {
    background: rgba(255, 68, 68, 0.1);
    border: 1px solid #ff4444;
    color: #ff4444;
  }

  /* Sections */
  .section {
    background: var(--cyber-bg-alt);
    border: 1px solid var(--cyber-border);
    padding: 15px;
  }

  .section-title {
    font-size: 10px;
    color: var(--cyber-text-dim);
    letter-spacing: 2px;
    margin-bottom: 12px;
  }

  /* Menu items */
  .menu-item {
    display: flex;
    align-items: center;
    width: 100%;
    padding: 12px;
    margin-bottom: 8px;
    background: var(--cyber-bg);
    border: 1px solid var(--cyber-border);
    color: var(--cyber-text);
    font-family: inherit;
    font-size: 12px;
    cursor: pointer;
    transition: all 0.2s;
    text-align: left;
  }

  .menu-item:hover {
    border-color: var(--cyber-cyan);
  }

  .menu-item.danger:hover {
    border-color: #ff4444;
  }

  .menu-icon {
    width: 24px;
    font-size: 14px;
  }

  .menu-text {
    flex: 1;
  }

  .menu-arrow {
    color: var(--cyber-text-dim);
  }

  /* About section */
  .about-info {
    background: var(--cyber-bg);
    padding: 10px;
    margin-bottom: 10px;
  }

  .about-row {
    display: flex;
    justify-content: space-between;
    padding: 5px 0;
    font-size: 11px;
  }

  .about-label {
    color: var(--cyber-text-dim);
  }

  .about-value {
    color: var(--cyber-cyan);
  }

  .chains-display {
    background: var(--cyber-bg);
    padding: 10px;
  }

  .chains-label {
    font-size: 10px;
    color: var(--cyber-text-dim);
    margin-bottom: 8px;
    letter-spacing: 1px;
  }

  .chains-icons {
    display: flex;
    gap: 15px;
  }

  .chain-badge {
    font-size: 11px;
  }

  /* Chain tabs */
  .chain-tabs {
    display: flex;
    gap: 5px;
  }

  .chain-tab {
    flex: 1;
    padding: 10px;
    background: transparent;
    border: 1px solid var(--cyber-border);
    color: var(--cyber-text-dim);
    font-family: inherit;
    font-size: 11px;
    cursor: pointer;
    transition: all 0.2s;
  }

  .chain-tab:hover {
    border-color: var(--chain-color);
    color: var(--chain-color);
  }

  .chain-tab.active {
    border-color: var(--chain-color);
    color: var(--chain-color);
    background: rgba(0, 255, 159, 0.05);
  }

  /* Endpoints */
  .endpoints-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin-bottom: 15px;
  }

  .endpoint-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 10px;
    background: var(--cyber-bg);
    border: 1px solid var(--cyber-border);
    transition: all 0.2s;
  }

  .endpoint-item.active {
    border-color: var(--cyber-cyan);
  }

  .endpoint-info {
    flex: 1;
    min-width: 0;
  }

  .endpoint-name {
    font-size: 12px;
    color: var(--cyber-text);
    margin-bottom: 2px;
  }

  .endpoint-url {
    font-size: 9px;
    color: var(--cyber-text-dim);
    word-break: break-all;
  }

  .endpoint-actions {
    display: flex;
    gap: 5px;
    margin-left: 10px;
  }

  .action-btn {
    background: transparent;
    border: 1px solid var(--cyber-border);
    color: var(--cyber-text-dim);
    padding: 4px 8px;
    font-family: inherit;
    font-size: 9px;
    cursor: pointer;
  }

  .action-btn:hover {
    border-color: var(--cyber-cyan);
    color: var(--cyber-cyan);
  }

  .action-btn.danger:hover {
    border-color: #ff4444;
    color: #ff4444;
  }

  .active-badge {
    color: var(--cyber-cyan);
    font-size: 10px;
    padding: 4px 8px;
  }

  .inline-error {
    padding: 8px;
    margin-bottom: 10px;
    font-size: 10px;
    color: #ff4444;
    background: rgba(255, 68, 68, 0.1);
    border: 1px solid #ff4444;
  }

  /* Add custom form */
  .add-custom-form {
    padding-top: 15px;
    border-top: 1px solid var(--cyber-border);
  }

  .form-group {
    margin-bottom: 12px;
  }

  .label {
    display: block;
    font-size: 10px;
    color: var(--cyber-text-dim);
    margin-bottom: 5px;
    letter-spacing: 1px;
  }

  .input {
    width: 100%;
    padding: 10px;
    background: var(--cyber-bg);
    border: 1px solid var(--cyber-border);
    color: var(--cyber-text);
    font-family: inherit;
    font-size: 12px;
    box-sizing: border-box;
  }

  .input:focus {
    outline: none;
    border-color: var(--cyber-cyan);
  }

  .form-actions {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px;
  }

  .add-custom-btn {
    width: 100%;
    padding: 10px;
    background: transparent;
    border: 1px dashed var(--cyber-border);
    color: var(--cyber-text-dim);
    font-family: inherit;
    font-size: 11px;
    cursor: pointer;
    transition: all 0.2s;
  }

  .add-custom-btn:hover {
    border-color: var(--cyber-cyan);
    color: var(--cyber-cyan);
  }

  /* Security settings */
  .setting-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 10px;
    margin-bottom: 8px;
    background: var(--cyber-bg);
    border: 1px solid var(--cyber-border);
  }

  .setting-label {
    font-size: 11px;
    color: var(--cyber-text);
  }

  .select {
    padding: 6px 10px;
    background: var(--cyber-bg-alt);
    border: 1px solid var(--cyber-border);
    color: var(--cyber-cyan);
    font-family: inherit;
    font-size: 10px;
    cursor: pointer;
  }

  .toggle {
    padding: 4px 10px;
    font-size: 10px;
    border: 1px solid var(--cyber-border);
    color: var(--cyber-text-dim);
  }

  .toggle.active {
    border-color: var(--cyber-cyan);
    color: var(--cyber-cyan);
    background: rgba(0, 255, 159, 0.1);
  }

  /* Warning boxes */
  .warning-box {
    padding: 12px;
    background: rgba(255, 214, 10, 0.1);
    border: 1px solid var(--cyber-yellow);
    font-size: 11px;
    color: var(--cyber-yellow);
  }

  .warning-box.danger {
    background: rgba(255, 68, 68, 0.1);
    border-color: #ff4444;
    color: #ff4444;
  }

  .warning-box strong {
    display: block;
    margin-bottom: 8px;
  }

  .warning-box p {
    margin: 0;
    line-height: 1.4;
  }

  /* Export key */
  .format-toggle {
    display: flex;
    gap: 5px;
    margin-bottom: 15px;
  }

  .toggle-btn {
    flex: 1;
    padding: 8px;
    background: transparent;
    border: 1px solid var(--cyber-border);
    color: var(--cyber-text-dim);
    font-family: inherit;
    font-size: 11px;
    cursor: pointer;
  }

  .toggle-btn.active {
    border-color: var(--cyber-cyan);
    color: var(--cyber-cyan);
    background: rgba(0, 255, 159, 0.1);
  }

  .key-display {
    background: var(--cyber-bg);
    border: 1px solid var(--cyber-border);
    padding: 12px;
    margin-bottom: 15px;
  }

  .key-value {
    font-size: 10px;
    word-break: break-all;
    color: var(--cyber-cyan);
    display: block;
  }

  .reminder-box {
    padding: 12px;
    background: rgba(0, 255, 159, 0.05);
    border: 1px solid var(--cyber-cyan);
    font-size: 11px;
    color: var(--cyber-text);
  }

  .reminder-box strong {
    color: var(--cyber-cyan);
    display: block;
    margin-bottom: 8px;
  }

  .reminder-box ul {
    margin: 0;
    padding-left: 16px;
  }

  .reminder-box li {
    margin: 4px 0;
    color: var(--cyber-text-dim);
  }

  /* Buttons */
  .button {
    width: 100%;
    padding: 12px;
    background: transparent;
    border: 1px solid var(--cyber-cyan);
    color: var(--cyber-cyan);
    font-family: inherit;
    font-size: 12px;
    cursor: pointer;
    margin-bottom: 8px;
    transition: all 0.2s;
  }

  .button:hover {
    background: rgba(0, 255, 159, 0.1);
  }

  .button.secondary {
    border-color: var(--cyber-border);
    color: var(--cyber-text-dim);
  }

  .button.secondary:hover {
    border-color: var(--cyber-text);
    color: var(--cyber-text);
    background: transparent;
  }

  .button.danger {
    border-color: #ff4444;
    color: #ff4444;
  }

  .button.danger:hover {
    background: rgba(255, 68, 68, 0.1);
  }
</style>
