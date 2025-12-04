# ZyberLink Brand Identity Guide

**Version:** 1.0.0
**Theme:** "Encrypted Light" - Glassmorphic Cyberpunk
**Updated:** November 2025

---

## Table of Contents

1. [Brand Concept](#brand-concept)
2. [Color Palette](#color-palette)
3. [Typography](#typography)
4. [Logo & Visual Identity](#logo--visual-identity)
5. [Design Principles](#design-principles)
6. [Component Guidelines](#component-guidelines)
7. [Animation & Motion](#animation--motion)
8. [Voice & Tone](#voice--tone)
9. [Usage Examples](#usage-examples)

---

## Brand Concept

### Core Idea: "Encrypted Light"

ZyberLink's identity is built on the concept of **invisible computation** - calculations that happen without revealing their contents, like light traveling through dark matter.

**Visual Metaphor:**
Data flows through encrypted channels, illuminated by cryptographic operations, creating beautiful glows in the darkness of privacy.

### Brand Attributes

```yaml
personality:
  - Cutting-edge tech
  - Privacy-first
  - Trustworthy
  - Hacker-friendly
  - Accessible yet sophisticated

emotions:
  primary: "Confidence in privacy"
  secondary: "Excitement about innovation"
  tertiary: "Trust in decentralization"

aesthetic:
  - Cyberpunk elegance
  - Glassmorphic depth
  - Luminous effects
  - Geometric precision
```

---

## Color Palette

### Primary Colors

#### Quantum Violet (Brand Color)

The signature color representing encryption, mystery, and advanced technology.

```css
--zyber-primary-600: #8B5CF6  /* Main brand color */
--zyber-primary-400: #A78BFA  /* Hover/highlights */
--zyber-primary-800: #6D28D9  /* Active states */
```

**Usage:**
- Primary CTAs
- Logo
- Key interactive elements
- Glow effects
- Borders and accents

**Avoid:**
- Large text blocks
- Background fills (use transparent variants)

#### Cyber Cyan (Accent)

Electric cyan for complementary energy and blockchain association.

```css
--zyber-accent-500: #06B6D4  /* Main accent */
--zyber-accent-400: #22D3EE  /* Lighter variant */
```

**Usage:**
- Links
- Secondary CTAs
- Info badges
- Success states
- Code highlights

### Semantic Colors

```css
Success:  #10B981  /* Green tech - successful operations */
Warning:  #F59E0B  /* Amber - cautionary states */
Error:    #EF4444  /* Red - error states */
```

### Neutral Palette (Dark Mode First)

```css
Background Base:      #0A0A0F  /* Almost black with violet tint */
Background Secondary: #13131A
Background Tertiary:  #1C1C26  /* Cards/panels */
Background Elevated:  #25253C  /* Modals/dropdowns */

Text Primary:         #F9FAFB  /* Pure white */
Text Secondary:       #D1D5DB  /* Light gray */
Text Tertiary:        #9CA3AF  /* Medium gray */
Text Disabled:        #6B7280
```

### Glow & Effects

```css
Glow Primary:  rgba(139, 92, 246, 0.4)  /* Violet glow */
Glow Accent:   rgba(6, 182, 212, 0.3)   /* Cyan glow */
Glow Success:  rgba(16, 185, 129, 0.3)  /* Green glow */
```

**When to use glows:**
- Active/focused states
- Hover effects on interactive elements
- Loading/computing states
- Highlighting important information

---

## Typography

### Font Families

#### 1. Space Grotesk (Headings)

Modern, geometric sans-serif with tech-forward personality.

```css
--font-heading: 'Space Grotesk', sans-serif;
```

**Characteristics:**
- Bold and confident
- Excellent readability at large sizes
- Slightly condensed letterforms
- Free on Google Fonts

**Usage:**
- H1-H6 headings
- Button labels
- Logo text
- Navigation items
- Hero text

#### 2. Inter (Body Text)

Clean, highly legible UI font designed for screens.

```css
--font-body: 'Inter', sans-serif;
```

**Characteristics:**
- Optimized for UI
- Wide range of weights
- Great readability
- Free on Google Fonts

**Usage:**
- Body copy
- Paragraphs
- Form labels
- UI text
- Descriptions

#### 3. JetBrains Mono (Code)

Monospace font with ligatures, designed for developers.

```css
--font-mono: 'JetBrains Mono', monospace;
```

**Characteristics:**
- Coding ligatures
- Clear character distinction
- Developer-friendly
- Free

**Usage:**
- Code snippets
- Terminal output
- Addresses/hashes
- Technical data
- Monospace UI elements

### Type Scale

Based on 1.25 modular scale:

```css
--text-xs:   12px  /* Labels, captions */
--text-sm:   14px  /* Secondary text */
--text-base: 16px  /* Body text (default) */
--text-lg:   18px  /* Subheadings */
--text-xl:   20px  /* H5 */
--text-2xl:  24px  /* H4 */
--text-3xl:  32px  /* H3 */
--text-4xl:  40px  /* H2 */
--text-5xl:  56px  /* H1 */
--text-6xl:  72px  /* Hero text */
```

### Typography Best Practices

**Headings:**
- Use Space Grotesk
- Font weight: 700 (bold)
- Letter spacing: -0.02em (tight)
- Line height: 1.1

**Body:**
- Use Inter
- Font weight: 400 (regular)
- Line height: 1.6 (comfortable reading)
- Max width: 65-75 characters

**Code:**
- Use JetBrains Mono
- Font size: 0.9em relative to body
- Background highlight for inline code
- Syntax highlighting for blocks

**Hierarchy Example:**

```html
<h1 class="text-gradient">Private Compute Marketplace</h1>
<p class="text-secondary">
  Execute FHE computations on Solana with multi-prover consensus
</p>
<code class="text-accent">balance_verification.fhe</code>
```

---

## Logo & Visual Identity

### Logo Concept

**Name:** "Encrypted Block"

The ZyberLink logo represents an encrypted blockchain block with partial cipher visualization.

#### Logo Construction

```
┌─────────────────────────────────────┐
│  ┏━━━━━━━━━━━━━━━━━━━━━━━━━━━┓   │
│  ┃  ▓▓▓  ZyberLink           ┃   │
│  ┃  ▓▓▓  ENCRYPTED COMPUTE    ┃   │
│  ┗━━━━━━━━━━━━━━━━━━━━━━━━━━━┛   │
└─────────────────────────────────────┘

Components:
- Geometric "Z" formed with pixel blocks
- Gradient fill (Violet → Cyan)
- Glow effect around edges
- Monospace subtitle
```

#### Logo Variants

**1. Full Logo (Primary)**
- Icon + "ZyberLink" text + tagline
- Use on landing pages, headers, documentation

**2. Logo Mark Only**
- Just the "Z" icon block
- Use for favicons, social media avatars, app icons

**3. Text Only**
- "ZyberLink" with gradient
- Use in tight horizontal spaces

**4. Monochrome**
- Single color (white or violet)
- Use on solid backgrounds, print, low-resolution

### Clear Space

Minimum clear space around logo: **2x the height of the "Z" icon**

### Minimum Size

- Digital: 120px width minimum
- Print: 1 inch width minimum
- Favicon: 32x32px

### Logo Don'ts

- Don't change colors outside brand palette
- Don't rotate or skew
- Don't add drop shadows (use glows instead)
- Don't place on busy backgrounds without backdrop
- Don't outline the logo

---

## Design Principles

### 1. Glassmorphism First

All cards and panels use glassmorphic design:

```css
.glass-card {
  background: rgba(28, 28, 38, 0.6);
  backdrop-filter: blur(20px) saturate(150%);
  border: 1px solid rgba(139, 92, 246, 0.2);
  box-shadow:
    0 8px 32px rgba(0, 0, 0, 0.4),
    inset 0 1px 0 rgba(255, 255, 255, 0.05);
}
```

**Key attributes:**
- Translucent backgrounds
- Backdrop blur
- Subtle borders with brand color
- Inner highlights (top edge glow)

### 2. Depth Through Glow

Instead of traditional shadows, use glows to create depth:

```css
/* Traditional shadow (avoid) */
box-shadow: 0 4px 8px rgba(0, 0, 0, 0.5);

/* ZyberLink glow (preferred) */
box-shadow:
  0 4px 16px rgba(139, 92, 246, 0.3),
  inset 0 1px 0 rgba(255, 255, 255, 0.1);
```

### 3. Motion with Purpose

Animations should communicate state, not just decoration:

**Good reasons to animate:**
- Loading/computing state
- State transitions (idle → active)
- User feedback (button click)
- Data updates

**Bad reasons:**
- "It looks cool"
- Auto-playing loops
- Distracting movements

### 4. Progressive Disclosure

Show complexity gradually. Don't overwhelm users.

**Example:**
```
[Simple View]
- 3 main actions visible

[Advanced View] (expandable)
- 10+ advanced options
- Technical details
```

### 5. Dark Mode First

All designs start with dark backgrounds:

- Reduces eye strain
- Hacker aesthetic
- Energy efficient (OLED)
- Makes glows pop

Light mode can be added later, but dark is primary.

---

## Component Guidelines

### Buttons

**Primary Button** (main actions)
```html
<button class="btn btn-primary">
  Connect Wallet
</button>
```

**When to use:**
- Single most important action on screen
- Submission actions (Submit Job, Create Account)
- Confirmations (Confirm Payment)

**Secondary Button** (alternative actions)
```html
<button class="btn btn-secondary">
  Learn More
</button>
```

**When to use:**
- Less important actions
- Cancellations
- Navigation

**Ghost Button** (subtle actions)
```html
<button class="btn btn-ghost">
  Skip
</button>
```

**When to use:**
- Tertiary actions
- Dismissals
- Optional steps

### Cards

**Standard Card**
```html
<div class="card">
  <div class="card-header">
    <h3 class="card-title">FHE Job #1234</h3>
    <p class="card-subtitle">Balance Verification</p>
  </div>
  <div class="card-body">
    <p>Status: Computing...</p>
  </div>
  <div class="card-footer">
    <button class="btn btn-secondary">Cancel</button>
    <button class="btn btn-primary">View Details</button>
  </div>
</div>
```

**Active Card** (selected state)
```html
<div class="card card-active">
  <!-- Content -->
</div>
```

**Card with Glow** (important/featured)
```html
<div class="card card-glow">
  <!-- Content -->
</div>
```

### Badges

**Status Indicators**
```html
<span class="badge badge-success">Active</span>
<span class="badge badge-warning">Pending</span>
<span class="badge badge-error">Failed</span>
<span class="badge badge-primary">Computing</span>
```

**When to use:**
- Status labels
- Tags
- Categories
- Counts

### Inputs

**Standard Input**
```html
<div class="input-group">
  <label class="input-label" for="amount">Amount</label>
  <input
    type="number"
    id="amount"
    class="input"
    placeholder="0.00"
  >
  <span class="input-helper">Enter amount in SOL</span>
</div>
```

**Input States:**
- Default: Subtle border
- Hover: Emphasized border
- Focus: Violet glow
- Error: Red glow + message
- Success: Green glow + checkmark

---

## Animation & Motion

### Animation Principles

1. **Duration Guidelines**
   - Micro-interactions: 150ms (fast)
   - Standard transitions: 250ms (base)
   - Complex animations: 350ms (slow)
   - Never exceed: 500ms

2. **Easing Functions**
   - Ease-out (default): Natural deceleration
   - Ease-in-out: Smooth start and end
   - Bounce: Playful interactions (use sparingly)

3. **Reduced Motion**
   Always respect user preferences:
   ```css
   @media (prefers-reduced-motion: reduce) {
     * {
       animation-duration: 0.01ms !important;
       transition-duration: 0.01ms !important;
     }
   }
   ```

### Key Animations

**Pulse Glow** (active/computing state)
```css
@keyframes pulse-glow {
  0%, 100% { opacity: 1; transform: scale(1); }
  50% { opacity: 0.8; transform: scale(1.02); }
}

.computing {
  animation: pulse-glow 3s ease-in-out infinite;
}
```

**Scan Line** (cyberpunk effect)
```css
@keyframes scan-line {
  from { transform: translateY(-100%); }
  to { transform: translateY(100%); }
}

.scan-effect::before {
  animation: scan-line 4s linear infinite;
}
```

**Encrypt/Decrypt Text** (data reveal)
```css
@keyframes encrypt {
  0% { opacity: 0; filter: blur(4px); }
  100% { opacity: 1; filter: blur(0); }
}

.reveal {
  animation: encrypt 0.6s ease-out;
}
```

**Computing Indicator** (progress bar)
```html
<div class="computing-indicator"></div>
```

Uses gradient bar that sweeps left to right continuously.

---

## Voice & Tone

### Brand Voice

**Confident, not arrogant**
"ZyberLink makes private computation simple."
NOT: "We're the best privacy solution ever."

**Technical, but accessible**
"Fully homomorphic encryption lets you compute on encrypted data."
NOT: "Leverage cutting-edge FHE cryptographic primitives."

**Hacker-friendly, not elitist**
"Build privacy-first apps on Solana."
NOT: "For expert cryptographers only."

### Writing Guidelines

**Headlines:**
- Clear and concise
- Action-oriented
- Under 60 characters

**Body Copy:**
- Short sentences
- Active voice
- Avoid jargon (or explain it)

**CTAs:**
- Verb-first
- Specific action
- 2-4 words

**Examples:**

Good CTAs:
- "Connect Wallet"
- "Submit Job"
- "View Results"
- "Start Computing"

Bad CTAs:
- "Click Here"
- "Go"
- "Submit"
- "Learn More About Our Advanced FHE Infrastructure"

---

## Usage Examples

### Landing Page Hero

```html
<section class="hero cyber-grid">
  <div class="container">
    <h1 class="text-gradient">
      Private Compute Marketplace
    </h1>
    <p class="text-lg text-secondary">
      Execute FHE computations on Solana with multi-prover consensus.
      Zero knowledge, full trust.
    </p>
    <div class="flex gap-4">
      <button class="btn btn-primary">
        Connect Wallet
      </button>
      <button class="btn btn-secondary">
        View Docs
      </button>
    </div>
  </div>
</section>
```

### Dashboard Card

```html
<div class="card card-glow">
  <div class="card-header">
    <div class="flex justify-between items-center">
      <h3 class="card-title">Active Jobs</h3>
      <span class="badge badge-primary">3</span>
    </div>
  </div>

  <div class="card-body">
    <div class="job-item">
      <div class="flex justify-between">
        <span class="text-primary">Balance Verification #1234</span>
        <span class="badge badge-warning">Computing</span>
      </div>
      <div class="computing-indicator"></div>
    </div>
  </div>

  <div class="card-footer">
    <button class="btn btn-ghost">View All</button>
  </div>
</div>
```

### Form Example

```html
<form class="glass-card">
  <h2 class="text-2xl text-primary">Submit FHE Job</h2>

  <div class="input-group">
    <label class="input-label" for="circuit">Circuit Type</label>
    <select id="circuit" class="input">
      <option>Balance Verification</option>
      <option>Private Transfer</option>
    </select>
  </div>

  <div class="input-group">
    <label class="input-label" for="data">Encrypted Witness</label>
    <textarea
      id="data"
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

  <button type="submit" class="btn btn-primary">
    Submit Job
  </button>
</form>
```

---

## Quick Reference

### Color Shortcuts

```css
Primary:   #8B5CF6  (Quantum Violet)
Accent:    #06B6D4  (Cyber Cyan)
Success:   #10B981
Warning:   #F59E0B
Error:     #EF4444
Background: #0A0A0F
Text:      #F9FAFB
```

### Font Imports

```html
<link rel="preconnect" href="https://fonts.googleapis.com">
<link href="https://fonts.googleapis.com/css2?family=Space+Grotesk:wght@400;700&family=Inter:wght@400;500;600&family=JetBrains+Mono&display=swap" rel="stylesheet">
```

### Essential Classes

```html
<!-- Typography -->
<h1 class="text-gradient">Heading</h1>
<p class="text-secondary">Body text</p>
<code class="text-accent">code</code>

<!-- Buttons -->
<button class="btn btn-primary">Primary</button>
<button class="btn btn-secondary">Secondary</button>

<!-- Cards -->
<div class="card">...</div>
<div class="card card-active">...</div>

<!-- Badges -->
<span class="badge badge-success">Success</span>

<!-- Layout -->
<div class="container">...</div>
<div class="flex gap-4 items-center">...</div>
```

---

## Resources

### Design Files
- Figma: [Link to Figma file]
- Logo Assets: `/design-system/assets/logo/`
- Icons: `/design-system/assets/icons/`

### Code
- Design Tokens: `/design-system/tokens.css`
- Components: `/design-system/components.css`

### Fonts
- [Space Grotesk](https://fonts.google.com/specimen/Space+Grotesk)
- [Inter](https://fonts.google.com/specimen/Inter)
- [JetBrains Mono](https://www.jetbrains.com/lp/mono/)

### Inspiration
- Cyberpunk aesthetics
- Glassmorphism UI
- Solana ecosystem design
- Privacy-first applications

---

**Last Updated:** November 2025
**Maintained by:** ZyberLink Design Team
**Questions?** Open an issue or discussion on GitHub
