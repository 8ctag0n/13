<script lang="ts">
  import { createEventDispatcher } from 'svelte';

  const dispatch = createEventDispatcher();

  function handleCreate() {
    dispatch('navigate', { route: 'create' });
  }

  function handleImport() {
    dispatch('navigate', { route: 'import' });
  }
</script>

<div class="welcome-container">
  <div class="logo-section">
    <div class="logo-glow">
      <pre class="logo-ascii">
   ╔══════════════════════╗
   ║  ░░░░░░░░░░░░░░░░░░  ║
   ║  ░░  ██████████  ░░  ║
   ║  ░░  ██  ██  ██  ░░  ║
   ║  ░░  ██████████  ░░  ║
   ║  ░░  ██  ██  ██  ░░  ║
   ║  ░░  ██████████  ░░  ║
   ║  ░░░░░░░░░░░░░░░░░░  ║
   ╚══════════════════════╝
      </pre>
    </div>

    <h1 class="title">[ZYBERLINK]</h1>
    <p class="subtitle">MULTI-CHAIN WALLET</p>

    <div class="chain-icons">
      <span class="chain-icon solana">◉ SOL</span>
      <span class="chain-icon starknet">▲ STRK</span>
      <span class="chain-icon zcash">Z ZEC</span>
    </div>
  </div>

  <div class="actions-section">
    <button class="button primary" on:click={handleCreate}>
      [+] CREATE NEW WALLET
    </button>

    <button class="button secondary" on:click={handleImport}>
      [↓] IMPORT EXISTING
    </button>
  </div>

  <div class="footer-section">
    <div class="footer-line"></div>
    <p class="footer-text">
      Non-custodial • Ed25519 • FHE-Ready
    </p>
    <p class="version">[v0.2.0]</p>
  </div>
</div>

<style>
  .welcome-container {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: space-between;
    min-height: 100%;
    padding: 20px;
    text-align: center;
  }

  .logo-section {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding-top: 20px;
  }

  .logo-glow {
    position: relative;
  }

  .logo-glow::before {
    content: '';
    position: absolute;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    width: 120px;
    height: 120px;
    background: radial-gradient(circle, rgba(0, 255, 159, 0.15) 0%, transparent 70%);
    border-radius: 50%;
    animation: pulse 3s ease-in-out infinite;
  }

  @keyframes pulse {
    0%, 100% { opacity: 0.5; transform: translate(-50%, -50%) scale(1); }
    50% { opacity: 1; transform: translate(-50%, -50%) scale(1.1); }
  }

  .logo-ascii {
    font-size: 10px;
    line-height: 1.2;
    color: var(--cyber-cyan);
    margin: 0;
    position: relative;
    z-index: 1;
  }

  .title {
    font-size: 28px;
    font-weight: bold;
    color: var(--cyber-cyan);
    margin: 15px 0 5px 0;
    letter-spacing: 4px;
    text-shadow: 0 0 20px rgba(0, 255, 159, 0.5);
  }

  .subtitle {
    font-size: 12px;
    color: var(--cyber-text-dim);
    letter-spacing: 3px;
    margin: 0 0 20px 0;
  }

  .chain-icons {
    display: flex;
    gap: 15px;
    margin-top: 10px;
  }

  .chain-icon {
    font-size: 11px;
    padding: 4px 8px;
    border: 1px solid var(--cyber-border);
    background: var(--cyber-bg-alt);
  }

  .chain-icon.solana {
    color: var(--cyber-cyan);
    border-color: var(--cyber-cyan);
  }

  .chain-icon.starknet {
    color: var(--cyber-purple);
    border-color: var(--cyber-purple);
  }

  .chain-icon.zcash {
    color: var(--cyber-yellow);
    border-color: var(--cyber-yellow);
  }

  .actions-section {
    width: 100%;
    max-width: 280px;
    margin: 30px 0;
  }

  .button {
    width: 100%;
    padding: 14px 20px;
    margin: 8px 0;
    background: transparent;
    border: 2px solid var(--cyber-cyan);
    color: var(--cyber-cyan);
    font-family: 'Courier New', monospace;
    font-size: 14px;
    font-weight: bold;
    cursor: pointer;
    transition: all 0.3s ease;
    position: relative;
    overflow: hidden;
  }

  .button::before {
    content: '';
    position: absolute;
    top: 0;
    left: -100%;
    width: 100%;
    height: 100%;
    background: linear-gradient(90deg, transparent, rgba(0, 255, 159, 0.2), transparent);
    transition: left 0.5s ease;
  }

  .button:hover::before {
    left: 100%;
  }

  .button:hover {
    background: rgba(0, 255, 159, 0.1);
    box-shadow: 0 0 25px rgba(0, 255, 159, 0.4);
    transform: translateY(-2px);
  }

  .button.secondary {
    border-color: var(--cyber-purple);
    color: var(--cyber-purple);
  }

  .button.secondary:hover {
    background: rgba(157, 78, 221, 0.1);
    box-shadow: 0 0 25px rgba(157, 78, 221, 0.4);
  }

  .footer-section {
    width: 100%;
    padding-top: 20px;
  }

  .footer-line {
    width: 80%;
    height: 1px;
    background: linear-gradient(90deg, transparent, var(--cyber-border), transparent);
    margin: 0 auto 15px auto;
  }

  .footer-text {
    font-size: 10px;
    color: var(--cyber-text-dim);
    letter-spacing: 1px;
    margin: 0 0 5px 0;
  }

  .version {
    font-size: 10px;
    color: var(--cyber-border);
    margin: 0;
  }
</style>
