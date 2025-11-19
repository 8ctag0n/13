<script>
  import { onMount } from 'svelte';

  // Define las secciones de la landing page
  const sections = [
    { id: 'hero', label: 'INIT', icon: '⚡' },
    { id: 'stats', label: 'STATS', icon: '📊' },
    { id: 'how-it-works', label: 'FLOW', icon: '🔄' },
    { id: 'timeline', label: 'CHRONICLE', icon: '📜' },
    { id: 'footer', label: 'INFO', icon: '📡' }
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

  function scrollToSection(index) {
    const section = sections[index];
    const element = document.getElementById(section.id);

    if (element) {
      const offsetTop = element.offsetTop - 80; // Offset para header si existe

      window.scrollTo({
        top: offsetTop,
        behavior: 'smooth'
      });
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
      on:click={() => scrollToSection(index)}
      aria-label="Navigate to {section.label}"
      title={section.label}
    >
      <!-- Dot Circle -->
      <div class="dot-circle">
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
    opacity: 1;
    transition: opacity 0.3s ease;
  }

  .global-nav:not(.visible) {
    opacity: 0.3;
  }

  /* Connection Line */
  .nav-line {
    position: absolute;
    top: 0;
    bottom: 0;
    left: 50%;
    width: 2px;
    background: linear-gradient(
      180deg,
      transparent 0%,
      var(--zyber-border-secondary) 10%,
      var(--zyber-cyan) 50%,
      var(--zyber-border-secondary) 90%,
      transparent 100%
    );
    transform: translateX(-50%);
    z-index: -1;
    opacity: 0.3;
  }

  /* Navigation Dot */
  .nav-dot {
    position: relative;
    background: none;
    border: none;
    padding: 0;
    cursor: pointer;
    display: flex;
    flex-direction: row;
    align-items: center;
    gap: var(--space-3);
    transition: all var(--transition-base);
  }

  .dot-circle {
    position: relative;
    width: 40px;
    height: 40px;
    border-radius: 50%;
    background: rgba(0, 0, 0, 0.8);
    backdrop-filter: blur(10px);
    border: 2px solid var(--zyber-border-secondary);
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
  }

  .dot-icon {
    font-size: var(--text-sm);
    opacity: 0.6;
    transition: all var(--transition-base);
    filter: grayscale(0.8);
  }

  /* Pulse Effect */
  .dot-pulse {
    position: absolute;
    top: -2px;
    left: -2px;
    right: -2px;
    bottom: -2px;
    border-radius: 50%;
    border: 2px solid var(--zyber-cyan);
    opacity: 0;
    transition: opacity var(--transition-base);
  }

  /* Hover State */
  .nav-dot:hover .dot-circle {
    border-color: var(--zyber-cyan);
    background: rgba(6, 182, 212, 0.1);
    transform: scale(1.15);
    box-shadow: 0 0 20px rgba(6, 182, 212, 0.4),
                0 4px 16px rgba(0, 0, 0, 0.4);
  }

  .nav-dot:hover .dot-icon {
    opacity: 1;
    filter: grayscale(0);
    transform: scale(1.1);
  }

  .nav-dot:hover .dot-pulse {
    opacity: 0.3;
    animation: pulse-expand 1s ease-out infinite;
  }

  /* Active State */
  .nav-dot.active .dot-circle {
    border-color: var(--zyber-cyan);
    background: var(--zyber-cyan);
    box-shadow: 0 0 30px rgba(6, 182, 212, 0.8),
                0 0 60px rgba(6, 182, 212, 0.4),
                0 4px 20px rgba(0, 0, 0, 0.5);
    transform: scale(1.2);
  }

  .nav-dot.active .dot-icon {
    opacity: 1;
    filter: grayscale(0) drop-shadow(0 0 8px rgba(0, 0, 0, 0.8));
    transform: scale(1.2);
  }

  .nav-dot.active .dot-pulse {
    opacity: 1;
    animation: pulse-active 2s ease-in-out infinite;
  }

  /* Label */
  .dot-label {
    position: absolute;
    right: 56px;
    white-space: nowrap;
    background: rgba(0, 0, 0, 0.95);
    backdrop-filter: blur(10px);
    border: 1px solid var(--zyber-border-secondary);
    border-radius: var(--radius-sm);
    padding: var(--space-1) var(--space-3);
    color: var(--zyber-text-muted);
    opacity: 0;
    pointer-events: none;
    transform: translateX(8px);
    transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
  }

  .label-bracket {
    color: var(--zyber-cyan);
    opacity: 0.5;
  }

  .nav-dot:hover .dot-label,
  .nav-dot.active .dot-label {
    opacity: 1;
    transform: translateX(0);
  }

  .nav-dot.active .dot-label {
    border-color: var(--zyber-cyan);
    color: var(--zyber-cyan);
    box-shadow: 0 0 20px rgba(6, 182, 212, 0.3),
                0 4px 12px rgba(0, 0, 0, 0.4);
  }

  .nav-dot.active .dot-label .label-bracket {
    opacity: 1;
  }

  /* Animations */
  @keyframes pulse-expand {
    0% {
      opacity: 0.3;
      transform: scale(1);
    }
    100% {
      opacity: 0;
      transform: scale(1.5);
    }
  }

  @keyframes pulse-active {
    0%, 100% {
      opacity: 0.6;
      transform: scale(1);
    }
    50% {
      opacity: 1;
      transform: scale(1.3);
    }
  }

  /* Responsive - Mobile */
  @media (max-width: 768px) {
    .global-nav {
      right: var(--space-2);
      gap: var(--space-4);
    }

    .dot-circle {
      width: 32px;
      height: 32px;
    }

    .dot-icon {
      font-size: var(--text-xs);
    }

    /* Ocultar labels en mobile */
    .dot-label {
      display: none;
    }
  }

  /* Ocultar en pantallas muy pequeñas o landscape mobile */
  @media (max-width: 480px), (max-height: 500px) and (orientation: landscape) {
    .global-nav {
      display: none;
    }
  }
</style>
