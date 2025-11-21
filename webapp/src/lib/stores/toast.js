import { writable } from 'svelte/store';

function createToastStore() {
  const { subscribe, update } = writable([]);

  let nextId = 1;

  return {
    subscribe,
    show: (message, type = 'info', duration = 3000) => {
      const id = nextId++;
      const toast = { id, message, type, duration };

      update(toasts => [...toasts, toast]);

      // Auto-remove after duration
      if (duration > 0) {
        setTimeout(() => {
          update(toasts => toasts.filter(t => t.id !== id));
        }, duration);
      }

      return id;
    },
    remove: (id) => {
      update(toasts => toasts.filter(t => t.id !== id));
    },
    clear: () => {
      update(() => []);
    },
    // Helper methods
    success: (message, duration) => {
      return createToastStore().show(message, 'success', duration);
    },
    error: (message, duration) => {
      return createToastStore().show(message, 'error', duration);
    },
    warning: (message, duration) => {
      return createToastStore().show(message, 'warning', duration);
    },
    info: (message, duration) => {
      return createToastStore().show(message, 'info', duration);
    }
  };
}

export const toastStore = createToastStore();
