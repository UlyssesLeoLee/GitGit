/**
 * Store + component tests for the working-tree status and diff view
 * (V0 task T4).
 *
 * The three cases that matter are the three answers, not three code
 * paths: a repository with changes, a repository without any (a clean
 * tree is a real answer and must not look like an error), and a refused
 * call whose typed kind drives the message. Each gets both a store case
 * and a render case, because the store can be right while the page still
 * shows nothing.
 */
import { describe, expect, it, beforeEach, afterEach } from 'vitest';
import { get } from 'svelte/store';
import { render, screen, fireEvent, cleanup, waitFor } from '@testing-library/svelte';
import {
  worktree,
  diff,
  loadStatus,
  loadDiff,
  resetWorktree,
  stagedEntries,
  unstagedEntries,
  untrackedEntries,
} from '../../src/lib/stores/worktree';
import type { RepoStatus } from '../../src/lib/api/types';
import RepoWorktree from '../../src/lib/components/RepoWorktree.svelte';
import { installMock } from '../../src/mocks/handlers';
import { tFor, locale } from '../../src/lib/i18n';

beforeEach(() => {
  cleanup();
  installMock();
  resetWorktree();
});

afterEach(() => {
  cleanup();
});

/**
 * Replace the invoke bridge so one test can drive the exact payload the
 * Rust side would produce. The real refusal paths live in Rust; this only
 * needs their wire shape.
 */
function respondWith(payloads: Record<string, unknown>): void {
  window.__TAURI_INTERNALS__ = {
    invoke: (cmd: string) => {
      if (cmd in payloads) return Promise.resolve(payloads[cmd]);
      return Promise.reject({ kind: 'Unknown', message: `no fixture for ${cmd}`, source: '' });
    },
  };
}

function rejectsWith(rejection: unknown): void {
  window.__TAURI_INTERNALS__ = { invoke: () => Promise.reject(rejection) };
}

const DIRTY: RepoStatus = {
  branch: 'main',
  head: 'a1b2c3d',
  upstream: 'origin/main',
  ahead: 1,
  behind: 0,
  is_clean: false,
  entries: [
    { path: 'staged.txt', orig_path: null, index_status: 'A', worktree_status: null, staged: true, unstaged: false, untracked: false },
    { path: 'src/dirty.ts', orig_path: null, index_status: null, worktree_status: 'M', staged: false, unstaged: true, untracked: false },
    { path: 'notes.txt', orig_path: 'old notes.txt', index_status: '?', worktree_status: '?', staged: false, unstaged: false, untracked: true },
  ],
};

const DIFF_TEXT = [
  'diff --git a/src/dirty.ts b/src/dirty.ts',
  'index 1234567..89abcde 100644',
  '--- a/src/dirty.ts',
  '+++ b/src/dirty.ts',
  '@@ -1 +1,2 @@',
  ' const before = 1;',
  '+const after = 2;',
].join('\n');

