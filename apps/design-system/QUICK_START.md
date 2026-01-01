# ZyberLink Design System - Quick Start

Guía rápida para empezar a usar la identidad visual de ZyberLink en 5 minutos.

---

## 1. Instalación (30 segundos)

### HTML básico

```html
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Tu App - ZyberLink</title>

  <!-- Fuentes de Google -->
  <link rel="preconnect" href="https://fonts.googleapis.com">
  <link href="https://fonts.googleapis.com/css2?family=Space+Grotesk:wght@600;700&family=Inter:wght@400;500;600&family=JetBrains+Mono&display=swap" rel="stylesheet">

  <!-- Design System CSS -->
  <link rel="stylesheet" href="design-system/tokens.css">
  <link rel="stylesheet" href="design-system/components.css">
</head>
<body>
  <!-- Tu contenido aquí -->
</body>
</html>
```

---

## 2. Componentes Esenciales (Copy-Paste Ready)

### Hero Section

```html
<section class="hero cyber-grid" style="min-height: 80vh; display: flex; align-items: center; justify-content: center;">
  <div class="container" style="text-align: center; max-width: 800px;">
    <h1 class="text-gradient" style="font-size: var(--text-6xl); margin-bottom: var(--space-6);">
      ZyberLink
    </h1>
    <p style="font-size: var(--text-xl); color: var(--zyber-text-secondary); margin-bottom: var(--space-8);">
      Private Compute Marketplace on Solana
    </p>
    <div class="flex gap-4 justify-center">
      <button class="btn btn-primary btn-lg">Connect Wallet</button>
      <button class="btn btn-secondary btn-lg">View Docs</button>
    </div>
  </div>
</section>
```

### Card de Dashboard

```html
<div class="card">
  <div class="card-header">
    <div class="flex justify-between items-center">
      <h3 class="card-title">Active FHE Jobs</h3>
      <span class="badge badge-primary">3</span>
    </div>
  </div>

  <div class="card-body">
    <!-- Job Item -->
    <div style="padding: var(--space-4); background: var(--zyber-bg-tertiary); border-radius: var(--radius-base); margin-bottom: var(--space-3);">
      <div class="flex justify-between" style="margin-bottom: var(--space-2);">
        <span class="text-primary">#1234 - Balance Verification</span>
        <span class="badge badge-warning">Computing</span>
      </div>
      <div class="computing-indicator"></div>
      <div class="flex gap-4" style="margin-top: var(--space-2); font-size: var(--text-sm); color: var(--zyber-text-tertiary);">
        <span>Provers: 3/3</span>
        <span>Fee: 0.02 SOL</span>
        <span>2 min ago</span>
      </div>
    </div>
  </div>

  <div class="card-footer">
    <button class="btn btn-ghost">View All</button>
    <button class="btn btn-primary">Create Job</button>
  </div>
</div>
```

### Formulario

```html
<div class="card" style="max-width: 500px;">
  <form>
    <h2 style="font-size: var(--text-2xl); color: var(--zyber-text-primary); margin-bottom: var(--space-6);">
      Submit FHE Job
    </h2>

    <div class="input-group">
      <label class="input-label" for="circuit">Circuit Type</label>
      <select id="circuit" class="input">
        <option>Balance Verification</option>
        <option>Private Transfer</option>
        <option>Data Verification</option>
      </select>
    </div>

    <div class="input-group">
      <label class="input-label" for="witness">Encrypted Witness</label>
      <textarea
        id="witness"
        class="input textarea"
        placeholder="Paste encrypted data..."
      ></textarea>
      <span class="input-helper">Data is never stored in plaintext</span>
    </div>

    <div class="input-group">
      <label class="input-label" for="fee">Prover Fee (SOL)</label>
      <input
        type="number"
        id="fee"
        class="input"
        placeholder="0.02"
        step="0.01"
      >
    </div>

    <button type="submit" class="btn btn-primary" style="width: 100%;">
      Submit Job
    </button>
  </form>
</div>
```

