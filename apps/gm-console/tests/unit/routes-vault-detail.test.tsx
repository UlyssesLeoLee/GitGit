import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { act, render, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { VaultKeyDetail } from '@/routes/VaultKeyDetail';
import { VaultDiff } from '@/routes/VaultDiff';
import { VaultRestore } from '@/routes/VaultRestore';
import { useToastsStore } from '@/stores/toasts';
import { useLocaleStore } from '@/stores/locale';
import { useThemeStore } from '@/stores/theme';

const api = vi.hoisted(() => ({
  repos: { health: vi.fn(), listRepos: vi.fn(), getRepo: vi.fn() },
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

function Harness({
  ui,
  path,
  pattern,
}: {
  ui: React.ReactElement;
  path: string;
  pattern: string;
}) {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false, gcTime: 0, staleTime: 0 } },
  });
  return render(
    <QueryClientProvider client={client}>
      <MemoryRouter initialEntries={[path]}>
        <Routes>
          <Route path={pattern} element={ui} />
          {/*
            `[FACT]` These routes navigate away when an action succeeds —
            `/vault` after a delete, `/vault/:key` after a restore. A
            `<Routes>` that does not match the new location logs
            `No routes matched location`, so without a catch-all the
            success path of every one of those cases was unverifiable
            except through the noise it printed. The destination is not
            what these tests are about; that the action completed is.
          */}
          <Route path="*" element={null} />
        </Routes>
      </MemoryRouter>
    </QueryClientProvider>,
  );
}

const versions = {
  key: 'openai',
  versions: [
    {
      version: 1,
      bytes_sha256: 'a'.repeat(64),
      byte_len: 10,
      created_at_unix_ms: 1_767_225_600_000,
      change_note: null,
    },
    {
      version: 2,
      bytes_sha256: 'b'.repeat(64),
      byte_len: 20,
      created_at_unix_ms: 1_767_312_000_000,
      change_note: 'rotated',
    },
  ],
};

const keyDetail = {
  key: 'openai',
  value: 'sk-test',
  current_version: 2,
  byte_len: 7,
  bytes_sha256: 'b'.repeat(64),
  created_at_unix_ms: 1_767_312_000_000,
  change_note: 'rotated',
};

beforeEach(() => {
  useToastsStore.getState().clear();
  useLocaleStore.setState({ locale: 'en' });
  useThemeStore.setState({ theme: 'system', effective: 'light' });
  Object.values(api.vault).forEach((f) => f.mockReset());
  Object.values(api.repos).forEach((f) => f.mockReset());
  api.vault.getKey.mockResolvedValue(keyDetail);
  api.vault.getVersions.mockResolvedValue(versions);
  api.vault.deleteKey.mockResolvedValue(undefined);
  api.vault.restoreVersion.mockResolvedValue({ new_version: 3 });
  api.vault.diffVersions.mockResolvedValue({ added: 1, removed: 0 });
});

afterEach(() => {
  // `[FACT]` Same reason as the other suites: `cleanup()` lives in
  // `vitest.setup.ts` and vitest runs this later-registered hook first,
  // so the tree is still mounted and a bare `clear()` re-renders outside
  // React's act environment.
  act(() => useToastsStore.getState().clear());
});

