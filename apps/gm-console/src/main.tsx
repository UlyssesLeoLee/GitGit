import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { QueryClientProvider } from '@tanstack/react-query';
import { App } from './App';
import { createQueryClient } from '@/lib/query-client';
import { useLocaleStore, useThemeStore } from '@/stores';
import './index.css';

async function bootstrapMocks(): Promise<void> {
  // MSW is opt-in via VITE_ENABLE_MOCKS. In production builds the
  // flag is `false`, so this is a no-op except during demos and tests.
  if (typeof __ENABLE_MOCKS__ === 'undefined' || !__ENABLE_MOCKS__) return;
  try {
    const { worker } = await import('./mocks/browser');
    await worker.start({
      onUnhandledRequest: 'bypass',
      serviceWorker: { url: '/mockServiceWorker.js' },
    });
  } catch (err) {
    // eslint-disable-next-line no-console
    console.warn('[mocks] failed to start MSW — falling back to real network', err);
  }
}

function Root(): JSX.Element {
  const client = createQueryClient();
  return (
    <StrictMode>
      <QueryClientProvider client={client}>
        <App />
      </QueryClientProvider>
    </StrictMode>
  );
}

async function main(): Promise<void> {
  // Hydrate persisted UI state before first render.
  useThemeStore.getState().hydrate();
  useLocaleStore.getState().hydrate();
  await bootstrapMocks();
  const rootEl = document.getElementById('root');
  if (!rootEl) throw new Error('root element missing');
  createRoot(rootEl).render(<Root />);
}

void main();