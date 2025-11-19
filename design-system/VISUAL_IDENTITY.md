# ZyberLink - Identidad Visual Completa

**Tema:** "Encrypted Light" - Glassmorphic Cyberpunk
**Versión:** 1.0.0
**Fecha:** Noviembre 2025

---

## Resumen Ejecutivo

ZyberLink tiene una identidad visual única y memorable que transmite:

- **Privacidad**: Colores violeta oscuros, efectos de encriptación
- **Tecnología avanzada**: Glassmorphismo, efectos glow, animaciones cyberpunk
- **Confianza**: Diseño profesional, jerarquía clara, accesibilidad
- **Innovación**: Vanguardia en FHE y blockchain

---

## 1. Paleta de Colores

### Colores Primarios

```
┌─────────────────────────────────────────────────┐
│  QUANTUM VIOLET (Brand Color)                  │
├─────────────────────────────────────────────────┤
│  #8B5CF6  ████████████  Primario               │
│  #A78BFA  ████████████  Light (Hover)          │
│  #6D28D9  ████████████  Dark (Active)          │
└─────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────┐
│  CYBER CYAN (Accent)                            │
├─────────────────────────────────────────────────┤
│  #06B6D4  ████████████  Accent Principal       │
│  #22D3EE  ████████████  Light                  │
│  #0891B2  ████████████  Dark                   │
└─────────────────────────────────────────────────┘
```

### Colores Semánticos

```
Success:  #10B981  ████████████  Verde tech
Warning:  #F59E0B  ████████████  Amber
Error:    #EF4444  ████████████  Rojo
```

### Backgrounds (Dark Mode First)

```
Base:       #0A0A0F  ████████████  Casi negro con tinte violeta
Secondary:  #13131A  ████████████  Fondo secundario
Tertiary:   #1C1C26  ████████████  Cards/panels
Elevated:   #25253C  ████████████  Modals/dropdowns
```

### Efecto Glow

Los glows reemplazan las sombras tradicionales para crear profundidad:

```css
/* Glow primario (violeta) */
box-shadow: 0 0 20px rgba(139, 92, 246, 0.4);

/* Glow accent (cyan) */
box-shadow: 0 0 20px rgba(6, 182, 212, 0.3);

/* Glow combinado con profundidad */
box-shadow:
  0 8px 32px rgba(0, 0, 0, 0.4),          /* Depth */
  0 0 24px rgba(139, 92, 246, 0.2),       /* Glow */
  inset 0 1px 0 rgba(255, 255, 255, 0.1); /* Highlight */
```

---

## 2. Tipografía

### Familias

```
┌─────────────────────────────────────────────────┐
│  HEADINGS - Space Grotesk                      │
│  ──────────────────────────────────────────────│
│  ZyberLink (Bold, 700)                         │
│  Private Compute (Semibold, 600)               │
│                                                 │
│  Características:                               │
│  - Geométrica moderna                           │
│  - Tech-forward                                 │
│  - Excelente en tamaños grandes                 │
└─────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────┐
│  BODY - Inter                                   │
│  ──────────────────────────────────────────────│
│  Execute FHE computations on Solana with       │
│  multi-prover consensus. (Regular, 400)        │
│                                                 │
│  Características:                               │
│  - Optimizada para UI                           │
│  - Legibilidad superior                         │
│  - Amplio rango de pesos                        │
└─────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────┐
│  CODE - JetBrains Mono                          │
│  ──────────────────────────────────────────────│
│  const result = tfhe.process(data);             │
│  balance_verification.fhe                       │
│                                                 │
│  Características:                               │
│  - Ligaduras para código                        │
│  - Distinción clara de caracteres               │
│  - Developer-friendly                           │
└─────────────────────────────────────────────────┘
```

### Escala Tipográfica (Modular 1.25)

```
Hero:        72px  (text-6xl)  ████████████████████████
H1:          56px  (text-5xl)  ██████████████████
H2:          40px  (text-4xl)  ██████████████
H3:          32px  (text-3xl)  ████████████
H4:          24px  (text-2xl)  ██████████
H5:          20px  (text-xl)   ████████
Body:        16px  (text-base) ██████
Small:       14px  (text-sm)   ████
Caption:     12px  (text-xs)   ███
```

