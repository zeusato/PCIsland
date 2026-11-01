import { defineConfig } from 'vitest/config';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { fileURLToPath, URL } from 'node:url';

const host = process.env.TAURI_DEV_HOST;

// One HTML entry per Tauri window (docs/02-system-design.md §7).
export default defineConfig({
  plugins: [svelte()],
  resolve: {
    conditions: process.env.VITEST ? ['browser'] : undefined,
    alias: { $lib: fileURLToPath(new URL('./src/lib', import.meta.url)) },
  },
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host ? { protocol: 'ws', host, port: 1421 } : undefined,
    watch: { ignored: ['**/src-tauri/**'] },
  },
  build: {
    target: 'es2022',
    rollupOptions: {
      input: {
        island: fileURLToPath(new URL('./island.html', import.meta.url)),
        main: fileURLToPath(new URL('./main.html', import.meta.url)),
        'web-player-toolbar': fileURLToPath(new URL('./web-player-toolbar.html', import.meta.url)),
        video: fileURLToPath(new URL('./video.html', import.meta.url)),
        'note-desktop': fileURLToPath(new URL('./note-desktop.html', import.meta.url)),
      },
    },
  },
  test: {
    environment: 'jsdom',
    include: ['src/**/*.test.ts'],
  },
});
