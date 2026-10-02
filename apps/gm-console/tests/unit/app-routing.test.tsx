import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { App } from '@/App';
import { useToastsStore } from '@/stores/toasts';
import { useLocaleStore } from '@/stores/locale';
import { useThemeStore } from '@/stores/theme';

const api = vi.hoisted(() => ({
  repos: {
    health: vi.fn(),
    listRepos: vi.fn(),
    getRepo: vi.fn(),
    getRepoRefs: vi.fn(),
    getRepoLog: vi.fn(),
  },
  vault: {
    listKeys: vi.fn(),
    getKey: vi.fn(),
    getVersions: vi.fn(),
    setVersion: vi.fn(),
    diffVersions: vi.fn(),
    restoreVersion: vi.fn(),
    deleteKey: vi.fn(),
  },
}));

vi.mock('@/api', () => api);

/** App owns a BrowserRouter, so drive it through the real history API. */
function goto(path: string) {
  window.history.pushState({}, '', path);
}

function renderApp() {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false, gcTime: 0, staleTime: 0 } },
  });
  return render(
    <QueryClientProvider client={client}>
      <App />
    </QueryClientProvider>,
  );
}

beforeEach(() => {
  useToastsStore.getState().clear();
  useLocaleStore.setState({ locale: 'en' });
  useThemeStore.setState({ theme: 'system', effective: 'light' });
  Object.values(api.repos).forEach((f) => {
    f.mockReset();
    f.mockResolvedValue([]);
  });
  Object.values(api.vault).forEach((f) => {
    f.mockReset();
    f.mockResolvedValue({});
  });
  api.repos.listRepos.mockResolvedValue([]);
  api.vault.listKeys.mockResolvedValue([]);
});

afterEach(() => {
  useToastsStore.getState().clear();
  window.history.pushState({}, '', '/');
});

describe('App routing', () => {
  it('renders the Repositories topbar and repo list at the root', async () => {
    goto('/');
    renderApp();
    expect(await screen.findByText('Repositories', undefined, { timeout: 5000 })).toBeInTheDocument();
  });

  it('shows the Vault topbar and key list on /vault', async () => {
    api.vault.listKeys.mockResolvedValue([]);
    goto('/vault');
    renderApp();
    // The topbar title and the page heading share the word, so assert the
    // element is present rather than unique.
    expect((await screen.findAllByText('Vault', undefined, { timeout: 5000 })).length).toBeGreaterThan(0);
  });

  it('shows the Settings topbar and form on /settings', async () => {
    api.repos.health.mockResolvedValue({ status: 'ok', vault_online: true, version: '0.1.0' });
    goto('/settings');
    renderApp();
    expect((await screen.findAllByText('Settings', undefined, { timeout: 5000 })).length).toBeGreaterThan(0);
  });

  it('shows the Repository topbar and fetches the named repo', async () => {
    api.repos.getRepo.mockResolvedValue({
      name: 'alpha',
      path: '/repos/alpha.git',
      refs: [],
      log: [],
    });
    goto('/repos/alpha');
    renderApp();
    expect(await screen.findByText('Repository')).toBeInTheDocument();
    await waitFor(() => expect(api.repos.getRepo).toHaveBeenCalledWith('alpha'), { timeout: 5000 });
  });

  it('redirects an unknown path to the 404 route', async () => {
    goto('/this-route-does-not-exist');
    renderApp();
    await waitFor(() => expect(window.location.pathname).toBe('/404'), { timeout: 5000 });
  });

  it('mounts the toast viewport alongside the app chrome', async () => {
    goto('/');
    renderApp();
    await screen.findByText('Repositories', undefined, { timeout: 5000 });
    expect(document.querySelector('[aria-live="polite"]')).toBeInTheDocument();
  });

  it('wires the sidebar navigation into the shell', async () => {
    goto('/');
    renderApp();
    await screen.findByText('Repositories', undefined, { timeout: 5000 });
    expect(screen.getByRole('navigation', { name: 'Sections' })).toBeInTheDocument();
    expect(screen.getByRole('group', { name: 'Language' })).toBeInTheDocument();
  });
}, 20000);
