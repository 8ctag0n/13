<script>
  let activeCase = 0;

  const useCases = [
    {
      id: 'healthcare',
      icon: '+',
      title: 'Healthcare Analytics',
      subtitle: 'MEDICAL_DATA_PRIVACY',
      description: 'Analyze patient records across hospitals without exposing sensitive health information. Run statistical queries on encrypted medical datasets for research while maintaining HIPAA compliance.',
      examples: [
        'Cross-hospital disease pattern analysis',
        'Drug interaction studies on encrypted records',
        'Population health metrics without data exposure'
      ],
      color: 'cyan'
    },
    {
      id: 'finance',
      icon: '$',
      title: 'Financial Services',
      subtitle: 'SECURE_TRANSACTIONS',
      description: 'Perform credit scoring, fraud detection, and risk analysis on encrypted financial data. Banks can collaborate on anti-money laundering without sharing customer details.',
      examples: [
        'Private credit scoring computations',
        'Encrypted portfolio risk analysis',
        'Cross-bank fraud detection networks'
      ],
      color: 'success'
    },
    {
      id: 'ml',
      icon: '*',
      title: 'Private ML/AI',
      subtitle: 'ENCRYPTED_INFERENCE',
      description: 'Run machine learning models on encrypted data. Users get predictions without revealing their inputs, and model owners protect their proprietary algorithms.',
      examples: [
        'Private medical diagnosis predictions',
        'Encrypted recommendation systems',
        'Secure facial recognition without image exposure'
      ],
      color: 'violet'
    },
    {
      id: 'voting',
      icon: '#',
      title: 'Secure Voting',
      subtitle: 'VERIFIABLE_ELECTIONS',
      description: 'Implement end-to-end verifiable electronic voting where votes remain encrypted throughout the counting process. Voters can verify their vote was counted without revealing their choice.',
      examples: [
        'Corporate shareholder voting',
        'DAO governance with private ballots',
        'National election pilot programs'
      ],
      color: 'cyan'
    },
    {
      id: 'supply',
      icon: '~',
      title: 'Supply Chain',
      subtitle: 'CONFIDENTIAL_LOGISTICS',
      description: 'Coordinate supply chains across competing companies without revealing proprietary pricing, inventory levels, or supplier relationships.',
      examples: [
        'Multi-party inventory optimization',
        'Encrypted demand forecasting',
        'Private supplier performance metrics'
      ],
      color: 'success'
    },
    {
      id: 'gaming',
      icon: '>',
      title: 'Gaming & NFTs',
      subtitle: 'HIDDEN_GAME_STATE',
      description: 'Create truly fair on-chain games where game state remains hidden until reveal. Enable encrypted NFT attributes that unlock based on conditions.',
      examples: [
        'Fog-of-war strategy games on-chain',
        'Sealed-bid NFT auctions',
        'Private in-game economies'
      ],
      color: 'violet'
    }
  ];

  function selectCase(index) {
    activeCase = index;
  }
</script>

