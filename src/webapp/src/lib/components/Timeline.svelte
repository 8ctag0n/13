<script>
  import TimelineNode from './TimelineNode.svelte';
  import TimelineNavigation from './TimelineNavigation.svelte';
  import { onMount } from 'svelte';

  let activeNode = 0;
  let scrollContainer;

  const timelineData = [
    {
      year: '1977',
      title: 'The First Lock',
      subtitle: 'RSA Algorithm',
      description: 'Ron Rivest, Adi Shamir, and Leonard Adleman invented RSA, the first practical public-key cryptosystem. For the first time, two parties could communicate securely without sharing a secret key beforehand. This breakthrough made the internet as we know it possible.',
      icon: '#',
      color: 'cyan',
      link: 'https://people.csail.mit.edu/rivest/Rsapaper.pdf',
      linkLabel: 'Read Original Paper'
    },
    {
      year: '1993',
      title: 'Cypherpunks Write Code',
      subtitle: 'PGP & The Privacy Movement',
      description: 'Phil Zimmermann released PGP (Pretty Good Privacy), bringing military-grade encryption to the masses. Eric Hughes declared: "Privacy is necessary for an open society." The cypherpunk movement was born—privacy as a fundamental human right, enforced by mathematics, not laws.',
      icon: '<>',
      color: 'violet',
      link: 'https://www.activism.net/cypherpunk/manifesto.html',
      linkLabel: 'Read Manifesto'
    },
    {
      year: '2009',
      title: 'The Decentralization Spell',
      subtitle: 'Bitcoin Breaks the Monopoly',
      description: 'Satoshi Nakamoto published the Bitcoin whitepaper, introducing blockchain and decentralized consensus. For the first time, digital scarcity and trustless value transfer became reality. No banks, no intermediaries, no single point of failure—just pure cryptographic truth.',
      icon: '₿',
      color: 'success',
      link: 'https://bitcoin.org/bitcoin.pdf',
      linkLabel: 'Read Whitepaper'
    },
    {
      year: '2014',
      title: 'Prove Without Revealing',
      subtitle: 'ZK-SNARKs Revolution',
      description: 'Zero-Knowledge Succinct Non-Interactive Arguments of Knowledge (ZK-SNARKs) went from theory to practice in Zcash. You could now prove the truth of a statement without revealing anything beyond its validity. Privacy and verification, together at last.',
      icon: 'zk',
      color: 'violet',
      link: 'https://zerocash-project.org/media/pdf/zerocash-extended-20140518.pdf',
      linkLabel: 'Read Zerocash Paper'
    },
    {
      year: '2024',
      title: 'Compute on Encrypted Data',
      subtitle: 'ZyberLink: FHE Meets DeFi',
      description: 'Fully Homomorphic Encryption (FHE) meets decentralized networks. ZyberLink enables computation on encrypted data without ever decrypting it. Your data remains private end-to-end, even from those processing it. The ultimate realization of the cypherpunk vision: computation without trust.',
      icon: '>>',
      color: 'cyan',
      link: null,
      linkLabel: 'Coming Soon'
    }
  ];

  onMount(() => {
    // Auto-scroll to show all nodes on mount
    if (scrollContainer) {
      const observer = new IntersectionObserver(
        (entries) => {
          entries.forEach((entry) => {
            if (entry.isIntersecting) {
              const index = parseInt(entry.target.dataset.index);
              activeNode = index;
            }
          });
        },
        { threshold: 0.6 }
      );

      const nodes = scrollContainer.querySelectorAll('.timeline-node');
      nodes.forEach((node, index) => {
        node.dataset.index = index;
        observer.observe(node);
      });

      // Keyboard navigation
      const handleKeyDown = (e) => {
        if (e.key === 'ArrowLeft') {
          e.preventDefault();
          scrollToNode(Math.max(0, activeNode - 1));
        } else if (e.key === 'ArrowRight') {
          e.preventDefault();
          scrollToNode(Math.min(timelineData.length - 1, activeNode + 1));
        }
      };

      window.addEventListener('keydown', handleKeyDown);

      return () => {
        observer.disconnect();
        window.removeEventListener('keydown', handleKeyDown);
      };
    }
  });

  function scrollToNode(index) {
    if (!scrollContainer) return;

    const track = scrollContainer.querySelector('.timeline-track');
    const nodes = track?.children;
    const targetNode = nodes?.[index];

    if (targetNode && scrollContainer) {
      const containerRect = scrollContainer.getBoundingClientRect();
      const nodeRect = targetNode.getBoundingClientRect();
      const scrollLeft = scrollContainer.scrollLeft;

      // Calculate position to center the node
      const targetScrollLeft = scrollLeft + (nodeRect.left - containerRect.left) - (containerRect.width / 2) + (nodeRect.width / 2);

      scrollContainer.scrollTo({
        left: targetScrollLeft,
        behavior: 'smooth'
      });
    }
  }

  function handleNavigate(event) {
    scrollToNode(event.detail.index);
  }
