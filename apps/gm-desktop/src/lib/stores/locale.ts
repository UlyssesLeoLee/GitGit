/**
 * Locale store. Defaults to `zh-CN` per the brief. We keep
 * persistence in `localStorage` so the user's preference survives
 * app restarts; the system locale is the fallback when nothing
 * has been saved yet.
 */

import { writable } from 'svelte/store';
import type { LocaleId } from '$lib/i18n';

const STORAGE_KEY = 'gm-desktop.locale';

function readPersistedLocale(): LocaleId | null {
  try {
    const v = localStorage.getItem(STORAGE_KEY);
    if (v === 'zh-CN' || v === 'en') return v;
  } catch (_err) {
    // localStorage may be unavailable in some test sandboxes; the
    // guard makes us fall through to the system-locale pick.
  }
  return null;
}

function pickInitial(): LocaleId {
  const stored = readPersistedLocale();
  if (stored) return stored;
  try {
    const sys = (navigator.language || 'zh-CN').toLowerCase();
    if (sys.startsWith('en')) return 'en';
  } catch (_err) {
    // `navigator` is undefined in a Node-only test run; fall through.
  }
  return 'zh-CN';
}

export const locale = writable<LocaleId>(pickInitial());

export async function initLocale(): Promise<void> {
  // Reserved for future async initializers (e.g. reading from a
  // backend user-pref table); the sync initializer already ran at
  // module evaluation.
}

export function setLocale(next: LocaleId): void {
  locale.set(next);
  try {
    localStorage.setItem(STORAGE_KEY, next);
  } catch (_err) {
    // localStorage may be write-protected in some test sandboxes;
    // the in-memory writable is still set.
  }
}
