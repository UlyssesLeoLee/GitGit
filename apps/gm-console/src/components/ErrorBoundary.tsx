import { Component, type ErrorInfo, type ReactNode } from 'react';
import { useToastsStore } from '@/stores/toasts';

interface Props {
  children: ReactNode;
  /** Optional label rendered in the fallback for debugging. */
  scope?: string;
}

interface State {
  hasError: boolean;
  message: string;
}

/**
 * Class-based error boundary. Each route wraps its content in one of
 * these so a thrown render error doesn't blank the whole app.
 */
export class ErrorBoundary extends Component<Props, State> {
  state: State = { hasError: false, message: '' };

  static getDerivedStateFromError(err: unknown): State {
    return { hasError: true, message: err instanceof Error ? err.message : String(err) };
  }

  componentDidCatch(err: Error, info: ErrorInfo): void {
    // eslint-disable-next-line no-console
    console.error('[ErrorBoundary]', this.props.scope ?? '<root>', err, info.componentStack);
  }

  private handleReload = (): void => {
    if (typeof window !== 'undefined') window.location.reload();
  };

  render(): ReactNode {
    if (this.state.hasError) {
      return (
        <div
          role="alert"
          aria-live="assertive"
          className="card m-4 p-6 border-rose-300 bg-rose-50 text-rose-900 dark:border-rose-700 dark:bg-rose-950/50 dark:text-rose-200"
        >
          <h2 className="text-lg font-semibold mb-2">{this.props.scope ?? 'Render error'}</h2>
          <p className="text-sm mb-4">{this.state.message || 'Unknown error'}</p>
          <button
            type="button"
            onClick={this.handleReload}
            className="btn-secondary"
            aria-label="Reload page"
          >
            Reload
          </button>
        </div>
      );
    }
    return this.props.children;
  }
}

/**
 * Lightweight hook so any child can push a global toast without
 * pulling in the store directly in every component.
 */
export function useToast(): {
  push: ReturnType<typeof useToastsStore.getState>['push'];
} {
  return { push: useToastsStore((s) => s.push) };
}