import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { useThemeStore } from '@/stores/theme';

const STORAGE_KEY = 'gm.theme';

/** jsdom has no matchMedia; install a controllable one. */
function stubPrefersDark(dark: boolean | null) {
  Object.defineProperty(window, 'matchMedia', {
    writable: true,
    configurable: true,
    value:
      dark === null
        ? undefined
        : (query: string) => ({
            matches: dark,
            media: query,
            onchange: null,
            addListener: vi.fn(),
            removeListener: vi.fn(),
            addEventListener: vi.fn(),
            removeEventListener: vi.fn(),
            dispatchEvent: vi.fn(),
          }),
  });
}

const hasDarkClass = () => document.documentElement.classList.contains('dark');

beforeEach(() => {
  localStorage.clear();
  document.documentElement.classList.remove('dark');
  useThemeStore.setState({ theme: 'system', effective: 'light' });
  stubPrefersDark(false);
});

afterEach(() => {
  localStorage.clear();
  document.documentElement.classList.remove('dark');
});

describe('setTheme', () => {
  it('stores an explicit dark choice and applies the DOM class', () => {
    useThemeStore.getState().setTheme('dark');
    expect(useThemeStore.getState().theme).toBe('dark');
    expect(useThemeStore.getState().effective).toBe('dark');
    expect(hasDarkClass()).toBe(true);
    expect(localStorage.getItem(STORAGE_KEY)).toBe('dark');
  });

  it('removes the DOM class when switching back to light', () => {
    useThemeStore.getState().setTheme('dark');
    useThemeStore.getState().setTheme('light');
    expect(useThemeStore.getState().effective).toBe('light');
    expect(hasDarkClass()).toBe(false);
    expect(localStorage.getItem(STORAGE_KEY)).toBe('light');
  });

  it('resolves the system arm against prefers-color-scheme: dark', () => {
    stubPrefersDark(true);
    useThemeStore.getState().setTheme('system');
    expect(useThemeStore.getState().theme).toBe('system');
    expect(useThemeStore.getState().effective).toBe('dark');
    expect(hasDarkClass()).toBe(true);
  });

  it('resolves the system arm against prefers-color-scheme: light', () => {
    stubPrefersDark(false);
    useThemeStore.getState().setTheme('system');
    expect(useThemeStore.getState().effective).toBe('light');
    expect(hasDarkClass()).toBe(false);
  });

  it('treats a missing matchMedia as light rather than throwing', () => {
    stubPrefersDark(null);
    expect(() => useThemeStore.getState().setTheme('system')).not.toThrow();
    expect(useThemeStore.getState().effective).toBe('light');
  });
});

describe('syncSystem', () => {
  it('is a no-op while an explicit theme is in force', () => {
    useThemeStore.getState().setTheme('light');
    stubPrefersDark(true);
    useThemeStore.getState().syncSystem();
    expect(useThemeStore.getState().effective).toBe('light');
    expect(hasDarkClass()).toBe(false);
  });

  it('picks up an OS theme change while the system arm is active', () => {
    stubPrefersDark(false);
    useThemeStore.getState().setTheme('system');
    expect(useThemeStore.getState().effective).toBe('light');

    stubPrefersDark(true);
    useThemeStore.getState().syncSystem();
    expect(useThemeStore.getState().effective).toBe('dark');
    expect(hasDarkClass()).toBe(true);
  });

  it('reacts to an OS theme change back to light', () => {
    stubPrefersDark(true);
    useThemeStore.getState().setTheme('system');
    stubPrefersDark(false);
    useThemeStore.getState().syncSystem();
    expect(useThemeStore.getState().effective).toBe('light');
    expect(hasDarkClass()).toBe(false);
  });
});

describe('hydrate', () => {
  it('restores a persisted dark choice', () => {
    localStorage.setItem(STORAGE_KEY, 'dark');
    useThemeStore.getState().hydrate();
    expect(useThemeStore.getState().theme).toBe('dark');
    expect(useThemeStore.getState().effective).toBe('dark');
    expect(hasDarkClass()).toBe(true);
  });

  it('restores a persisted light choice', () => {
    localStorage.setItem(STORAGE_KEY, 'light');
    useThemeStore.getState().hydrate();
    expect(useThemeStore.getState().effective).toBe('light');
    expect(hasDarkClass()).toBe(false);
  });

  it('restores a persisted system choice and resolves it', () => {
    localStorage.setItem(STORAGE_KEY, 'system');
    stubPrefersDark(true);
    useThemeStore.getState().hydrate();
    expect(useThemeStore.getState().theme).toBe('system');
    expect(useThemeStore.getState().effective).toBe('dark');
  });

  it('falls back to system when nothing is persisted', () => {
    expect(localStorage.getItem(STORAGE_KEY)).toBeNull();
    useThemeStore.getState().hydrate();
    expect(useThemeStore.getState().theme).toBe('system');
  });

  it('ignores a corrupt persisted value', () => {
    localStorage.setItem(STORAGE_KEY, 'chartreuse');
    useThemeStore.getState().hydrate();
    expect(useThemeStore.getState().theme).toBe('system');
  });
});
