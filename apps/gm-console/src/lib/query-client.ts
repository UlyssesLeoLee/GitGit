import { QueryClient } from '@tanstack/react-query';

/**
 * Singleton React Query client. We expose a factory so tests can
 * instantiate an isolated client per suite without touching global
 * state.
 */
export function createQueryClient(): QueryClient {
  return new QueryClient({
    defaultOptions: {
      queries: {
        staleTime: 15_000,
        gcTime: 5 * 60_000,
        retry: (failureCount, err) => {
          // Network errors get one retry; 4xx responses get none.
          if (err && typeof err === 'object' && 'status' in err) {
            const status = (err as { status?: number }).status;
            if (typeof status === 'number' && status >= 400 && status < 500) return false;
          }
          return failureCount < 1;
        },
        refetchOnWindowFocus: false,
      },
      mutations: { retry: 0 },
    },
  });
}