<script>
  import { onMount } from 'svelte';
  import { currentRoute, navigateTo } from '../stores/router';

  // Secciones de la landing page + rutas de la app
  const sections = [
    { id: 'hero', label: 'INIT', icon: '>', type: 'scroll' },
    { id: 'stats', label: 'STATS', icon: '#', type: 'scroll' },
    { id: 'how-it-works', label: 'FLOW', icon: '~', type: 'scroll' },
    { id: 'use-cases', label: 'CASES', icon: '*', type: 'scroll' },
    { id: 'timeline', label: 'CHRONICLE', icon: '|', type: 'scroll' },
    { id: 'footer', label: 'INFO', icon: 'i', type: 'scroll' },
    { id: 'dashboard', label: 'APP', icon: '+', type: 'route' }
  ];

  let activeSection = 0;
  let visible = true;

  onMount(() => {
    // Intersection Observer para detectar sección activa
    const observer = new IntersectionObserver(
      (entries) => {
        entries.forEach((entry) => {
          if (entry.isIntersecting) {
            const sectionId = entry.target.id;
            const index = sections.findIndex(s => s.id === sectionId);
            if (index !== -1) {
              activeSection = index;
            }
          }
        });
      },
      {
        root: null,
        rootMargin: '-40% 0px -40% 0px', // Activar cuando esté en el centro del viewport
        threshold: 0.1
      }
    );

    // Observar todas las secciones
    sections.forEach(section => {
      const element = document.getElementById(section.id);
      if (element) {
        observer.observe(element);
      }
    });

    // Ocultar en scroll (opcional, para UX limpia)
    let scrollTimeout;
    let lastScrollY = window.scrollY;

    const handleScroll = () => {
      const currentScrollY = window.scrollY;

      // Ocultar si scroll rápido
      visible = false;

      // Mostrar después de parar de scrollear
      clearTimeout(scrollTimeout);
      scrollTimeout = setTimeout(() => {
        visible = true;
      }, 150);

      lastScrollY = currentScrollY;
    };

    window.addEventListener('scroll', handleScroll, { passive: true });

    return () => {
      observer.disconnect();
      window.removeEventListener('scroll', handleScroll);
    };
  });

  function handleNavigation(index) {
    const section = sections[index];

    if (section.type === 'route') {
      navigateTo(section.id);
    } else {
      const element = document.getElementById(section.id);
      if (element) {
        const offsetTop = element.offsetTop - 80;
        window.scrollTo({
          top: offsetTop,
          behavior: 'smooth'
        });
      }
    }
  }
</script>

