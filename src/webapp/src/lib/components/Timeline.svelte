<script>
  import { onMount, onDestroy } from 'svelte';

  let activeLevel = -1;
  let decryptBox;
  let levelElements = [];
  let decryptedLevels = new Set(); // Track which levels have been decrypted
  let isVisible = false; // Lazy load - only render when visible
  let containerRef;
  let visibilityObserver;
  let activeIntervals = []; // Track all intervals for cleanup

  const cypherChars = '⟁⧖⟟⍦⌧01xX|\\/*';

  const levels = [
    {
      id: 'the-first-lock',
      file: 'the-first-lock.rsa',
      year: '1977',
      title: 'LVL 1 — The First Lock',
      subtitle: 'RSA Algorithm',
      description: 'Cryptography is born. The lock appears.',
      insight: 'Ron Rivest, Adi Shamir, and Leonard Adleman invented RSA, the first practical public-key cryptosystem. For the first time, two parties could communicate securely without sharing a secret key beforehand. This breakthrough made the internet as we know it possible.',
      color: 'cyan',
      link: 'https://people.csail.mit.edu/rivest/Rsapaper.pdf',
      linkLabel: 'Read Original Paper'
    },
    {
      id: 'privacy-as-right',
      file: 'privacy-as-a-right.pgp',
      year: '1993',
      title: 'LVL 2 — Privacy as a Right',
      subtitle: 'PGP / Cypherpunks',
      description: '"Privacy is necessary for an open society."',
      insight: 'Phil Zimmermann released PGP (Pretty Good Privacy), bringing military-grade encryption to the masses. Eric Hughes declared: "Privacy is necessary for an open society." The cypherpunk movement was born—privacy as a fundamental human right, enforced by mathematics, not laws.',
      color: 'violet',
      link: 'https://www.activism.net/cypherpunk/manifesto.html',
      linkLabel: 'Read Manifesto'
    },
    {
      id: 'trustless-value',
      file: 'trustless-value.btc',
      year: '2009',
      title: 'LVL 3 — Trustless Value',
      subtitle: 'Bitcoin',
      description: 'Value moves without permission.',
      insight: 'Satoshi Nakamoto published the Bitcoin whitepaper, introducing blockchain and decentralized consensus. For the first time, digital scarcity and trustless value transfer became reality. No banks, no intermediaries, no single point of failure—just pure cryptographic truth.',
      color: 'success',
      link: 'https://bitcoin.org/bitcoin.pdf',
      linkLabel: 'Read Whitepaper'
    },
    {
      id: 'hide-the-truth',
      file: 'hide-the-truth.zkp',
      year: '2014',
      title: 'LVL 4 — Hide the Truth',
      subtitle: 'ZK-SNARKs',
      description: 'Prove without revealing.',
      insight: 'Zero-Knowledge Succinct Non-Interactive Arguments of Knowledge (ZK-SNARKs) went from theory to practice in Zcash. You could now prove the truth of a statement without revealing anything beyond its validity. Privacy and verification, together at last.',
      color: 'violet',
      link: 'https://zerocash-project.org/media/pdf/zerocash-extended-20140518.pdf',
      linkLabel: 'Read Zerocash Paper'
    },
    {
      id: 'compute-blind',
      file: 'compute-blind.fhe',
      year: '2024',
      title: 'LVL 5 — Compute Blind',
      subtitle: 'FHE / ZyberLink',
      description: 'Computation without decryption.',
      insight: 'Fully Homomorphic Encryption (FHE) meets decentralized networks. ZyberLink enables computation on encrypted data without ever decrypting it. Your data remains private end-to-end, even from those processing it. The ultimate realization of the cypherpunk vision: computation without trust.',
      color: 'cyan',
      link: null,
      linkLabel: 'Coming Soon'
    }
  ];

  function decryptEffect(el, text, speed = 30) {
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
        // Remove from tracked intervals
        activeIntervals = activeIntervals.filter(id => id !== interval);
        el.textContent = text;
      }
    }, speed);
    // Track interval for cleanup
    activeIntervals.push(interval);
  }

  // Clear all active intervals
  function clearAllIntervals() {
    activeIntervals.forEach(id => clearInterval(id));
    activeIntervals = [];
  }

  function decryptTitle(el, text) {
    decryptEffect(el, text, 25);
  }

  function showDecryptPanel(level, index) {
    if (activeLevel === index) return;
    activeLevel = index;

    // Clear panel
    if (decryptBox) {
      decryptBox.innerHTML = '';

      const panelLines = [
        `> mount /levels/${level.file}`,
        `> status: SCANNING...`,
        `> decrypt: OK`,
        `> year: ${level.year}`,
        `> subtitle: ${level.subtitle}`,
        `> insight: ${level.insight}`,
      ];

      // Add link line if available
      if (level.link) {
        panelLines.push(`> source: ${level.linkLabel}`);
      }

      let lineIndex = 0;
      const panelInterval = setInterval(() => {
        if (lineIndex >= panelLines.length) {
          clearInterval(panelInterval);
          activeIntervals = activeIntervals.filter(id => id !== panelInterval);
          // Add clickable link at the end
          if (level.link) {
            setTimeout(() => {
              if (decryptBox) {
                const linkEl = document.createElement('a');
                linkEl.href = level.link;
                linkEl.target = '_blank';
                linkEl.rel = 'noopener noreferrer';
                linkEl.className = 'decrypt-link text-mono';
                linkEl.textContent = `[-> ${level.linkLabel}]`;
                decryptBox.appendChild(linkEl);
              }
            }, 200);
          }
          return;
        }
        const line = document.createElement('div');
        line.className = 'decrypt-line';
        if (lineIndex === 0) line.className += ' text-muted';
        if (lineIndex === 2) line.className += ' text-success';
        if (lineIndex === 3) line.className += ' text-cyan';
        if (lineIndex === 4) line.className += ' text-violet';
        if (lineIndex === 5) line.className += ' text-muted'; // insight
        decryptEffect(line, panelLines[lineIndex], 20);
        decryptBox.appendChild(line);
        lineIndex++;
      }, 180);
      activeIntervals.push(panelInterval);
    }

    // Decrypt the level title (only if not already decrypted)
    if (!decryptedLevels.has(index)) {
      const titleEl = levelElements[index]?.querySelector('.level-title');
      if (titleEl) {
        decryptTitle(titleEl, level.title);
        decryptedLevels.add(index);
      }
    }
  }

  // Generate scrambled text for a given length
  function generateScramble(length) {
    let result = '';
    for (let i = 0; i < length; i++) {
      result += cypherChars[Math.floor(Math.random() * cypherChars.length)];
    }
    return result;
  }

  let levelObserver;

  function initializeTimeline() {
    if (!isVisible) return;

    // Initialize all titles with scrambled text (only once)
    levelElements.forEach((el, i) => {
      if (el) {
        const titleEl = el.querySelector('.level-title');
        if (titleEl && titleEl.textContent === '') {
          titleEl.textContent = generateScramble(levels[i].title.length);
        }
      }
    });

    // Set up intersection observer for scroll-based activation
    if (levelObserver) levelObserver.disconnect();

    levelObserver = new IntersectionObserver(
      (entries) => {
        entries.forEach((entry) => {
          if (entry.isIntersecting) {
            const index = parseInt(entry.target.dataset.index);
            showDecryptPanel(levels[index], index);
          }
        });
      },
      { threshold: 0.6 }
    );

    levelElements.forEach((el, i) => {
      if (el) {
        el.dataset.index = i;
        levelObserver.observe(el);
      }
    });
  }

  onMount(() => {
    // Lazy load: only initialize when visible
    visibilityObserver = new IntersectionObserver(
      (entries) => {
        entries.forEach((entry) => {
          if (entry.isIntersecting && !isVisible) {
            isVisible = true;
            // Small delay to ensure DOM is ready
            setTimeout(initializeTimeline, 100);
          }
        });
      },
      { threshold: 0.1, rootMargin: '100px' }
    );

    if (containerRef) {
      visibilityObserver.observe(containerRef);
    }
  });

  onDestroy(() => {
    // Clean up all intervals
    clearAllIntervals();
    // Disconnect observers
    if (visibilityObserver) visibilityObserver.disconnect();
    if (levelObserver) levelObserver.disconnect();
  });
