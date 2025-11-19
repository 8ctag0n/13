# ZyberLink Design System

**"Encrypted Light" - Glassmorphic Cyberpunk**

Una identidad visual completa y distintiva para ZyberLink, el marketplace de computación privada FHE en Solana.

---

## Inicio Rápido

### Ver el Demo
```bash
cd design-system
open demo.html  # Abre en tu navegador
```

### Usar en tu Proyecto
```html
<!-- Importa estos archivos -->
<link rel="stylesheet" href="design-system/tokens.css">
<link rel="stylesheet" href="design-system/components.css">
```

---

## Documentación

### Para Empezar
- **[QUICK_START.md](QUICK_START.md)** - Guía de 5 minutos con código copy-paste
- **[demo.html](demo.html)** - Demo interactivo de todos los componentes

### Guías Completas
- **[BRAND_GUIDE.md](BRAND_GUIDE.md)** - Guía completa de marca (colores, tipografía, logo, voz)
- **[VISUAL_IDENTITY.md](VISUAL_IDENTITY.md)** - Identidad visual con ejemplos visuales
- **[README.md](README.md)** - Documentación técnica del sistema

### Código
- **[tokens.css](tokens.css)** - Design tokens (colores, espaciado, tipografía)
- **[components.css](components.css)** - Librería de componentes completa

### Assets
- **[assets/logo/](assets/logo/)** - Logos en SVG (full, icon, wordmark)

---

## Concepto de Diseño

### Tema: "Encrypted Light"

Computación invisible iluminada por glows criptográficos.

### Características Distintivas