### Navigation Bar

```html
<nav style="background: var(--zyber-bg-secondary); border-bottom: 1px solid var(--zyber-divider); padding: var(--space-4) 0;">
  <div class="container">
    <div class="flex justify-between items-center">
      <!-- Logo -->
      <div class="flex items-center gap-3">
        <div style="width: 40px; height: 40px; background: var(--gradient-brand); border-radius: var(--radius-md); display: flex; align-items: center; justify-content: center; font-family: var(--font-heading); font-weight: 900; color: white; font-size: 20px;">
          Z
        </div>
        <span class="text-gradient" style="font-family: var(--font-heading); font-size: var(--text-xl); font-weight: 700;">
          ZyberLink
        </span>
      </div>

      <!-- Nav Items -->
      <div class="flex gap-6">
        <a href="#" style="color: var(--zyber-text-secondary); font-weight: 500;">Dashboard</a>
        <a href="#" style="color: var(--zyber-text-secondary); font-weight: 500;">Jobs</a>
        <a href="#" style="color: var(--zyber-text-secondary); font-weight: 500;">Docs</a>
      </div>

      <!-- CTA -->
      <button class="btn btn-primary">Connect Wallet</button>
    </div>
  </div>
</nav>
```

### Stats Grid

```html
<div class="container" style="margin-top: var(--space-16);">
  <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: var(--space-6);">
    <!-- Stat 1 -->
    <div class="card" style="text-align: center;">
      <div class="text-gradient" style="font-size: var(--text-4xl); font-weight: 700; font-family: var(--font-heading); margin-bottom: var(--space-2);">
        2,847
      </div>
      <div style="font-size: var(--text-sm); color: var(--zyber-text-tertiary); text-transform: uppercase; letter-spacing: 0.05em;">
        Jobs Completed
      </div>
    </div>

    <!-- Stat 2 -->
    <div class="card" style="text-align: center;">
      <div class="text-gradient" style="font-size: var(--text-4xl); font-weight: 700; font-family: var(--font-heading); margin-bottom: var(--space-2);">
        156
      </div>
      <div style="font-size: var(--text-sm); color: var(--zyber-text-tertiary); text-transform: uppercase; letter-spacing: 0.05em;">
        Active Provers
      </div>
    </div>

    <!-- Stat 3 -->
    <div class="card" style="text-align: center;">
      <div class="text-gradient" style="font-size: var(--text-4xl); font-weight: 700; font-family: var(--font-heading); margin-bottom: var(--space-2);">
        99.8%
      </div>
      <div style="font-size: var(--text-sm); color: var(--zyber-text-tertiary); text-transform: uppercase; letter-spacing: 0.05em;">
        Consensus Rate
      </div>
    </div>
  </div>
</div>
```

---

## 3. Elementos UI Rápidos

### Botones

```html
<!-- Primario (CTA principal) -->
<button class="btn btn-primary">Connect Wallet</button>

<!-- Secundario (alternativa) -->
<button class="btn btn-secondary">Learn More</button>

<!-- Ghost (terciario) -->
<button class="btn btn-ghost">Cancel</button>

<!-- Accent (especial) -->
<button class="btn btn-accent">Start Now</button>

<!-- Tamaños -->
<button class="btn btn-primary btn-sm">Small</button>
<button class="btn btn-primary">Base</button>
<button class="btn btn-primary btn-lg">Large</button>
```

### Badges de Estado

```html
<span class="badge badge-primary">Computing</span>
<span class="badge badge-accent">Active</span>
<span class="badge badge-success">Verified</span>
<span class="badge badge-warning">Pending</span>
<span class="badge badge-error">Failed</span>
```

### Inputs

```html
<!-- Input estándar -->
<div class="input-group">
  <label class="input-label" for="example">Label</label>
  <input type="text" id="example" class="input" placeholder="Placeholder">
  <span class="input-helper">Helper text</span>
</div>

<!-- Input con error -->
<div class="input-group">
  <label class="input-label">Email</label>
  <input type="email" class="input input-error" value="invalid@">
  <span class="input-helper input-error-text">Invalid email format</span>
</div>

<!-- Textarea -->
<div class="input-group">
  <label class="input-label" for="data">Data</label>
  <textarea id="data" class="input textarea" placeholder="Enter data..."></textarea>
</div>
```

