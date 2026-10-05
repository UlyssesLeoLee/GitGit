/**
 * Tests for the `/repos/:name` route component (`src/routes/RepoDetail.svelte`).
 *
 * This file was 0% covered. The cases below pin the parts of the page
 * that answer a question rather than decorate it:
 *
 *  - the three mutually exclusive states — loading / not found / loaded —
 *    and that "not found" names the repository it could not find,
 *  - the ref list grouped by family (local / remote / tag) with an
 *    explicit placeholder for a family that has no refs, so an empty
 *    group is not confused with a missing one,
 *  - the branch-graph block, whose prefix characters are the only thing
 *    distinguishing a local ref from a remote one there,
 *  - and the clone-URL button, which does nothing at all until the detail
 *    has arrived.
 *
 * `[FACT]` The heading is a single `<h1>` that swaps between the
 * repository name and the loading string, so it is queried by role
 * rather than by text — and the page's own loading card renders the same
 * catalogue string, so a bare `getByText` on it is ambiguous.
 */
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { get } from 'svelte/store';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import RepoDetail from '../../src/routes/RepoDetail.svelte';
import { toasts } from '../../src/lib/stores/toasts';
import type { RepoDetail as RepoDetailDto } from '../../src/lib/api/types';
import { cat, deferred, resetRouteStores, useInvoke } from './routes-harness';

const ALPHA: RepoDetailDto = {
  name: 'alpha',
  path: '/srv/gitgit/alpha.git',
  default_branch: 'main',
  refs: [
    { sha: '1111111111111111111111111111111111111111', name: 'refs/heads/main', kind: 'local' },
    { sha: '2222222222222222222222222222222222222222', name: 'refs/heads/feature/x', kind: 'local' },
    { sha: '3333333333333333333333333333333333333333', name: 'refs/remotes/origin/main', kind: 'remote' },
    { sha: '4444444444444444444444444444444444444444', name: 'refs/tags/v0.1.0', kind: 'tag' },
  ],
  commits: [
    {
      sha: 'aaaaaaaabbbbbbbbccccccccdddddddd',
      short_sha: 'aaaaaaaabbbbbbbbccccccccdddddddd',
      author: 'Ulysses',
      email: 'ulysses@example.com',
      message: 'feat: ingest requirement IDs',
      date_iso: '2026-09-19T12:34:56+09:00',
    },
  ],
};

const EMPTY: RepoDetailDto = {
  name: 'bare',
  path: '/srv/gitgit/bare.git',
  default_branch: 'main',
  refs: [],
  commits: [],
};

/** Install a clipboard that reports success, and record what it was given. */
function stubClipboard(): ReturnType<typeof vi.fn> {
  const writeText = vi.fn(async () => {});
  Object.defineProperty(navigator, 'clipboard', { value: { writeText }, configurable: true });
  return writeText;
}

/** The `<ul>` under the "local" / "remote" / "tag" sub-heading. */
function refGroup(family: string): HTMLElement {
  const heading = screen
    .getAllByRole('heading', { level: 3 })
    .find((h) => h.textContent?.trim() === family);
  if (!heading?.nextElementSibling) throw new Error(`no ref group for ${family}`);
  return heading.nextElementSibling as HTMLElement;
}

function heading(): string {
  return screen.getByRole('heading', { level: 1 }).textContent?.trim() ?? '';
}

/**
 * The not-found card's message, byte for byte as it is rendered.
 *
 * `[FACT]` `src/routes/RepoDetail.svelte:75` closes its Svelte
 * expression twice — `…replace('{name}', params?.name ?? '')}}` — so the
 * surplus brace is emitted as literal text and the page reads
 * "Repository ghost not found}". Measured, not inferred. That is a real
 * source defect; it is reported rather than fixed from this lane, and
 * these expectations pin today's output so a fix has to be a
 * deliberate, visible change to them.
 */
async function notFoundMessage(): Promise<string> {
  const stem = cat('repos.notFound').split('{name}')[0]?.trim() ?? '';
  const p = await screen.findByText((text) => text.includes(stem));
  return p.textContent ?? '';
}

beforeEach(() => {
  resetRouteStores();
});

afterEach(() => {
  cleanup();
  delete (navigator as unknown as Record<string, unknown>)['clipboard'];
  vi.restoreAllMocks();
});

