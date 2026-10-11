import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import tailwindcss from '@tailwindcss/vite';
import { resolve } from 'node:path';
export default defineConfig({
  root: 'ui', clearScreen: false,
  plugins: [react(), tailwindcss()],
  server: { port: 5173, strictPort: true },
  build: { outDir: '../dist', emptyOutDir: true, target: 'es2022', rollupOptions: { input: {
    overlay: resolve(__dirname, 'ui/overlay.html'), island: resolve(__dirname, 'ui/island.html'),
    panel: resolve(__dirname, 'ui/panel.html'), dashboard: resolve(__dirname, 'ui/dashboard.html') } } },
  test: { root: '.', include: ['ui/src/**/*.test.ts'] },
});
