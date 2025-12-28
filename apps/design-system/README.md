# ZyberLink Design System

> "Encrypted Light" - Glassmorphic Cyberpunk Design Language

**Version:** 1.0.0
**Status:** Production Ready
**Last Updated:** November 2025

---

## Quick Start

### 1. View the Demo

Open `demo.html` in your browser to see all components in action:

```bash
cd design-system
open demo.html  # macOS
xdg-open demo.html  # Linux
start demo.html  # Windows
```

Or serve it locally:

```bash
python3 -m http.server 8000
# Visit http://localhost:8000/demo.html
```

### 2. Import Styles

Add these to your HTML:

```html
<!-- Google Fonts -->
<link rel="preconnect" href="https://fonts.googleapis.com">
<link href="https://fonts.googleapis.com/css2?family=Space+Grotesk:wght@400;600;700&family=Inter:wght@400;500;600&family=JetBrains+Mono:wght@400;500&display=swap" rel="stylesheet">

<!-- Design System CSS -->
<link rel="stylesheet" href="design-system/tokens.css">
<link rel="stylesheet" href="design-system/components.css">
```

### 3. Use Components

```html
<!-- Hero Section -->
<section class="hero cyber-grid">
  <h1 class="text-gradient">ZyberLink</h1>
  <p class="text-secondary">Private Compute Marketplace</p>
  <button class="btn btn-primary">Connect Wallet</button>
</section>

<!-- Card -->
<div class="card">
  <div class="card-header">
    <h3 class="card-title">FHE Job</h3>
  </div>
  <div class="card-body">
    <p class="text-secondary">Computing...</p>
    <div class="computing-indicator"></div>
  </div>
</div>
```

---

## File Structure

```
design-system/
├── tokens.css           # Design tokens (colors, spacing, etc.)
├── components.css       # Component library
├── demo.html           # Live component showcase
├── BRAND_GUIDE.md      # Complete brand guidelines
├── README.md           # This file
└── assets/
    ├── logo/           # Logo files (SVG, PNG)
    └── icons/          # Icon set
```

---

## Core Concepts

### Theme: "Encrypted Light"

The visual language is based on the concept of invisible computation - calculations happening through encrypted channels, creating beautiful glows in the darkness.

### Design Pillars

1. **Glassmorphism** - Translucent cards with backdrop blur
2. **Glow over Shadow** - Depth through luminous effects
3. **Dark Mode First** - Designed for dark backgrounds
4. **Motion with Purpose** - Animations communicate state
5. **Developer-Friendly** - Clean code, clear patterns

---

## Key Features

### Color System

