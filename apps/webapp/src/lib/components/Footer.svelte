<script>
  import { navigateTo } from '../stores/router';

  // TODO: Update these URLs when ZyberLink resources are ready
  const ZYBERLINK_URLS = {
    // Documentation & Resources
    docs: 'https://www.gitbook.com',           // GitBook docs (placeholder)
    fheGuide: 'https://github.com/8ctag0n/13', // FHE-Helper repo
    apiReference: null,                         // → Add API docs URL
    github: 'https://github.com/8ctag0n/13',   // FHE-Helper GitHub repo

    // Community
    discord: null,           // → Add Discord invite link
    twitter: null,           // → Add Twitter/X profile URL
    blog: null,              // → Add Medium/Substack/Ghost blog URL
    forum: null,             // → Add community forum URL

    // Legal
    terms: null,             // → Add Terms of Service URL
    privacy: null,           // → Add Privacy Policy URL
    security: null,          // → Add Security page URL
    bugBounty: null          // → Add Bug Bounty program URL
  };

  const footerSections = {
    platform: {
      title: 'PLATFORM',
      links: [
        { label: 'Dashboard', action: () => navigateTo('dashboard') },
        { label: 'Create Job', action: () => navigateTo('create-job') },
        { label: 'Browse Jobs', action: () => navigateTo('dashboard') },
        { label: 'Become a Prover', action: () => navigateTo('dashboard'), disabled: true }
      ]
    },
    resources: {
      title: 'RESOURCES',
      links: [
        { label: 'Documentation', action: () => openUrl(ZYBERLINK_URLS.docs), disabled: !ZYBERLINK_URLS.docs },
        { label: 'FHE Guide', action: () => openUrl(ZYBERLINK_URLS.fheGuide), disabled: !ZYBERLINK_URLS.fheGuide },
        { label: 'API Reference', action: () => openUrl(ZYBERLINK_URLS.apiReference), disabled: !ZYBERLINK_URLS.apiReference },
        { label: 'GitHub', action: () => openUrl(ZYBERLINK_URLS.github), disabled: !ZYBERLINK_URLS.github }
      ]
    },
    community: {
      title: 'COMMUNITY',
      links: [
        { label: 'Discord', action: () => openUrl(ZYBERLINK_URLS.discord), disabled: !ZYBERLINK_URLS.discord },
        { label: 'Twitter', action: () => openUrl(ZYBERLINK_URLS.twitter), disabled: !ZYBERLINK_URLS.twitter },
        { label: 'Blog', action: () => openUrl(ZYBERLINK_URLS.blog), disabled: !ZYBERLINK_URLS.blog },
        { label: 'Forum', action: () => openUrl(ZYBERLINK_URLS.forum), disabled: !ZYBERLINK_URLS.forum }
      ]
    },
    legal: {
      title: 'LEGAL',
      links: [
        { label: 'Terms of Service', action: () => openUrl(ZYBERLINK_URLS.terms), disabled: !ZYBERLINK_URLS.terms },
        { label: 'Privacy Policy', action: () => openUrl(ZYBERLINK_URLS.privacy), disabled: !ZYBERLINK_URLS.privacy },
        { label: 'Security', action: () => openUrl(ZYBERLINK_URLS.security), disabled: !ZYBERLINK_URLS.security },
        { label: 'Bug Bounty', action: () => openUrl(ZYBERLINK_URLS.bugBounty), disabled: !ZYBERLINK_URLS.bugBounty }
      ]
    }
  };

  function openUrl(url) {
    if (url) {
      window.open(url, '_blank', 'noopener,noreferrer');
    }
  }

  let year = new Date().getFullYear();
</script>

<footer class="footer">
  <div class="container">
    <!-- ASCII Art Border Top -->
    <div class="footer-border-top text-mono text-muted">
      ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    </div>

    <!-- Footer Content -->
    <div class="footer-content">
      <!-- Brand Section -->
      <div class="footer-brand">
        <div class="brand-ascii text-cyan text-mono">
          <pre>╔══╗ ╦ ╦ ╔══╗ ╔═══╗ ╦═╗
