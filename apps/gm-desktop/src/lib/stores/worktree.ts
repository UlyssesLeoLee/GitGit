/**
 * Working-tree status and diff store (V0 task T4).
 *
 * Three states are modelled explicitly rather than collapsed into "no
 * data", because they are three different answers:
 *
 *  - `status: 'clean'` — git reported zero changed paths. A clean working
 *    tree is a real answer and the UI says so.
 *  - `status: 'dirty'` — at least one entry, grouped by the user's
 *    question (staged / not staged / untracked).
 *  - `status: 'error'` — the call was refused, with the typed `AppError`
 *    kind kept so the page can pick a localized message. A bare
 *    repository is the common case here and has its own kind.
 *
 * The diff is a separate slot from the status because they answer
 * different questions and are fetched independently: `target` picks which
 * of staged / worktree / head is being asked and is never inferred.
 */

import { writable, get } from 'svelte/store';
import { normalizeError } from '$lib/utils/errors';
import * as tauri from '$lib/api/tauri';
import type { DiffTarget, RepoDiff, RepoStatus, StatusEntry } from '$lib/api/types';

export type WorktreeStatus = 'idle' | 'loading' | 'clean' | 'dirty' | 'error';

export interface WorktreeState {
  status: WorktreeStatus;
  branch: string | null;
  head: string | null;
  upstream: string | null;
  ahead: number;
  behind: number;
  entries: StatusEntry[];
  /** Typed `AppError.kind`, or null when there is no error. */
  errorKind: string | null;
  /** Raw detail for the error panel; never used as the headline. */
  errorMessage: string;
}

export interface DiffState {
  status: 'idle' | 'loading' | 'ready' | 'empty' | 'error';
  target: DiffTarget;
  /** Repo-relative path the current diff is scoped to, if any. */
  path: string | null;
  text: string;
  truncated: boolean;
  /** True when the text was synthesized for an untracked path. */
  untracked: boolean;
  files: number;
  errorKind: string | null;
  errorMessage: string;
}

const EMPTY_STATUS: WorktreeState = {
  status: 'idle',
  branch: null,
  head: null,
  upstream: null,
  ahead: 0,
  behind: 0,
  entries: [],
  errorKind: null,
  errorMessage: '',
};

const EMPTY_DIFF: DiffState = {
  status: 'idle',
  target: 'worktree',
  path: null,
  text: '',
  truncated: false,
  untracked: false,
  files: 0,
  errorKind: null,
  errorMessage: '',
};

export const worktree = writable<WorktreeState>({ ...EMPTY_STATUS });
export const diff = writable<DiffState>({ ...EMPTY_DIFF });

/** Return both stores to their initial state between tests / repo changes. */
export function resetWorktree(): void {
  worktree.set({ ...EMPTY_STATUS });
  diff.set({ ...EMPTY_DIFF });
}

/*
 * `[FACT]` This store used to carry its own `readError`, guarded on
 * `'kind' in err` and falling back to a kind of `'Unknown'`. Two
 * consequences, both measured:
 *
 *  - A rejection carrying a `message` but no `kind` — which the mock
 *    layer and any non-Tauri caller produce, per `utils/errors.ts` —
 *    missed the guard entirely and was rendered through `String(err)`,
 *    i.e. the user saw the literal text `[object Object]` as the detail
 *    line. `normalizeError` reads `kind` and `message` independently, so
 *    that payload now yields the backend's own words.
 *  - The `'Unknown'` fallback became `Internal`. This is a
 *    consistency change, not a rendering fix: `RepoWorktree.svelte`'s
 *    `errorHeadline()` falls through to `repos.error.generic` for any
 *    unrecognised kind, so `'Unknown'` degraded gracefully and the page
 *    was never broken by it. `Internal` is chosen instead because it is
 *    the one fallback kind with a catalogue entry, so a caller that
 *    localizes from the kind has something to show.
 */

/** Fetch the working-tree status for `name`. */
export async function loadStatus(name: string, root?: string | null): Promise<void> {
  if (!name) return;
  worktree.set({ ...get(worktree), status: 'loading', errorKind: null, errorMessage: '' });
  try {
    const next: RepoStatus = await tauri.repoStatus(name, root ?? null);
    worktree.set({
      status: next.is_clean ? 'clean' : 'dirty',
      branch: next.branch ?? null,
      head: next.head ?? null,
      upstream: next.upstream ?? null,
      ahead: next.ahead ?? 0,
      behind: next.behind ?? 0,
      entries: next.entries ?? [],
      errorKind: null,
      errorMessage: '',
    });
  } catch (err) {
    const { kind, message } = normalizeError(err);
    worktree.set({ ...EMPTY_STATUS, status: 'error', errorKind: kind, errorMessage: message });
  }
}

/** Fetch the diff for one of the three comparisons. */
export async function loadDiff(
  name: string,
  target: DiffTarget,
  path?: string | null,
  root?: string | null
): Promise<void> {
  if (!name) return;
  diff.set({ ...get(diff), status: 'loading', target, errorKind: null, errorMessage: '' });
  try {
    const next: RepoDiff = await tauri.repoDiff(name, target, path ?? null, root ?? null);
    diff.set({
      // An empty diff is a distinct state from an error: for a clean
      // tree it is the correct answer to "what has not been staged?".
      status: next.text.trim() === '' ? 'empty' : 'ready',
      target: next.target,
      path: next.path ?? null,
      text: next.text,
      truncated: next.truncated,
      untracked: next.untracked,
      files: next.files,
      errorKind: null,
      errorMessage: '',
    });
  } catch (err) {
    const { kind, message } = normalizeError(err);
    diff.set({ ...EMPTY_DIFF, status: 'error', target, errorKind: kind, errorMessage: message });
  }
}

/* --- grouping helpers, shared by the component and the tests --------- */

/** Entries staged in the index, i.e. what a commit would contain. */
export function stagedEntries(entries: StatusEntry[]): StatusEntry[] {
  return entries.filter((e) => e.staged);
}

/** Entries changed in the work tree but not yet staged. */
export function unstagedEntries(entries: StatusEntry[]): StatusEntry[] {
  return entries.filter((e) => e.unstaged && !e.staged);
}

/**
 * Entries git has never seen.
 *
 * Reported separately rather than folded into "unstaged": `git diff` says
 * nothing about these files, so treating them as ordinary modifications
 * would point the user at a diff that does not exist.
 */
export function untrackedEntries(entries: StatusEntry[]): StatusEntry[] {
  return entries.filter((e) => e.untracked);
}

/** Count for a group, with a plural-free label the catalogue owns. */
export function groupCount(entries: StatusEntry[]): number {
  return entries.length;
}