describe('store / worktree — status', () => {
  it('separates staged, unstaged and untracked entries', async () => {
    respondWith({ repo_status: DIRTY });
    await loadStatus('demo');
    const s = get(worktree);
    expect(s.status).toBe('dirty');
    expect(stagedEntries(s.entries).map((e) => e.path)).toEqual(['staged.txt']);
    expect(unstagedEntries(s.entries).map((e) => e.path)).toEqual(['src/dirty.ts']);
    expect(untrackedEntries(s.entries).map((e) => e.path)).toEqual(['notes.txt']);
  });

  it('treats a clean tree as an answer, not an error', async () => {
    // The distinction that a naive `entries.length === 0` check gets
    // wrong: empty means clean here, while a failed call means error.
    respondWith({ repo_status: { ...DIRTY, is_clean: true, entries: [] } });
    await loadStatus('demo');
    const s = get(worktree);
    expect(s.status).toBe('clean');
    expect(s.entries).toEqual([]);
    expect(s.errorKind).toBeNull();
  });

  it('keeps the typed kind of a refusal so the page can pick a message', async () => {
    rejectsWith({
      kind: 'NotAWorkTree',
      message: 'demo is a bare repository: it has no working tree',
      source: '"NotAWorkTree"',
    });
    await loadStatus('demo');
    const s = get(worktree);
    expect(s.status).toBe('error');
    expect(s.errorKind).toBe('NotAWorkTree');
    // The raw message can name a host path, so it is kept out of the
    // headline and only the kind decides what the user is told.
    expect(s.errorMessage).toContain('bare repository');
  });

  it('survives a rejection that is not an AppError payload', async () => {
    // `[FACT]` This used to assert `errorKind` was `'Unknown'`. That was
    // the fallback kind in the store's own `readError`, and it is now
    // `Internal`, because the store calls `normalizeError` and
    // `Internal` is the one fallback kind with a catalogue entry
    // (`errors.kind.Internal`). The old value was not a rendering bug —
    // `RepoWorktree.svelte`'s `errorHeadline()` falls through to
    // `repos.error.generic` for any unrecognised kind, so the page
    // degraded gracefully either way. This is a consistency change, and
    // the message is the part that was actually broken; see below.
    rejectsWith(new Error('bridge is gone'));
    await loadStatus('demo');
    expect(get(worktree).status).toBe('error');
    expect(get(worktree).errorKind).toBe('Internal');
    // The thrown `Error`'s own words survive, which the old fallback
    // also managed — but only for this arm. See the next case.
    expect(get(worktree).errorMessage).toBe('bridge is gone');
  });

  it('keeps the message of a payload that carries no kind', async () => {
    // `[FACT]` The defect this pins: `readError` guarded on
    // `'kind' in err`, so a rejection with a `message` and no `kind`
    // fell through to `String(err)` and the detail line rendered the
    // literal `[object Object]`. That shape is not hypothetical — it is
    // what the mock layer and any non-Tauri caller produce, per
    // `utils/errors.ts`. `normalizeError` reads `kind` and `message`
    // independently, so the backend's own words now reach the page.
    rejectsWith({ message: 'the backend said this, with no kind' });
    await loadStatus('demo');

    const s = get(worktree);
    expect(s.status).toBe('error');
    expect(s.errorMessage).toBe('the backend said this, with no kind');
    expect(s.errorMessage).not.toBe('[object Object]');
    // No kind in the payload, so the catalogue-backed fallback is used.
    expect(s.errorKind).toBe('Internal');
  });

  it('reports an empty diff as empty rather than as an error', async () => {
    respondWith({
      repo_status: DIRTY,
      repo_diff: { target: 'worktree', path: null, text: '', truncated: false, untracked: false, files: 0 },
    });
    await loadDiff('demo', 'worktree');
    expect(get(diff).status).toBe('empty');
  });

  it('passes the target through instead of inferring one', async () => {
    const seen: string[] = [];
    window.__TAURI_INTERNALS__ = {
      invoke: (cmd: string, args?: Record<string, unknown>) => {
        if (cmd === 'repo_diff') {
          seen.push(String(args?.target));
          return Promise.resolve({
            target: String(args?.target),
            path: null,
            text: 'diff --git a/a b/a',
            truncated: false,
            untracked: false,
            files: 1,
          });
        }
        return Promise.resolve(DIRTY);
      },
    };
    await loadDiff('demo', 'staged');
    await loadDiff('demo', 'head');
    expect(seen).toEqual(['staged', 'head']);
  });
});

