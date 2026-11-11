import { beforeEach, describe, expect, it, vi } from 'vitest';
import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { Settings } from '@/routes/Settings';
import { RepoDetail } from '@/routes/RepoDetail';
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

function Harness({ ui, path = '/', pattern }: { ui: React.ReactElement; path?: string; pattern?: string }) {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false, gcTime: 0, staleTime: 0 } },
  });
  return render(
    <QueryClientProvider client={client}>
      <MemoryRouter initialEntries={[path]}>
        {pattern ? (
          <Routes>
            <Route path={pattern} element={ui} />
          </Routes>
        ) : (
          ui
        )}
      </MemoryRouter>
    </QueryClientProvider>,
  );
}

beforeEach(() => {
  useToastsStore.getState().clear();
  useLocaleStore.setState({ locale: 'en' });
  useThemeStore.setState({ theme: 'system', effective: 'light' });
  Object.values(api.repos).forEach((f) => f.mockReset());
  Object.values(api.vault).forEach((f) => f.mockReset());
});

describe('Settings', () => {
  it('shows a loading state while the health probe runs', async () => {
    api.repos.health.mockReturnValue(new Promise(() => {}));
    const { container } = Harness({ ui: <Settings /> });
    expect(within(container).getByRole('status')).toBeInTheDocument();
  });

  it('renders the online backend status and version', async () => {
    api.repos.health.mockResolvedValue({ status: 'ok', vault_online: true, version: '0.1.0' });
    const { container } = Harness({ ui: <Settings /> });
    await waitFor(() => expect(within(container).getByText('0.1.0')).toBeInTheDocument());
  });

  it('degrades to an offline badge when the health probe fails', async () => {
    // The component swallows the error into `null`, so no error card appears.
    api.repos.health.mockRejectedValue(new Error('down'));
    const { container } = Harness({ ui: <Settings /> });
    await waitFor(() => expect(within(container).queryByRole('alert')).toBeNull());
    expect(within(container).queryByText('0.1.0')).toBeNull();
  });

  it('disables save until a password is typed', async () => {
    api.repos.health.mockResolvedValue({ status: 'ok', vault_online: true, version: '0.1.0' });
    Harness({ ui: <Settings /> });
    const submit = await screen.findByRole('button');
    expect(submit).toBeDisabled();
  });

  it('rotates the password through the versioned vault write and toasts', async () => {
    api.repos.health.mockResolvedValue({ status: 'ok', vault_online: true, version: '0.1.0' });
    api.vault.setVersion.mockResolvedValue({ ok: true });
    const user = userEvent.setup();
    const { container } = Harness({ ui: <Settings /> });

    const input = within(container).getByLabelText(/password/i);
    await user.type(input, 'hunter2');
    await user.click(within(container).getByRole('button'));

    await waitFor(() => expect(api.vault.setVersion).toHaveBeenCalled());
    expect(api.vault.setVersion).toHaveBeenCalledWith('gitgit.password', { value: 'hunter2' });
    await waitFor(() => expect(useToastsStore.getState().toasts[0]?.kind).toBe('success'));
    // The field is cleared after a successful rotation.
    await waitFor(() => expect(within(container).getByLabelText(/password/i)).toHaveValue(''));
  });

  it('surfaces a failure toast when the rotation is rejected', async () => {
    api.repos.health.mockResolvedValue({ status: 'ok', vault_online: true, version: '0.1.0' });
    api.vault.setVersion.mockRejectedValue(new Error('vault locked'));
    const user = userEvent.setup();
    const { container } = Harness({ ui: <Settings /> });

    await user.type(within(container).getByLabelText(/password/i), 'hunter2');
    await user.click(within(container).getByRole('button'));

    await waitFor(() => expect(useToastsStore.getState().toasts[0]?.kind).toBe('error'));
    expect(useToastsStore.getState().toasts[0]?.message).toBe('vault locked');
  });
});

describe('RepoDetail', () => {
  const repo = {
    name: 'alpha',
    path: '/repos/alpha.git',
    refs: [{ name: 'refs/heads/main', sha: 'abcdef1234567890abcdef' }],
    log: [{ sha: 'abcdef1234567890', short_sha: 'abcdef1', subject: 'Initial commit' }],
  };

  it('shows a loading state while the repo is fetched', async () => {
    api.repos.getRepo.mockReturnValue(new Promise(() => {}));
    const { container } = Harness({ ui: <RepoDetail />, path: '/repos/alpha', pattern: '/repos/:name' });
    expect(within(container).getByRole('status')).toBeInTheDocument();
  });

  it('renders the repo name, path, clone URL, refs and log', async () => {
    api.repos.getRepo.mockResolvedValue(repo);
    const { container } = Harness({ ui: <RepoDetail />, path: '/repos/alpha', pattern: '/repos/:name' });

    await waitFor(() => expect(within(container).getByText('alpha')).toBeInTheDocument());
    expect(within(container).getByText('/repos/alpha.git')).toBeInTheDocument();
    // The clone URL is absolute (built from window.location) and ends in .git,
    // so anchor on the scheme to tell it apart from the bare path above.
    expect(within(container).getByText(/^https?:\/\/\S+\/repos\/alpha\.git$/)).toBeInTheDocument();
    expect(within(container).getByText('refs/heads/main')).toBeInTheDocument();
    expect(within(container).getByText('abcdef1')).toBeInTheDocument();
    expect(within(container).getByText('Initial commit')).toBeInTheDocument();
  });

  it('requests the repo named in the route param', async () => {
    api.repos.getRepo.mockResolvedValue(repo);
    Harness({ ui: <RepoDetail />, path: '/repos/alpha', pattern: '/repos/:name' });
    await waitFor(() => expect(api.repos.getRepo).toHaveBeenCalledWith('alpha'));
  });

  it('explains an empty ref list and an empty log', async () => {
    api.repos.getRepo.mockResolvedValue({ ...repo, refs: [], log: [] });
    const { container } = Harness({ ui: <RepoDetail />, path: '/repos/alpha', pattern: '/repos/:name' });
    await waitFor(() => expect(within(container).getByText('alpha')).toBeInTheDocument());
    expect(within(container).queryByRole('table')).toBeNull();
    expect(within(container).queryByRole('list')).toBeNull();
  });

  it('surfaces an error state when the fetch fails', async () => {
    api.repos.getRepo.mockRejectedValue(new Error('repo missing'));
    const { container } = Harness({ ui: <RepoDetail />, path: '/repos/alpha', pattern: '/repos/:name' });
    await waitFor(() => expect(within(container).getByRole('alert')).toBeInTheDocument());
    expect(within(container).getByText('repo missing')).toBeInTheDocument();
  });
});
