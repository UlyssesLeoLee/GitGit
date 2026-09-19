import type { ReactNode } from 'react';

interface LoadingProps {
  label?: string;
  fullscreen?: boolean;
  /** Optional children rendered above the spinner (e.g. an action bar). */
  children?: ReactNode;
}

/**
 * Loading indicator with an `aria-live` polite region. Use the
 * `fullscreen` flag to take over the viewport (rarely needed —
 * prefer leaving page chrome visible).
 */
export function Loading({ label = 'Loading…', fullscreen = false, children }: LoadingProps) {
  const container = fullscreen
    ? 'fixed inset-0 z-40 flex items-center justify-center bg-white/70 backdrop-blur dark:bg-slate-950/70'
    : 'flex items-center justify-center py-12';
  return (
    <div role="status" aria-live="polite" className={container}>
      <div className="flex flex-col items-center gap-3 text-slate-500 dark:text-slate-400">
        <span
          aria-hidden="true"
          className="inline-block h-8 w-8 animate-spin rounded-full border-2 border-brand-500 border-t-transparent"
        />
        <span className="text-sm">{label}</span>
        {children}
      </div>
    </div>
  );
}