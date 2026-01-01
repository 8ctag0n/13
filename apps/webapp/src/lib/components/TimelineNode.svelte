<script>
  import { onMount } from 'svelte';

  export let year;
  export let title;
  export let subtitle;
  export let description;
  export let icon;
  export let color = 'cyan';
  export let active = false;
  export let link = null;
  export let linkLabel = '';

  let expanded = false;
  let titleElement;
  let subtitleElement;
  let yearElement;
  let hasDecrypted = false;

  // Decrypt effect characters
  const cypherChars = '⟁⧖⟟⍦⌧01xX|\\/*#@$%&';

  function decryptEffect(el, text, speed = 25) {
    if (!el) return;
    let i = 0;
    const interval = setInterval(() => {
      const partial = text.substring(0, i);
      const scramble = Array(text.length - i)
        .fill()
        .map(() => cypherChars[Math.floor(Math.random() * cypherChars.length)])
        .join('');
      el.textContent = partial + scramble;
      i++;
      if (i > text.length) {
        clearInterval(interval);
        el.textContent = text;
      }
    }, speed);
  }

  function triggerDecrypt() {
    if (hasDecrypted) return;
    hasDecrypted = true;
    decryptEffect(yearElement, year, 40);
    setTimeout(() => decryptEffect(titleElement, title, 30), 100);
    setTimeout(() => decryptEffect(subtitleElement, subtitle, 20), 300);
  }

  // Trigger decrypt when becoming active
  $: if (active && !hasDecrypted) {
    triggerDecrypt();
  }

  function toggleExpand() {
    expanded = !expanded;
    if (!hasDecrypted) triggerDecrypt();
  }

  function openLink(url) {
    if (url) {
      window.open(url, '_blank', 'noopener,noreferrer');
    }
  }

  onMount(() => {
    // Initialize with scrambled text
    if (yearElement) yearElement.textContent = Array(year.length).fill().map(() => cypherChars[Math.floor(Math.random() * cypherChars.length)]).join('');
    if (titleElement) titleElement.textContent = Array(title.length).fill().map(() => cypherChars[Math.floor(Math.random() * cypherChars.length)]).join('');
    if (subtitleElement) subtitleElement.textContent = Array(subtitle.length).fill().map(() => cypherChars[Math.floor(Math.random() * cypherChars.length)]).join('');
  });
</script>