╔══╝ ╚╦╝ ╠══╣ ║╔══╝ ╠╦╝
╚═══  ╩  ╚══╝ ╚═══╗ ╩╚═
               ╚═══╝
   LINK_TO_PRIVACY</pre>
        </div>
        <div class="brand-tagline text-mono text-muted mt-4">
          <span class="text-violet">&gt;</span> Privacy is not optional—
          <br>
          <span class="text-violet">&gt;</span> it's fundamental.
        </div>
      </div>

      <!-- Links Grid -->
      <div class="footer-links">
        {#each Object.entries(footerSections) as [key, section]}
          <div class="footer-column">
            <h4 class="column-title text-mono text-cyan">{section.title}</h4>
            <ul class="link-list">
              {#each section.links as link}
                <li>
                  <button
                    class="footer-link text-mono"
                    class:disabled={link.disabled}
                    on:click={link.action}
                    disabled={link.disabled}
                  >
                    <span class="link-arrow text-violet">&gt;</span>
                    <span class="link-text">{link.label}</span>
                    {#if link.disabled}
                      <span class="coming-soon text-xs">soon</span>
                    {/if}
                  </button>
                </li>
              {/each}
            </ul>
          </div>
        {/each}
      </div>
    </div>

    <!-- Footer Bottom -->
    <div class="footer-bottom">
      <div class="footer-divider text-mono text-muted">
        ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
      </div>

      <div class="footer-meta">
        <div class="meta-left text-mono text-muted text-xs">
          <span>&copy; {year} ZyberLink</span>
          <span class="separator">|</span>
          <span>Powered by Solana + FHE</span>
        </div>

        <div class="meta-right">
          <div class="terminal-prompt text-mono text-xs">
            <span class="text-muted">&gt; Connection terminated. Privacy preserved.</span>
            <span class="cursor-blink text-cyan">█</span>
          </div>
        </div>
      </div>
    </div>
  </div>
</footer>

<style>
  .footer {
    width: 100%;
    background: rgba(0, 0, 0, 0.3);
    border-top: 2px solid var(--zyber-border-primary);
    margin-top: var(--space-20);
    padding: var(--space-12) 0 var(--space-6) 0;
  }

  .footer-border-top {
    font-size: var(--text-xs);
    opacity: 0.3;
    text-align: center;
    margin-bottom: var(--space-8);
    overflow: hidden;
  }

  .footer-content {
    display: grid;
    grid-template-columns: 1fr 2fr;
    gap: var(--space-12);
    margin-bottom: var(--space-8);
  }

  .footer-brand {
    display: flex;
    flex-direction: column;
  }

  .brand-ascii {
    font-size: var(--text-xs);
    line-height: 1.2;
    text-shadow: var(--zyber-glow-cyan);
  }

  .brand-tagline {
    font-size: var(--text-sm);
    line-height: 1.6;
  }

  .footer-links {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: var(--space-8);
  }

  .footer-column {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }

  .column-title {
    font-size: var(--text-sm);
    font-weight: 600;
    margin-bottom: var(--space-2);
    text-shadow: var(--zyber-glow-cyan);
  }

  .link-list {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .footer-link {
    background: none;
    border: none;
    color: var(--zyber-text-muted);
    cursor: pointer;
    padding: var(--space-1) 0;
    display: flex;
    align-items: center;
    gap: var(--space-2);
    font-size: var(--text-xs);
    transition: all var(--transition-base);
    text-align: left;
  }

  .footer-link:hover {
    color: var(--zyber-text-primary);
    transform: translateX(4px);
  }

  .footer-link:hover .link-arrow {
    text-shadow: var(--zyber-glow-violet);
  }

  .footer-link.disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .footer-link.disabled:hover {
    color: var(--zyber-text-muted);
    transform: none;
  }

  .link-arrow {
    flex-shrink: 0;
    font-weight: 600;
  }

  .link-text {
    flex: 1;
  }

  .coming-soon {
    opacity: 0.6;
    font-style: italic;
  }

  .footer-bottom {
    margin-top: var(--space-8);
  }

  .footer-divider {
    font-size: var(--text-xs);
    opacity: 0.3;
    text-align: center;
    margin-bottom: var(--space-6);
    overflow: hidden;
  }

  .footer-meta {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding-top: var(--space-4);
  }

  .meta-left {
    display: flex;
    gap: var(--space-3);
    align-items: center;
  }

  .separator {
    opacity: 0.5;
  }

  .terminal-prompt {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  /* Responsive */
  @media (max-width: 1024px) {
    .footer-links {
      grid-template-columns: repeat(2, 1fr);
    }
  }

  @media (max-width: 768px) {
    .footer-content {
      grid-template-columns: 1fr;
    }

    .footer-links {
      grid-template-columns: 1fr;
    }

    .footer-meta {
      flex-direction: column;
      gap: var(--space-4);
      text-align: center;
    }

    .meta-left {
      flex-direction: column;
      gap: var(--space-2);
    }
  }
</style>
