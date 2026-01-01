<script>
  import { createEventDispatcher } from 'svelte';

  export let timelineData = [];
  export let activeNode = 0;

  const dispatch = createEventDispatcher();

  function scrollToNode(index) {
    dispatch('navigate', { index });
  }

  // Calcular progreso (0-100%)
  $: progress = timelineData.length > 0
    ? (activeNode / (timelineData.length - 1)) * 100
    : 0;
</script>

<div class="timeline-navigation">
  <!-- Progress Bar -->
  <div class="progress-container">
    <div class="progress-bar">
      <div class="progress-fill" style="width: {progress}%"></div>
      <div class="progress-glow" style="left: {progress}%"></div>
    </div>
  </div>

  <!-- Navigation Dots with Year Labels -->
  <div class="nav-dots">
    {#each timelineData as node, index}
      <button
        class="nav-node"
        class:active={activeNode === index}
        on:click={() => scrollToNode(index)}
        aria-label="Navigate to {node.year}"
      >
        <!-- Connection Line (entre dots) -->
        {#if index < timelineData.length - 1}
          <div class="node-connector" class:passed={activeNode > index}></div>
        {/if}

        <!-- Dot Container -->
        <div class="node-dot-container">
          <!-- Outer Glow -->
          <div class="node-glow color-{node.color}"></div>

          <!-- Main Dot -->
          <div class="node-dot color-{node.color}">
            <div class="dot-pulse color-{node.color}"></div>
            <div class="dot-inner">
              <span class="dot-icon">{node.icon}</span>
            </div>
          </div>

          <!-- Year Label -->
          <div class="year-label text-mono">
            <span class="year-bracket">[</span>
            {node.year}
            <span class="year-bracket">]</span>
          </div>

          <!-- Title Label (aparece en hover) -->
          <div class="title-label text-mono text-xs">
            {node.title}
          </div>
        </div>
      </button>
    {/each}
  </div>

  <!-- Navigation Arrows -->
  <div class="nav-arrows">
    <button
      class="arrow-button arrow-prev"
      disabled={activeNode === 0}
      on:click={() => scrollToNode(Math.max(0, activeNode - 1))}
      aria-label="Previous node"
    >
      <span class="text-mono">←</span>
    </button>

    <div class="nav-counter text-mono text-xs">
      <span class="counter-current text-cyan">{activeNode + 1}</span>
      <span class="counter-separator">/</span>
      <span class="counter-total text-muted">{timelineData.length}</span>
    </div>

    <button
      class="arrow-button arrow-next"
      disabled={activeNode === timelineData.length - 1}
      on:click={() => scrollToNode(Math.min(timelineData.length - 1, activeNode + 1))}
      aria-label="Next node"
    >
      <span class="text-mono">→</span>
    </button>
  </div>

  <!-- Keyboard Hint -->
  <div class="keyboard-hint text-mono text-xs text-muted">
    <span class="text-cyan">&lt;</span>
    USE_ARROWS_TO_NAVIGATE
    <span class="text-cyan">&gt;</span>
  </div>
</div>

<style>
  .timeline-navigation {
    width: 100%;
    max-width: 1200px;
    margin: 0 auto;
    padding: var(--space-6) var(--space-4);
    display: flex;
    flex-direction: column;
    gap: var(--space-6);
  }

  /* Progress Bar */
  .progress-container {
    width: 100%;
    padding: 0 var(--space-4);
  }

  .progress-bar {
    position: relative;
    width: 100%;
    height: 4px;
    background: rgba(6, 182, 212, 0.1);
    border-radius: var(--radius-full);
    overflow: visible;
  }

  .progress-fill {
    position: absolute;
    top: 0;
    left: 0;
    height: 100%;
    background: linear-gradient(
      90deg,
      var(--zyber-violet) 0%,
      var(--zyber-cyan) 50%,
      var(--zyber-success) 100%
    );
    border-radius: var(--radius-full);
    transition: width 0.6s cubic-bezier(0.4, 0, 0.2, 1);
    box-shadow: 0 0 20px rgba(6, 182, 212, 0.6);
  }

  .progress-glow {
    position: absolute;
    top: 50%;
    transform: translate(-50%, -50%);
    width: 12px;
    height: 12px;
    background: var(--zyber-cyan);
    border-radius: 50%;
    box-shadow: 0 0 30px rgba(6, 182, 212, 1),
                0 0 50px rgba(6, 182, 212, 0.6);
    transition: left 0.6s cubic-bezier(0.4, 0, 0.2, 1);
    animation: glow-pulse 2s ease-in-out infinite;
  }

  @keyframes glow-pulse {
    0%, 100% {
      opacity: 0.8;
      transform: translate(-50%, -50%) scale(1);
    }
    50% {
      opacity: 1;
      transform: translate(-50%, -50%) scale(1.3);
    }
  }

  /* Navigation Dots */
  .nav-dots {
    display: flex;
    justify-content: space-between;
    align-items: center;
    position: relative;
    padding: 0 var(--space-2);
  }

  .nav-node {
    background: none;
    border: none;
    padding: 0;
    cursor: pointer;
    position: relative;
    flex: 1;
    display: flex;
    justify-content: center;
  }

  /* Connection Line between dots */
  .node-connector {
    position: absolute;
    top: 20px;
    left: 50%;
    width: 100%;
    height: 2px;
    background: linear-gradient(
      90deg,
      var(--zyber-border-secondary) 0%,
      transparent 100%
    );
    z-index: 0;
    transition: all var(--transition-base);
  }

  .node-connector.passed {
    background: linear-gradient(
      90deg,
      var(--zyber-cyan) 0%,
      var(--zyber-border-secondary) 100%
    );
    box-shadow: 0 0 10px rgba(6, 182, 212, 0.5);
  }

  .node-dot-container {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-2);
    z-index: 1;
  }

  /* Outer Glow */
  .node-glow {
    position: absolute;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    width: 60px;
    height: 60px;
    border-radius: 50%;
    opacity: 0;
    transition: opacity var(--transition-base);
    pointer-events: none;
  }

  .node-glow.color-cyan {
    background: radial-gradient(circle, rgba(6, 182, 212, 0.4) 0%, transparent 70%);
  }

  .node-glow.color-violet {
    background: radial-gradient(circle, rgba(139, 92, 246, 0.4) 0%, transparent 70%);
  }

  .node-glow.color-success {
    background: radial-gradient(circle, rgba(34, 197, 94, 0.4) 0%, transparent 70%);
  }

  .nav-node:hover .node-glow,
  .nav-node.active .node-glow {
    opacity: 1;
  }

  /* Main Dot */
  .node-dot {
    position: relative;
    width: 40px;
    height: 40px;
    border-radius: 50%;
    background: rgba(0, 0, 0, 0.8);
    backdrop-filter: blur(10px);
    border: 2px solid var(--zyber-border-secondary);
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.4);
  }

  .node-dot.color-cyan {
    border-color: rgba(6, 182, 212, 0.5);
  }

  .node-dot.color-violet {
    border-color: rgba(139, 92, 246, 0.5);
  }

  .node-dot.color-success {
    border-color: rgba(34, 197, 94, 0.5);
  }

  .nav-node:hover .node-dot {
    transform: scale(1.2);
    box-shadow: 0 0 25px rgba(6, 182, 212, 0.6),
                0 4px 16px rgba(0, 0, 0, 0.5);
  }

  .nav-node:hover .node-dot.color-cyan {
    border-color: var(--zyber-cyan);
    background: rgba(6, 182, 212, 0.15);
  }

  .nav-node:hover .node-dot.color-violet {
    border-color: var(--zyber-violet);
    background: rgba(139, 92, 246, 0.15);
  }

  .nav-node:hover .node-dot.color-success {
    border-color: var(--zyber-success);
    background: rgba(34, 197, 94, 0.15);
  }

  .nav-node.active .node-dot {
    transform: scale(1.3);
  }

  .nav-node.active .node-dot.color-cyan {
    background: var(--zyber-cyan);
    border-color: var(--zyber-cyan);
    box-shadow: 0 0 40px rgba(6, 182, 212, 1),
                0 0 80px rgba(6, 182, 212, 0.5);
  }

  .nav-node.active .node-dot.color-violet {
    background: var(--zyber-violet);
    border-color: var(--zyber-violet);
    box-shadow: 0 0 40px rgba(139, 92, 246, 1),
                0 0 80px rgba(139, 92, 246, 0.5);
  }

  .nav-node.active .node-dot.color-success {
    background: var(--zyber-success);
    border-color: var(--zyber-success);
    box-shadow: 0 0 40px rgba(34, 197, 94, 1),
                0 0 80px rgba(34, 197, 94, 0.5);
  }

  /* Dot Pulse */
  .dot-pulse {
    position: absolute;
    top: -2px;
    left: -2px;
    right: -2px;
    bottom: -2px;
    border-radius: 50%;
    opacity: 0;
  }

  .nav-node.active .dot-pulse {
    animation: pulse-ring 2s ease-in-out infinite;
  }

  .dot-pulse.color-cyan {
    border: 2px solid var(--zyber-cyan);
  }

  .dot-pulse.color-violet {
    border: 2px solid var(--zyber-violet);
  }

  .dot-pulse.color-success {
    border: 2px solid var(--zyber-success);
  }

  @keyframes pulse-ring {
    0%, 100% {
      opacity: 0;
      transform: scale(1);
    }
    50% {
      opacity: 0.8;
      transform: scale(1.5);
    }
  }

  .dot-inner {
    font-size: var(--text-base);
    filter: grayscale(0.6);
    transition: all var(--transition-base);
  }

  .nav-node:hover .dot-inner,
  .nav-node.active .dot-inner {
    filter: grayscale(0) drop-shadow(0 0 8px rgba(0, 0, 0, 0.8));
    transform: scale(1.1);
  }

  /* Year Label */
  .year-label {
    font-size: var(--text-sm);
    color: var(--zyber-text-muted);
    transition: all var(--transition-base);
    opacity: 0.7;
  }

  .year-bracket {
    color: var(--zyber-cyan);
    opacity: 0.4;
    transition: opacity var(--transition-base);
  }

  .nav-node:hover .year-label,
  .nav-node.active .year-label {
    color: var(--zyber-cyan);
    opacity: 1;
    transform: scale(1.1);
  }

  .nav-node:hover .year-bracket,
  .nav-node.active .year-bracket {
    opacity: 1;
  }

  /* Title Label (hover tooltip) */
  .title-label {
    position: absolute;
    top: -48px;
    white-space: nowrap;
    background: rgba(0, 0, 0, 0.95);
    backdrop-filter: blur(10px);
    border: 1px solid var(--zyber-cyan);
    border-radius: var(--radius-sm);
    padding: var(--space-2) var(--space-3);
    color: var(--zyber-cyan);
    opacity: 0;
    pointer-events: none;
    transform: translateY(8px);
    transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
    box-shadow: 0 0 20px rgba(6, 182, 212, 0.4),
                0 4px 12px rgba(0, 0, 0, 0.5);
    z-index: 10;
  }

  .nav-node:hover .title-label {
    opacity: 1;
    transform: translateY(0);
  }

  /* Navigation Arrows */
  .nav-arrows {
    display: flex;
    justify-content: center;
    align-items: center;
    gap: var(--space-6);
    margin-top: var(--space-4);
  }

  .arrow-button {
    background: rgba(6, 182, 212, 0.1);
    border: 1px solid var(--zyber-border-secondary);
    border-radius: var(--radius-full);
    width: 48px;
    height: 48px;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transition: all var(--transition-base);
    font-size: var(--text-xl);
    color: var(--zyber-cyan);
  }

  .arrow-button:hover:not(:disabled) {
    background: rgba(6, 182, 212, 0.2);
    border-color: var(--zyber-cyan);
    transform: scale(1.1);
    box-shadow: 0 0 20px rgba(6, 182, 212, 0.4);
  }

  .arrow-button:active:not(:disabled) {
    transform: scale(0.95);
  }

  .arrow-button:disabled {
    opacity: 0.3;
    cursor: not-allowed;
  }

  .nav-counter {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-4);
    background: rgba(6, 182, 212, 0.05);
    border: 1px solid var(--zyber-border-secondary);
    border-radius: var(--radius-md);
  }

  .counter-current {
    font-weight: 600;
    font-size: var(--text-base);
  }

  .counter-separator {
    opacity: 0.5;
  }

  /* Keyboard Hint */
  .keyboard-hint {
    text-align: center;
    opacity: 0.6;
    animation: pulse-subtle 2s ease-in-out infinite;
  }

  @keyframes pulse-subtle {
    0%, 100% {
      opacity: 0.4;
    }
    50% {
      opacity: 0.8;
    }
  }

  /* Responsive */
  @media (max-width: 768px) {
    .timeline-navigation {
      padding: var(--space-4) var(--space-2);
      gap: var(--space-4);
    }

    .node-dot {
      width: 32px;
      height: 32px;
    }

    .dot-inner {
      font-size: var(--text-sm);
    }

    .year-label {
      font-size: var(--text-xs);
    }

    .title-label {
      display: none; /* Ocultar tooltips en mobile */
    }

    .arrow-button {
      width: 40px;
      height: 40px;
      font-size: var(--text-lg);
    }

    .keyboard-hint {
      display: none; /* Ocultar en mobile */
    }
  }
</style>
