import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { act, render, screen, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { App } from '@/App';
import { useToastsStore } from '@/stores/toasts';
import { useLocaleStore } from '@/stores/locale';
import { useThemeStore } from '@/stores/theme';
import { useCredentialsStore } from '@/stores/credentials';

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
  // `[FACT]` `App` renders behind `LoginGate`, which shows the sign-in
  // form until a credential is held. These cases are about *routing*, so
  // they start from the signed-in state an operator is normally in;
  // without this they would all fail on "Repositories" not being
  // rendered, which says nothing about routing.
  //
  // The gate's own behaviour — refused credential, 401 handling, the
  // probe endpoint — is pinned in `login-gate.test.tsx`, which does not
  // mock `@/api` and therefore exercises the real client.
  useCredentialsStore.getState().signIn({ user: 'admin', password: 'test-password' });
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
  // `[FACT]` These two writes reach a **mounted** tree. `cleanup()` is
  // registered in `vitest.setup.ts`, which loads before this file, and
  // vitest's default `sequence.hooks: 'stack'` runs the later-registered
  // hook first — so the components are still on the tree here, and
  // `signOut()` re-renders `LoginGate` while `clear()` re-renders the
  // toast viewport. Both land outside React's act environment and emit
  // "An update to X inside a test was not wrapped in act(...)".
  //
  // Wrapping the writes is the fix rather than `cleanup()`-first, because
  // tearing the tree down first would mean each case no longer asserts
  // against a mounted app — the coverage this file exists to provide.
  act(() => useToastsStore.getState().clear());
  act(() => useCredentialsStore.getState().signOut());
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
    await waitFor(() => expect(api.repos.getRepo).toHaveBeenCalledWith('alpha'), {
      timeout: 15000,
    });
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
