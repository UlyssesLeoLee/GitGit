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

/**
 * Read the typed `AppError` payload off whatever `invoke` rejected with.
 *
 * The Rust side serializes `AppError` as `{ kind, message, source }`; a
 * non-object rejection (a thrown `Error`, or a missing bridge) is
 * reported as an unknown kind rather than swallowed, so the page still
 * shows something the user can act on.
 */
function readError(err: unknown): { kind: string; message: string } {
  if (err && typeof err === 'object' && 'kind' in err) {
    const e = err as { kind?: unknown; message?: unknown };
    return {
      kind: typeof e.kind === 'string' ? e.kind : 'Unknown',
      message: typeof e.message === 'string' ? e.message : '',
    };
  }
  return { kind: 'Unknown', message: err instanceof Error ? err.message : String(err) };
}

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
    const { kind, message } = readError(err);
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
    const { kind, message } = readError(err);
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
