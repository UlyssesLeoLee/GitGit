import { useEffect, useState, type ReactNode } from 'react';
import { Sidebar } from './Sidebar';

interface AppShellProps {
  children: ReactNode;
  /** Optional content rendered in the topbar (e.g. breadcrumbs). */
  topbar?: ReactNode;
}

/**
 * Application chrome: sidebar + topbar + main content. Mobile
 * collapses the sidebar into a drawer that the topbar toggle opens.
 */
export function AppShell({ children, topbar }: AppShellProps) {
  const [drawerOpen, setDrawerOpen] = useState(false);
  useEffect(() => {
    // Esc closes the mobile drawer.
    function onKey(e: KeyboardEvent): void {
      if (e.key === 'Escape') setDrawerOpen(false);
    }
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, []);
  return (
    <div className="flex h-full min-h-screen">
      <Sidebar open={drawerOpen} onClose={() => setDrawerOpen(false)} />
      <div className="flex min-w-0 flex-1 flex-col">
        <header className="sticky top-0 z-20 flex items-center gap-3 border-b border-slate-200 bg-white/80 px-4 py-3 backdrop-blur dark:border-slate-800 dark:bg-slate-950/80">
          <button
            type="button"
            className="btn-secondary md:hidden"
            onClick={() => setDrawerOpen((v) => !v)}
            aria-label={drawerOpen ? 'Close menu' : 'Open menu'}
            aria-expanded={drawerOpen}
          >
            ☰
          </button>
          <div className="min-w-0 flex-1 truncate text-sm text-slate-600 dark:text-slate-300">
            {topbar}
          </div>
        </header>
        <main className="min-w-0 flex-1 px-4 py-6 md:px-8">{children}</main>
      </div>
    </div>
  );
}