### Computing Indicator (Barra de progreso)

```html
<div class="computing-indicator"></div>
```

### Texto con Gradient

```html
<h1 class="text-gradient">Amazing Title</h1>
```

---

## 4. Layouts Comunes

### Container Centrado

```html
<div class="container">
  <!-- Max-width centrado con padding -->
  <h1>Content here</h1>
</div>
```

### Grid Responsive

```html
<div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(300px, 1fr)); gap: var(--space-6);">
  <div class="card">Card 1</div>
  <div class="card">Card 2</div>
  <div class="card">Card 3</div>
</div>
```

### Flexbox

```html
<!-- Horizontal con gap -->
<div class="flex gap-4">
  <button class="btn btn-primary">Action 1</button>
  <button class="btn btn-secondary">Action 2</button>
</div>

<!-- Centrado -->
<div class="flex items-center justify-center" style="min-height: 50vh;">
  <h1>Centered Content</h1>
</div>

<!-- Space between -->
<div class="flex justify-between items-center">
  <span>Left</span>
  <span>Right</span>
</div>
```

---

## 5. Fondos y Efectos

### Cyber Grid Background

```html
<section class="cyber-grid" style="padding: var(--space-16) 0;">
  <!-- Content -->
</section>
```

### Hex Pattern Background

```html
<section class="hex-pattern" style="padding: var(--space-16) 0;">
  <!-- Content -->
</section>
```

### Card con Glow

```html
<div class="card card-glow">
  <!-- Content destacado -->
</div>
```

### Card Activa

```html
<div class="card card-active">
  <!-- Card seleccionada -->
</div>
```

### Pulse Glow Animation

```html
<div class="animate-pulse-glow">
  <!-- Elemento con animación -->
</div>
```

### Scan Line Effect

```html
<div class="scan-effect">
  <!-- Elemento con efecto de escaneo -->
</div>
```

---

## 6. Paleta de Colores (CSS Custom Properties)

```html
<!-- Usando colores del sistema -->
<div style="color: var(--zyber-primary-600);">Texto violeta</div>
<div style="background: var(--zyber-bg-tertiary);">Fondo oscuro</div>
<div style="border: 1px solid var(--zyber-border-default);">Con borde</div>

<!-- Ejemplos comunes -->
<style>
.my-element {
  /* Texto */
  color: var(--zyber-text-primary);      /* Blanco */
  color: var(--zyber-text-secondary);    /* Gris claro */
  color: var(--zyber-text-tertiary);     /* Gris medio */

  /* Backgrounds */
  background: var(--zyber-bg-base);      /* Fondo principal */
  background: var(--zyber-bg-tertiary);  /* Cards */
  background: var(--zyber-bg-elevated);  /* Modals */

  /* Bordes */
  border: 1px solid var(--zyber-border-default);
  border: 1px solid var(--zyber-border-emphasis);

  /* Espaciado */
  padding: var(--space-4);
  margin: var(--space-6);
  gap: var(--space-3);

  /* Border radius */
  border-radius: var(--radius-md);       /* 12px - botones */
  border-radius: var(--radius-lg);       /* 16px - cards */
}
</style>
```

---

## 7. Tipografía Rápida

```html
<!-- Headings -->
<h1 style="font-size: var(--text-5xl);">H1 Title</h1>
<h2 style="font-size: var(--text-4xl);">H2 Title</h2>
<h3 style="font-size: var(--text-3xl);">H3 Title</h3>

<!-- Body sizes -->
<p style="font-size: var(--text-base);">Normal text</p>
<p style="font-size: var(--text-sm);">Small text</p>
<p style="font-size: var(--text-xs);">Extra small</p>

<!-- Code -->
<code>inline code</code>
<pre><code>const x = 42;</code></pre>

<!-- Colores de texto -->
<p class="text-primary">Primary text</p>
<p class="text-secondary">Secondary text</p>
<p class="text-tertiary">Tertiary text</p>
<p class="text-accent">Accent text (cyan)</p>
```