describe('VaultKeyDetail', () => {
  it('waits for both the key and its versions before rendering', async () => {
    api.vault.getKey.mockReturnValue(new Promise(() => {}));
    const { container } = Harness({
      ui: <VaultKeyDetail />,
      path: '/vault/openai',
      pattern: '/vault/:key',
    });
    expect(within(container).getByRole('status')).toBeInTheDocument();
  });

  it('decodes the route param and asks the API for that key', async () => {
    Harness({ ui: <VaultKeyDetail />, path: '/vault/team%2Fkey', pattern: '/vault/:key' });
    await waitFor(() => expect(api.vault.getKey).toHaveBeenCalledWith('team/key'));
  });

  it('renders the decoded key as the heading', async () => {
    const { container } = Harness({
      ui: <VaultKeyDetail />,
      path: '/vault/team%2Fkey',
      pattern: '/vault/:key',
    });
    await waitFor(() =>
      expect(within(container).getByRole('heading', { name: 'team/key' })).toBeInTheDocument(),
    );
  });

  it('offers diff and restore links once there are two or more versions', async () => {
    const { container } = Harness({
      ui: <VaultKeyDetail />,
      path: '/vault/openai',
      pattern: '/vault/:key',
    });
    await waitFor(() => expect(within(container).getAllByRole('link').length).toBeGreaterThan(0));
    const hrefs = within(container)
      .getAllByRole('link')
      .map((a) => a.getAttribute('href'));
    expect(hrefs).toContain('/vault/openai/diff');
    expect(hrefs).toContain('/vault/openai/restore');
  });

  it('hides the diff link when only one version exists', async () => {
    api.vault.getVersions.mockResolvedValue({ key: 'openai', versions: [versions.versions[0]] });
    const { container } = Harness({
      ui: <VaultKeyDetail />,
      path: '/vault/openai',
      pattern: '/vault/:key',
    });
    await waitFor(() =>
      expect(within(container).getByRole('heading', { name: 'openai' })).toBeInTheDocument(),
    );
    const hrefs = within(container)
      .getAllByRole('link')
      .map((a) => a.getAttribute('href'));
    expect(hrefs).not.toContain('/vault/openai/diff');
  });

  it('surfaces an error state when the key fetch fails', async () => {
    api.vault.getKey.mockRejectedValue(new Error('key missing'));
    const { container } = Harness({
      ui: <VaultKeyDetail />,
      path: '/vault/openai',
      pattern: '/vault/:key',
    });
    await waitFor(() => expect(within(container).getByRole('alert')).toBeInTheDocument());
  });

  it('deletes the key after the user confirms', async () => {
    const user = userEvent.setup();
    const confirmSpy = vi.spyOn(window, 'confirm').mockReturnValue(true);
    const { container } = Harness({
      ui: <VaultKeyDetail />,
      path: '/vault/openai',
      pattern: '/vault/:key',
    });
    await waitFor(() =>
      expect(within(container).getByRole('button', { name: /delete/i })).toBeInTheDocument(),
    );
    await user.click(within(container).getByRole('button', { name: /delete/i }));
    await waitFor(() => expect(api.vault.deleteKey).toHaveBeenCalledWith('openai'));
    confirmSpy.mockRestore();
  });

  it('does not delete when the confirmation is declined', async () => {
    const user = userEvent.setup();
    const confirmSpy = vi.spyOn(window, 'confirm').mockReturnValue(false);
    const { container } = Harness({
      ui: <VaultKeyDetail />,
      path: '/vault/openai',
      pattern: '/vault/:key',
    });
    await waitFor(() =>
      expect(within(container).getByRole('button', { name: /delete/i })).toBeInTheDocument(),
    );
    await user.click(within(container).getByRole('button', { name: /delete/i }));
    expect(api.vault.deleteKey).not.toHaveBeenCalled();
    confirmSpy.mockRestore();
  });

  it('raises a toast when the delete is rejected', async () => {
    const user = userEvent.setup();
    const confirmSpy = vi.spyOn(window, 'confirm').mockReturnValue(true);
    api.vault.deleteKey.mockRejectedValue(new Error('delete refused'));
    const { container } = Harness({
      ui: <VaultKeyDetail />,
      path: '/vault/openai',
      pattern: '/vault/:key',
    });
    await waitFor(() =>
      expect(within(container).getByRole('button', { name: /delete/i })).toBeInTheDocument(),
    );
    await user.click(within(container).getByRole('button', { name: /delete/i }));
    await waitFor(() => expect(useToastsStore.getState().toasts[0]?.kind).toBe('error'));
    confirmSpy.mockRestore();
  });
});

