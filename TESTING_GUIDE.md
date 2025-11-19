# Testing Guide - Navigation System

## Quick Start

```bash
cd webapp
npm run dev
```

Open `http://localhost:5173` in your browser.

---

## Test Scenarios

### 1. Global Navigation (Right Side Dots)

#### Visual Tests
- [ ] **Default State**: 5 dots visible on right side with vertical line
- [ ] **Icons**: Each dot shows correct emoji (⚡📊🔄📜📡)
- [ ] **Connection Line**: Vertical gradient line connects all dots

#### Interaction Tests
- [ ] **Hover**: Dot scales up, border glows cyan, label appears to the left
- [ ] **Click**: Smooth scroll to corresponding section
- [ ] **Active State**: Current section's dot is cyan solid with pulse animation
- [ ] **Auto-update**: Active dot changes as you scroll through sections

#### Section Mapping
```
⚡ INIT      → Hero section (logo + boot sequence)
📊 STATS     → Stats banner (metrics)
🔄 FLOW      → How It Works section
📜 CHRONICLE → Timeline section
📡 INFO      → Footer
```

#### Expected Behavior
1. Page loads → Hero dot (⚡) is active
2. Scroll down → Stats dot (📊) becomes active
3. Continue scrolling → Flow dot (🔄) becomes active
4. Click on Chronicle dot (📜) → Smooth scroll to Timeline
5. Timeline comes into view → Chronicle dot becomes active

---

### 2. Timeline Navigation (Internal Controls)

#### Visual Tests
- [ ] **Progress Bar**: Horizontal bar at top shows current position
- [ ] **Glow Marker**: Pulsing circle moves along progress bar
- [ ] **Navigation Dots**: 5 year-based dots (1977, 1993, 2009, 2014, 2024)
- [ ] **Year Labels**: Each dot shows its year below
- [ ] **Color Coding**: Cyan (1977), Violet (1993), Green (2009), Violet (2014), Cyan (2024)
- [ ] **Arrow Controls**: Left/Right arrows with counter (X/5)
- [ ] **Keyboard Hint**: Text "< USE_ARROWS_TO_NAVIGATE >" visible

#### Interaction Tests

**Mouse Navigation:**
- [ ] **Click on year dot**: Timeline scrolls horizontally to that node
- [ ] **Hover on dot**: Title tooltip appears above ("The First Lock", etc)
- [ ] **Hover on dot**: Glow aura appears, icon brightens
- [ ] **Active dot**: Scales larger, solid color, pulse animation

**Keyboard Navigation:**
- [ ] **Press ← (Left Arrow)**: Navigate to previous timeline node
- [ ] **Press → (Right Arrow)**: Navigate to next timeline node
- [ ] **At first node**: Left arrow does nothing
- [ ] **At last node**: Right arrow does nothing

**Arrow Button Navigation:**
- [ ] **Click [←] button**: Go to previous node (if not at first)
- [ ] **Click [→] button**: Go to next node (if not at last)
- [ ] **At boundaries**: Arrow button is disabled (opacity 0.3)
- [ ] **Counter updates**: Shows "1/5", "2/5", etc as you navigate

#### Progress Bar Behavior
- [ ] **Node 1 (1977)**: Progress bar 0% filled
- [ ] **Node 2 (1993)**: Progress bar 25% filled
- [ ] **Node 3 (2009)**: Progress bar 50% filled
- [ ] **Node 4 (2014)**: Progress bar 75% filled
- [ ] **Node 5 (2024)**: Progress bar 100% filled
- [ ] **Glow marker**: Moves smoothly along with progress

#### Timeline Cards
- [ ] **Scroll sync**: Active node in navigation matches centered card
- [ ] **Manual scroll**: Scrolling timeline horizontally updates navigation
- [ ] **Click expand**: Node cards expand to show full description

---

### 3. Responsive Testing

#### Desktop (>1366px)
- [ ] Global nav visible on right side
- [ ] All labels appear on hover
- [ ] Timeline navigation shows all controls
- [ ] Dots are 40px size

#### Tablet (768px - 1366px)
- [ ] Global nav visible but smaller (32px dots)
- [ ] Labels hidden, only icons
- [ ] Timeline navigation adapted
- [ ] Touch targets are adequate (44px+)

#### Mobile (<768px)
- [ ] Global nav completely hidden
- [ ] Timeline navigation shows arrows + dots
- [ ] Keyboard hint hidden
- [ ] Year labels smaller
- [ ] Touch-friendly (no hover states required)

#### Mobile Landscape (<500px height)
- [ ] Global nav hidden (not enough vertical space)
- [ ] Timeline navigation functional
- [ ] Scrolling works smoothly

---

### 4. Accessibility Testing

#### Keyboard Navigation
- [ ] **Tab**: Focus moves through navigation elements
- [ ] **Enter/Space**: Activate focused element
- [ ] **Arrow keys**: Navigate timeline when focused
- [ ] **Focus indicators**: Visible outline on focused elements

#### Screen Reader
- [ ] **ARIA labels**: Each button has descriptive label
  - "Navigate to INIT"
  - "Go to 1977"
  - "Previous node"
  - "Next node"
- [ ] **Semantic HTML**: Proper use of `<nav>`, `<button>`, `<section>`

#### Color Contrast
- [ ] **Labels**: Cyan text on dark background meets WCAG AA
- [ ] **Year labels**: Gray text readable
- [ ] **Active states**: High contrast, clearly visible

#### Touch Targets
- [ ] **All interactive elements**: Minimum 44x44px effective area
- [ ] **Global nav dots**: 40px + padding
- [ ] **Timeline dots**: 40px + padding
- [ ] **Arrow buttons**: 48px (desktop), 40px (mobile)

