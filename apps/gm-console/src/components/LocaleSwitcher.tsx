import { SUPPORTED_LOCALES } from '@/i18n';
import { useLocaleStore } from '@/stores/locale';

const LABELS: Record<string, string> = {
  'zh-CN': '中文',
  en: 'EN',
};

/**
 * Locale switcher. Two-button group is sufficient for the v0.1 set
 * of locales; grows naturally if more are added.
 */
export function LocaleSwitcher() {
  const locale = useLocaleStore((s) => s.locale);
  const setLocale = useLocaleStore((s) => s.setLocale);
  return (
    <div
      role="group"
      aria-label="Language"
      className="inline-flex overflow-hidden rounded-md border border-slate-300 bg-white text-sm dark:border-slate-700 dark:bg-slate-900"
    >
      {SUPPORTED_LOCALES.map((code) => (
        <button
          key={code}
          type="button"
          aria-pressed={locale === code}
          aria-label={`Language: ${code}`}
          onClick={() => setLocale(code)}
          className={
            'px-2.5 py-1.5 transition ' +
            (locale === code
              ? 'bg-brand-600 text-white'
              : 'text-slate-700 hover:bg-slate-100 dark:text-slate-200 dark:hover:bg-slate-800')
          }
        >
          {LABELS[code] ?? code}
        </button>
      ))}
    </div>
  );
}