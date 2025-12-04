import { defineConfig } from 'vitest/config';
import { svelte } from '@sveltejs/vite-plugin-svelte';

export default defineConfig({
  plugins: [svelte({ hot: !process.env.VITEST })],
  test: {
    include: ['tests/**/*.{test,spec}.{js,ts}'],
    environment: 'jsdom',
    setupFiles: ['./tests/setup.js'],
    globals: true,
    coverage: {
      reporter: ['text', 'json', 'html'],
      exclude: [
        'node_modules/',
        'tests/',
        'src-tauri/',
        '*.config.js',
        '*.config.ts',
      ],
    },
  },
  resolve: {
    alias: {
      '@': '/src',
    },
  },
});
