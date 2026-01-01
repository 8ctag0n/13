<script>
  export let compact = false;
</script>

<div class="futarchy-logo {compact ? 'compact' : ''}">
  <div class="logo-title text-mono">FUTARCHY.MARKETS</div>
  <pre class="logo-ascii text-mono">
███████╗██╗   ██╗████████╗ █████╗ ██████╗  ██████╗██╗  ██╗██╗   ██╗
██╔════╝██║   ██║╚══██╔══╝██╔══██╗██╔══██╗██╔════╝██║  ██║╚██╗ ██╔╝
█████╗  ██║   ██║   ██║   ███████║██████╔╝██║     ███████║ ╚████╔╝
██╔══╝  ██║   ██║   ██║   ██╔══██║██╔══██╗██║     ██╔══██║  ╚██╔╝
██║     ╚██████╔╝   ██║   ██║  ██║██║  ██║╚██████╗██║  ██║   ██║
╚═╝      ╚═════╝    ╚═╝   ╚═╝  ╚═╝╚═╝  ╚═╝ ╚═════╝╚═╝  ╚═╝   ╚═╝
  </pre>
</div>

<style>
  .futarchy-logo {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-2);
  }

  .logo-title {
    color: var(--zyber-cyber-cyan);
    text-shadow: var(--zyber-glow-cyan);
    letter-spacing: 0.2em;
    font-size: var(--text-xs);
  }

  .logo-ascii {
    font-size: 9px;
    line-height: 1.15;
    margin: 0;
    background: linear-gradient(135deg, var(--zyber-cyber-cyan), var(--zyber-quantum-violet));
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
  }

  .compact .logo-ascii {
    font-size: 6px;
  }

  @media (max-width: 720px) {
    .logo-ascii {
      font-size: 6px;
    }
  }
</style>
