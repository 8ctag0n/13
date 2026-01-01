<script>
  import { onMount } from 'svelte';

  let bootSteps = [
    { text: 'Initializing privacy layer...', completed: false, delay: 400 },
    { text: 'Loading FHE protocols...', completed: false, delay: 600 },
    { text: 'Connecting to network...', completed: false, delay: 500 },
    { text: 'Verifying zero-knowledge proofs...', completed: false, delay: 700 },
    { text: 'System ready.', completed: false, delay: 300 }
  ];

  let currentStep = -1;
  let bootComplete = false;

  onMount(() => {
    let totalDelay = 0;

    bootSteps.forEach((step, index) => {
      totalDelay += step.delay;
      setTimeout(() => {
        currentStep = index;
        bootSteps = bootSteps;
      }, totalDelay);

      setTimeout(() => {
        bootSteps[index].completed = true;
        bootSteps = bootSteps;

        if (index === bootSteps.length - 1) {
          bootComplete = true;
        }
      }, totalDelay + 200);
    });
  });
</script>

<div class="boot-sequence">
  <div class="boot-window">
    {#each bootSteps as step, i}
      {#if i <= currentStep}
        <div class="boot-line text-mono fade-in-fast"
             class:completed={step.completed}
             class:final={i === bootSteps.length - 1}>
          <span class="boot-prompt text-cyan">&gt;</span>
          <span class="boot-text">{step.text}</span>
          {#if step.completed && i < bootSteps.length - 1}
            <span class="boot-check text-success">[✓]</span>
          {/if}
        </div>
      {/if}
    {/each}

    {#if bootComplete}
      <div class="boot-cursor text-cyan fade-in">
        <span>&gt;</span> <span class="cursor-blink">█</span>
      </div>
    {/if}
  </div>
</div>

<style>
  .boot-sequence {
    width: 100%;
    margin: var(--space-6) 0;
  }

  .boot-window {
    background: rgba(0, 0, 0, 0.4);
    border: 1px solid var(--zyber-border-secondary);
    border-radius: var(--radius-md);
    padding: var(--space-6);
    font-family: 'Courier New', monospace;
    min-height: 180px;
    box-shadow: 0 4px 20px rgba(6, 182, 212, 0.1);
  }

  .boot-line {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    margin-bottom: var(--space-2);
    font-size: var(--text-sm);
    opacity: 0.7;
    transition: opacity 0.3s ease;
  }

  .boot-line.completed {
    opacity: 1;
  }

  .boot-line.final {
    font-weight: 600;
    color: var(--zyber-success);
    font-size: var(--text-base);
    margin-top: var(--space-4);
  }

  .boot-prompt {
    flex-shrink: 0;
    text-shadow: var(--zyber-glow-cyan);
  }

  .boot-text {
    flex: 1;
  }

  .boot-check {
    flex-shrink: 0;
    font-weight: 600;
    animation: checkmark-appear 0.3s ease;
  }

  .boot-cursor {
    margin-top: var(--space-4);
    font-size: var(--text-sm);
  }

  @keyframes checkmark-appear {
    from {
      opacity: 0;
      transform: scale(0.5);
    }
    to {
      opacity: 1;
      transform: scale(1);
    }
  }

  .fade-in-fast {
    animation: fadeIn 0.2s ease;
  }

  @keyframes fadeIn {
    from {
      opacity: 0;
      transform: translateY(-5px);
    }
    to {
      opacity: 0.7;
      transform: translateY(0);
    }
  }

  .fade-in {
    animation: fadeIn 0.3s ease;
  }
</style>
