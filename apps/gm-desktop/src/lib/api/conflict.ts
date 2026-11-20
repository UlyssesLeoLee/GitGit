/**
 * Tauri bridge for the conflict arbitration (ADR-0024).
 *
 * The shape of this module is a copy of `$lib/api/ai.ts`: named event
 * constants, one command per action, and a subscription helper that
 * attaches every event name and detaches all of them if any of the
 * attachments fails.
 *
 * `[FACT]` The event names are mirrored from
 * `apps/gm-desktop/src-tauri/src/commands/ai.rs`
 * (`EVENT_CONFLICT_TOKEN` and friends). If a name changes there it must
 * change here, which is why they are constants rather than string
 * literals at the call sites.
 *
 * `[FACT]` The command names are the `#[tauri::command]` function names
 * on the Rust side: `ai_conflict_start`, `ai_conflict_cancel`,
 * `ai_key_set` and `ai_key_status`.
 */

import { invoke as tauriInvoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type {
  ConflictEventDto,
  ConflictRegion,
  ConflictRegions,
  ConflictStartedDto,
} from './types';

export const CONFLICT_EVENT_TOKEN = 'ai-conflict://token';
export const CONFLICT_EVENT_MODEL = 'ai-conflict://model';
export const CONFLICT_EVENT_DONE = 'ai-conflict://done';
export const CONFLICT_EVENT_FAILED = 'ai-conflict://failed';
export const CONFLICT_EVENT_CANCELLED = 'ai-conflict://cancelled';

const CONFLICT_EVENTS = [
  CONFLICT_EVENT_TOKEN,
  CONFLICT_EVENT_MODEL,
  CONFLICT_EVENT_DONE,
  CONFLICT_EVENT_FAILED,
  CONFLICT_EVENT_CANCELLED,
] as const;

/**
 * The region as it goes over the wire.
 *
 * `[FACT]` `ConflictRegionDto` on the Rust side is deserialized with
 * `rename_all = "camelCase"` and carries exactly these eight fields.
 * The webview's own `ConflictRegion` adds an `id` and a `scopeLabel`
 * for display, so the two are mapped rather than passed through: serde
 * ignores unknown fields, but a payload that is exactly the contract is
 * a payload that cannot drift from it by accident.
 */
export interface ConflictRegionPayload {
  path: string;
  baseOid: string;
  index: number;
  ours: string;
  base: string;
  theirs: string;
  contextBefore: string;
  contextAfter: string;
}

/** The wire shape of one region. */
export function toRegionPayload(region: ConflictRegion): ConflictRegionPayload {
  return {
    path: region.path,
    baseOid: region.baseOid,
    index: region.index,
    ours: region.ours,
    base: region.base,
    theirs: region.theirs,
    contextBefore: region.contextBefore,
    contextAfter: region.contextAfter,
  };
}

export interface ConflictRequest {
  region: ConflictRegion;
  provider: string | null;
  model: string | null;
  baseUrl: string | null;
}

/**
 * Start an arbitration for one region.
 *
 * `[FACT]` The command returns as soon as the request is accepted; the
 * answer arrives on the `ai-conflict://` events, and the `done` event
 * carries the *validated* outcome — accepted with a replacement, or
 * rejected with a reason.
 */
export async function aiConflictStart(req: ConflictRequest): Promise<ConflictStartedDto> {
  return await tauriInvoke('ai_conflict_start', {
    region: toRegionPayload(req.region),
    provider: req.provider,
    model: req.model,
    baseUrl: req.baseUrl,
  });
}

/** Stop the running arbitration. Returns the session id that was stopped. */
export async function aiConflictCancel(): Promise<string> {
  return await tauriInvoke('ai_conflict_cancel');
}

/**
 * Store a provider key in the vault.
 *
 * `[FACT]` `ai_key_set` returns nothing: a key never travels back to the
 * webview, and there is deliberately no `ai_key_get`.
 */
export async function aiKeySet(provider: string, key: string): Promise<void> {
  await tauriInvoke('ai_key_set', { provider, key });
}

/**
 * Whether a key for `provider` can be resolved.
 *
 * `[FACT]` The answer is a presence, never a value: the Rust side
 * resolves the key (vault first, `GITGIT_AI_API_KEY` second) and reports
 * only whether one was found.
 *
 * `[PROPOSAL]` ADR-0024 §3.7.1: the "ask AI" option is shown but
 * disabled when there is no key. That needs the app to *know* there is
 * no key before the request, and the webview cannot see the process
 * environment — so the backend has to answer.
 */
export async function aiKeyStatus(provider: string): Promise<boolean> {
  return await tauriInvoke('ai_key_status', { provider });
}

/**
 * The conflict regions of one repository.
 *
 * `[PROPOSAL]` ADR-0024 §3.1: the regions come from `git merge-file`'s
 * output, parsed by the Rust core in `src/repo/conflict.rs`.
 *
 * `[FACT]` `root` is the working-tree root the repository name is
 * resolved under — the same argument `repo_status` and `repo_diff` take.
 * The desktop keeps its repositories as bare `<name>.git` directories and
 * the checkouts live under the root the user picked, so a command that
 * did not forward it would resolve a bare repository, where `git` answers
 * `NotAWorkTree`.
 *
 * `[FACT]` The paths the core refuses to read (an add/delete or rename
 * conflict, binary content) come back in `needsGit` rather than being
 * dropped: they are work git still has to do.
 */
export async function conflictRegions(
  repo: string,
  root: string | null
): Promise<ConflictRegions> {
  const result = await tauriInvoke('conflict_regions', { repo, root });
  const payload = (result ?? {}) as Partial<ConflictRegions>;
  return {
    regions: Array.isArray(payload.regions) ? payload.regions : [],
    needsGit: Array.isArray(payload.needsGit) ? payload.needsGit : [],
  };
}

/**
 * Subscribe to every conflict event. Returns one unlisten function that
 * detaches all five listeners, so a caller cannot leak a subscription
 * by forgetting one of them.
 *
 * `[FACT]` This is `listenReviewEvents`'s shape, copied: the five
 * `listen` calls are sequential, so a failure in the middle would
 * otherwise leave the earlier listeners attached with no handle the
 * caller could ever use. The failure path detaches everything that was
 * already open and re-throws.
 */
export async function listenConflictEvents(
  handler: (event: ConflictEventDto) => void
): Promise<UnlistenFn> {
  const attached: UnlistenFn[] = [];
  const detachAll = (): void => {
    // `[FACT]` Detach everything, even if one of them throws. A listener
    // that fails to release must not strand the rest, and on the failure
    // path the throw here would replace the error the caller actually
    // needs.
    for (const unlisten of attached.splice(0)) {
      try {
        unlisten();
      } catch {
        // Best effort.
      }
    }
  };

  try {
    for (const name of CONFLICT_EVENTS) {
      const unlisten = await listen<ConflictEventDto>(name, (e) => handler(e.payload));
      attached.push(unlisten);
    }
  } catch (e) {
    detachAll();
    throw e;
  }

  return detachAll;
}