<nav class="global-nav" class:visible>
  <!-- Connection Line -->
  <div class="nav-line"></div>

  <!-- Navigation Dots -->
  {#each sections as section, index}
    <button
      class="nav-dot"
      class:active={activeSection === index}
      on:click={() => handleNavigation(index)}
      aria-label="Navigate to {section.label}"
      title={section.label}
    >
      <!-- Dot Square -->
      <div class="dot-square">
        <div class="dot-pulse"></div>
        <div class="dot-icon text-mono">{section.icon}</div>
      </div>

      <!-- Label (aparece en hover/active) -->
      <div class="dot-label text-mono text-xs">
        <span class="label-bracket">[</span>
        {section.label}
        <span class="label-bracket">]</span>
      </div>
    </button>
  {/each}
</nav>

<style>
  .global-nav {
    position: fixed;
    right: var(--space-6);
    top: 50%;
    transform: translateY(-50%);
    z-index: 1000;
    display: flex;
    flex-direction: column;
    gap: var(--space-6);
    align-items: center;
    transition: opacity 0.3s ease;
  }

  .global-nav:not(.visible) { opacity: 0.3; }

  .nav-line {
    position: absolute;
    top: 10px;
    bottom: 10px;
    left: 50%;
    width: 1px;
    background: linear-gradient(
      180deg,
      transparent 0%,
      var(--zyber-border-muted) 15%,
      var(--zyber-cyber-cyan) 50%,
      var(--zyber-border-muted) 85%,
      transparent 100%
    );
    transform: translateX(-50%);
    z-index: -1;
    opacity: 0.4;
  }

  /* --- SQUARE ITEM --- */
  .nav-dot {
    position: relative;
    background: none;
    border: none;
    cursor: pointer;
    display: flex;
    flex-direction: row;
    align-items: center;
    gap: var(--space-3);
    padding: 0;
    transition: all var(--transition-base);
  }

  .dot-square {
    position: relative;
    width: 36px;
    height: 36px;
    background: rgba(10, 15, 25, 0.95);
    border: 1px solid var(--zyber-border-secondary);
    border-radius: 4px;
    backdrop-filter: blur(10px);
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all .25s cubic-bezier(0.4,0,0.2,1);
    box-shadow: 0 2px 8px rgba(0,0,0,0.4);
  }

  .dot-icon {
    font-size: var(--text-xs);
    color: var(--zyber-text-muted);
    opacity: 0.7;
    transition: all var(--transition-base);
    z-index: 1;
  }

  .dot-pulse {
    position: absolute;
    inset: -3px;
    border: 1px solid var(--zyber-cyber-cyan);
    border-radius: 6px;
    opacity: 0;
    transition: opacity var(--transition-base);
  }

  /* HOVER */
  .nav-dot:hover .dot-square {
    border-color: var(--zyber-cyber-cyan);
    background: rgba(6, 182, 212, 0.08);
    transform: scale(1.1);
    box-shadow:
      0 0 15px rgba(6, 182, 212, 0.3),
      0 2px 10px rgba(0,0,0,0.4);
  }
  .nav-dot:hover .dot-icon {
    opacity: 1;
    color: var(--zyber-cyber-cyan);
  }
  .nav-dot:hover .dot-pulse {
    opacity: 0.5;
    animation: pulse-square 1.2s ease-out infinite;
  }

  /* ACTIVE */
  .nav-dot.active .dot-square {
    background: rgba(6, 182, 212, 0.15);
    border-color: var(--zyber-cyber-cyan);
    transform: scale(1.15);
    box-shadow:
      0 0 20px rgba(6, 182, 212, 0.5),
      0 0 40px rgba(6, 182, 212, 0.2),
      inset 0 0 10px rgba(6, 182, 212, 0.1);
  }
  .nav-dot.active .dot-icon {
    opacity: 1;
    color: var(--zyber-cyber-cyan);
    text-shadow: 0 0 8px rgba(6, 182, 212, 0.8);
  }
  .nav-dot.active .dot-pulse {
    opacity: 0.8;
    animation: pulse-square-active 2s infinite;
  }

  /* LABEL */
  .dot-label {
    position: absolute;
    right: 48px;
    white-space: nowrap;
    background: rgba(10, 15, 25, 0.95);
    border: 1px solid var(--zyber-border-muted);
    border-radius: 3px;
    padding: var(--space-1) var(--space-2);
    opacity: 0;
    transform: translateX(8px);
    transition: all .2s ease;
    color: var(--zyber-text-muted);
    pointer-events: none;
  }
  .label-bracket {
    color: var(--zyber-cyber-cyan);
    opacity: 0.6;
  }
  .nav-dot:hover .dot-label {
    opacity: 1;
    transform: translateX(0);
    border-color: var(--zyber-border-secondary);
  }
  .nav-dot.active .dot-label {
    opacity: 1;
    transform: translateX(0);
    border-color: var(--zyber-cyber-cyan);
    color: var(--zyber-text-primary);
  }
  .nav-dot.active .dot-label .label-bracket {
    opacity: 1;
  }

  /* ANIMACIONES */
  @keyframes pulse-square {
    0%   { opacity: 0.4; transform: scale(1); }
    100% { opacity: 0;   transform: scale(1.45); }
  }
  @keyframes pulse-square-active {
    0%,100% { opacity: 0.6; transform: scale(1); }
    50%     { opacity: 1;   transform: scale(1.25); }
  }

  @media (max-width: 768px) {
    .global-nav { right: var(--space-2); }
    .dot-label { display: none; }
  }
  @media (max-width: 480px), (max-height: 500px) and (orientation: landscape) {
    .global-nav { display: none; }
  }
</style>
