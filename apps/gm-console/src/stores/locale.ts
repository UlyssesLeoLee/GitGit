import { create } from 'zustand';
import { DEFAULT_LOCALE, type Locale } from '@/i18n';

const STORAGE_KEY = 'gm.locale';

function readStored(): Locale {
  if (typeof localStorage === 'undefined') return DEFAULT_LOCALE;
  const raw = localStorage.getItem(STORAGE_KEY);
  if (raw === 'zh-CN' || raw === 'en') return raw;
  return DEFAULT_LOCALE;
}

function writeStored(value: Locale): void {
  if (typeof localStorage === 'undefined') return;
  try {
    localStorage.setItem(STORAGE_KEY, value);
  } catch {
    /* storage may be unavailable */
  }
}

interface LocaleState {
  locale: Locale;
  setLocale: (next: Locale) => void;
  hydrate: () => void;
}

export const useLocaleStore = create<LocaleState>((set) => ({
  locale: DEFAULT_LOCALE,
  setLocale(next) {
    writeStored(next);
    if (typeof document !== 'undefined') document.documentElement.lang = next;
    set({ locale: next });
  },
  hydrate() {
    const locale = readStored();
    if (typeof document !== 'undefined') document.documentElement.lang = locale;
    set({ locale });
  },
}));