---

## 8. Page Template Completo

```html
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>ZyberLink - Private Compute Marketplace</title>

  <!-- Fonts -->
  <link rel="preconnect" href="https://fonts.googleapis.com">
  <link href="https://fonts.googleapis.com/css2?family=Space+Grotesk:wght@600;700&family=Inter:wght@400;500;600&family=JetBrains+Mono&display=swap" rel="stylesheet">

  <!-- Design System -->
  <link rel="stylesheet" href="design-system/tokens.css">
  <link rel="stylesheet" href="design-system/components.css">
</head>
<body>

  <!-- Navigation -->
  <nav style="background: var(--zyber-bg-secondary); border-bottom: 1px solid var(--zyber-divider); padding: var(--space-4) 0;">
    <div class="container flex justify-between items-center">
      <div class="text-gradient" style="font-family: var(--font-heading); font-size: var(--text-xl); font-weight: 700;">
        ZyberLink
      </div>
      <button class="btn btn-primary">Connect Wallet</button>
    </div>
  </nav>

  <!-- Hero -->
  <section class="cyber-grid" style="min-height: 80vh; display: flex; align-items: center; justify-content: center;">
    <div class="container" style="text-align: center; max-width: 800px;">
      <h1 class="text-gradient" style="font-size: var(--text-6xl); margin-bottom: var(--space-6);">
        ZyberLink
      </h1>
      <p style="font-size: var(--text-xl); color: var(--zyber-text-secondary); margin-bottom: var(--space-8);">
        Private Compute Marketplace on Solana
      </p>
      <div class="flex gap-4 justify-center">
        <button class="btn btn-primary btn-lg">Get Started</button>
        <button class="btn btn-secondary btn-lg">Learn More</button>
      </div>
    </div>
  </section>

  <!-- Cards -->
  <section style="padding: var(--space-16) 0;">
    <div class="container">
      <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(300px, 1fr)); gap: var(--space-6);">
        <div class="card">
          <h3 class="card-title">Feature 1</h3>
          <p class="text-secondary">Description here</p>
        </div>
        <div class="card">
          <h3 class="card-title">Feature 2</h3>
          <p class="text-secondary">Description here</p>
        </div>
        <div class="card">
          <h3 class="card-title">Feature 3</h3>
          <p class="text-secondary">Description here</p>
        </div>
      </div>
    </div>
  </section>

  <!-- Footer -->
  <footer style="border-top: 1px solid var(--zyber-divider); padding: var(--space-8) 0; text-align: center;">
    <div class="container">
      <p class="text-tertiary" style="font-size: var(--text-sm);">
        Built with privacy, powered by decentralization.
      </p>
    </div>
  </footer>

</body>
</html>
```

---

## 9. Tips Rápidos

### Do's

- Usa `class="text-gradient"` para títulos principales
- Aplica `class="card"` para containers de contenido
- Usa `class="btn btn-primary"` para CTAs principales
- Aplica `cyber-grid` o `hex-pattern` en fondos de sección
- Usa design tokens (`var(--zyber-*)`) en lugar de valores hardcoded

### Don'ts

- No uses más de un `btn-primary` por sección
- No apliques gradients a todo (solo headings principales)
- No uses colores fuera de la paleta
- No ignores los estados de focus
- No animes sin propósito

---

## 10. Recursos

- **Demo completo**: `design-system/demo.html`
- **Guía de marca**: `design-system/BRAND_GUIDE.md`
- **Identidad visual**: `design-system/VISUAL_IDENTITY.md`
- **Logos**: `design-system/assets/logo/`

---

Ahora tienes todo lo que necesitas para implementar la identidad de ZyberLink.

Copia, pega, personaliza y construye algo alucinante!
