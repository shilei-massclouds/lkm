import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

export default defineConfig({
  plugins: [svelte()],
  build: {
    outDir: process.env.LKM_ANIMATE_OUT_DIR || 'dist',
    emptyOutDir: true,
    cssCodeSplit: false,
    lib: {
      entry: 'src/main.ts',
      name: 'LkmSignalPlayer',
      formats: ['iife'],
      fileName: () => 'player.js'
    },
    rollupOptions: {
      output: {
        assetFileNames: (assetInfo) => assetInfo.names?.some((name) => name.endsWith('.css'))
          ? 'player.css'
          : '[name][extname]'
      }
    }
  }
});