describe('component / RepoWorktree', () => {
  it('renders the three groups and the branch line for a dirty tree', async () => {
    respondWith({ repo_status: DIRTY });
    render(RepoWorktree, { props: { name: 'demo' } });

    await waitFor(() => expect(screen.getByTestId('wt-dirty')).toBeTruthy());
    expect(screen.getByTestId('wt-group-staged').textContent).toContain('staged.txt');
    expect(screen.getByTestId('wt-group-unstaged').textContent).toContain('src/dirty.ts');
    expect(screen.getByTestId('wt-group-untracked').textContent).toContain('notes.txt');
    // Interpolation must reach the rendered text, not the catalogue key.
    const branch = screen.getByTestId('wt-branch').textContent ?? '';
    expect(branch).toContain('main');
    expect(branch).not.toContain('{branch}');
    // A rename keeps its source path visible.
    expect(screen.getByTestId('wt-group-untracked').textContent).toContain('old notes.txt');
  });

  it('shows a clean tree as a real answer', async () => {
    respondWith({ repo_status: { ...DIRTY, is_clean: true, entries: [] } });
    render(RepoWorktree, { props: { name: 'demo' } });
    const clean = await screen.findByTestId('wt-clean');
    expect(clean.textContent?.trim()).toBe(tFor(get(locale), 'repos.statusClean'));
    expect(screen.queryByTestId('wt-error')).toBeNull();
    expect(screen.queryByTestId('wt-dirty')).toBeNull();
  });

  it('says what to do when the repository is bare', async () => {
    rejectsWith({
      kind: 'NotAWorkTree',
      message: 'demo is a bare repository: it has no working tree',
      source: '"NotAWorkTree"',
    });
    render(RepoWorktree, { props: { name: 'demo' } });
    const panel = await screen.findByTestId('wt-error');
    const detail = screen.getByTestId('wt-error-detail');
    // The headline is the remedy, not the raw git message.
    expect(detail.textContent?.trim()).toBe(tFor(get(locale), 'repos.error.notWorkTree'));
    expect(detail.textContent).not.toContain('bare repository: it has no working tree');
    expect(panel.textContent).toContain('bare repository');
    expect(screen.getByTestId('wt-retry')).toBeTruthy();
  });

  it('shows the diff text the store received, escaped as text', async () => {
    respondWith({
      repo_status: DIRTY,
      repo_diff: {
        target: 'worktree',
        path: 'src/dirty.ts',
        text: DIFF_TEXT + '\n+<img src=x onerror=alert(1)>\n',
        truncated: true,
        untracked: false,
        files: 1,
      },
    });
    render(RepoWorktree, { props: { name: 'demo' } });
    const pre = await screen.findByTestId('wt-diff-text');
    // Diff text is subprocess output: it must never become markup.
    expect(pre.textContent).toContain('+const after = 2;');
    expect(pre.textContent).toContain('<img src=x onerror=alert(1)>');
    expect(pre.querySelector('img')).toBeNull();
    expect(screen.getByTestId('wt-diff-truncated')).toBeTruthy();
  });

  it('labels a diff that was synthesized for an untracked file', async () => {
    // `git diff` reports nothing for an untracked file, so the backend
    // produces a new-file diff. The user must be told which it is.
    respondWith({
      repo_status: DIRTY,
      repo_diff: {
        target: 'worktree',
        path: 'notes.txt',
        text: 'diff --git a/notes.txt b/notes.txt\nnew file mode 100644\n@@ -0,0 +1 @@\n+first note\n',
        truncated: false,
        untracked: true,
        files: 1,
      },
    });
    render(RepoWorktree, { props: { name: 'demo' } });
    await waitFor(() => expect(screen.getByTestId('wt-diff-untracked')).toBeTruthy());
    expect(screen.getByTestId('wt-diff-text').textContent).toContain('new file mode 100644');
  });

  it('asks for the comparison the user picks', async () => {
    const seen: string[] = [];
    window.__TAURI_INTERNALS__ = {
      invoke: (cmd: string, args?: Record<string, unknown>) => {
        if (cmd === 'repo_diff') {
          seen.push(String(args?.target));
          return Promise.resolve({
            target: String(args?.target),
            path: args?.path == null ? null : String(args.path),
            text: 'diff --git a/a b/a',
            truncated: false,
            untracked: false,
            files: 1,
          });
        }
        return Promise.resolve(DIRTY);
      },
    };
    render(RepoWorktree, { props: { name: 'demo' } });
    await waitFor(() => expect(screen.getByTestId('wt-dirty')).toBeTruthy());

    await fireEvent.click(screen.getByTestId('wt-target-staged'));
    await waitFor(() => expect(seen).toContain('staged'));
    await fireEvent.click(screen.getByTestId('wt-target-head'));
    await waitFor(() => expect(seen).toContain('head'));
  });

  it('scopes the diff to a clicked entry, then back to all changes', async () => {
    const scoped: (string | null)[] = [];
    window.__TAURI_INTERNALS__ = {
      invoke: (cmd: string, args?: Record<string, unknown>) => {
        if (cmd === 'repo_diff') {
          scoped.push(args?.path == null ? null : String(args.path));
          return Promise.resolve({
            target: String(args?.target),
            path: args?.path == null ? null : String(args.path),
            text: 'diff --git a/x b/x',
            truncated: false,
            untracked: false,
            files: 1,
          });
        }
        return Promise.resolve(DIRTY);
      },
    };
    render(RepoWorktree, { props: { name: 'demo' } });
    await waitFor(() => expect(screen.getByTestId('wt-dirty')).toBeTruthy());

    const entry = screen
      .getAllByTestId('wt-entry')
      .find((el) => el.textContent?.includes('src/dirty.ts'));
    expect(entry).toBeTruthy();
    await fireEvent.click(entry as HTMLElement);
    await waitFor(() => expect(scoped).toContain('src/dirty.ts'));

    await fireEvent.click(screen.getByTestId('wt-view-all'));
    await waitFor(() => expect(scoped[scoped.length - 1]).toBeNull());
  });
});

describe('i18n for the status and diff surface', () => {
  const KEYS = [
    'repos.statusHeading',
    'repos.statusClean',
    'repos.statusDirty',
    'repos.statusLoading',
    'repos.status.staged',
    'repos.status.unstaged',
    'repos.status.untracked',
    'repos.status.none',
    'repos.status.branch',
    'repos.status.detached',
    'repos.status.aheadBehind',
    'repos.status.renamedFrom',
    'repos.diffHeading',
    'repos.diff.empty',
    'repos.diff.truncated',
    'repos.diff.untracked',
    'repos.diff.fileCount',
    'repos.diff.viewAll',
    'repos.diff.pickFile',
    'repos.error.heading',
    'repos.error.retry',
    'repos.error.notWorkTree',
    'repos.error.invalidName',
    'repos.error.invalidTarget',
    'repos.error.generic',
    'errors.kind.NotAWorkTree',
    'errors.kind.InvalidDiffTarget',
  ];

  it('has both locales for every status and diff key', () => {
    for (const key of KEYS) {
      expect(tFor('en', key), `missing en for ${key}`).not.toBe(key);
      expect(tFor('zh-CN', key), `missing zh for ${key}`).not.toBe(key);
    }
  });

  it('keeps placeholders identical across locales', () => {
    // A `{n}` left untranslated renders as a literal brace in one
    // language, which reads as a broken string rather than a number.
    const placeholders = (s: string) => (s.match(/\{[a-z]+\}/g) ?? []).sort();
    for (const key of [
      'repos.statusDirty',
      'repos.status.branch',
      'repos.status.aheadBehind',
      'repos.status.renamedFrom',
      'repos.diff.fileCount',
    ]) {
      expect(placeholders(tFor('en', key)), key).toEqual(placeholders(tFor('zh-CN', key)));
    }
  });
});