describe('route / repo detail — the three states', () => {
  it('shows the loading state until the detail answers, then the name', async () => {
    const gate = deferred<RepoDetailDto | null>();
    useInvoke({ repo_detail: () => gate.promise });
    render(RepoDetail, { props: { params: { name: 'alpha' } } });

    // The page must not claim to know the repository before it does.
    expect(heading()).toBe(cat('common.loading'));
    expect(screen.queryByTestId('repo-worktree')).toBeNull();

    gate.resolve(ALPHA);
    await waitFor(() => expect(heading()).toBe('alpha'));
    expect(screen.getByTestId('repo-worktree')).toBeTruthy();
  });

  it('names the repository it could not find', async () => {
    // `repoDetail` answers `null` for an unknown name. A generic "not
    // found" here would leave the user guessing which of several
    // repositories the link was pointing at.
    useInvoke({ repo_detail: () => null });
    render(RepoDetail, { props: { params: { name: 'ghost' } } });

    // Trailing `}` included on purpose — see `notFoundMessage`.
    expect(await notFoundMessage()).toBe(
      cat('repos.notFound').replace('{name}', 'ghost') + '}'
    );
    // The refs and commits halves stay away: there is nothing to show.
    expect(screen.queryByRole('heading', { name: cat('repos.refsHeading') })).toBeNull();
    expect(screen.queryByTestId('repo-worktree')).toBeNull();
  });

  it('treats a missing route param as not found instead of loading forever', async () => {
    // `/repos/` with no segment reaches this component with no `params`
    // at all. `[FACT]` The heading still reads the loading string in
    // that state — it is `{#if detail}{name}{:else}{loading}{/if}` and
    // `detail` is null on the not-found path — so the page says
    // "Loading…" and "not found" at the same time. That is a real defect
    // in `RepoDetail.svelte`, reported rather than fixed here; the
    // assertion pins today's behaviour so a fix has to be a deliberate
    // change to this case.
    const seen = useInvoke();
    render(RepoDetail);

    expect(await notFoundMessage()).toBe(
      cat('repos.notFound').replace('{name}', '') + '}'
    );
    // No name means no question to ask the backend.
    expect(seen.filter((c) => c.cmd === 'repo_detail')).toHaveLength(0);
    expect(heading()).toBe(cat('common.loading'));
  });
});

describe('route / repo detail — refs and commits', () => {
  it('groups the refs by family and shows a placeholder for an empty one', async () => {
    useInvoke({ repo_detail: () => ALPHA });
    render(RepoDetail, { props: { params: { name: 'alpha' } } });
    await waitFor(() => expect(heading()).toBe('alpha'));

    // Local: two refs, each as a 10-char short sha plus its full name.
    const local = refGroup('local');
    expect(local.textContent).toContain('refs/heads/main');
    expect(local.textContent).toContain('1111111111');
    expect(local.textContent).toContain('refs/heads/feature/x');
    expect(local.textContent).toContain('2222222222');
    // Remote and tag each have exactly one here.
    expect(refGroup('remote').textContent).toContain('refs/remotes/origin/main');
    expect(refGroup('tag').textContent).toContain('refs/tags/v0.1.0');
    // A ref belongs to exactly one family: the same name must not be
    // listed twice just because two kinds matched.
    expect(refGroup('local').textContent).not.toContain('origin/main');
  });

  it('renders an em dash, not an empty box, for a family with no refs', async () => {
    // A remote-less repository is normal. An empty `<ul>` would read as
    // a rendering failure.
    const noRemote: RepoDetailDto = {
      ...ALPHA,
      refs: [{ sha: '1111111111111111111111111111111111111111', name: 'refs/heads/main', kind: 'local' }],
    };
    useInvoke({ repo_detail: () => noRemote });
    render(RepoDetail, { props: { params: { name: 'alpha' } } });
    await waitFor(() => expect(heading()).toBe('alpha'));

    expect(refGroup('remote').textContent?.trim()).toBe('—');
    expect(refGroup('tag').textContent?.trim()).toBe('—');
    // The families that do have refs are unaffected.
    expect(refGroup('local').textContent).toContain('refs/heads/main');
  });

  it('says so plainly when the repository has no refs or commits at all', async () => {
    useInvoke({ repo_detail: () => EMPTY });
    render(RepoDetail, { props: { params: { name: 'bare' } } });
    await waitFor(() => expect(heading()).toBe('bare'));

    expect(screen.getByText(cat('repos.refs.none'))).toBeTruthy();
    expect(screen.getByText(cat('repos.commits.none'))).toBeTruthy();
    // Neither the per-family breakdown nor the graph block: there is
    // nothing to break down.
    expect(refGroupOrNull('local')).toBeNull();
    expect(screen.queryByText('refs/heads/main')).toBeNull();
  });

  it('lists the recent commits with their short sha, message and author', async () => {
    useInvoke({ repo_detail: () => ALPHA });
    render(RepoDetail, { props: { params: { name: 'alpha' } } });
    await waitFor(() => expect(heading()).toBe('alpha'));

    const item = screen.getByText('feat: ingest requirement IDs').closest('li') as HTMLElement;
    // 12 characters, per `shortSha(c.sha, 12)`.
    expect(item.textContent).toContain('aaaaaaaabbbb');
    expect(item.textContent).toContain('Ulysses · 2026-09-19T12:34:56+09:00');
    expect(screen.queryByText(cat('repos.commits.none'))).toBeNull();
  });

  it('draws the branch graph with a prefix per ref family', async () => {
    // The `<pre>` is the only place a local ref and a remote ref are
    // told apart, so the prefixes are the behaviour, not decoration.
    useInvoke({ repo_detail: () => ALPHA });
    render(RepoDetail, { props: { params: { name: 'alpha' } } });
    await waitFor(() => expect(heading()).toBe('alpha'));

    const pre = screen.getByText(cat('repos.branchGraph')).closest('details')?.querySelector('pre');
    // The `<pre>` is written across several source lines, so it carries
    // blank leading / trailing lines. Only those are dropped — the two
    // leading spaces that mark a local ref are the behaviour under test
    // and `trim()` would eat them.
    const lines = (pre?.textContent ?? '').split('\n');
    while (lines.length > 0 && lines[0]?.trim() === '') lines.shift();
    while (lines.length > 0 && lines[lines.length - 1]?.trim() === '') lines.pop();
    // Shortest sha in the block: 7 characters.
    expect(lines).toEqual([
      '  1111111 refs/heads/main',
      '  2222222 refs/heads/feature/x',
      '~ 3333333 refs/remotes/origin/main',
      '* 4444444 refs/tags/v0.1.0',
    ]);
  });
});

