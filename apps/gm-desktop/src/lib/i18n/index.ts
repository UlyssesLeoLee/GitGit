/**
 * i18n catalog. Two bundles (zh-CN + en) registered against a tiny
 * lookup table; default locale is `zh-CN` per the brief. We keep the
 * surface area deliberately small (no ICU, no Interpolation hook)
 * because the app's strings are short technical labels.
 *
 * Adding a new language: 1) drop a new file under `src/lib/i18n/`,
 * 2) import + register it in the `LOCALES` map below.
 */

import { locale } from '$lib/stores/locale';
import { get } from 'svelte/store';
import zh from './zh-CN';
import en from './en';

export type LocaleId = 'zh-CN' | 'en';
export type Catalog = Readonly<Record<string, string>>;
export type CatalogWithDot = Readonly<Record<string, string | undefined>>;

const LOCALES: Record<LocaleId, Catalog> = {
  'zh-CN': zh,
  'en': en,
};

/**
 * Return the localized string for `key`, falling back through:
 *   active locale → 'en' → key.
 * Subscriptions to `locale` ensure the consumer re-renders on switch.
 */
export function tFor(localeId: LocaleId, key: string): string {
  const primary = LOCALES[localeId];
  if (primary && Object.prototype.hasOwnProperty.call(primary, key)) {
    return (primary as CatalogWithDot)[key] as string;
  }
  const en = LOCALES.en;
  if (en && Object.prototype.hasOwnProperty.call(en, key)) {
    return (en as CatalogWithDot)[key] as string;
  }
  return key;
}

/** Reactive-style wrapper used inside Svelte components. */
export function t(key: string): string {
  const current = get(locale);
  return tFor(current, key);
}

/** Allow components to subscribe via `$t('foo.bar')`. */
export { locale };
