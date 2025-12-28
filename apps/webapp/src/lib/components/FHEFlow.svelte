<script>
  export let animated = true;
</script>

<div class="fhe-flow" class:animated>
  <div class="flow-container text-mono">
    <div class="flow-node user-node">
      <div class="node-content">
        <div class="node-label text-xs text-muted">YOU</div>
      </div>
    </div>

    <div class="flow-arrow">
      <div class="arrow-line"></div>
      <div class="arrow-head">▶</div>
    </div>

    <div class="flow-node encrypt-node">
      <div class="node-box">
        <div class="node-border-top">╔═══════════╗</div>
        <div class="node-label-main text-cyan">ENCRYPTED</div>
        <div class="node-label-sub text-xs">COMPUTE</div>
        <div class="node-border-bottom">╚═══════════╝</div>
      </div>
      <div class="node-pulse"></div>
    </div>

    <div class="flow-arrow">
      <div class="arrow-line"></div>
      <div class="arrow-head">▶</div>
    </div>

    <div class="flow-node user-node">
      <div class="node-content">
        <div class="node-label text-xs text-muted">YOU</div>
      </div>
    </div>
  </div>

  <div class="flow-caption text-xs text-muted text-center mt-3">
    ZERO-KNOWLEDGE MAGIC
  </div>
</div>

<style>
  .fhe-flow {
    width: 100%;
    padding: var(--space-6) 0;
  }

  .flow-container {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: var(--space-4);
  }

  .flow-node {
    position: relative;
  }

  .user-node {
    width: 80px;
    height: 80px;
    border: 2px solid var(--zyber-border-secondary);
    border-radius: var(--radius-md);
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(0, 0, 0, 0.3);
    transition: all var(--transition-base);
  }

  .user-node:hover {
    border-color: var(--zyber-cyan);
    box-shadow: 0 0 20px rgba(6, 182, 212, 0.3);
  }

  .node-content {
    text-align: center;
  }

  .node-label {
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.1em;
  }

  .encrypt-node {
    position: relative;
  }

  .node-box {
    padding: var(--space-4) var(--space-6);
    background: rgba(6, 182, 212, 0.1);
    border: 2px solid var(--zyber-cyan);
    border-radius: var(--radius-md);
    position: relative;
    z-index: 2;
    transition: all var(--transition-base);
  }

  .encrypt-node:hover .node-box {
    background: rgba(6, 182, 212, 0.15);
    box-shadow: 0 0 30px rgba(6, 182, 212, 0.4);
  }

  .node-border-top,
  .node-border-bottom {
    font-size: var(--text-xs);
    text-shadow: var(--zyber-glow-cyan);
    line-height: 1;
  }

  .node-label-main {
    font-size: var(--text-sm);
    font-weight: 600;
    margin: var(--space-1) 0;
    text-shadow: var(--zyber-glow-cyan);
  }

  .node-label-sub {
    font-weight: 500;
    opacity: 0.8;
  }

  .node-pulse {
    position: absolute;
    top: -4px;
    left: -4px;
    right: -4px;
    bottom: -4px;
    border: 2px solid var(--zyber-cyan);
    border-radius: var(--radius-md);
    opacity: 0;
    z-index: 1;
  }

  .animated .node-pulse {
    animation: pulse-border 2s ease-in-out infinite;
  }

  .flow-arrow {
    display: flex;
    align-items: center;
    gap: 2px;
    color: var(--zyber-cyan);
  }

  .arrow-line {
    width: 40px;
    height: 2px;
    background: var(--zyber-cyan);
    position: relative;
  }

  .animated .arrow-line {
    animation: flow-pulse 2s ease-in-out infinite;
  }

  .arrow-head {
    font-size: var(--text-sm);
    text-shadow: var(--zyber-glow-cyan);
    animation: arrow-pulse 2s ease-in-out infinite;
  }

  .flow-caption {
    font-style: italic;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    opacity: 0.7;
  }

  @keyframes pulse-border {
    0%, 100% {
      opacity: 0;
      transform: scale(1);
    }
    50% {
      opacity: 0.5;
      transform: scale(1.05);
    }
  }

  @keyframes flow-pulse {
    0%, 100% {
      opacity: 0.5;
    }
    50% {
      opacity: 1;
      box-shadow: 0 0 10px var(--zyber-cyan);
    }
  }

  @keyframes arrow-pulse {
    0%, 100% {
      opacity: 0.7;
      transform: translateX(0);
    }
    50% {
      opacity: 1;
      transform: translateX(4px);
    }
  }

  /* Responsive */
  @media (max-width: 768px) {
    .flow-container {
      flex-direction: column;
      gap: var(--space-3);
    }

    .flow-arrow {
      transform: rotate(90deg);
    }

    .arrow-line {
      width: 30px;
    }
  }
</style>
