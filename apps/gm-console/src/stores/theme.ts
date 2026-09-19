import { create } from 'zustand';

export type Theme = 'light' | 'dark' | 'system';

interface ThemeState {
  theme: Theme;
  /** The *effective* theme (always 'light' or 'dark' — what the DOM class reflects). */
  effective: 'light' | 'dark';
  setTheme: (next: Theme) => void;
  /** Re-evaluate the `system` arm against current prefers-color-scheme. */
  syncSystem: () => void;
  hydrate: () => void;
}

const STORAGE_KEY = 'gm.theme';

function readStored(): Theme {
  if (typeof localStorage === 'undefined') return 'system';
  const raw = localStorage.getItem(STORAGE_KEY);
  if (raw === 'light' || raw === 'dark' || raw === 'system') return raw;
  return 'system';
}

function writeStored(value: Theme): void {
  if (typeof localStorage === 'undefined') return;
  try {
    localStorage.setItem(STORAGE_KEY, value);
  } catch {
    /* storage may be unavailable */
  }
}

function systemPrefersDark(): boolean {
  if (typeof window === 'undefined' || !window.matchMedia) return false;
  return window.matchMedia('(prefers-color-scheme: dark)').matches;
}

function effectiveFrom(theme: Theme): 'light' | 'dark' {
  return theme === 'system' ? (systemPrefersDark() ? 'dark' : 'light') : theme;
}

function applyToDom(effective: 'light' | 'dark'): void {
  if (typeof document === 'undefined') return;
  document.documentElement.classList.toggle('dark', effective === 'dark');
}

/**
 * Theme store. The "system" arm follows OS prefers-color-scheme; the
 * light/dark arms are explicit user choices that win until cleared.
 */
export const useThemeStore = create<ThemeState>((set, get) => ({
  theme: 'system',
  effective: 'light',
  setTheme(next) {
    writeStored(next);
    const effective = effectiveFrom(next);
    applyToDom(effective);
    set({ theme: next, effective });
  },
  syncSystem() {
    const cur = get().theme;
    if (cur !== 'system') return;
    const effective = effectiveFrom(cur);
    applyToDom(effective);
    set({ effective });
  },
  hydrate() {
    const theme = readStored();
    const effective = effectiveFrom(theme);
    applyToDom(effective);
    set({ theme, effective });
  },
}));