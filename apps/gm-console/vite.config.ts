import { defineConfig, loadEnv } from 'vite';
import react from '@vitejs/plugin-react';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));

export default defineConfig(({ mode }) => {
  const env = loadEnv(mode, __dirname, '');
  const apiTarget = env.VITE_API_PROXY_TARGET ?? 'http://127.0.0.1:8080';
  const enableMocks = env.VITE_ENABLE_MOCKS ?? 'true';
  return {
    plugins: [react()],
    resolve: {
      alias: {
        '@': path.resolve(__dirname, './src'),
      },
    },
    server: {
      port: 5173,
      strictPort: false,
      host: '127.0.0.1',
      proxy: {
        '/api': {
          target: apiTarget,
          changeOrigin: true,
          secure: false,
        },
      },
    },
    preview: {
      port: 4173,
      host: '127.0.0.1',
    },
    build: {
      target: 'es2020',
      sourcemap: mode !== 'production',
      outDir: 'dist',
      emptyOutDir: true,
      chunkSizeWarningLimit: 600,
      rollupOptions: {
        output: {
          manualChunks: {
            react: ['react', 'react-dom', 'react-router-dom'],
            query: ['@tanstack/react-query'],
          },
        },
      },
    },
    define: {
      __ENABLE_MOCKS__: JSON.stringify(enableMocks === 'true'),
      __APP_VERSION__: JSON.stringify(env.npm_package_version ?? '0.1.0'),
    },
    test: {
      globals: true,
      environment: 'jsdom',
      setupFiles: ['./vitest.setup.ts'],
      css: false,
      coverage: {
        provider: 'v8',
        reporter: ['text', 'json', 'html'],
        include: ['src/**/*.{ts,tsx}'],
        exclude: [
          'src/**/main.tsx',
          'src/**/index.css',
          'src/**/*.d.ts',
          'src/mocks/**',
          'src/i18n/**',
        ],
        thresholds: {
          lines: 70,
          statements: 70,
          branches: 60,
          functions: 70,
        },
      },
    },
  };
});