### Efecto Gradient Text

Para headings principales:

```css
.hero-title {
  background: linear-gradient(135deg, #A78BFA 0%, #06B6D4 100%);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
}
```

Resultado visual:
```
████████████████████  (Violeta → Cyan gradient)
█ ZyberLink ██████
████████████████████
```

---

## 3. Logo

### Concepto: "Bloque Encriptado"

El logo representa un bloque de blockchain con datos parcialmente cifrados.

```
┌─────────────────────────────────────┐
│ ╔═══════════════════════════════╗  │
│ ║  ▓▓▓▓                         ║  │
│ ║  ▓▓▓  ZyberLink               ║  │
│ ║  ▓▓▓  ENCRYPTED COMPUTE       ║  │
│ ║                                ║  │
│ ╚═══════════════════════════════╝  │
└─────────────────────────────────────┘

Elementos:
- Marco glassmorphic con borde violeta
- Letra "Z" formada con bloques pixelados
- Gradient violeta → cyan
- Glow effect alrededor
- Píxeles de "encriptación" dispersos
```

### Variantes

**1. Logo Completo** (`zyberlink-full.svg`)
- Icono + Texto + Tagline
- Uso: Headers, landing pages, documentación

**2. Solo Icono** (`zyberlink-icon.svg`)
- Solo el bloque "Z"
- Uso: Favicons, avatares, app icons (64x64)

**3. Solo Texto** (`zyberlink-wordmark.svg`)
- "ZyberLink" con gradient
- Uso: Espacios horizontales reducidos

### Espacio Mínimo

Dejar al menos **2x altura del icono** de espacio libre alrededor.

---

## 4. Componentes Visuales

### Glassmorphic Cards

El elemento más distintivo del diseño:

```
┌────────────────────────────────────────┐
│  ╔════════════════════════════════╗   │  ← Borde violeta (0.2 opacity)
│  ║ [Blur background]              ║   │
│  ║                                 ║   │  ← Background semi-transparente
│  ║  FHE Job #1234                  ║   │     con backdrop-blur
│  ║  Balance Verification           ║   │
│  ║                                 ║   │
│  ║  Status: Computing...           ║   │
│  ║  ▓▓▓▓▓▓░░░░░░░░░░░░  (40%)     ║   │  ← Computing indicator
│  ║                                 ║   │
│  ╚════════════════════════════════╝   │
│          ↑                             │
│      Glow effect                       │
└────────────────────────────────────────┘
```

Propiedades CSS:
```css
background: rgba(28, 28, 38, 0.6);
backdrop-filter: blur(20px) saturate(150%);
border: 1px solid rgba(139, 92, 246, 0.2);
box-shadow:
  0 8px 32px rgba(0, 0, 0, 0.4),
  inset 0 1px 0 rgba(255, 255, 255, 0.05);
```

### Buttons

**Primary Button** (CTA principal)
```
┌────────────────────────┐
│  Connect Wallet        │  ← Gradient violeta
│  [Shimmer effect →]    │     + Shimmer en hover
└────────────────────────┘
      ↓ Glow violeta
```

**Secondary Button** (Alternativa)
```
┌────────────────────────┐
│  Learn More            │  ← Transparente con borde
│                        │     violeta
└────────────────────────┘
```

**Ghost Button** (Terciario)
```
  Cancel                    ← Sin borde, hover = bg sutil
```

### Status Badges

```
[Computing]  ← Badge violeta (primary)
[Active]     ← Badge cyan (accent)
[Verified]   ← Badge verde (success)
[Pending]    ← Badge amber (warning)
[Failed]     ← Badge rojo (error)
```

---

## 5. Animaciones Signature

### 1. Pulse Glow (Estado activo/computando)

```
Frame 1: ●  (100% opacity, scale 1)
Frame 2: ◉  (80% opacity, scale 1.02)
Frame 3: ●  (100% opacity, scale 1)

Duración: 3s infinite
```

