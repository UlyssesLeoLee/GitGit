/**
 * i18n catalog. Two bundles (zh-CN + en) registered against a tiny
 * lookup table; default locale is `zh-CN` per the brief. We keep the
 * surface area deliberately small (no ICU, no Interpolation hook)
 * because the app's strings are short technical labels.
 *
 * Adding a new language: 1) drop a new file under `src/lib/i18n/`,
 * 2) import + register it in the `LOCALES` map below.
 *
 * Svelte 5 compatibility note
 * ----------------------------
 * The `$store` auto-subscribe shorthand only works inside template
 * `{}` expressions — not in `<script>` blocks when the project runs
 * in Svelte 5 runes mode. We therefore expose the translated catalog
 * as a real `derived(locale, …)` store (`catalog`) and ask consumers
 * to access strings via `{$catalog['key']}` in templates, or
 * `get(catalog)['key']` / `tFor(get(locale), 'key')` in script blocks.
 *
 * A pure helper `t(key)` is also exported for one-off reads (e.g.
 * from imperative code paths), but it is *not* automatically
 * reactive — use `catalog` for anything rendered in a template.
 */

import { derived, get } from 'svelte/store';
import { locale, type LocaleId } from '$lib/stores/locale';
import zh from './zh-CN';
import en from './en';

export type { LocaleId };
export type Catalog = Readonly<Record<string, string>>;
export type CatalogWithDot = Readonly<Record<string, string | undefined>>;

const LOCALES: Record<LocaleId, Catalog> = {
  'zh-CN': zh,
  'en': en,
};

/**
 * Reactive catalog store. Subscribes to `locale` and re-emits the
 * matching `LOCALES[localeId]` map. Use as `{$catalog['key']}` in
 * templates, or `get(catalog)['key']` in script.
 */
export const catalog = derived(locale, ($locale) => {
  return LOCALES[$locale] ?? LOCALES.en;
});

/**
 * Return the localized string for `key`, falling back through:
 *   active locale → 'en' → key.
 * Pure function — does not subscribe to any store.
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

/**
 * One-off translation helper for imperative code paths. Reads the
 * current locale via `get(catalog)` (the derived store).
 *
 * For template rendering, prefer `{$catalog['key']}` — that gives you
 * reactive re-renders on locale switch for free.
 */
export function t(key: string): string {
  const map = get(catalog);
  if (Object.prototype.hasOwnProperty.call(map, key)) {
    return (map as CatalogWithDot)[key] as string;
  }
  // Fall through to the en bundle, then the key itself.
  return tFor('en', key);
}

/** Re-exported so consumers can subscribe to locale changes. */
export { locale };