</script>

<section id="timeline" class="timeline-section" bind:this={containerRef}>
  <div class="timeline-header">
    <h2 class="text-mono text-uppercase text-center mb-2">
      <span class="text-violet">></span> The Cipher Chronicles
    </h2>
    <p class="text-sm text-muted text-center text-mono mb-8">
      SCROLL_TO_DECRYPT_HISTORY
    </p>
  </div>

  {#if isVisible}
    <div class="timeline-container">
      <!-- Left Column: Levels -->
      <div class="levels-column">
        {#each levels as level, index}
          <button
            class="level tui-box"
            class:active={activeLevel === index}
            bind:this={levelElements[index]}
            on:click={() => showDecryptPanel(level, index)}
          >
            <div class="level-title text-mono text-{level.color}"></div>
            <div class="level-year text-mono text-xs text-muted">{level.year} — {level.subtitle}</div>
            <p class="level-desc text-sm text-muted">{level.description}</p>
          </button>
        {/each}
      </div>

      <!-- Right Column: Decrypt Info -->
      <div class="details-column tui-box">
        <div class="details-header text-mono text-xs text-muted mb-4">
          > DECRYPT_OUTPUT
        </div>
        <div class="decrypt-box text-mono" bind:this={decryptBox}>
          <div class="text-muted">Select a level to decrypt...</div>
        </div>
        <div class="terminal-cursor"></div>
      </div>
    </div>
  {:else}
    <!-- Placeholder while loading -->
    <div class="timeline-placeholder">
      <div class="text-mono text-muted text-center">
        [LOADING_CIPHER_CHRONICLES...]
      </div>
    </div>
  {/if}
</section>

<style>
  .timeline-section {
    width: 100%;
    padding: var(--space-12) 0;
    background: linear-gradient(180deg, transparent 0%, rgba(6, 182, 212, 0.02) 50%, transparent 100%);
    border-top: 1px solid var(--zyber-border-secondary);
    border-bottom: 1px solid var(--zyber-border-secondary);
  }

  .timeline-placeholder {
    min-height: 400px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .timeline-header {
    margin-bottom: var(--space-8);
  }

  .timeline-header h2 {
    font-size: var(--text-2xl);
    font-weight: 600;
  }

  .timeline-container {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--space-6);
    max-width: 1200px;
    margin: 0 auto;
    padding: 0 var(--space-6);
  }

  /* Left Column - Levels */
  .levels-column {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }

  .level {
    width: 100%;
    padding: var(--space-4);
    background: rgba(6, 182, 212, 0.03);
    border: 1px solid var(--zyber-border-muted);
    cursor: pointer;
    text-align: left;
    transition: all 0.3s ease;
  }

  .level:hover {
    transform: translateX(8px);
    border-color: var(--zyber-cyber-cyan);
    box-shadow: 0 0 20px rgba(6, 182, 212, 0.2);
    background: rgba(6, 182, 212, 0.08);
  }

  .level.active {
    border-color: var(--zyber-cyber-cyan);
    background: rgba(6, 182, 212, 0.1);
    box-shadow: 0 0 30px rgba(6, 182, 212, 0.3);
  }

  .level-title {
    font-size: var(--text-base);
    font-weight: 600;
    margin-bottom: var(--space-1);
    letter-spacing: 0.02em;
  }

  .level-year {
    margin-bottom: var(--space-2);
    opacity: 0.8;
  }

  .level-desc {
    margin: 0;
    line-height: 1.4;
  }

  /* Right Column - Decrypt Panel */
  .details-column {
    position: sticky;
    top: 100px;
    height: fit-content;
    min-height: 400px;
    padding: var(--space-6);
    background: rgba(0, 10, 20, 0.8);
    border: 1px solid rgba(6, 182, 212, 0.3);
  }

  .details-header {
    padding-bottom: var(--space-3);
    border-bottom: 1px solid var(--zyber-border-muted);
  }

  .decrypt-box {
    font-size: var(--text-sm);
    line-height: 1.8;
    white-space: pre-wrap;
    color: var(--zyber-cyber-cyan);
  }

  :global(.decrypt-line) {
    margin-bottom: var(--space-2);
  }

  :global(.decrypt-link) {
    display: inline-block;
    margin-top: var(--space-4);
    padding: var(--space-2) var(--space-3);
    background: rgba(6, 182, 212, 0.1);
    border: 1px solid var(--zyber-cyber-cyan);
    color: var(--zyber-cyber-cyan);
    text-decoration: none;
    font-size: var(--text-sm);
    transition: all 0.2s ease;
  }

  :global(.decrypt-link:hover) {
    background: rgba(6, 182, 212, 0.2);
    box-shadow: 0 0 15px rgba(6, 182, 212, 0.4);
    transform: translateX(4px);
  }

  .terminal-cursor {
    display: inline-block;
    width: 8px;
    height: 16px;
    background: var(--zyber-cyber-cyan);
    animation: blink 1s step-end infinite;
    margin-top: var(--space-4);
  }

  @keyframes blink {
    50% { opacity: 0; }
  }

  /* Colors */
  .text-cyan { color: var(--zyber-cyber-cyan); }
  .text-violet { color: var(--zyber-quantum-violet); }
  .text-success { color: var(--zyber-success); }
  .text-muted { color: var(--zyber-text-muted); }

  /* Responsive */
  @media (max-width: 900px) {
    .timeline-container {
      grid-template-columns: 1fr;
    }

    .details-column {
      position: relative;
      top: 0;
      min-height: 300px;
    }
  }
</style>
