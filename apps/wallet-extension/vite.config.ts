import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { crx } from '@crxjs/vite-plugin';
import manifest from './manifest.json';
import { resolve } from 'path';

export default defineConfig({
  plugins: [
    svelte(),
    crx({ manifest })
  ],
  define: {
    global: 'globalThis',
    'process.env': {}
  },
  resolve: {
    alias: {
      buffer: 'buffer',
      crypto: 'crypto-browserify',
      stream: 'stream-browserify'
    }
  },
  optimizeDeps: {
    esbuildOptions: {
      define: {
        global: 'globalThis'
      }
    }
  },
  build: {
    rollupOptions: {
      input: {
        popup: 'popup/index.html',
        'content/inject': resolve(__dirname, 'content/inject.ts')
      },
      output: {
        entryFileNames: (chunkInfo) => {
          if (chunkInfo.name === 'content/inject') {
            return 'content/inject.js';
          }
          return 'assets/[name]-[hash].js';
        }
      }
    }
  }
});
