<script>
  import { createEventDispatcher } from 'svelte';

  const dispatch = createEventDispatcher();

  export let checked = false;
  export let name = '';
  export let value = '';
  export let label = '';
  export let description = '';
  export let disabled = false;

  function handleChange(event) {
    checked = event.currentTarget.checked;
    dispatch('change', { value, checked });
  }
</script>

<label class="terminal-radio {checked ? 'checked' : ''} {disabled ? 'disabled' : ''}">
  <input
    type="radio"
    {name}
    {value}
    {checked}
    {disabled}
    on:change={handleChange}
  />
  <span class="indicator text-mono">{checked ? '[*]' : '[ ]'}</span>
  <span class="radio-label text-mono">{label}</span>
  {#if description}
    <span class="radio-description text-mono text-xs text-muted">{description}</span>
  {/if}
</label>

<style>
  .terminal-radio {
    display: grid;
    grid-template-columns: auto 1fr;
    align-items: center;
    gap: var(--space-2) var(--space-3);
    padding: var(--space-2) var(--space-3);
    border: 1px solid var(--zyber-border-muted);
    background: rgba(0, 0, 0, 0.35);
    cursor: pointer;
    transition: all var(--transition-base);
  }

  .terminal-radio input {
    position: absolute;
    opacity: 0;
    pointer-events: none;
  }

  .terminal-radio:hover {
    border-color: var(--zyber-border-secondary);
  }

  .terminal-radio.checked {
    border-color: var(--zyber-cyber-cyan);
    box-shadow: var(--zyber-glow-cyan);
  }

  .terminal-radio.disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .terminal-radio:focus-within {
    outline: 1px solid var(--zyber-cyber-cyan);
    outline-offset: 2px;
  }

  .indicator {
    color: var(--zyber-cyber-cyan);
  }

  .radio-label {
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .radio-description {
    grid-column: 2;
  }
</style>