1. **Glassmorphism** - Cards translúcidos con backdrop blur
2. **Glow Effects** - Profundidad creada con luminiscencia, no sombras
3. **Dark Mode First** - Diseñado para fondos oscuros
4. **Quantum Violet** - Color de marca único (#8B5CF6)
5. **Cyber Cyan** - Acento complementario (#06B6D4)
6. **Animaciones con propósito** - Pulse glow, scan lines, computing indicators

---

## Paleta de Colores

```css
/* Primarios */
--zyber-primary-600: #8B5CF6;  /* Quantum Violet */
--zyber-accent-500: #06B6D4;   /* Cyber Cyan */

/* Semánticos */
--zyber-success-500: #10B981;  /* Verde tech */
--zyber-warning-500: #F59E0B;  /* Amber */
--zyber-error-500: #EF4444;    /* Rojo */

/* Backgrounds */
--zyber-bg-base: #0A0A0F;      /* Negro violeta */
--zyber-bg-tertiary: #1C1C26;  /* Cards */

/* Texto */
--zyber-text-primary: #F9FAFB; /* Blanco */
--zyber-text-secondary: #D1D5DB; /* Gris claro */
```

---

## Tipografía

```css
/* Headings */
font-family: 'Space Grotesk', sans-serif;
font-weight: 700;

/* Body */
font-family: 'Inter', sans-serif;
font-weight: 400;

/* Code */
font-family: 'JetBrains Mono', monospace;
```

---

## Componentes Principales

### Buttons
```html
<button class="btn btn-primary">Primary</button>
<button class="btn btn-secondary">Secondary</button>
<button class="btn btn-ghost">Ghost</button>
```

### Cards
```html
<div class="card">
  <div class="card-header">
    <h3 class="card-title">Title</h3>
  </div>
  <div class="card-body">Content</div>
  <div class="card-footer">Actions</div>
</div>
```

### Badges
```html
<span class="badge badge-primary">Computing</span>
<span class="badge badge-success">Verified</span>
```

### Inputs
```html
<div class="input-group">
  <label class="input-label">Label</label>
  <input type="text" class="input">
</div>
```

### Computing Indicator
```html
<div class="computing-indicator"></div>
```

---

## Efectos Especiales

### Gradient Text
```html
<h1 class="text-gradient">ZyberLink</h1>
```

### Cyber Grid Background
```html
<section class="cyber-grid">
  <!-- Content -->
</section>
```

### Pulse Glow Animation
```html
<div class="animate-pulse-glow">
  <!-- Elemento activo -->
</div>
```

---

## Estructura de Archivos

```
design-system/
├── INDEX.md                # Este archivo (punto de entrada)
├── QUICK_START.md         # Guía rápida de 5 min
├── BRAND_GUIDE.md         # Guía completa de marca
├── VISUAL_IDENTITY.md     # Identidad visual detallada
├── README.md              # Documentación técnica
│
├── demo.html              # Demo interactivo
├── tokens.css             # Design tokens
├── components.css         # Componentes
│
└── assets/
    └── logo/
        ├── zyberlink-full.svg      # Logo completo
        ├── zyberlink-icon.svg      # Solo icono
        └── zyberlink-wordmark.svg  # Solo texto
```

---

## Casos de Uso

### Landing Page
Ver: [QUICK_START.md#hero-section](QUICK_START.md#hero-section)

### Dashboard
Ver: [QUICK_START.md#card-de-dashboard](QUICK_START.md#card-de-dashboard)

### Formularios
Ver: [QUICK_START.md#formulario](QUICK_START.md#formulario)

### Navigation
Ver: [QUICK_START.md#navigation-bar](QUICK_START.md#navigation-bar)

---

## Navegación Rápida

### Necesito...

**Ver ejemplos visuales**
→ Abre [demo.html](demo.html)

**Código copy-paste**
→ Lee [QUICK_START.md](QUICK_START.md)

**Guía de colores y tipografía**
→ Lee [BRAND_GUIDE.md](BRAND_GUIDE.md)

**Conceptos de diseño**
→ Lee [VISUAL_IDENTITY.md](VISUAL_IDENTITY.md)

**Documentación técnica**
→ Lee [README.md](README.md)

**Logos en SVG**
→ Ve a [assets/logo/](assets/logo/)

**Variables CSS**
→ Revisa [tokens.css](tokens.css)

**Componentes CSS**
→ Revisa [components.css](components.css)

---

## Principios de Diseño

1. **Dark Mode First** - Todo diseñado para fondos oscuros
2. **Glow over Shadow** - Profundidad con luminiscencia
3. **Progressive Disclosure** - Complejidad revelada gradualmente
4. **Motion with Purpose** - Animaciones que comunican estado
5. **Glassmorphism Everywhere** - Cards translúcidos con blur

---

## Performance

- **CSS Bundle**: ~23KB minificado total
- **Fonts**: Cargadas desde Google Fonts con `display=swap`
- **Animaciones**: Respetan `prefers-reduced-motion`
- **Accesibilidad**: WCAG AA compliant

---

## Browser Support

- Chrome/Edge 90+
- Firefox 88+
- Safari 14+

Requiere soporte para:
- CSS Custom Properties
- Backdrop Filter
- CSS Grid
- CSS Animations

---

## Personalización

### Cambiar Color de Marca

Edita en `tokens.css`:
```css
:root {
  --zyber-primary-600: #TU_COLOR;
}
```

### Agregar Componente

Sigue el patrón en `components.css`:
```css
.mi-componente {
  background: var(--zyber-bg-tertiary);
  border: 1px solid var(--zyber-border-default);
  border-radius: var(--radius-md);
  padding: var(--space-4);
  transition: var(--transition-base);
}
```

---

## Integración con Frameworks

### React
```jsx
import './design-system/tokens.css';
import './design-system/components.css';

function App() {
  return (
    <div className="card">
      <h3 className="card-title">Title</h3>
    </div>
  );
}
```

### Vue
```vue
<template>
  <div class="card">
    <h3 class="card-title">Title</h3>
  </div>
</template>

<style>
@import './design-system/tokens.css';
@import './design-system/components.css';
</style>
```

### Vanilla JS
```html
<link rel="stylesheet" href="design-system/tokens.css">
<link rel="stylesheet" href="design-system/components.css">

<div class="card">
  <h3 class="card-title">Title</h3>
</div>
```

---

## Roadmap

### v1.1 (Próximamente)
- Librería de iconos SVG
- Componentes adicionales (Modal, Dropdown, Tooltip)
- Archivo de Figma
- Storybook

### v2.0 (Futuro)
- Soporte para light mode
- Variantes de componentes
- Sistema de theming
- CLI generator

---

## Recursos Externos

### Fuentes
- [Space Grotesk](https://fonts.google.com/specimen/Space+Grotesk)
- [Inter](https://fonts.google.com/specimen/Inter)
- [JetBrains Mono](https://www.jetbrains.com/lp/mono/)

### Inspiración
- Glassmorphism UI
- Cyberpunk aesthetics
- Solana ecosystem design
- Privacy-first applications

---

## Soporte

**Issues**: GitHub Issues
**Discussions**: GitHub Discussions
**Email**: design@zyberlink.io (ejemplo)

---

## Licencia

MIT License - Ver LICENSE en la raíz del proyecto

---

## Créditos

**Diseñado por**: ZyberLink Team
**Concepto**: "Encrypted Light" - Glassmorphic Cyberpunk
**Versión**: 1.0.0
**Fecha**: Noviembre 2025

---

**Built with privacy, designed with purpose.**

*La identidad visual que hace que la privacidad se vea tan buena como se siente segura.*
