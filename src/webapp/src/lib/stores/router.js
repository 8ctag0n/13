import { writable, derived } from 'svelte/store';

// Whitelist of valid routes (security: prevent arbitrary hash navigation)
const VALID_ROUTES = ['landing', 'dashboard', 'create-job', 'metrics', 'my-jobs'];

// Pattern routes (routes with parameters)
const PATTERN_ROUTES = [
  { pattern: /^job-(\d+)$/, name: 'job-details' }
];

// Validate route against whitelist or patterns
function validateRoute(route) {
  // Check static routes
  if (VALID_ROUTES.includes(route)) return route;

  // Check pattern routes
  for (const patternRoute of PATTERN_ROUTES) {
    if (patternRoute.pattern.test(route)) {
      return route; // Return the full route with parameter
    }
  }

  return 'landing';
}

// Extract route params (e.g., job-123 -> { jobId: '123' })
export function getRouteParams(route) {
  for (const patternRoute of PATTERN_ROUTES) {
    const match = route.match(patternRoute.pattern);
    if (match) {
      if (patternRoute.name === 'job-details') {
        return { jobId: match[1] };
      }
    }
  }
  return {};
}

// Check if route matches a pattern
export function isPatternRoute(route, patternName) {
  for (const patternRoute of PATTERN_ROUTES) {
    if (patternRoute.name === patternName && patternRoute.pattern.test(route)) {
      return true;
    }
  }
  return false;
}

// Get initial route from hash or default to landing
function getInitialRoute() {
  if (typeof window === 'undefined') return 'landing';
  const hash = window.location.hash.slice(1); // Remove #
  return validateRoute(hash || 'landing');
}

// Router store with hash-based navigation support
export const currentRoute = writable(getInitialRoute());

export function navigateTo(route) {
  // Validate route for security
  const validatedRoute = validateRoute(route);

  // Update both the store and the URL hash
  currentRoute.set(validatedRoute);

  if (typeof window !== 'undefined') {
    // Update URL hash without triggering page reload
    window.location.hash = validatedRoute;
  }
}

// Initialize hash-based routing listener (call this in main app)
export function initHashRouter() {
  if (typeof window === 'undefined') return;

  // Handle hash changes from browser back/forward buttons
  window.addEventListener('hashchange', () => {
    const hash = window.location.hash.slice(1);
    const route = validateRoute(hash || 'landing');
    currentRoute.set(route);
  });

  // Set initial route from hash
  const initialHash = window.location.hash.slice(1);
  if (initialHash) {
    currentRoute.set(validateRoute(initialHash));
  }

  // Expose navigation API to window for E2E tests
  if (typeof window !== 'undefined') {
    window.navigateTo = navigateTo;
    window.currentRoute = currentRoute;
  }
}
