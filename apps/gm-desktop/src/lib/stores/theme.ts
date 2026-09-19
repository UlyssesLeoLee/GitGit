/**
 * Theme store. Three modes: `light`, `dark`, `auto` (system).
 * The actual rendered palette comes from Tailwind's `darkMode:
 * 'class'` machinery, driven by the `data-theme` attribute on
 * `<html>` (wired up in `App.svelte`).
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

function applyToDocument(mode: ThemeMode): void {
  if (typeof document === 'undefined') return;
  document.documentElement.dataset.theme = mode;
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
