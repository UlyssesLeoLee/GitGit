/**
 * Theme store. Three modes: `light`, `dark`, `auto` (system).
 * The actual rendered palette comes from Tailwind's `darkMode:
 * 'class'` machinery, driven by the `data-theme` attribute on the
 * document element.
 *
 * `[FACT]` This file writes that attribute itself, via the subscription
 * below — not `App.svelte`. An older comment here claimed the wiring
 * was "in `App.svelte`", which was never true: the `App.svelte` attempt
 * used `<svelte:head><html data-theme=...>`, and the HTML parser drops a
 * nested `<html>` start tag outright, so the value never arrived and the
 * component crashed on mount instead.
 *
 * Persistence key: `gm-desktop.theme`.
 */

import { writable } from 'svelte/store';

export type ThemeMode = 'light' | 'dark' | 'auto';

const STORAGE_KEY = 'gm-desktop.theme';

function readPersisted(): ThemeMode | null {
  try {
    const v = localStorage.getItem(STORAGE_KEY);
    if (v === 'light' || v === 'dark' || v === 'auto') return v;
  } catch (_err) {
    // ignore — fall through
  }
  return null;
}

export const theme = writable<ThemeMode>(readPersisted() ?? 'auto');

/**
 * Resolve the effective palette to apply via Tailwind. For `auto`
 * we mirror the system `(prefers-color-scheme: dark)` media query
 * via `matchMedia`.
 */
export function isEffectivelyDark(mode: ThemeMode): boolean {
  if (mode === 'dark') return true;
  if (mode === 'light') return false;
  if (typeof window === 'undefined' || !window.matchMedia) return false;
  return window.matchMedia('(prefers-color-scheme: dark)').matches;
}

function applyToDocument(mode: ThemeMode): void {
  if (typeof document === 'undefined') return;
  // `[FACT]` Write the *resolved* palette, not the raw mode. Tailwind's
  // `darkMode` selector list in `tailwind.config.js` is
  // `['class', '[data-theme="dark"]', '[data-theme="auto"]']` — `auto` is
  // listed as a dark selector. Writing the mode string straight through
  // therefore made `auto` render the dark palette even on a light
  // system, and the `isEffectivelyDark()` helper that exists to settle
  // exactly that question was never called on this path.
  document.documentElement.dataset.theme = isEffectivelyDark(mode) ? 'dark' : 'light';
}

theme.subscribe((mode) => {
  applyToDocument(mode);
});

export function initTheme(): void {
  // Subscribe-side effect already applied on module load; nothing
  // else to do here. Reserved for future async bootstrap (e.g.
  // reading theme from the user-prefs vault).
}

export function setTheme(next: ThemeMode): void {
  theme.set(next);
  try {
    localStorage.setItem(STORAGE_KEY, next);
  } catch (_err) {
    // ignore
  }
}
