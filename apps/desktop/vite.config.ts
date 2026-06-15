import { resolve } from 'node:path';
import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

// Tauri expects a fixed dev server port (see tauri.conf.json devUrl).
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
  },
  build: {
    target: 'safari15',
    rollupOptions: {
      input: {
        main: resolve(import.meta.dirname, 'index.html'),
        settings: resolve(import.meta.dirname, 'settings.html'),
        hud: resolve(import.meta.dirname, 'hud.html'),
        changes: resolve(import.meta.dirname, 'changes.html'),
        scratchpad: resolve(import.meta.dirname, 'scratchpad.html'),
      },
    },
  },
});