<section class="use-cases-section">
  <div class="section-header">
    <h2 class="text-mono text-uppercase text-center mb-2">
      <span class="text-cyan">&gt;</span> Use Cases
    </h2>
    <p class="text-sm text-muted text-center text-mono mb-8">
      REAL_WORLD_APPLICATIONS_OF_FHE_COMPUTING
    </p>
  </div>

  <div class="use-cases-grid">
    <!-- Case Selector (Left) -->
    <div class="case-selector">
      {#each useCases as useCase, index}
        <button
          class="case-tab"
          class:active={activeCase === index}
          on:click={() => selectCase(index)}
        >
          <div class="tab-icon text-mono text-{useCase.color}">{useCase.icon}</div>
          <div class="tab-content">
            <div class="tab-title text-mono">{useCase.title}</div>
            <div class="tab-subtitle text-xs text-muted">{useCase.subtitle}</div>
          </div>
          <div class="tab-arrow text-{useCase.color}">&gt;</div>
        </button>
      {/each}
    </div>

    <!-- Case Detail (Right) -->
    <div class="case-detail tui-box">
      <!-- ASCII Corners -->
      <div class="corner-tl text-mono text-{useCases[activeCase].color}">+--</div>
      <div class="corner-tr text-mono text-{useCases[activeCase].color}">--+</div>
      <div class="corner-bl text-mono text-{useCases[activeCase].color}">+--</div>
      <div class="corner-br text-mono text-{useCases[activeCase].color}">--+</div>

      <div class="detail-header">
        <div class="detail-icon text-{useCases[activeCase].color} text-mono">
          [{useCases[activeCase].icon}]
        </div>
        <div class="detail-titles">
          <h3 class="detail-title text-mono">{useCases[activeCase].title}</h3>
          <div class="detail-subtitle text-mono text-muted text-xs">
            {useCases[activeCase].subtitle}
          </div>
        </div>
      </div>

      <div class="detail-description text-muted">
        {useCases[activeCase].description}
      </div>

      <div class="detail-examples">
        <div class="examples-header text-mono text-xs text-{useCases[activeCase].color} mb-3">
          &gt; EXAMPLE_APPLICATIONS:
        </div>
        <ul class="examples-list">
          {#each useCases[activeCase].examples as example}
            <li class="example-item text-mono text-sm">
              <span class="bullet text-{useCases[activeCase].color}">[*]</span>
              {example}
            </li>
          {/each}
        </ul>
      </div>

      <div class="detail-cta">
        <div class="cta-prompt text-mono text-xs text-muted">
          &gt; Ready to build with FHE?
        </div>
        <a href="https://github.com/8ctag0n/13" target="_blank" rel="noopener noreferrer" class="cta-link text-mono text-sm">
          <span class="text-{useCases[activeCase].color}">[</span>
          EXPLORE_SDK
          <span class="text-{useCases[activeCase].color}">]</span>
          <span class="arrow">-&gt;</span>
        </a>
      </div>
    </div>
  </div>
</section>

<style>
  .use-cases-section {
    width: 100%;
    max-width: 1100px;
    margin: var(--space-16) auto;
    padding: var(--space-8) var(--space-4);
  }

  .section-header h2 {
    font-size: var(--text-2xl);
    font-weight: 600;
  }

  .use-cases-grid {
    display: grid;
    grid-template-columns: 320px 1fr;
    gap: var(--space-6);
    align-items: start;
  }

  /* Case Selector */
  .case-selector {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .case-tab {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-3) var(--space-4);
    background: rgba(0, 0, 0, 0.3);
    border: 1px solid var(--zyber-border-secondary);
    border-radius: var(--radius-md);
    cursor: pointer;
    transition: all 0.2s ease;
    text-align: left;
  }

  .case-tab:hover {
    background: rgba(6, 182, 212, 0.05);
    border-color: var(--zyber-border-primary);
    transform: translateX(4px);
  }

  .case-tab.active {
    background: rgba(6, 182, 212, 0.1);
    border-color: var(--zyber-cyan);
    box-shadow: 0 0 20px rgba(6, 182, 212, 0.2);
  }

  .tab-icon {
    font-size: var(--text-xl);
    font-weight: 600;
    width: 32px;
    text-align: center;
    flex-shrink: 0;
  }

  .tab-content {
    flex: 1;
    min-width: 0;
  }

  .tab-title {
    font-size: var(--text-sm);
    font-weight: 500;
    color: var(--zyber-text-primary);
  }

  .tab-subtitle {
    margin-top: 2px;
  }

  .tab-arrow {
    font-weight: 600;
    opacity: 0;
    transition: opacity 0.2s ease;
  }

  .case-tab:hover .tab-arrow,
  .case-tab.active .tab-arrow {
    opacity: 1;
  }

  /* Case Detail */
  .case-detail {
    position: relative;
    padding: var(--space-8);
    min-height: 400px;
    background: linear-gradient(
      135deg,
      rgba(0, 0, 0, 0.5) 0%,
      rgba(6, 182, 212, 0.03) 50%,
      rgba(0, 0, 0, 0.5) 100%
    );
    backdrop-filter: blur(10px);
  }

  .corner-tl, .corner-tr, .corner-bl, .corner-br {
    position: absolute;
    font-size: var(--text-sm);
    opacity: 0.5;
  }

  .corner-tl { top: var(--space-2); left: var(--space-2); }
  .corner-tr { top: var(--space-2); right: var(--space-2); }
  .corner-bl { bottom: var(--space-2); left: var(--space-2); }
  .corner-br { bottom: var(--space-2); right: var(--space-2); }

  .detail-header {
    display: flex;
    align-items: center;
    gap: var(--space-4);
    margin-bottom: var(--space-6);
  }

  .detail-icon {
    font-size: var(--text-3xl);
    font-weight: 600;
    text-shadow: 0 0 20px currentColor;
  }

  .detail-title {
    font-size: var(--text-xl);
    font-weight: 600;
    color: var(--zyber-text-primary);
  }

  .detail-description {
    font-size: var(--text-base);
    line-height: 1.7;
    margin-bottom: var(--space-6);
    padding-bottom: var(--space-6);
    border-bottom: 1px solid var(--zyber-border-secondary);
  }

  .detail-examples {
    margin-bottom: var(--space-6);
  }

  .examples-list {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .example-item {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    color: var(--zyber-text-secondary);
  }

  .bullet {
    flex-shrink: 0;
  }

  .detail-cta {
    margin-top: auto;
    padding-top: var(--space-6);
    border-top: 1px solid var(--zyber-border-secondary);
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .cta-link {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    text-decoration: none;
    color: var(--zyber-text-primary);
    padding: var(--space-2) var(--space-4);
    border: 1px solid var(--zyber-border-secondary);
    border-radius: var(--radius-md);
    transition: all 0.2s ease;
  }

  .cta-link:hover {
    border-color: var(--zyber-cyan);
    background: rgba(6, 182, 212, 0.1);
  }

  .cta-link .arrow {
    transition: transform 0.2s ease;
  }

  .cta-link:hover .arrow {
    transform: translateX(4px);
  }

  /* Color variants */
  .text-cyan { color: var(--zyber-cyan); }
  .text-violet { color: var(--zyber-violet); }
  .text-success { color: var(--zyber-success); }

  /* Responsive */
  @media (max-width: 900px) {
    .use-cases-grid {
      grid-template-columns: 1fr;
    }

    .case-selector {
      flex-direction: row;
      flex-wrap: wrap;
      gap: var(--space-2);
    }

    .case-tab {
      flex: 1;
      min-width: 140px;
      flex-direction: column;
      text-align: center;
      padding: var(--space-3);
    }

    .tab-content {
      text-align: center;
    }

    .tab-arrow {
      display: none;
    }

    .case-detail {
      min-height: auto;
    }
  }

  @media (max-width: 480px) {
    .case-tab {
      min-width: 100%;
    }

    .detail-cta {
      flex-direction: column;
      gap: var(--space-3);
      text-align: center;
    }
  }
</style>
