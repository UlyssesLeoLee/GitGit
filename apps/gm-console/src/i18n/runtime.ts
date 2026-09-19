import { useLocaleStore } from '@/stores/locale';
import { DEFAULT_LOCALE, SUPPORTED_LOCALES, type Locale, type Messages, type MessagesByLocale } from './index';
import { en } from './en';
import { zhCN } from './zh-CN';

const MESSAGES: MessagesByLocale = {
  'zh-CN': zhCN,
  en: en,
};

export { MESSAGES, SUPPORTED_LOCALES, DEFAULT_LOCALE };
export type { Locale, Messages };

/**
 * Plain translator. Returns the resolved string or a `__key__` marker
 * when the key is missing — never throws, never returns `undefined`.
 */
export function translate(locale: Locale, key: string, args?: unknown[]): string {
  const segments = key.split('.');
  let cursor: unknown = MESSAGES[locale];
  for (const seg of segments) {
    if (cursor && typeof cursor === 'object' && seg in (cursor as Record<string, unknown>)) {
      cursor = (cursor as Record<string, unknown>)[seg];
    } else {
      return `__${key}__`;
    }
  }
  if (typeof cursor === 'function') {
    try {
      return String((cursor as (...a: unknown[]) => unknown)(...(args ?? [])));
    } catch {
      return `__${key}__`;
    }
  }
  if (typeof cursor === 'string') return cursor;
  return `__${key}__`;
}

/** React hook returning a memoised `t` bound to the active locale. */
export function useTranslate(): (key: string, args?: unknown[]) => string {
  const locale = useLocaleStore((s) => s.locale);
  return (key: string, args?: unknown[]) => translate(locale, key, args);
}