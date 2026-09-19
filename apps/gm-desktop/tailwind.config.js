/** @type {import('tailwindcss').Config} */
export default {
  content: ['./index.html', './src/**/*.{svelte,ts}'],
  darkMode: ['class', '[data-theme="dark"]', '[data-theme="auto"]'],
  theme: {
    extend: {
      colors: {
        // Accent: gitgit's own brand seed (`#22c55e` — slipstream
        // green). The hover / active states follow the same scale.
        accent: {
          50: '#f0fdf4',
          100: '#dcfce7',
          500: '#22c55e',
          600: '#16a34a',
          700: '#15803d',
        },
      },
      fontFamily: {
        // System stack — avoids bundling web fonts into the desktop
        // shell (Tauri's WebView already has access to the OS fonts).
        sans: ['system-ui', 'Segoe UI', 'PingFang SC', 'Microsoft YaHei', 'sans-serif'],
        mono: ['ui-monospace', 'Cascadia Code', 'Consolas', 'monospace'],
      },
    },
  },
  plugins: [],
};
