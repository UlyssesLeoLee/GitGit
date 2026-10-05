/**
 * Store tests for `src/lib/stores/theme.ts`.
 *
 * This store was 0% covered (lines 14-61). It is the only store in the
 * app whose initial value is read from `localStorage` at *module
 * evaluation* time, so every case here has to control what is in
 * storage before the module is imported. `loadTheme()` does that with
 * `vi.resetModules()` plus a dynamic import, which yields a fresh
 * module — and therefore a fresh store — per case. Importing it once
 * at the top of the file would make the first case's persisted value
 * leak into every case after it.
 *
 * The cases concentrate on the two failure paths that are invisible in
 * a normal run: a storage that cannot be read and one that cannot be
 * written. Both are guarded with `try/catch` in the source, and a
 * theme the user cannot change is worse than an unpersisted one.
 */
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { get } from 'svelte/store';

const STORAGE_KEY = 'gm-desktop.theme';

/** Import a fresh copy of the module, i.e. re-run its initializer. */
async function loadTheme() {
  vi.resetModules();
  return await import('../../src/lib/stores/theme');
}

/** A `matchMedia` stand-in, which jsdom does not provide. */
function stubMatchMedia(matches: boolean) {
  const fn = vi.fn((media: string) => ({
    matches,
    media,
    onchange: null,
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    addListener: vi.fn(),
    removeListener: vi.fn(),
    dispatchEvent: vi.fn(),
  }));
  Object.defineProperty(window, 'matchMedia', { value: fn, configurable: true, writable: true });
  return fn;
}

beforeEach(() => {
  localStorage.clear();
  delete document.documentElement.dataset.theme;
});

afterEach(() => {
  vi.restoreAllMocks();
  // jsdom ships no matchMedia, so the cases that stub it have to take
  // it back out or `auto` would keep answering from a stale stub.
  delete (window as unknown as Record<string, unknown>)['matchMedia'];
});

describe('store / theme — the persisted initial value', () => {
  it('starts on auto when nothing has been persisted', async () => {
    const { theme } = await loadTheme();
    expect(get(theme)).toBe('auto');
  });

  it('restores a persisted mode instead of falling back to auto', async () => {
    // The point of persistence: a user who chose dark gets dark on the
    // next launch, not a one-frame flash of the default.
    localStorage.setItem(STORAGE_KEY, 'dark');
    const { theme } = await loadTheme();
    expect(get(theme)).toBe('dark');
  });

  it('restores light just as faithfully', async () => {
    localStorage.setItem(STORAGE_KEY, 'light');
    const { theme } = await loadTheme();
    expect(get(theme)).toBe('light');
  });

  it('ignores a persisted value that is not one of the three modes', async () => {
    // A stale or hand-edited value must not put the store into a mode
    // the rest of the app has no branch for.
    localStorage.setItem(STORAGE_KEY, 'sepia');
    const { theme } = await loadTheme();
    expect(get(theme)).toBe('auto');
  });

  it('reads storage under the documented key', async () => {
    // The key is the on-disk contract with the previous launch. A
    // rename here silently discards every existing user's choice.
    localStorage.setItem('gm-desktop.theme', 'dark');
    const { theme } = await loadTheme();
    expect(get(theme)).toBe('dark');
    expect(localStorage.getItem('gm-desktop.theme')).toBe('dark');
  });

  it('starts on auto when storage cannot be read at all', async () => {
    vi.spyOn(Storage.prototype, 'getItem').mockImplementation(() => {
      throw new Error('storage is blocked by policy');
    });
    const { theme } = await loadTheme();
    // A read failure degrades to the default rather than crashing the
    // app before it has rendered anything.
    expect(get(theme)).toBe('auto');
  });

  it('applies the restored mode to the document on load', async () => {
    // Tailwind's `darkMode: 'class'` machinery reads this attribute, so
    // the store has to push it as a side effect, not only hold the value.
    localStorage.setItem(STORAGE_KEY, 'dark');
    await loadTheme();
    expect(document.documentElement.dataset.theme).toBe('dark');
  });
});

describe('store / theme — setTheme', () => {
  it('changes the store and the document attribute together', async () => {
    const { theme, setTheme } = await loadTheme();
    setTheme('light');
    expect(get(theme)).toBe('light');
    expect(document.documentElement.dataset.theme).toBe('light');
  });

  it('persists the new mode', async () => {
    const { setTheme } = await loadTheme();
    setTheme('dark');
    expect(localStorage.getItem(STORAGE_KEY)).toBe('dark');
  });

  it('persists each of the three modes verbatim', async () => {
    const { setTheme } = await loadTheme();
    for (const mode of ['light', 'dark', 'auto'] as const) {
      setTheme(mode);
      expect(localStorage.getItem(STORAGE_KEY)).toBe(mode);
    }
  });

  it('still switches when storage refuses the write', async () => {
    // `[FACT]` Measured behaviour, and the behaviour the `try/catch`
    // exists for: the in-memory store is updated first, so a
    // read-only profile still gets a working theme toggle — it just
    // does not survive a restart.
    vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => {
      throw new Error('quota exceeded');
    });
    const { theme, setTheme } = await loadTheme();
    setTheme('dark');
    expect(get(theme)).toBe('dark');
    expect(document.documentElement.dataset.theme).toBe('dark');
  });

  it('leaves an explicit choice alone when the app bootstraps', async () => {
    // `initTheme` is a reserved no-op today. What matters is that
    // bootstrap does not reset a choice the user has already made.
    const { theme, setTheme, initTheme } = await loadTheme();
    setTheme('dark');
    initTheme();
    expect(get(theme)).toBe('dark');
  });
});

describe('store / theme — isEffectivelyDark', () => {
  it('is true for dark without asking the system', async () => {
    const { isEffectivelyDark } = await loadTheme();
    const matchMedia = stubMatchMedia(false);
    expect(isEffectivelyDark('dark')).toBe(true);
    // An explicit choice must not be overridden by the OS setting.
    expect(matchMedia).not.toHaveBeenCalled();
  });

  it('is false for light without asking the system', async () => {
    const { isEffectivelyDark } = await loadTheme();
    const matchMedia = stubMatchMedia(true);
    expect(isEffectivelyDark('light')).toBe(false);
    expect(matchMedia).not.toHaveBeenCalled();
  });

  it('mirrors the system preference for auto', async () => {
    const { isEffectivelyDark } = await loadTheme();
    stubMatchMedia(true);
    expect(isEffectivelyDark('auto')).toBe(true);
  });

  it('reports light for auto when the system prefers light', async () => {
    const { isEffectivelyDark } = await loadTheme();
    stubMatchMedia(false);
    expect(isEffectivelyDark('auto')).toBe(false);
  });

  it('asks the system about the colour-scheme query specifically', async () => {
    const { isEffectivelyDark } = await loadTheme();
    const matchMedia = stubMatchMedia(true);
    isEffectivelyDark('auto');
    // A different query would answer the wrong question.
    expect(matchMedia).toHaveBeenCalledWith('(prefers-color-scheme: dark)');
  });

  it('falls back to light for auto when matchMedia is unavailable', async () => {
    // jsdom has no matchMedia, which is also true of some embedded
    // WebViews. `auto` must degrade to light rather than throw.
    const { isEffectivelyDark } = await loadTheme();
    expect((window as unknown as Record<string, unknown>)['matchMedia']).toBeUndefined();
    expect(isEffectivelyDark('auto')).toBe(false);
  });
});
