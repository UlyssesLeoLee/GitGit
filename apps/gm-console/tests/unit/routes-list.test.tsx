import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { render, screen, waitFor, within } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import type { ReactElement } from 'react';
import { NotFound } from '@/routes/NotFound';
import { Home } from '@/routes/Home';
import { Vault } from '@/routes/Vault';
import { useToastsStore } from '@/stores/toasts';
import { useLocaleStore } from '@/stores/locale';
import { useThemeStore } from '@/stores/theme';

const api = vi.hoisted(() => ({
  repos: {
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
  health: { health: vi.fn() },
}));

vi.mock('@/api', () => api);

/** Fresh QueryClient per test, retries off so error states surface at once. */
function renderRoute(ui: ReactElement, path = '/') {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false, gcTime: 0, staleTime: 0 } },
  });
  return render(
    <QueryClientProvider client={client}>
      <MemoryRouter initialEntries={[path]}>{ui}</MemoryRouter>
    </QueryClientProvider>,
  );
}

beforeEach(() => {
  useToastsStore.getState().clear();
  useLocaleStore.setState({ locale: 'en' });
  useThemeStore.setState({ theme: 'system', effective: 'light' });
  Object.values(api.repos).forEach((f) => f.mockReset());
  Object.values(api.vault).forEach((f) => f.mockReset());
  api.health.health.mockReset();
});

afterEach(() => {
  useToastsStore.getState().clear();
});

describe('NotFound', () => {
  it('shows a 404 empty state with a link home', async () => {
    renderRoute(<NotFound />);
    // The icon is decorative (aria-hidden), so assert on the card and the
    // escape hatch rather than on the glyph text.
    expect(screen.getByRole('status')).toBeInTheDocument();
    expect(screen.getByRole('link')).toHaveAttribute('href', '/');
  });
});

describe('Home', () => {
  it('shows a loading state while the query is in flight', async () => {
    // Never resolves; see the Vault loading test for why.
    api.repos.listRepos.mockReturnValue(new Promise(() => {}));
    const { container } = renderRoute(<Home />);
    // Scope to this render's container so a lingering node elsewhere in the
    // document cannot make the role lookup ambiguous.
    expect(within(container).getByRole('status')).toBeInTheDocument();
  });

  it('renders an empty state when there are no repos', async () => {
    api.repos.listRepos.mockResolvedValue([]);
    const { container } = renderRoute(<Home />);
    await waitFor(() => expect(within(container).getByRole('status')).toBeInTheDocument());
  });

  it('renders a card per repo with ref count, head ref and shortened sha', async () => {
    api.repos.listRepos.mockResolvedValue([
      {
        name: 'alpha',
        ref_count: 3,
        head_ref: 'refs/heads/main',
        head_sha: 'abcdef1234567890',
        path: '/repos/alpha.git',
      },
      { name: 'beta', ref_count: 1, head_ref: null, head_sha: null, path: '/repos/beta.git' },
    ]);
    const { container } = renderRoute(<Home />);

    await waitFor(() => expect(within(container).getByText('alpha')).toBeInTheDocument());
    expect(within(container).getByText('beta')).toBeInTheDocument();
    expect(within(container).getByText('3 refs')).toBeInTheDocument();
    expect(within(container).getByText('refs/heads/main')).toBeInTheDocument();
    expect(within(container).getByText('abcdef1')).toBeInTheDocument();
    // A repo with no head ref falls back to an em dash.
    expect(within(container).getAllByText('—').length).toBeGreaterThan(0);
  });

  it('links each card to the encoded repo detail route', async () => {
    api.repos.listRepos.mockResolvedValue([
      { name: 'my repo', ref_count: 0, head_ref: null, head_sha: null, path: '/repos/my repo.git' },
    ]);
    const { container } = renderRoute(<Home />);
    await waitFor(() => expect(within(container).getByRole('link')).toBeInTheDocument());
    expect(within(container).getByRole('link').getAttribute('href')).toBe('/repos/my%20repo');
  });

  it('surfaces an error state when the query fails', async () => {
    api.repos.listRepos.mockRejectedValue(new Error('server unreachable'));
    const { container } = renderRoute(<Home />);
    await waitFor(() => expect(within(container).getByRole('alert')).toBeInTheDocument());
    expect(within(container).getByText('server unreachable')).toBeInTheDocument();
  });
});

describe('Vault', () => {
  it('shows a loading state while keys are fetched', async () => {
    // Never resolves: the point is to observe the pending state. Resolving it
    // later would schedule a state update that leaks into the next test.
    api.vault.listKeys.mockReturnValue(new Promise(() => {}));
    const { container } = renderRoute(<Vault />);
    expect(within(container).getByRole('status')).toBeInTheDocument();
  });

  it('renders an empty state when the vault has no keys', async () => {
    api.vault.listKeys.mockResolvedValue([]);
    const { container } = renderRoute(<Vault />);
    await waitFor(() =>
      expect(within(container).getByRole('status')).toBeInTheDocument(),
    );
  });

  it('renders a row per key with version count and byte size', async () => {
    api.vault.listKeys.mockResolvedValue([
      { key: 'openai', version_count: 4, byte_len: 1536, current_version: 3 },
      { key: 'github', version_count: 1, byte_len: null, current_version: null },
    ]);
    const { container } = renderRoute(<Vault />);

    await waitFor(() => expect(within(container).getByText('openai')).toBeInTheDocument());
    expect(within(container).getByText('github')).toBeInTheDocument();
    expect(within(container).getByText(/v3/)).toBeInTheDocument();
    expect(within(container).getAllByRole('link')).toHaveLength(2);
  });

  it('links each key to its encoded detail route', async () => {
    api.vault.listKeys.mockResolvedValue([
      { key: 'team/key', version_count: 1, byte_len: null, current_version: null },
    ]);
    const { container } = renderRoute(<Vault />);
    await waitFor(() => expect(within(container).getByRole('link')).toBeInTheDocument());
    expect(within(container).getByRole('link').getAttribute('href')).toBe('/vault/team%2Fkey');
  });

  it('surfaces an error state when the query fails', async () => {
    api.vault.listKeys.mockRejectedValue(new Error('vault offline'));
    const { container } = renderRoute(<Vault />);
    await waitFor(() => expect(within(container).getByRole('alert')).toBeInTheDocument());
    expect(within(container).getByText('vault offline')).toBeInTheDocument();
  });
});
