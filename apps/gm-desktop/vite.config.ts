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
    // `[FACT]` `127.0.0.1`, not `localhost`, and the difference is the
    // whole reason `pnpm tauri:dev` starts at all.
    //
    // Node has resolved `localhost` verbatim (the DNS result order it
    // prefers, not a sorted one) since 17, and on Windows `::1` comes
    // back first. Vite therefore bound the IPv6 loopback only. The Tauri
    // CLI then polled `http://localhost:5173` over IPv4, got a refused
    // connection, and sat at
    //
    //     Warn Waiting for your frontend dev server to start on
    //          http://localhost:5173/...
    //
    // indefinitely — with Vite's own banner already claiming it was
    // `ready`. Nothing in CI sees this: the workflow runs `build`, which
    // never opens a socket, and `test`, which never starts the dev
    // server. It was found by running the app.
    //
    // `TAURI_DEV_HOST` still wins when set, because that is how the CLI
    // points the dev server at a LAN address for device debugging.
    host: process.env.TAURI_DEV_HOST || '127.0.0.1',
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
        // `[FACT]` These numbers are enforced. History, because the
        // distinction matters to anyone tempted to move them:
        //
        //   1. For a long time they were not. `pnpm test` is
        //      `vitest run` without `--coverage` and that is what the
        //      workflow's `Unit tests` step called, so nothing read this
        //      block at all — a threshold in a config file that no job
        //      evaluates is the appearance of a gate, not a gate. The
        //      coverage run also exited non-zero the whole time, because
        //      vitest fails a run that misses a threshold.
        //   2. The suite was brought up over 2026-10-05: the route
        //      components and the stores/api layer had zero coverage,
        //      and writing those tests took `All files` from 42.55% to
        //      94.78% lines. The workflow now runs `test:coverage` in its
        //      own `Coverage gate` step.
        //
        // Measured headroom at that point: 94.78% lines, 94.23%
        // statements, 95.17% functions, 78.43% branches, against
        // 70/70/60/55 below.
        //
        // Do not lower these to match a future number. The point of
        // declaring them was to say what the code should be held to; a
        // drop in coverage is missing tests, not a budget to reallocate.
        //
        // `[FACT]` An earlier version of this comment blamed the low
        // number on the `coverage-v8` 2 -> 5 provider change. That was
        // wrong and is retracted: the per-file report showed entire
        // route components sitting at 0%, which is real untested code,
        // not a reporting artifact.
        lines: 70,
        functions: 60,
        statements: 70,
        branches: 55,
      },
    },
  },
});