<div class="timeline-node" class:active class:expanded>
  <button class="node-button" on:click={toggleExpand}>
    <!-- Connection Line -->
    <div class="node-line color-{color}"></div>

    <!-- Node Dot -->
    <div class="node-dot color-{color}">
      <div class="node-pulse color-{color}"></div>
      <div class="dot-inner text-mono">{icon}</div>
    </div>

    <!-- Node Card -->
    <div class="node-card tui-box">
      <!-- ASCII Corners -->
      <div class="corner-tl text-mono glow-{color}">╔═</div>
      <div class="corner-tr text-mono glow-{color}">═╗</div>
      <div class="corner-bl text-mono glow-{color}">╚═</div>
      <div class="corner-br text-mono glow-{color}">═╝</div>

      <div class="node-content">
        <div class="node-year text-mono text-{color}" bind:this={yearElement}>{year}</div>
        <div class="node-title text-mono" bind:this={titleElement}>{title}</div>
        <div class="node-subtitle text-xs text-muted" bind:this={subtitleElement}>{subtitle}</div>

        {#if expanded}
          <div class="node-description text-sm mt-4 fade-in">
            <div class="description-text text-muted">
              {description}
            </div>

            {#if link || linkLabel}
              <div class="node-link mt-4">
                <a
                  href={link || '#'}
                  target={link ? '_blank' : '_self'}
                  rel={link ? 'noopener noreferrer' : ''}
                  class="link-button text-mono text-xs"
                  class:disabled={!link}
                  on:click|stopPropagation={link ? null : (e) => e.preventDefault()}
                >
                  <span class="link-icon text-{color}">→</span>
                  <span class="link-text">{linkLabel}</span>
                </a>
              </div>
            {/if}
          </div>
        {/if}

        <div class="expand-hint text-xs text-muted mt-3">
          {expanded ? '[-] Collapse' : '[+] Expand'}
        </div>
      </div>
    </div>
  </button>
</div>

<style>
  .timeline-node {
    flex-shrink: 0;
    width: 320px;
    position: relative;
    scroll-snap-align: center;
  }

  .node-button {
    background: none;
    border: none;
    padding: 0;
    cursor: pointer;
    width: 100%;
    text-align: left;
    position: relative;
  }

  /* Connection Line */
  .node-line {
    position: absolute;
    top: 32px;
    left: -160px;
    width: 320px;
    height: 2px;
    background: var(--zyber-border-secondary);
    z-index: 0;
  }

  .timeline-node:first-child .node-line {
    display: none;
  }

  .node-line.color-cyan { background: linear-gradient(90deg, transparent, var(--zyber-cyan)); }
  .node-line.color-violet { background: linear-gradient(90deg, transparent, var(--zyber-violet)); }
  .node-line.color-success { background: linear-gradient(90deg, transparent, var(--zyber-success)); }

  /* Node Dot */
  .node-dot {
    position: absolute;
    top: 16px;
    left: 50%;
    transform: translateX(-50%);
    width: 32px;
    height: 32px;
    border-radius: 50%;
    background: var(--zyber-bg-primary);
    border: 2px solid var(--zyber-border-secondary);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 10;
    transition: all var(--transition-base);
  }

  .node-dot.color-cyan {
    border-color: var(--zyber-cyan);
    box-shadow: 0 0 20px rgba(6, 182, 212, 0.3);
  }

  .node-dot.color-violet {
    border-color: var(--zyber-violet);
    box-shadow: 0 0 20px rgba(139, 92, 246, 0.3);
  }

  .node-dot.color-success {
    border-color: var(--zyber-success);
    box-shadow: 0 0 20px rgba(34, 197, 94, 0.3);
  }

  .node-pulse {
    position: absolute;
    top: -2px;
    left: -2px;
    right: -2px;
    bottom: -2px;
    border-radius: 50%;
    opacity: 0;
  }

  .active .node-pulse {
    animation: pulse 2s ease-in-out infinite;
  }

  .node-pulse.color-cyan {
    border: 2px solid var(--zyber-cyan);
  }

  .node-pulse.color-violet {
    border: 2px solid var(--zyber-violet);
  }

  .node-pulse.color-success {
    border: 2px solid var(--zyber-success);
  }

  .dot-inner {
    font-size: var(--text-xs);
  }

  /* Node Card */
  .node-card {
    margin-top: 60px;
    padding: var(--space-6);
    min-height: 200px;
    position: relative;
    background: linear-gradient(
      135deg,
      rgba(0, 0, 0, 0.5) 0%,
      rgba(6, 182, 212, 0.05) 50%,
      rgba(0, 0, 0, 0.5) 100%
    );
    backdrop-filter: blur(15px);
    transition: all 0.4s cubic-bezier(0.4, 0, 0.2, 1);
    box-shadow: 0 4px 20px rgba(0, 0, 0, 0.3);
  }

  .node-button:hover .node-card {
    transform: translateY(-8px);
    box-shadow: 0 12px 40px rgba(6, 182, 212, 0.3),
                0 0 60px rgba(6, 182, 212, 0.1);
    background: linear-gradient(
      135deg,
      rgba(0, 0, 0, 0.5) 0%,
      rgba(6, 182, 212, 0.1) 50%,
      rgba(0, 0, 0, 0.5) 100%
    );
  }

  .active .node-card {
    border-color: var(--zyber-cyan);
    box-shadow: 0 8px 30px rgba(6, 182, 212, 0.4),
                0 0 80px rgba(6, 182, 212, 0.2);
  }

  .expanded .node-card {
    min-height: 300px;
  }

  /* ASCII Corners */
  .corner-tl, .corner-tr, .corner-bl, .corner-br {
    position: absolute;
    font-size: var(--text-lg);
    opacity: 0.3;
    transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
  }

  .corner-tl { top: -4px; left: -4px; }
  .corner-tr { top: -4px; right: -4px; }
  .corner-bl { bottom: -4px; left: -4px; }
  .corner-br { bottom: -4px; right: -4px; }

  .node-button:hover .corner-tl,
  .node-button:hover .corner-tr,
  .node-button:hover .corner-bl,
  .node-button:hover .corner-br {
    opacity: 0.8;
  }

  .active .corner-tl,
  .active .corner-tr,
  .active .corner-bl,
  .active .corner-br {
    opacity: 1;
    animation: corner-pulse 2s ease-in-out infinite;
  }

  @keyframes corner-pulse {
    0%, 100% {
      opacity: 1;
      filter: drop-shadow(0 0 3px currentColor);
    }
    50% {
      opacity: 0.7;
      filter: drop-shadow(0 0 8px currentColor);
    }
  }

  .glow-cyan {
    color: var(--zyber-cyan);
    text-shadow: var(--zyber-glow-cyan);
  }

  .glow-violet {
    color: var(--zyber-violet);
    text-shadow: var(--zyber-glow-violet);
  }

  .glow-success {
    color: var(--zyber-success);
    text-shadow: 0 0 10px var(--zyber-success);
  }

  /* Node Content */
  .node-content {
    position: relative;
    z-index: 1;
  }

  .node-year {
    font-size: var(--text-2xl);
    font-weight: 600;
    margin-bottom: var(--space-2);
    letter-spacing: 0.1em;
    transition: text-shadow 0.3s ease;
  }

  .node-title {
    font-size: var(--text-base);
    font-weight: 600;
    margin-bottom: var(--space-1);
    transition: text-shadow 0.3s ease;
  }

  .node-subtitle {
    text-transform: uppercase;
    letter-spacing: 0.05em;
    transition: text-shadow 0.3s ease;
  }

  /* Glow effect during decrypt */
  .active .node-year,
  .active .node-title {
    text-shadow: 0 0 10px currentColor;
  }

  .node-description {
    padding-top: var(--space-4);
    border-top: 1px solid var(--zyber-border-secondary);
  }

  .description-text {
    line-height: 1.6;
  }

  .expand-hint {
    opacity: 0.6;
    transition: opacity var(--transition-base);
  }

  .node-button:hover .expand-hint {
    opacity: 1;
  }

  /* Link Button */
  .node-link {
    display: flex;
    justify-content: center;
  }

  .link-button {
    background: rgba(6, 182, 212, 0.1);
    border: 1px solid var(--zyber-border-secondary);
    border-radius: var(--radius-md);
    padding: var(--space-2) var(--space-4);
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    cursor: pointer;
    transition: all var(--transition-base);
    text-decoration: none;
    color: inherit;
  }

  .link-button:not(.disabled):hover {
    background: rgba(6, 182, 212, 0.2);
    border-color: var(--zyber-cyan);
    transform: translateX(4px);
  }

  .link-button.disabled {
    opacity: 0.5;
    cursor: not-allowed;
    pointer-events: none;
  }

  .link-icon {
    font-weight: 600;
    transition: transform var(--transition-base);
  }

  .link-button:not(.disabled):hover .link-icon {
    transform: translateX(4px);
  }

  .link-text {
    color: var(--zyber-text-primary);
  }

  /* Color Variants */
  .text-cyan { color: var(--zyber-cyan); }
  .text-violet { color: var(--zyber-violet); }
  .text-success { color: var(--zyber-success); }

  .color-cyan { border-color: var(--zyber-cyan); }
  .color-violet { border-color: var(--zyber-violet); }
  .color-success { border-color: var(--zyber-success); }

  @keyframes pulse {
    0%, 100% {
      opacity: 0;
      transform: scale(1);
    }
    50% {
      opacity: 0.5;
      transform: scale(1.2);
    }
  }

  .fade-in {
    animation: fadeIn 0.3s ease;
  }

  @keyframes fadeIn {
    from {
      opacity: 0;
      transform: translateY(-10px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }
</style>