Uso: Indicar que algo está procesándose

### 2. Scan Line (Efecto cyberpunk)

```
┌─────────────────┐
│ ═══════         │  ← Línea horizontal que baja
│                 │
│                 │
│         ═══════ │
└─────────────────┘

Duración: 4s infinite
```

Uso: Fondos de secciones importantes

### 3. Computing Indicator

```
[░░░░░░░░░░░░░░░░░░░░]  Frame 1
[▓▓▓▓▓░░░░░░░░░░░░░░]  Frame 2
[░░░░░░░░▓▓▓▓▓░░░░░░]  Frame 3
[░░░░░░░░░░░░░░▓▓▓▓▓]  Frame 4

Gradient: violeta → cyan que se mueve
Duración: 2s infinite
```

Uso: Barra de progreso para computaciones

### 4. Encrypt/Decrypt Text

```
Frame 1: ░░░░░░░ (borroso, invisible)
Frame 2: ▓░░▓░▓░ (parcialmente visible)
Frame 3: Private (completamente visible)

Duración: 0.6s ease-out
```

Uso: Revelar información privada

---

## 6. Patrones de Fondo

### Cyber Grid

```
┌───┬───┬───┬───┐
│   │   │   │   │
├───┼───┼───┼───┤
│   │   │   │   │  ← Grid violeta sutil (0.1 opacity)
├───┼───┼───┼───┤
│   │   │   │   │
└───┴───┴───┴───┘
```

CSS:
```css
background-image:
  linear-gradient(rgba(139, 92, 246, 0.1) 1px, transparent 1px),
  linear-gradient(90deg, rgba(139, 92, 246, 0.1) 1px, transparent 1px);
background-size: 50px 50px;
```

### Hex Pattern

```
  ⬡   ⬡   ⬡
    ⬡   ⬡   ⬡    ← Hexágonos sutiles
  ⬡   ⬡   ⬡
    ⬡   ⬡   ⬡
```

Uso: Fondos de secciones hero

---

## 7. Mood Board Visual

### Inspiración de Color

```
Noche cyberpunk:
┌─────────────────────────────────┐
│  ████████████████  ← Violeta   │
│  ████████████████  ← Cyan      │
│  ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓  ← Negro      │
│  ░░░░░░░░░░░░░░░  ← Glow       │
└─────────────────────────────────┘
```

### Palabras Clave

```
┌────────────────┬────────────────┬────────────────┐
│ PRIVACIDAD     │ TECNOLOGÍA     │ CONFIANZA      │
├────────────────┼────────────────┼────────────────┤
│ - Encriptación │ - Cyberpunk    │ - Profesional  │
│ - Oscuridad    │ - Futurista    │ - Limpio       │
│ - Misterio     │ - Innovación   │ - Accesible    │
│ - Protección   │ - Vanguardia   │ - Transparente │
└────────────────┴────────────────┴────────────────┘
```

### Atmósfera

```
┌──────────────────────────────────────┐
│                                      │
│   "Luces en la oscuridad"            │
│                                      │
│   Computación invisible iluminada    │
│   por glows cryptográficos           │
│                                      │
└──────────────────────────────────────┘
```

---

## 8. Casos de Uso

### Landing Page Hero

```
╔════════════════════════════════════════════════════╗
║                                                    ║
║           ████████████████████                     ║
║           █ ZyberLink ████████  ← Gradient text   ║
║           ████████████████████                     ║
║                                                    ║
║      Private Compute Marketplace on Solana        ║
║                                                    ║
║   ┌──────────────────┐  ┌──────────────────┐     ║
║   │ Connect Wallet   │  │ View Docs        │     ║
║   └──────────────────┘  └──────────────────┘     ║
║                                                    ║
╚════════════════════════════════════════════════════╝
            ↑ Cyber grid background
```

### Dashboard Card

