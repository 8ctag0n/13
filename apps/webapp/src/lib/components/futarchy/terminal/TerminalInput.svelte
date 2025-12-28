<script>
  import { createEventDispatcher } from 'svelte';

  const dispatch = createEventDispatcher();

  export let value = '';
  export let label = '';
  export let placeholder = '';
  export let disabled = false;
  export let suffix = '';
  export let prefix = '>';
  export let name = '';
  export let type = 'text';
  export let hint = '';

  function handleInput() {
    dispatch('input', value);
  }

  function handleKeydown(event) {
    dispatch('keydown', event);
  }
</script>

<div class="terminal-input {disabled ? 'disabled' : ''}">
  {#if label}
    <div class="input-label text-mono text-xs text-muted">{label}</div>
  {/if}

  <div class="input-field">
    {#if prefix}
      <span class="input-prefix text-mono">{prefix}</span>
    {/if}
    <input
      class="text-mono"
      {name}
      {type}
      bind:value
      {placeholder}
      {disabled}
      on:input={handleInput}
      on:keydown={handleKeydown}
    />
    {#if suffix}
      <span class="input-suffix text-mono">{suffix}</span>
    {/if}
  </div>

  {#if hint}
    <div class="input-hint text-mono text-xs text-muted">{hint}</div>
  {/if}
</div>

<style>
  .terminal-input {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .input-label {
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }

  .input-field {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
    background: rgba(0, 0, 0, 0.35);
    border: 1px solid var(--zyber-border-secondary);
    box-shadow: inset 0 0 12px rgba(6, 182, 212, 0.08);
    position: relative;
  }

  .input-field::after {
    content: '';
    position: absolute;
    left: var(--space-3);
    right: var(--space-3);
    bottom: 6px;
    height: 1px;
    background: repeating-linear-gradient(
      90deg,
      rgba(6, 182, 212, 0.4),
      rgba(6, 182, 212, 0.4) 6px,
      transparent 6px,
      transparent 10px
    );
    pointer-events: none;
  }

  .input-prefix {
    color: var(--zyber-cyber-cyan);
  }

  .input-suffix {
    color: var(--zyber-text-secondary);
    white-space: nowrap;
  }

  input {
    flex: 1;
    background: transparent;
    border: none;
    outline: none;
    color: var(--zyber-text-primary);
    font-size: var(--text-sm);
  }

  input::placeholder {
    color: var(--zyber-text-muted);
  }

  .terminal-input.disabled {
    opacity: 0.6;
  }

  .input-hint {
    line-height: 1.4;
  }
</style>
