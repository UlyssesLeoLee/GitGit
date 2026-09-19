import { useThemeStore, type Theme } from '@/stores/theme';

const OPTIONS: ReadonlyArray<{ value: Theme; label: string }> = [
  { value: 'light', label: '☀' },
  { value: 'system', label: '◐' },
  { value: 'dark', label: '☾' },
];

/**
 * Three-way theme toggle. Compact, icon-only, accessible (radiogroup).
 * Persists to localStorage via the store.
 */
export function ThemeToggle() {
  const theme = useThemeStore((s) => s.theme);
  const setTheme = useThemeStore((s) => s.setTheme);
  return (
    <div
      role="radiogroup"
      aria-label="Theme"
      className="inline-flex overflow-hidden rounded-md border border-slate-300 bg-white text-sm dark:border-slate-700 dark:bg-slate-900"
    >
      {OPTIONS.map((opt) => (
        <button
          key={opt.value}
          type="button"
          role="radio"
          aria-checked={theme === opt.value}
          aria-label={`Theme: ${opt.value}`}
          onClick={() => setTheme(opt.value)}
          className={
            'px-2.5 py-1.5 transition ' +
            (theme === opt.value
              ? 'bg-brand-600 text-white'
              : 'text-slate-700 hover:bg-slate-100 dark:text-slate-200 dark:hover:bg-slate-800')
          }
        >
          <span aria-hidden="true">{opt.label}</span>
        </button>
      ))}
    </div>
  );
}