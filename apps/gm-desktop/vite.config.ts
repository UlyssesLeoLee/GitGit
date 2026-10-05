import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { svelteTesting } from '@testing-library/svelte/vite';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

const projectRoot = path.dirname(fileURLToPath(import.meta.url));

// Standard Tauri 2.x + Svelte 5 + Vite layout.
// Browser-side bundle goes to `dist/`, which Tauri's `beforeBuildCommand`
// in `tauri.conf.json` then packages into the OS-specific bundle.
export default defineConfig({
  plugins: [svelte(), svelteTesting()],
  resolve: {
    alias: {
      $lib: path.resolve(projectRoot, 'src/lib'),
      $routes: path.resolve(projectRoot, 'src/routes'),
      $mocks: path.resolve(projectRoot, 'src/mocks'),
    },
    // `svelteTesting()` adds the `browser` condition and the `ssr.noExternal`
    // rule for `@testing-library/svelte`, but its own condition splice only
    // fires when a `node` condition is already present, which it never is
    // here. Setting the condition at the top level of `resolve` (gated on
    // VITEST so the production build keeps its own defaults) is what makes
    // `svelte` resolve to the client build instead of `index-server.js`.
    ...(process.env.VITEST ? { conditions: ['browser'] } : {}),
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
        // `[FACT]` These numbers are NOT enforced anywhere today, and an
        // earlier version of this comment claimed they were "a soft
        // warning". Neither was true. Two separate facts:
        //
        //   1. `pnpm test` is `vitest run`, without `--coverage`, and that
        //      is what `.github/workflows/gm-desktop.yml` calls. Coverage is
        //      therefore never measured in CI, so these thresholds are
        //      never evaluated at all. A threshold in a config file that no
        //      job reads is the appearance of a gate, not a gate.
        //   2. If `pnpm test:coverage` were run, it would exit non-zero:
        //      vitest fails a run that misses a threshold. Measured
        //      2026-10-05: 37.71% lines / 50% functions at the base commit,
        //      45.06% / 53.19% after the T9 review UI landed.
        //
        // So the thresholds stay where they are — lowering them to match
        // today's numbers would destroy the only statement of intent this
        // file makes — and the gap is tracked as real work: raise actual
        // coverage, then make CI run `test:coverage` once it can pass.
        lines: 70,
        functions: 60,
        statements: 70,
        branches: 55,
      },
    },
  },
});