**Primary:** Quantum Violet (#8B5CF6)
- Brand color
- CTAs and key interactions

**Accent:** Cyber Cyan (#06B6D4)
- Complementary energy
- Links and secondary actions

**Semantic:** Success/Warning/Error
- State communication

### Typography

**Headings:** Space Grotesk
- Modern, geometric
- Bold presence
- Tech-forward feel

**Body:** Inter
- Optimized legibility
- Wide weight range
- Clean and professional

**Code:** JetBrains Mono
- Developer-friendly
- Ligature support
- Clear character distinction

### Components

**Buttons:** 4 variants (Primary, Secondary, Ghost, Accent)
**Cards:** Glassmorphic with hover effects
**Forms:** Clear states with glow focus
**Badges:** Status indicators with semantic colors
**Animations:** Pulse glow, scan lines, computing indicators

---

## Browser Support

- Chrome/Edge 90+
- Firefox 88+
- Safari 14+

**Critical Features:**
- CSS Custom Properties
- Backdrop Filter
- Grid Layout
- CSS Animations

---

## Performance

### CSS Bundle Sizes

- `tokens.css`: ~8KB (minified)
- `components.css`: ~15KB (minified)
- **Total:** ~23KB

### Optimization Tips

1. **Tree-shake unused components** in production
2. **Load fonts async** with `font-display: swap`
3. **Use CSS containment** for large lists
4. **Respect `prefers-reduced-motion`**

---

## Customization

### Changing Brand Colors

Edit `tokens.css`:

```css
:root {
  --zyber-primary-600: #YOUR_COLOR;  /* Main brand */
  --zyber-accent-500: #YOUR_COLOR;   /* Accent */
}
```

### Adding Components

Follow the pattern in `components.css`:

```css
/* Component name */
.my-component {
  /* Reset/base styles */
  display: flex;

  /* Layout */
  padding: var(--space-4);

  /* Visual */
  background: var(--zyber-bg-tertiary);
  border: 1px solid var(--zyber-border-default);
  border-radius: var(--radius-md);

  /* Interaction */
  transition: var(--transition-base);
}

.my-component:hover {
  border-color: var(--zyber-border-emphasis);
  box-shadow: var(--shadow-glow-primary);
}
```

### Dark/Light Mode Toggle

Currently dark-first. To add light mode:

```css
@media (prefers-color-scheme: light) {
  :root {
    --zyber-bg-base: #FFFFFF;
    --zyber-text-primary: #111827;
    /* Override other tokens */
  }
}
```

---

## Component Reference

### Buttons

```html
<button class="btn btn-primary">Primary</button>
<button class="btn btn-secondary">Secondary</button>
<button class="btn btn-ghost">Ghost</button>
<button class="btn btn-accent">Accent</button>

<!-- Sizes -->
<button class="btn btn-primary btn-sm">Small</button>
<button class="btn btn-primary">Base</button>
<button class="btn btn-primary btn-lg">Large</button>
```

### Cards

```html
<div class="card">
  <div class="card-header">
    <h3 class="card-title">Title</h3>
    <p class="card-subtitle">Subtitle</p>
  </div>
  <div class="card-body">
    Content goes here
  </div>
  <div class="card-footer">
    <button class="btn btn-primary">Action</button>
  </div>
</div>

<!-- Variants -->
<div class="card card-active">Active state</div>
<div class="card card-glow">Featured</div>
```

### Forms

```html
<div class="input-group">
  <label class="input-label" for="field">Label</label>
  <input
    type="text"
    id="field"
    class="input"
    placeholder="Placeholder"
  >
  <span class="input-helper">Helper text</span>
</div>

<!-- States -->
<input class="input input-error" />
<input class="input input-success" />
```

### Badges

```html
<span class="badge badge-primary">Primary</span>
<span class="badge badge-accent">Accent</span>
<span class="badge badge-success">Success</span>
<span class="badge badge-warning">Warning</span>
<span class="badge badge-error">Error</span>
```

### Animations

```html
<!-- Pulse glow (active state) -->
<div class="animate-pulse-glow">...</div>

<!-- Scan line effect -->
<div class="scan-effect">...</div>

<!-- Computing indicator -->
<div class="computing-indicator"></div>

<!-- Encrypt/reveal animation -->
<div class="animate-encrypt">...</div>
```

### Utilities

```html
<!-- Text -->
<h1 class="text-gradient">Gradient text</h1>
<p class="text-primary">Primary color</p>
<p class="text-secondary">Secondary color</p>

<!-- Layout -->
<div class="container">Centered container</div>
<div class="flex gap-4 items-center">Flexbox</div>

<!-- Background patterns -->
<section class="cyber-grid">Grid pattern</section>
<section class="hex-pattern">Hexagon pattern</section>
```

---

## Best Practices

### Do's

- Use glassmorphic cards for content containers
- Apply glows to interactive elements on hover/focus
- Use gradient text for hero headings
- Leverage design tokens (CSS variables)
- Respect accessibility (WCAG AA minimum)
- Test with `prefers-reduced-motion`

### Don'ts

- Don't use multiple gradients in one section
- Don't apply glow to every element
- Don't ignore focus states
- Don't use custom colors outside the palette
- Don't animate without purpose
- Don't override font families unless necessary

---

## Accessibility

### Built-in Features

- **Focus indicators:** 3px outline with glow
- **Touch targets:** Minimum 44x44px
- **Color contrast:** WCAG AA compliant
- **Motion sensitivity:** Respects `prefers-reduced-motion`
- **Semantic HTML:** Proper heading hierarchy

### Testing Checklist

- [ ] Keyboard navigation works
- [ ] Focus visible on all interactive elements
- [ ] Color contrast meets WCAG AA
- [ ] Text remains readable at 200% zoom
- [ ] Animations disabled when `prefers-reduced-motion: reduce`
- [ ] Screen reader compatible

---

## Integration Examples

### React

```jsx
import './design-system/tokens.css';
import './design-system/components.css';

function App() {
  return (
    <div className="card">
      <div className="card-header">
        <h3 className="card-title">FHE Job</h3>
      </div>
      <div className="card-body">
        <p className="text-secondary">Computing...</p>
        <div className="computing-indicator" />
      </div>
    </div>
  );
}
```

### Vue

```vue
<template>
  <div class="card">
    <div class="card-header">
      <h3 class="card-title">FHE Job</h3>
    </div>
    <div class="card-body">
      <p class="text-secondary">Computing...</p>
      <div class="computing-indicator" />
    </div>
  </div>
</template>

<style>
@import './design-system/tokens.css';
@import './design-system/components.css';
</style>
```

### Vanilla JS

```html
<!DOCTYPE html>
<html>
<head>
  <link rel="stylesheet" href="design-system/tokens.css">
  <link rel="stylesheet" href="design-system/components.css">
</head>
<body class="cyber-grid">
  <div class="container">
    <h1 class="text-gradient">ZyberLink</h1>
    <button class="btn btn-primary" onclick="handleClick()">
      Connect Wallet
    </button>
  </div>
</body>
</html>
```

---

## Roadmap

### v1.1 (Planned)

- [ ] Icon library (SVG sprite)
- [ ] Additional components (Dropdown, Modal, Tooltip)
- [ ] CSS-in-JS export (styled-components, emotion)
- [ ] Figma design file
- [ ] Storybook integration

### v2.0 (Future)

- [ ] Light mode support
- [ ] Component variants (compact, comfortable, spacious)
- [ ] Theming system (multi-brand)
- [ ] Animation presets library
- [ ] Component generator CLI

---

## Contributing

Currently in active development. Contributions welcome after initial release.

**How to contribute:**

1. Follow existing patterns in `components.css`
2. Use design tokens from `tokens.css`
3. Test across browsers
4. Ensure accessibility
5. Update `demo.html` with examples

---

## License

MIT License - See root LICENSE file

---

## Support

- **Documentation:** [BRAND_GUIDE.md](./BRAND_GUIDE.md)
- **Issues:** GitHub Issues
- **Discussions:** GitHub Discussions
- **Demo:** [demo.html](./demo.html)

---

## Credits

**Design System by:** ZyberLink Team
**Inspiration:**
- Glassmorphism UI trend
- Solana ecosystem design
- Cyberpunk aesthetics
- Privacy-first applications

**Fonts:**
- Space Grotesk by Florian Karsten
- Inter by Rasmus Andersson
- JetBrains Mono by JetBrains

---

Built with privacy, designed with purpose.
