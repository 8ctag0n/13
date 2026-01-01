<script>
  // Logo ASCII con efecto de desencriptación
</script>

<div class="logo">
  <div class="logo-encryption text-mono">
    <span class="char-encrypted">▓▓▓</span><span class="char-semi">▒▒▒</span><span class="char-light">░░░</span>
  </div>
  <pre class="logo-text text-mono glow-pulse">
███████╗██╗   ██╗██████╗ ███████╗██████╗ ██╗     ██╗███╗   ██╗██╗  ██╗
╚══███╔╝╚██╗ ██╔╝██╔══██╗██╔════╝██╔══██╗██║     ██║████╗  ██║██║ ██╔╝
   ███╔╝  ╚████╔╝ ██████╔╝█████╗  ██████╔╝██║     ██║██╔██╗ ██║█████╔╝
  ███╔╝    ╚██╔╝  ██╔══██╗██╔══╝  ██╔══██╗██║     ██║██║╚██╗██║██╔═██╗
 ███████╗   ██║   ██████╔╝███████╗██║  ██║███████╗██║██║ ╚████║██║  ██╗
 ╚══════╝   ╚═╝   ╚═════╝ ╚══════╝╚═╝  ╚═╝╚══════╝╚═╝╚═╝  ╚═══╝╚═╝  ╚═╝
  </pre>
</div>

<style>
  .logo {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-2);
    margin-bottom: var(--space-6);
  }

  .logo-encryption {
    font-size: var(--text-2xl);
    letter-spacing: 0.1em;
  }

  .char-encrypted {
    color: var(--zyber-quantum-violet);
    text-shadow: var(--zyber-glow-violet);
    animation: glow-pulse 2s ease-in-out infinite;
  }

  .char-semi {
    color: var(--zyber-cyber-cyan);
    text-shadow: var(--zyber-glow-cyan);
    opacity: 0.7;
  }

  .char-light {
    color: var(--zyber-cyber-cyan);
    opacity: 0.3;
  }

  .logo-text {
    font-size: 10px;
    line-height: 1.1;
    background: linear-gradient(
      135deg,
      var(--zyber-quantum-violet),
      var(--zyber-cyber-cyan)
    );
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
    text-shadow: none;
    margin: 0;
  }

  /* Responsive */
  @media (max-width: 768px) {
    .logo-text {
      font-size: 6px;
    }

    .logo-encryption {
      font-size: var(--text-lg);
    }
  }

  @media (max-width: 480px) {
    .logo-text {
      font-size: 5px;
    }
  }
</style>