</script>

<section class="timeline-section">
  <div class="timeline-header">
    <h2 class="text-mono text-uppercase text-center mb-2">
      <span class="text-violet">&gt;</span> The Cipher Chronicles
    </h2>
    <p class="text-sm text-muted text-center text-mono mb-8">
      FROM_THE_FIRST_ENCRYPTION_TO_FULLY_HOMOMORPHIC_FUTURE
    </p>
  </div>

  <!-- Enhanced Timeline Navigation -->
  <TimelineNavigation
    {timelineData}
    {activeNode}
    on:navigate={handleNavigate}
  />

  <!-- Timeline Scroll Container -->
  <div class="timeline-wrapper">
    <div class="timeline-container" bind:this={scrollContainer}>
      <div class="timeline-track">
        {#each timelineData as node, index}
          <TimelineNode
            year={node.year}
            title={node.title}
            subtitle={node.subtitle}
            description={node.description}
            icon={node.icon}
            color={node.color}
            active={activeNode === index}
            link={node.link}
            linkLabel={node.linkLabel}
          />
        {/each}
      </div>
    </div>
  </div>
</section>

<style>
  .timeline-section {
    width: 100%;
    max-width: 100%;
    margin: var(--space-16) 0;
    padding: var(--space-12) 0;
    background: linear-gradient(180deg, transparent 0%, rgba(6, 182, 212, 0.03) 50%, transparent 100%);
    border-top: 1px solid var(--zyber-border-secondary);
    border-bottom: 1px solid var(--zyber-border-secondary);
  }

  .timeline-header {
    margin-bottom: var(--space-8);
  }

  .timeline-header h2 {
    font-size: var(--text-2xl);
    font-weight: 600;
  }

  /* Timeline Wrapper */
  .timeline-wrapper {
    position: relative;
    width: 100%;
  }

  .timeline-container {
    width: 100%;
    overflow-x: auto;
    overflow-y: visible;
    scroll-snap-type: x mandatory;
    scroll-behavior: smooth;
    padding: var(--space-8) var(--space-4);
    -webkit-overflow-scrolling: touch;
  }

  /* Custom Scrollbar */
  .timeline-container::-webkit-scrollbar {
    height: 8px;
  }

  .timeline-container::-webkit-scrollbar-track {
    background: rgba(6, 182, 212, 0.05);
    border-radius: var(--radius-full);
  }

  .timeline-container::-webkit-scrollbar-thumb {
    background: var(--zyber-cyan);
    border-radius: var(--radius-full);
    box-shadow: var(--zyber-glow-cyan);
  }

  .timeline-track {
    display: flex;
    gap: var(--space-8);
    padding: 0 calc(50vw - 160px);
    position: relative;
  }

  /* Responsive */
  @media (max-width: 768px) {
    .timeline-section {
      padding: var(--space-8) 0;
    }

    .timeline-track {
      padding: 0 var(--space-4);
    }

    .timeline-header h2 {
      font-size: var(--text-xl);
    }
  }
</style>
