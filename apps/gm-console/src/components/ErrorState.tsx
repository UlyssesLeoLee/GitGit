import type { ReactNode } from 'react';
import { ApiError, NetworkError } from '@/api/errors';

interface ErrorStateProps {
  error: unknown;
  title?: string;
  onRetry?: () => void;
  /** Override the descriptive text under the title. */
  description?: ReactNode;
}

function describe(error: unknown): { title: string; body: ReactNode } {
  if (error instanceof NetworkError) {
    return {
      title: 'Network error',
      body: 'Could not reach the backend. Check that gitgit-server is running and reachable.',
    };
  }
  if (error instanceof ApiError) {
    return {
      title: error.isUnauthenticated ? 'Authentication required' : 'Request failed',
      body: (
        <span className="block">
          <span className="block">{error.message}</span>
          {error.code ? (
            <span className="mt-1 block font-mono text-xs text-slate-400">code: {error.code}</span>
          ) : null}
        </span>
      ),
    };
  }
  if (error instanceof Error) {
    return { title: 'Unexpected error', body: error.message };
  }
  return { title: 'Unexpected error', body: 'Unknown failure' };
}

/**
 * Error placeholder with an optional retry button. Treats ApiError
 * and NetworkError specially so the user sees actionable messages.
 */
export function ErrorState({ error, title, onRetry, description }: ErrorStateProps) {
  const info = describe(error);
  return (
    <div
      role="alert"
      className="card flex flex-col items-start gap-3 border-rose-300 bg-rose-50 p-6 text-rose-900 dark:border-rose-800 dark:bg-rose-950/40 dark:text-rose-200"
    >
      <p className="text-base font-semibold">{title ?? info.title}</p>
      <div className="text-sm">{description ?? info.body}</div>
      {onRetry ? (
        <button
          type="button"
          onClick={onRetry}
          className="btn-secondary"
          aria-label="Retry request"
        >
          Retry
        </button>
      ) : null}
    </div>
  );
}