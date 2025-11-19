import { writable } from 'svelte/store';

// Simple router store
export const currentRoute = writable('landing');

export function navigateTo(route) {
  currentRoute.set(route);
}