```
╔═══════════════════════════════════════╗
║  Active Jobs                [3]       ║  ← Header
╠═══════════════════════════════════════╣
║                                       ║
║  #1234 - Balance Verification         ║
║  [Computing] ▓▓▓▓▓▓░░░░░░░  (60%)    ║  ← Job item
║  Provers: 3/3  |  Fee: 0.02 SOL      ║
║                                       ║
║  #1235 - Private Transfer             ║
║  [Success] ✓                          ║
║  Provers: 3/3  |  Fee: 0.02 SOL      ║
║                                       ║
╠═══════════════════════════════════════╣
║           [Create New Job]            ║  ← Footer
╚═══════════════════════════════════════╝
```

### Form

```
╔═══════════════════════════════════════╗
║  Submit FHE Job                       ║
╠═══════════════════════════════════════╣
║                                       ║
║  CIRCUIT TYPE                         ║
║  ┌─────────────────────────────────┐ ║
║  │ Balance Verification        ▼   │ ║
║  └─────────────────────────────────┘ ║
║                                       ║
║  ENCRYPTED WITNESS                    ║
║  ┌─────────────────────────────────┐ ║
║  │ Paste encrypted data...         │ ║
║  │                                 │ ║
║  └─────────────────────────────────┘ ║
║  Data is never stored in plaintext    ║
║                                       ║
║  PROVER FEE (SOL)                     ║
║  ┌─────────────────────────────────┐ ║
║  │ 0.02                            │ ║
║  └─────────────────────────────────┘ ║
║                                       ║
║         ┌─────────────────┐          ║
║         │  Submit Job     │          ║
║         └─────────────────┘          ║
║                                       ║
╚═══════════════════════════════════════╝
```

---

## 9. Principios de Diseño

### 1. Dark Mode First
Todo diseñado para fondos oscuros. Light mode es opcional.

### 2. Glow over Shadow
Profundidad creada con glows luminosos, no sombras tradicionales.

### 3. Progressive Disclosure
Mostrar complejidad gradualmente. No abrumar al usuario.

### 4. Motion with Purpose
Animaciones solo cuando comunican estado o feedback.

### 5. Glassmorphism Everywhere
Cards y paneles siempre con efecto glass (translucent + blur).

---

## 10. Quick Reference

### Colores Esenciales

```
Primary:    #8B5CF6
Accent:     #06B6D4
Success:    #10B981
Background: #0A0A0F
Text:       #F9FAFB
```

### Fonts

```
Headings: Space Grotesk (700)
Body:     Inter (400)
Code:     JetBrains Mono (400)
```

### Espaciado (8pt grid)

```
xs:  4px
sm:  8px
md:  16px
lg:  24px
xl:  32px
2xl: 48px
```

### Border Radius

```
sm:   4px  (badges)
md:   12px (buttons)
lg:   16px (cards)
xl:   24px (modals)
```

---

## 11. Archivos del Sistema

```
design-system/
├── tokens.css              # Variables CSS
├── components.css          # Componentes
├── demo.html              # Demo interactivo
├── BRAND_GUIDE.md         # Guía completa
├── VISUAL_IDENTITY.md     # Este archivo
└── assets/
    └── logo/
        ├── zyberlink-full.svg      # Logo completo
        ├── zyberlink-icon.svg      # Solo icono
        └── zyberlink-wordmark.svg  # Solo texto
```

---

## 12. Próximos Pasos

### Para implementar la identidad:

1. **Ver el demo**: Abre `demo.html` en tu navegador
2. **Importar CSS**: Usa `tokens.css` y `components.css` en tu proyecto
3. **Usar componentes**: Copia ejemplos de `demo.html`
4. **Personalizar**: Ajusta tokens en `tokens.css` si es necesario

### Para crear nuevos diseños:

1. **Usa la paleta**: Solo colores del sistema
2. **Aplica glassmorphism**: Cards con backdrop-blur
3. **Añade glows**: En hovers y estados activos
4. **Anima con propósito**: Solo para comunicar estado
5. **Mantén dark mode**: Fondos oscuros siempre

---

## Créditos

**Diseñado por:** ZyberLink Team
**Concepto:** "Encrypted Light" - Glassmorphic Cyberpunk
**Inspiración:** Solana, Cyberpunk aesthetics, Privacy-first design

Built with privacy, designed with purpose.
