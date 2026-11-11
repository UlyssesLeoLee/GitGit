/**
 * Working-tree root store.
 *
 * `repo_status` and `repo_diff` both take an optional `root`: the
 * directory the repository **name** is resolved under. Nothing in the
 * shipped app ever supplied one, so on a default install every
 * repository answered `NotAWorkTree` — gitgit's own repositories are
 * created by `git init --bare`, and `git status` in a bare repository
 * exits 128 (`fatal: this operation must be run in a work tree`).
 * This store is what makes that argument reachable by a user.
 *
 * Persistence key: `gm-desktop.worktreeRoot`, the same
 * `localStorage` convention `theme.ts` and `locale.ts` already use.
 *
 * The read is validated, not trusted. A value that could not have come
 * out of the directory picker (a serialized JSON object, a
 * quote-wrapped copy/paste, a truncated write, anything with a control
 * character) is treated as "not set" rather than handed to git.
 */

import { writable, get } from 'svelte/store';
import { open } from '@tauri-apps/plugin-dialog';

/** Exported so a test can assert the key rather than re-type it. */
export const STORAGE_KEY = 'gm-desktop.worktreeRoot';

/** Windows refuses a path component longer than this, so a longer
 *  string is corruption rather than a real folder. */
const MAX_PATH_CHARS = 4096;

/** Control characters, NUL included: no filesystem path contains one. */
// eslint-disable-next-line no-control-regex
const CONTROL_CHARS = /[\u0000-\u001F\u007F]/;

/**
 * Whether a string can be a working-tree root handed to the backend.
 *
 * Rejected: empty / whitespace-only, over-long, containing a control
 * character, and containing a double quote. The quote rule is the one
 * that does the real work: the picker can only ever return a path the
 * OS reported, and neither Windows nor POSIX allows `"` in a path, so
 * any value containing one is a serialized object or a quote-wrapped
 * paste rather than something this app wrote.
 */
export function isValidRoot(value: unknown): value is string {
  if (typeof value !== 'string') return false;
  const trimmed = value.trim();
  if (trimmed.length === 0 || trimmed.length > MAX_PATH_CHARS) return false;
  if (trimmed.includes('"')) return false;
  return !CONTROL_CHARS.test(trimmed);
}

/** Read the persisted root, or null when absent, stale or corrupted. */
export function readPersistedRoot(): string | null {
  try {
    const v = localStorage.getItem(STORAGE_KEY);
    if (v !== null && isValidRoot(v)) return v.trim();
  } catch (_err) {
    // localStorage can be unavailable (private mode, a test sandbox);
    // fall through to "not set" rather than failing to boot.
  }
  return null;
}

export const worktreeRoot = writable<string | null>(readPersistedRoot());

/**
 * What happened to a pick attempt. Four answers, because "nothing
 * happened" and "it was refused" are different things for a user:
 *
 *  - `set` — a folder was chosen and persisted.
 *  - `cancelled` — the user closed the dialog. **Not** an error and
 *    deliberately not a clear: the previous value is left alone.
 *  - `invalid` — a value came back that no filesystem could have
 *    produced, so it was refused and nothing was written.
 *  - `error` — the dialog itself could not be opened (a browser tab
 *    with no Tauri runtime, or a denied capability).
 */
export type PickOutcome = 'set' | 'cancelled' | 'invalid' | 'error';

/** Persist a validated root, or clear the setting when given null. */
export function setWorktreeRoot(value: string | null): boolean {
  if (value === null) {
    clearWorktreeRoot();
    return true;
  }
  if (!isValidRoot(value)) return false;
  worktreeRoot.set(value.trim());
  try {
    localStorage.setItem(STORAGE_KEY, value.trim());
  } catch (_err) {
    // A write-protected store still gets the in-memory value for this
    // session, which is the same trade `theme.ts` makes.
  }
  return true;
}

/** Forget the working-tree root; the backend falls back to its own repos dir. */
export function clearWorktreeRoot(): void {
  worktreeRoot.set(null);
  try {
    localStorage.removeItem(STORAGE_KEY);
  } catch (_err) {
    // ignore — the in-memory value is already cleared.
  }
}

/**
 * Open the directory picker and persist what comes back.
 *
 * `title` is passed in rather than read from the catalogue here,
 * because this module is a store and not a component: the caller owns
 * the locale.
 */
export async function pickWorktreeRoot(title: string): Promise<PickOutcome> {
  let selected: unknown;
  try {
    selected = await open({ directory: true, multiple: false, title });
  } catch (_err) {
    return 'error';
  }
  if (selected === null || selected === undefined) return 'cancelled';
  // `multiple: false` returns a bare string, but an array is the other
  // legal shape if the option is ever changed, and a string[] would be
  // nonsense as a root. Refuse anything that is not one string.
  if (typeof selected !== 'string') return 'invalid';
  if (!isValidRoot(selected)) return 'invalid';
  setWorktreeRoot(selected);
  return 'set';
}

/** Current value, for script blocks. */
export function currentRoot(): string | null {
  return get(worktreeRoot);
}
