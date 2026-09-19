import { NavLink, useLocation } from 'react-router-dom';
import clsx from 'clsx';
import { useTranslate } from '@/i18n/runtime';
import { LocaleSwitcher } from './LocaleSwitcher';
import { ThemeToggle } from './ThemeToggle';

interface NavItem {
  to: string;
  labelKey: string;
  match: (pathname: string) => boolean;
}

const NAV_ITEMS: ReadonlyArray<NavItem> = [
  { to: '/', labelKey: 'app.nav.repos', match: (p) => p === '/' || p.startsWith('/repos') },
  { to: '/vault', labelKey: 'app.nav.vault', match: (p) => p.startsWith('/vault') },
  { to: '/settings', labelKey: 'app.nav.settings', match: (p) => p.startsWith('/settings') },
];

interface SidebarProps {
  open: boolean;
  onClose: () => void;
}

/**
 * Two-pane shell. On mobile, the sidebar slides in as a drawer
 * (`open` prop); on `md:` it becomes a persistent column.
 */
export function Sidebar({ open, onClose }: SidebarProps) {
  const t = useTranslate();
  const { pathname } = useLocation();
  return (
    <>
      {/* Mobile backdrop */}
      <div
        aria-hidden="true"
        onClick={onClose}
        className={
          'fixed inset-0 z-30 bg-slate-900/50 transition-opacity md:hidden ' +
          (open ? 'opacity-100' : 'pointer-events-none opacity-0')
        }
      />
      <aside
        aria-label="Primary"
        className={clsx(
          'fixed inset-y-0 left-0 z-40 flex w-64 flex-col border-r border-slate-200 bg-white p-4 transition-transform md:static md:translate-x-0 dark:border-slate-800 dark:bg-slate-950',
          open ? 'translate-x-0' : '-translate-x-full md:translate-x-0',
        )}
      >
        <header className="mb-6 flex flex-col gap-1">
          <span className="text-lg font-semibold tracking-tight text-brand-600 dark:text-brand-400">
            {t('app.title')}
          </span>
          <span className="text-xs text-slate-500 dark:text-slate-400">{t('app.subtitle')}</span>
        </header>
        <nav aria-label="Sections" className="flex-1 space-y-1">
          {NAV_ITEMS.map((item) => {
            const active = item.match(pathname);
            return (
              <NavLink
                key={item.to}
                to={item.to}
                end={item.to === '/'}
                onClick={onClose}
                aria-current={active ? 'page' : undefined}
                className={clsx(
                  'block rounded-md px-3 py-2 text-sm font-medium transition',
                  active
                    ? 'bg-brand-50 text-brand-700 dark:bg-brand-900/30 dark:text-brand-200'
                    : 'text-slate-700 hover:bg-slate-100 dark:text-slate-200 dark:hover:bg-slate-800',
                )}
              >
                {t(item.labelKey)}
              </NavLink>
            );
          })}
        </nav>
        <footer className="mt-4 flex items-center justify-between gap-2 border-t border-slate-200 pt-4 dark:border-slate-800">
          <LocaleSwitcher />
          <ThemeToggle />
        </footer>
      </aside>
    </>
  );
}