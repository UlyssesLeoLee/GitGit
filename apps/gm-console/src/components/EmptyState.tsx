import type { ReactNode } from 'react';

interface EmptyStateProps {
  title?: string;
  description?: string;
  icon?: ReactNode;
  action?: ReactNode;
}

/**
 * Empty-state placeholder. Used when a query returns an empty array
 * or a list has no items. Pass an `action` to provide the user a
 * next step (e.g. "Create key").
 */
export function EmptyState({
  title = 'Nothing here yet',
  description,
  icon,
  action,
}: EmptyStateProps) {
  return (
    <div
      role="status"
      className="card flex flex-col items-center gap-3 p-12 text-center text-slate-500 dark:text-slate-400"
    >
      <div aria-hidden="true" className="text-4xl">
        {icon ?? '∅'}
      </div>
      <p className="text-base font-medium text-slate-700 dark:text-slate-200">{title}</p>
      {description ? <p className="max-w-md text-sm">{description}</p> : null}
      {action ? <div className="mt-2">{action}</div> : null}
    </div>
  );
}