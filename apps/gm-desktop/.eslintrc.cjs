/**
 * ESLint flat config (v9+). We keep the rule surface intentionally
 * narrow — TypeScript-strict does most of the lifting; ESLint only
 * adds style / unused-import / no-shadow rules so the Svelte 5
 * `runes` syntax isn't accidentally lint-flagged.
 */
module.exports = {
  root: true,
  env: { browser: true, es2022: true, node: true },
  parser: '@typescript-eslint/parser',
  parserOptions: {
    sourceType: 'module',
    ecmaVersion: 2022,
  },
  plugins: ['@typescript-eslint'],
  extends: [
    'eslint:recommended',
    'plugin:@typescript-eslint/recommended',
  ],
  ignorePatterns: ['dist/', 'node_modules/', 'src-tauri/target/', 'src-tauri/gen/'],
  rules: {
    '@typescript-eslint/no-explicit-any': 'warn',
    '@typescript-eslint/no-unused-vars': ['warn', { 'argsIgnorePattern': '^_' }],
    '@typescript-eslint/no-empty-function': 'off',
    'no-empty': 'off',
  },
  overrides: [
    {
      files: ['*.svelte'],
      // ESLint's Svelte support is via the experimental parser; we
      // rely on svelte-check instead and keep this layer no-op.
      parser: 'svelte-eslint-parser',
      parserOptions: {
        parser: '@typescript-eslint/parser',
        extraFileExtensions: ['.svelte'],
      },
    },
    {
      files: ['tests/**/*.ts'],
      env: { node: true },
    },
  ],
};