describe('VaultDiff', () => {
  // `VaultDiff` reads base/head from `window.location.search` directly
  // (its own `useQueryParam`), not from the router location — so the test
  // has to drive the real URL, not just MemoryRouter's initialEntries.
  const setUrl = (search: string) =>
    window.history.replaceState({}, '', `/vault/openai/diff${search}`);

  afterEach(() => window.history.replaceState({}, '', '/'));

  it('shows an empty state when there are fewer than two versions', async () => {
    api.vault.getVersions.mockResolvedValue({ key: 'openai', versions: [versions.versions[0]] });
    const { container } = Harness({
      ui: <VaultDiff />,
      path: '/vault/openai/diff',
      pattern: '/vault/:key/diff',
    });
    await waitFor(() => expect(within(container).getByRole('status')).toBeInTheDocument());
  });

  it('requests the diff for an ascending base/head pair from the query string', async () => {
    setUrl('?base=1&head=2');
    Harness({ ui: <VaultDiff />, path: '/vault/openai/diff', pattern: '/vault/:key/diff' });
    await waitFor(() => expect(api.vault.diffVersions).toHaveBeenCalledWith('openai', 1, 2));
  });

  it('does not run the diff when the query string is absent', async () => {
    setUrl('');
    Harness({ ui: <VaultDiff />, path: '/vault/openai/diff', pattern: '/vault/:key/diff' });
    // Let the versions query settle, then confirm the diff stayed disabled.
    await waitFor(() => expect(api.vault.getVersions).toHaveBeenCalled());
    // `[FACT]` The sleep is a negative-assertion guard — it gives the
    // versions query time to resolve so that "not called" means "never
    // called" rather than "not called yet". It runs inside `act` because
    // that resolution re-renders `VaultDiff`, and a bare timer leaves
    // those updates outside React's act environment.
    await act(async () => {
      await new Promise((r) => setTimeout(r, 20));
    });
    expect(api.vault.diffVersions).not.toHaveBeenCalled();
  });

  it('does not run the diff when base is not lower than head', async () => {
    setUrl('?base=2&head=1');
    Harness({ ui: <VaultDiff />, path: '/vault/openai/diff', pattern: '/vault/:key/diff' });
    await waitFor(() => expect(api.vault.getVersions).toHaveBeenCalled());
    // `[FACT]` The sleep is a negative-assertion guard — it gives the
    // versions query time to resolve so that "not called" means "never
    // called" rather than "not called yet". It runs inside `act` because
    // that resolution re-renders `VaultDiff`, and a bare timer leaves
    // those updates outside React's act environment.
    await act(async () => {
      await new Promise((r) => setTimeout(r, 20));
    });
    expect(api.vault.diffVersions).not.toHaveBeenCalled();
  });

  it('does not run the diff for a zero or negative version number', async () => {
    setUrl('?base=0&head=2');
    Harness({ ui: <VaultDiff />, path: '/vault/openai/diff', pattern: '/vault/:key/diff' });
    await waitFor(() => expect(api.vault.getVersions).toHaveBeenCalled());
    // `[FACT]` The sleep is a negative-assertion guard — it gives the
    // versions query time to resolve so that "not called" means "never
    // called" rather than "not called yet". It runs inside `act` because
    // that resolution re-renders `VaultDiff`, and a bare timer leaves
    // those updates outside React's act environment.
    await act(async () => {
      await new Promise((r) => setTimeout(r, 20));
    });
    expect(api.vault.diffVersions).not.toHaveBeenCalled();
  });

  it('surfaces an error state when the diff query fails', async () => {
    setUrl('?base=1&head=2');
    api.vault.diffVersions.mockRejectedValue(new Error('diff exploded'));
    const { container } = Harness({
      ui: <VaultDiff />,
      path: '/vault/openai/diff',
      pattern: '/vault/:key/diff',
    });
    await waitFor(() => expect(within(container).getByRole('alert')).toBeInTheDocument());
  });
});

describe('VaultRestore', () => {
  it('waits for the version timeline', async () => {
    api.vault.getVersions.mockReturnValue(new Promise(() => {}));
    const { container } = Harness({
      ui: <VaultRestore />,
      path: '/vault/openai/restore',
      pattern: '/vault/:key/restore',
    });
    expect(within(container).getByRole('status')).toBeInTheDocument();
  });

  it('asks the API for the decoded key versions', async () => {
    Harness({
      ui: <VaultRestore />,
      path: '/vault/team%2Fkey/restore',
      pattern: '/vault/:key/restore',
    });
    await waitFor(() => expect(api.vault.getVersions).toHaveBeenCalledWith('team/key'));
  });

  it('shows an empty state when there are no versions to restore to', async () => {
    api.vault.getVersions.mockResolvedValue({ key: 'openai', versions: [] });
    const { container } = Harness({
      ui: <VaultRestore />,
      path: '/vault/openai/restore',
      pattern: '/vault/:key/restore',
    });
    await waitFor(() => expect(within(container).getByRole('status')).toBeInTheDocument());
  });

  it('restores the version named in the target query param', async () => {
    const user = userEvent.setup();
    const { container } = Harness({
      ui: <VaultRestore />,
      path: '/vault/openai/restore?target=1',
      pattern: '/vault/:key/restore',
    });
    const button = await within(container).findByRole('button', { name: /restore/i });
    await user.click(button);
    await waitFor(() =>
      expect(api.vault.restoreVersion).toHaveBeenCalledWith('openai', { target_version: 1 }),
    );
  });

  it('defaults to the first version when no target is given', async () => {
    const user = userEvent.setup();
    const { container } = Harness({
      ui: <VaultRestore />,
      path: '/vault/openai/restore',
      pattern: '/vault/:key/restore',
    });
    const button = await within(container).findByRole('button', { name: /restore/i });
    await user.click(button);
    await waitFor(() => expect(api.vault.restoreVersion).toHaveBeenCalled());
    const [, body] = api.vault.restoreVersion.mock.calls[0] as [string, { target_version: number }];
    expect(body.target_version).toBe(1);
  });

  it('raises an error toast when the restore is rejected', async () => {
    const user = userEvent.setup();
    api.vault.restoreVersion.mockRejectedValue(new Error('restore refused'));
    const { container } = Harness({
      ui: <VaultRestore />,
      path: '/vault/openai/restore',
      pattern: '/vault/:key/restore',
    });
    const button = await within(container).findByRole('button', { name: /restore/i });
    await user.click(button);
    await waitFor(() => expect(useToastsStore.getState().toasts[0]?.kind).toBe('error'));
  });
});