---

### 5. Performance Testing

#### Smooth Animations
- [ ] **Hover transitions**: No jank, smooth 60fps
- [ ] **Scroll behavior**: Smooth, not janky
- [ ] **Pulse animations**: Consistent 60fps
- [ ] **No layout shift**: Elements don't jump on interaction

#### Intersection Observer
- [ ] **Efficient**: No performance impact during scroll
- [ ] **Accurate**: Detects correct active section
- [ ] **Throttled**: Updates smoothly without lag

#### Bundle Size
- [ ] Check bundle size impact:
  ```bash
  npm run build
  ls -lh dist/assets/*.js
  ```
- [ ] GlobalNavigation + TimelineNavigation should add ~19KB total

---

### 6. Cross-Browser Testing

Test in:
- [ ] Chrome/Edge (latest)
- [ ] Firefox (latest)
- [ ] Safari (latest)
- [ ] Mobile Safari (iOS)
- [ ] Chrome Mobile (Android)

Check for:
- [ ] Intersection Observer support (should work in all modern browsers)
- [ ] Smooth scroll behavior
- [ ] CSS backdrop-filter (graceful degradation)
- [ ] Touch events on mobile

---

### 7. Edge Cases

#### Global Navigation
- [ ] **Rapid scrolling**: Active state updates correctly
- [ ] **Manual scroll vs click**: Both methods work consistently
- [ ] **Section at page top**: Correct dot activates
- [ ] **Section at page bottom**: Footer dot activates
- [ ] **Narrow viewport**: Navigation adapts or hides

#### Timeline Navigation
- [ ] **Rapid clicking**: No broken state, smooth transitions
- [ ] **Keyboard spam**: Arrow keys handle repeated presses gracefully
- [ ] **Progress bar edges**: Handles 0% and 100% correctly
- [ ] **Node expansion**: Navigation still functional when card expanded
- [ ] **Browser back/forward**: Navigation state recovers

---

## Bug Reporting Template

If you find issues, document them like this:

```
ISSUE: [Brief description]

STEPS TO REPRODUCE:
1. Action 1
2. Action 2
3. Action 3

EXPECTED:
What should happen

ACTUAL:
What actually happened

BROWSER: Chrome 120
VIEWPORT: 1920x1080
SCREENSHOT: [if applicable]
```

---

## Performance Metrics to Check

Open Chrome DevTools → Performance tab:

### Target Metrics
- **FPS**: Should stay at 60fps during animations
- **Scripting**: <5ms per frame
- **Rendering**: <10ms per frame
- **Painting**: <5ms per frame

### Memory
- **Initial load**: ~2MB additional for navigation components
- **After interactions**: No memory leaks
- **Observer cleanup**: Properly disconnected on unmount

### Network
- **GlobalNavigation.svelte**: ~7KB
- **TimelineNavigation.svelte**: ~12KB
- **Total added**: ~19KB to bundle

---

## Visual Regression Checklist

Compare with design specs:

### Global Navigation
- [ ] Dot size: 40px (desktop), 32px (mobile)
- [ ] Gap between dots: 24px (desktop), 16px (mobile)
- [ ] Connection line: 2px width, gradient
- [ ] Hover scale: 1.15x
- [ ] Active scale: 1.2x
- [ ] Label padding: 8px 12px
- [ ] Colors match design tokens (cyan #06b6d4)

### Timeline Navigation
- [ ] Progress bar height: 4px
- [ ] Dot size: 40px (desktop), 32px (mobile)
- [ ] Glow marker: 12px diameter
- [ ] Arrow buttons: 48px (desktop), 40px (mobile)
- [ ] Year label font-size: 14px
- [ ] Title tooltip: 12px font
- [ ] Colors: Cyan/Violet/Green per era

---

## Automated Testing Commands

```bash
# Run dev server
npm run dev

# Build production
npm run build

# Check bundle size
npm run build && du -sh dist/

# Lighthouse audit (if configured)
npm run lighthouse

# Type checking
npm run check
```

---

## Known Limitations

1. **Intersection Observer**: Requires modern browser (all major browsers since 2019)
2. **Smooth scroll**: May have browser-specific behavior differences
3. **Backdrop-filter**: Limited support in older browsers (graceful degradation)
4. **Mobile landscape**: Global nav hidden if height <500px (by design)

---

## Success Criteria

Navigation system is production-ready when:

- [ ] All visual states render correctly
- [ ] All interactions work smoothly (click, hover, keyboard)
- [ ] Responsive behavior works on all breakpoints
- [ ] Accessibility requirements met (WCAG AA)
- [ ] Performance targets achieved (60fps, <20KB)
- [ ] No console errors
- [ ] Works in all target browsers
- [ ] Edge cases handled gracefully

---

## Quick Debug Commands

```javascript
// Check if GlobalNavigation is mounted
document.querySelector('.global-nav')

// Check active section
document.querySelector('.global-nav .nav-dot.active')

// Check timeline active node
document.querySelector('.nav-node.active')

// Monitor Intersection Observer
// (Check console for activeSection updates)

// Force scroll to section
document.getElementById('timeline').scrollIntoView({ behavior: 'smooth' })
```

---

## Feedback Collection

While testing, note:
1. **UX Pain Points**: Anything confusing or unexpected
2. **Visual Issues**: Misalignments, color problems, animation glitches
3. **Performance Issues**: Jank, lag, slow responses
4. **Accessibility Issues**: Keyboard nav problems, contrast issues
5. **Suggestions**: Ideas for improvement

Happy testing!
