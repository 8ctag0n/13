<script>
  import { onMount } from 'svelte';
  import { currentRoute, navigateTo } from '../stores/router';

  // Secciones de la landing page + rutas de la app
  // APP está en el medio con forma de octágono para destacar
  const sections = [
    { id: 'hero', label: 'INIT', icon: '>', type: 'scroll' },
    { id: 'stats', label: 'STATS', icon: '#', type: 'scroll' },
    { id: 'how-it-works', label: 'FLOW', icon: '~', type: 'scroll' },
    { id: 'dashboard', label: 'APP', icon: '◈', type: 'route', isApp: true },
    { id: 'use-cases', label: 'CASES', icon: '*', type: 'scroll' },
    { id: 'timeline', label: 'CHRONICLE', icon: '|', type: 'scroll' },
    { id: 'footer', label: 'INFO', icon: 'i', type: 'scroll' }
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
      <!-- Dot Shape (Square or Octagon for APP) -->
      <div class="dot-shape" class:dot-octagon={section.isApp}>
        <div class="dot-pulse" class:octagon-pulse={section.isApp}></div>
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

  /* --- NAV ITEM --- */
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

  .dot-shape {
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

  /* OCTAGON SHAPE for APP button */
  .dot-shape.dot-octagon {
    width: 44px;
    height: 44px;
    border-radius: 0;
    border: 2px solid var(--zyber-cyber-cyan);
    background: rgba(6, 182, 212, 0.1);
    clip-path: polygon(
      30% 0%, 70% 0%,
      100% 30%, 100% 70%,
      70% 100%, 30% 100%,
      0% 70%, 0% 30%
    );
    box-shadow:
      0 0 15px rgba(6, 182, 212, 0.4),
      0 0 30px rgba(6, 182, 212, 0.2),
      inset 0 0 15px rgba(6, 182, 212, 0.1);
    animation: octagon-glow 3s ease-in-out infinite;
  }
  .dot-shape.dot-octagon::before {
    content: '';
    position: absolute;
    inset: 0;
    background: linear-gradient(135deg, rgba(6, 182, 212, 0.2) 0%, transparent 50%, rgba(139, 92, 246, 0.2) 100%);
    clip-path: polygon(
      30% 0%, 70% 0%,
      100% 30%, 100% 70%,
      70% 100%, 30% 100%,
      0% 70%, 0% 30%
    );
  }
  .dot-shape.dot-octagon::after {
    content: '';
    position: absolute;
    inset: -4px;
    border: 1px solid var(--zyber-cyber-cyan);
    opacity: 0.3;
    clip-path: polygon(
      30% 0%, 70% 0%,
      100% 30%, 100% 70%,
      70% 100%, 30% 100%,
      0% 70%, 0% 30%
    );
    animation: octagon-ring 2s ease-out infinite;
  }

  /* Octagon icon always cyan */
  .dot-shape.dot-octagon .dot-icon {
    color: var(--zyber-cyber-cyan);
    opacity: 1;
    text-shadow: 0 0 8px rgba(6, 182, 212, 0.6);
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
  .nav-dot:hover .dot-shape {
    border-color: var(--zyber-cyber-cyan);
    background: rgba(6, 182, 212, 0.08);
    transform: scale(1.1);
    box-shadow:
      0 0 15px rgba(6, 182, 212, 0.3),
      0 2px 10px rgba(0,0,0,0.4);
  }
  .nav-dot:hover .dot-shape.dot-octagon {
    border: 2px solid var(--zyber-cyber-cyan);
    background: rgba(6, 182, 212, 0.2);
    transform: scale(1.15);
    box-shadow:
      0 0 25px rgba(6, 182, 212, 0.6),
      0 0 50px rgba(6, 182, 212, 0.3),
      inset 0 0 20px rgba(6, 182, 212, 0.2);
  }
  .nav-dot:hover .dot-icon {
    opacity: 1;
    color: var(--zyber-cyber-cyan);
  }
  .nav-dot:hover .dot-pulse {
    opacity: 0.5;
    animation: pulse-square 1.2s ease-out infinite;
  }
  .nav-dot:hover .dot-pulse.octagon-pulse {
    animation: pulse-octagon 1.2s ease-out infinite;
  }

  /* ACTIVE */
  .nav-dot.active .dot-shape {
    background: rgba(6, 182, 212, 0.15);
    border-color: var(--zyber-cyber-cyan);
    transform: scale(1.15);
    box-shadow:
      0 0 20px rgba(6, 182, 212, 0.5),
      0 0 40px rgba(6, 182, 212, 0.2),
      inset 0 0 10px rgba(6, 182, 212, 0.1);
  }
  .nav-dot.active .dot-shape.dot-octagon {
    border: 2px solid var(--zyber-cyber-cyan);
    background: rgba(6, 182, 212, 0.25);
    transform: scale(1.2);
    box-shadow:
      0 0 30px rgba(6, 182, 212, 0.7),
      0 0 60px rgba(6, 182, 212, 0.4),
      0 0 100px rgba(6, 182, 212, 0.2),
      inset 0 0 25px rgba(6, 182, 212, 0.3);
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
  .nav-dot.active .dot-pulse.octagon-pulse {
    animation: pulse-octagon-active 2s infinite;
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
  @keyframes pulse-octagon {
    0%   { opacity: 0.5; transform: scale(1); }
    100% { opacity: 0;   transform: scale(1.5); }
  }
  @keyframes pulse-octagon-active {
    0%,100% { opacity: 0.7; transform: scale(1); }
    50%     { opacity: 1;   transform: scale(1.3); }
  }

  @keyframes octagon-glow {
    0%, 100% {
      box-shadow:
        0 0 15px rgba(6, 182, 212, 0.4),
        0 0 30px rgba(6, 182, 212, 0.2),
        inset 0 0 15px rgba(6, 182, 212, 0.1);
    }
    50% {
      box-shadow:
        0 0 20px rgba(6, 182, 212, 0.6),
        0 0 40px rgba(6, 182, 212, 0.3),
        inset 0 0 20px rgba(6, 182, 212, 0.15);
    }
  }

  @keyframes octagon-ring {
    0% {
      opacity: 0.5;
      transform: scale(1);
    }
    100% {
      opacity: 0;
      transform: scale(1.5);
    }
  }

  @media (max-width: 768px) {
    .global-nav { right: var(--space-2); }
    .dot-label { display: none; }
  }
  @media (max-width: 480px), (max-height: 500px) and (orientation: landscape) {
    .global-nav { display: none; }
  }
</style>