describe('route / repo detail — the header controls', () => {
  it('links back to the list through the hash router', async () => {
    useInvoke({ repo_detail: () => ALPHA });
    render(RepoDetail, { props: { params: { name: 'alpha' } } });

    // The anchor's own text is "← Back to list", so the label is matched
    // by predicate rather than by exact string.
    const back = screen.getByText((text) => text.includes(cat('repos.back')));
    // `#/repos` and not `/repos`: the `#` is the whole contract with the
    // local router, and a plain path would be a full page load.
    expect(back.getAttribute('href')).toBe('#/repos');
  });

  it('copies the clone URL for the repository it is showing', async () => {
    const writeText = stubClipboard();
    useInvoke({ repo_detail: () => ALPHA });
    render(RepoDetail, { props: { params: { name: 'alpha' } } });
    await waitFor(() => expect(heading()).toBe('alpha'));

    await fireEvent.click(screen.getByRole('button', { name: cat('repos.copyCloneUrl') }));

    // `[FACT]` The URL is built in the component with a hardcoded bind
    // address, not asked of the backend the way `Repos.svelte` does — so
    // it is wrong for an install that bound a different port. Reported,
    // not fixed here; the assertion pins what the page really does.
    await waitFor(() =>
      expect(writeText).toHaveBeenCalledWith('http://127.0.0.1:38080/repos/alpha.git')
    );
    await waitFor(() =>
      expect(get(toasts).map((t) => [t.kind, t.message])).toEqual([
        ['success', cat('common.copiedToClipboard')],
      ])
    );
  });

  it('does nothing when the copy is asked for before the detail arrives', async () => {
    // The button is live during the loading state, and there is no name
    // to put in a URL yet. It must be a no-op, not a copy of a URL
    // ending in "undefined".
    const writeText = stubClipboard();
    const gate = deferred<RepoDetailDto | null>();
    useInvoke({ repo_detail: () => gate.promise });
    render(RepoDetail, { props: { params: { name: 'alpha' } } });

    await fireEvent.click(screen.getByRole('button', { name: cat('repos.copyCloneUrl') }));

    expect(writeText).not.toHaveBeenCalled();
    expect(get(toasts)).toEqual([]);

    gate.resolve(ALPHA);
    await waitFor(() => expect(heading()).toBe('alpha'));
  });
});

/** The ref `<ul>` for a family, or `null` when the breakdown is absent. */
function refGroupOrNull(family: string): HTMLElement | null {
  const headingEl = screen
    .getAllByRole('heading', { level: 3 })
    .find((h) => h.textContent?.trim() === family);
  return (headingEl?.nextElementSibling as HTMLElement | undefined) ?? null;
}
