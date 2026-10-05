/**
 * Tests for the `/repos` route component (`src/routes/Repos.svelte`).
 *
 * This file was 0% covered. The gaps the cases below close are the ones a
 * smoke test cannot see:
 *
 *  - the search box filters the *grid* (a `$derived` over `$repos`, not a
 *    backend query), and the count line follows the filter rather than the
 *    list length,
 *  - an empty result is worded as "no data", so a search that matches
 *    nothing must not read as "you have no repositories",
 *  - the two per-card buttons reach different commands — `clone_url` (then
 *    the clipboard) and `open_repo_in_shell` — and only the first one
 *    reports success; a failure on the second is surfaced, not swallowed,
 *  - a failed refresh keeps the list already on screen instead of blanking
 *    it, which is what `refreshRepos` is written to do.
 *
 * `[FACT]` Every assertion is scoped to the card under test: with two or
 * more repositories on screen the copy/open labels repeat once per card,
 * so an unscoped `getByRole('button', { name })` is ambiguous.
 */
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { get } from 'svelte/store';
import { cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import Repos from '../../src/routes/Repos.svelte';
import { repos } from '../../src/lib/stores/repos';
import { toasts } from '../../src/lib/stores/toasts';
import type { RepoSummary } from '../../src/lib/api/types';
import { callsTo, cat, deferred, resetRouteStores, useInvoke } from './routes-harness';

const LIST: RepoSummary[] = [
  { name: 'alpha', path: '/srv/gitgit/alpha.git', default_branch: 'main', size_bytes: 512 },
  { name: 'beta', path: '/srv/gitgit/beta.git', default_branch: 'trunk', size_bytes: 20480 },
];

/** `formatBytes` output for the fixture sizes, measured, not recomputed. */
const ALPHA_SIZE = '512 B';
const BETA_SIZE = '20.0 KB';

/** Install a clipboard that reports success, and record what it was given. */
function stubClipboard(): ReturnType<typeof vi.fn> {
  const writeText = vi.fn(async () => {});
  Object.defineProperty(navigator, 'clipboard', { value: { writeText }, configurable: true });
  return writeText;
}

function card(name: string): HTMLElement {
  return screen.getByTestId(`repo-${name}`);
}

/** Copy / open / detail control of one card, by its catalogue label. */
function cardButton(name: string, key: string): HTMLElement {
  return within(card(name)).getByRole('button', { name: cat(key) });
}

beforeEach(() => {
  resetRouteStores();
});

afterEach(() => {
  cleanup();
  delete (navigator as unknown as Record<string, unknown>)['clipboard'];
  vi.restoreAllMocks();
});

describe('route / repos — the list', () => {
  it('renders one card per repository, with the fields the user picks from', async () => {
    useInvoke({ list_repos: () => LIST });
    render(Repos);
    await screen.findByTestId('repo-alpha');

    const alpha = card('alpha');
    // Name, default branch, path and size: everything the card claims to
    // show. A card that dropped the branch pill would leave the user
    // guessing which branch a clone lands on.
    expect(within(alpha).getByText('alpha')).toBeTruthy();
    expect(within(alpha).getByText('main')).toBeTruthy();
    expect(within(alpha).getByText('/srv/gitgit/alpha.git')).toBeTruthy();
    expect(within(alpha).getByText(ALPHA_SIZE)).toBeTruthy();
    // The path is also the tooltip, because the visible text is clipped.
    expect(within(alpha).getByTitle('/srv/gitgit/alpha.git')).toBeTruthy();

    expect(within(card('beta')).getByText('trunk')).toBeTruthy();
    expect(within(card('beta')).getByText(BETA_SIZE)).toBeTruthy();
    expect(screen.queryByTestId('repos-empty')).toBeNull();
  });

  it('counts what it is showing, not what the backend returned', async () => {
    useInvoke({ list_repos: () => LIST });
    render(Repos);
    await screen.findByTestId('repo-alpha');

    // `{n}` interpolated, never left as a literal placeholder.
    expect(screen.getByTestId('repos-count').textContent).toBe(
      cat('repos.countOne').replace('{n}', '2')
    );
    expect(screen.getByTestId('repos-count').textContent).not.toContain('{n}');
  });

  it('links to the detail page through the hash router, from both links', async () => {
    // Without the `#` prefix these anchors trigger a real page load and
    // throw the app state away; the name link and the "Detail" button are
    // two separate elements, so both are checked.
    useInvoke({ list_repos: () => LIST });
    render(Repos);
    await screen.findByTestId('repo-beta');

    const links = Array.from(card('beta').querySelectorAll('a'));
    expect(links).toHaveLength(2);
    for (const a of links) {
      expect(a.getAttribute('href'), a.textContent ?? '').toBe('#/repos/beta');
    }
  });

  it('filters the grid from the search box without asking the backend again', async () => {
    const seen = useInvoke({ list_repos: () => LIST });
    render(Repos);
    await screen.findByTestId('repo-alpha');

    await fireEvent.input(screen.getByTestId('repos-search'), { target: { value: 'BETA' } });

    // Case-insensitive: the input is lower-cased on both sides.
    expect(screen.queryByTestId('repo-beta')).not.toBeNull();
    expect(screen.queryByTestId('repo-alpha')).toBeNull();
    expect(screen.getByTestId('repos-count').textContent).toBe(
      cat('repos.countOne').replace('{n}', '1')
    );
    // The filter is a client-side `$derived`, so no second list command.
    expect(callsTo(seen, 'list_repos')).toHaveLength(1);
  });

  it('words a search that matches nothing as no data, not as no repositories', async () => {
    // The dangerous failure is a search box that makes the user believe
    // their repositories are gone, so the two are kept distinguishable.
    useInvoke({ list_repos: () => LIST });
    render(Repos);
    await screen.findByTestId('repo-alpha');

    await fireEvent.input(screen.getByTestId('repos-search'), {
      target: { value: 'no-such-repo' },
    });

    expect(screen.getByTestId('repos-empty').textContent?.trim()).toBe(cat('common.empty'));
    expect(screen.queryByTestId('repos-grid')).toBeNull();
    expect(screen.getByTestId('repos-count').textContent).toBe(
      cat('repos.countOne').replace('{n}', '0')
    );
  });

  it('says so plainly when the backend returned no repositories at all', async () => {
    useInvoke({ list_repos: () => [] });
    render(Repos);

    await waitFor(() => expect(screen.getByTestId('repos-empty')).toBeTruthy());
    expect(get(repos)).toEqual([]);
    expect(screen.queryByTestId('repos-grid')).toBeNull();
  });

  it('disables the refresh control while the list is in flight', async () => {
    // A double click would issue two `list_repos` calls and let the
    // slower one win with a stale list.
    const gate = deferred<RepoSummary[]>();
    useInvoke({ list_repos: () => gate.promise });
    render(Repos);

    const refresh = screen.getByRole('button', { name: cat('common.refresh') });
    expect(refresh.hasAttribute('disabled')).toBe(true);

    gate.resolve(LIST);
    await screen.findByTestId('repo-alpha');
    // The store is updated inside `refreshRepos`, before `onMount` clears
    // its own `loading` flag, so the card can appear a tick before the
    // control comes back. Wait for the control, not for the card.
    await waitFor(() => expect(refresh.hasAttribute('disabled')).toBe(false));
  });

  it('keeps the list already on screen when a refresh fails', async () => {
    // `refreshRepos` swallows the failure on purpose. The observable
    // consequence is that a transient error does not blank the page.
    let attempt = 0;
    useInvoke({
      list_repos: () => {
        attempt += 1;
        return attempt === 1 ? LIST : Promise.reject(new Error('gitgit: connection reset'));
      },
    });
    render(Repos);
    await screen.findByTestId('repo-alpha');

    await fireEvent.click(screen.getByRole('button', { name: cat('common.refresh') }));
    await waitFor(() =>
      expect(screen.getByRole('button', { name: cat('common.refresh') }).hasAttribute('disabled')).toBe(
        false
      )
    );

    expect(screen.getByTestId('repo-alpha')).toBeTruthy();
    expect(screen.getByTestId('repos-count').textContent).toBe(
      cat('repos.countOne').replace('{n}', '2')
    );
  });
});

describe('route / repos — the per-card actions', () => {
  it('copies the clone URL the backend computed, and says it did', async () => {
    const writeText = stubClipboard();
    const seen = useInvoke({ list_repos: () => LIST });
    render(Repos);
    await screen.findByTestId('repo-alpha');

    await fireEvent.click(cardButton('alpha', 'repos.copyCloneUrl'));

    // The URL comes from `clone_url`, not from string-building in the
    // page: the backend knows the server's real bind address. The
    // handler is async — `clone_url` first, then the clipboard — so the
    // click itself cannot be the sync point.
    await waitFor(() => expect(callsTo(seen, 'clone_url')).toHaveLength(1));
    expect(callsTo(seen, 'clone_url')[0]?.args).toMatchObject({ name: 'alpha' });
    await waitFor(() =>
      expect(writeText).toHaveBeenCalledWith('http://127.0.0.1:38080/repos/alpha.git')
    );
    await waitFor(() =>
      expect(get(toasts).map((t) => [t.kind, t.message])).toEqual([
        ['success', cat('common.copiedToClipboard')],
      ])
    );
  });

  it('reports a refused copy as an error rather than a success', async () => {
    // The copy can legitimately fail — a denied clipboard permission is
    // the common case in a WebView2 build. Claiming success there is the
    // bug this pins.
    Object.defineProperty(navigator, 'clipboard', {
      value: {
        writeText: vi.fn(async () => {
          throw new Error('Write permission denied');
        }),
      },
      configurable: true,
    });
    Object.defineProperty(document, 'execCommand', {
      value: vi.fn(() => false),
      configurable: true,
      writable: true,
    });
    useInvoke({ list_repos: () => LIST });
    render(Repos);
    await screen.findByTestId('repo-alpha');

    await fireEvent.click(cardButton('alpha', 'repos.copyCloneUrl'));

    await waitFor(() =>
      expect(get(toasts).map((t) => t.kind)).toEqual(['error'])
    );
    expect(get(toasts)[0]?.message).toBe(cat('common.copiedToClipboard'));
    delete (document as unknown as Record<string, unknown>)['execCommand'];
  });

  it('opens a repository in the file manager, quietly on success', async () => {
    const seen = useInvoke({ list_repos: () => LIST });
    render(Repos);
    await screen.findByTestId('repo-beta');

    await fireEvent.click(cardButton('beta', 'repos.openInFinder'));

    await waitFor(() => expect(callsTo(seen, 'open_repo_in_shell')).toHaveLength(1));
    expect(callsTo(seen, 'open_repo_in_shell')[0]?.args).toMatchObject({ name: 'beta' });
    // Nothing happened that the user needs to be told about.
    expect(get(toasts)).toEqual([]);
  });

  it('surfaces a refused file-manager launch instead of failing silently', async () => {
    // `onOpen` catches and reports; an unhandled rejection here would
    // leave the user clicking a button that does nothing.
    useInvoke({
      list_repos: () => LIST,
      open_repo_in_shell: () => Promise.reject(new Error('no file manager on this host')),
    });
    render(Repos);
    await screen.findByTestId('repo-alpha');

    await fireEvent.click(cardButton('alpha', 'repos.openInFinder'));

    await waitFor(() => expect(get(toasts)).toHaveLength(1));
    expect(get(toasts)[0]?.kind).toBe('error');
    expect(get(toasts)[0]?.message).toContain('no file manager on this host');
  });
});
