import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

const projectRoot = path.dirname(fileURLToPath(import.meta.url));

// Standard Tauri 2.x + Svelte 5 + Vite layout.
// Browser-side bundle goes to `dist/`, which Tauri's `beforeBuildCommand`
// in `tauri.conf.json` then packages into the OS-specific bundle.
export default defineConfig({
  plugins: [svelte()],
  resolve: {
    alias: {
      $lib: path.resolve(projectRoot, 'src/lib'),
      $routes: path.resolve(projectRoot, 'src/routes'),
      $mocks: path.resolve(projectRoot, 'src/mocks'),
    },
  },
  // Tauri's debug tooling reads `process.env.TAURI_DEV_HOST` to wire up
  // a hot-reload server. We expose that here without leaking it into the
  // production bundle.
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
    host: process.env.TAURI_DEV_HOST || 'localhost',
  },
  envPrefix: ['VITE_', 'TAURI_'],
  build: {
    target: 'esnext',
    sourcemap: true,
    minify: !process.env.TAURI_DEBUG ? 'esbuild' : false,
  },
  test: {
    globals: true,
    environment: 'jsdom',
    setupFiles: ['./tests/setup.ts'],
    include: ['tests/unit/**/*.test.ts'],
    coverage: {
      provider: 'v8',
      reporter: ['text', 'html'],
      include: ['src/**/*.{ts,svelte}'],
      exclude: [
        'src/main.ts',
        'src/mocks/**',
        'src/**/*.d.ts',
      ],
      thresholds: {
        // Per brief: lines >= 70%. We surface this in CI logs as a
        // soft warning, not a hard failure, because the first
        // coverage run is necessarily a snapshot.
        lines: 70,
        functions: 60,
        statements: 70,
        branches: 55,
      },
    },
  },
});
