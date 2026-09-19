import sveltePreprocess from 'svelte-preprocess';

export default {
  // Svelte 5 with the official `vite-plugin-svelte` driver. The
  // preprocess step allows TypeScript to be embedded in component
  // `<script lang="ts">` blocks — we keep `strict: true` in
  // `tsconfig.json` so the TypeScript pass fails the build on real
  // errors.
  preprocess: sveltePreprocess({
    typescript: true,
    postcss: true,
  }),
  // Compiler options from ADR-0020: Svelte 5 runes-friendly.
  compilerOptions: {
    runes: true,
    dev: process.env.NODE_ENV !== 'production',
  },
};
