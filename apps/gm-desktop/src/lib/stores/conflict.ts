/**
 * Conflict-arbitration store (ADR-0024 §3.4 / §3.7).
 *
 * The shape of this module is a copy of `$lib/stores/review.ts`: all the
 * arbitration state lives here rather than in the page component, so the
 * event-ordering rules are testable without rendering. The four rules
 * that matter are the same four `/review` holds:
 *
 * - The event subscription is attached **before** `ai_conflict_start` is
 *   invoked. The Rust side spawns the stream and begins emitting
 *   immediately, so subscribing afterwards drops the first tokens — the
 *   one thing a user would definitely notice.
 * - Events whose `sessionId` is not the current one are dropped. A
 *   cancelled stream's last events can still be in flight, and appending
 *   them to the next arbitration would splice two answers together with
 *   no visible boundary.
 * - A channel that closes without a terminal event is a failure, not a
 *   completion. The UI must not render a truncated answer as a finished
 *   one.
 * - The accumulated text is bounded, so a looping or hostile provider
 *   cannot grow the webview without limit.
 *
 * ADR §3.7.2 adds a fifth, which is the same idea one level down: an
 * event that names a different **region** is dropped too, so a token
 * from one region can never land on another.
 */

import { get, writable } from 'svelte/store';
import * as tauri from '$lib/api/conflict';
import { normalizeError } from '$lib/utils/errors';
import type { ConflictEventDto, ConflictRegion, ConflictStartedDto } from '$lib/api/types';

/**
 * Hard cap on accumulated answer text, in characters.
 *
 * `[FACT]` The same number as `/review`'s `REVIEW_MAX_TEXT_CHARS`
 * (200 000), and the same number the Rust side caps its own accumulation
 * at (`CONFLICT_MAX_ANSWER_CHARS` in `commands/ai.rs`). Model output is
 * untrusted: it is appended verbatim into a live DOM text node. Bounded
 * accumulation keeps a looping or hostile provider from growing the
 * webview without limit. Svelte escapes the text on render, so this is a
 * memory bound, not an XSS defence.
 */
export const CONFLICT_MAX_TEXT_CHARS = 200_000;

/**
 * The message recorded when a session ends without a terminal event.
 *
 * `[FACT]` The same wording the Rust side uses when the provider's
 * channel closes with neither a finish nor an error
 * (`commands/ai.rs`, the `pump_conflict` tail).
 */
export const CONFLICT_NO_TERMINAL_MESSAGE =
  'the arbitration stream ended before the provider finished';

/**
 * The conflict-marker family ADR-0024 §3.4.2's second check refuses.
 *
 * `[FACT]` The same four families the Rust side refuses
 * (`src/ai/conflict.rs`: `<<<<<<<`, `|||||||`, `=======`, `>>>>>>>`).
 * The front-end applies them to a hand-written edit as a local
 * pre-flight only — the Rust side validates the same thing again when
 * the region is written back.
 */
export const CONFLICT_MARKERS: readonly string[] = [
  '<<<<<<<',
  '|||||||',
  '=======',
  '>>>>>>>',
];

export type ConflictStatus =
  | 'idle'
  | 'starting'
  | 'streaming'
  | 'done'
  | 'cancelled'
  | 'error'
  | 'unsupported';

/** The four options the dialog offers (ADR §3.4.3). */
export type ConflictChoice = 'ours' | 'theirs' | 'manual' | 'ai';

/** Whether a key for the chosen provider could be resolved. */
export type KeyStatus = 'unknown' | 'present' | 'absent';

export interface ConflictState {
  status: ConflictStatus;
  /** The stream's session id. Null until the start command resolves. */
  sessionId: string | null;
  /**
   * The region this arbitration is about, by the id the **Rust** side
   * computed (`ConflictRegion::id`). Events that name a different
   * region are dropped (ADR §3.7.2).
   */
  regionId: string | null;
  provider: string;
  /** The model the provider reported, not the one that was requested. */
  model: string;
  /** The answer accumulated so far, exactly as the model produced it. */
  text: string;
  tokenCount: number;
  /** `AppError.kind` for the last failure, for i18n lookup. */
  errorKind: string | null;
  /** Raw message for diagnosis. Never rendered in place of a localized one. */
  errorMessage: string | null;
  /** Provider the user picked, when it cannot stream. */
  unsupportedProvider: string | null;
  /** Secrets the Rust sanitizer rewrote before sending. */
  redactions: number;
  /** True once `CONFLICT_MAX_TEXT_CHARS` was reached. */
  truncated: boolean;
  /**
   * What became of the model's answer.
   *
   * `[FACT]` `null` until the `done` event carries an outcome. A
   * rejected answer carries its reason and no replacement — a refusal is
   * never partial.
   */
  outcome: 'accepted' | 'rejected' | null;
  /** The accepted replacement text, verbatim. */
  suggestion: string | null;
  /** Why the answer was refused, when it was. */
  rejectionReason: string | null;
  /**
   * True once the user has opened the diff of the suggestion.
   *
   * `[PROPOSAL]` ADR §3.4.3: "the accept button is disabled until the
   * user has looked at the diff". That is a state-machine constraint,
   * not a style, so it lives in the state and not in a CSS class.
   */
  diffSeen: boolean;
  /** Whether a key could be resolved for the chosen provider. */
  keyStatus: KeyStatus;
  /**
   * The replacement the user chose, and the choice that produced it.
   *
   * `[PROPOSAL]` ADR §3.4.4: no failure path writes a file, so the
   * resolution is what the dialog shows, not what it commits.
   */
  resolution: { choice: ConflictChoice; text: string } | null;
  /** The regions of the selected repository. */
  regions: ConflictRegion[];
  /** The region currently open in the dialog. */
  selectedRegionId: string | null;
}

const INITIAL: ConflictState = {
  status: 'idle',
  sessionId: null,
  regionId: null,
  provider: '',
  model: '',
  text: '',
  tokenCount: 0,
  errorKind: null,
  errorMessage: null,
  unsupportedProvider: null,
  redactions: 0,
  truncated: false,
  outcome: null,
  suggestion: null,
  rejectionReason: null,
  diffSeen: false,
  keyStatus: 'unknown',
  resolution: null,
  regions: [],
  selectedRegionId: null,
};

export const conflict = writable<ConflictState>({ ...INITIAL });

/** True while a start is in flight or a stream is open. */
export function isBusy(s: ConflictState): boolean {
  return s.status === 'starting' || s.status === 'streaming';
}

/**
 * Whether the "accept" button may be pressed.
 *
 * `[PROPOSAL]` ADR §3.4.3: the suggestion is presented as a diff, and
 * the accept button is disabled until the user has looked at it. Two
 * conditions have to hold: the `done` event must have carried the
 * **accepted** outcome (a refused answer has nothing to accept), and the
 * diff must have been opened.
 */
export function canAccept(s: ConflictState): boolean {
  return (
    s.status === 'done' &&
    s.outcome === 'accepted' &&
    s.suggestion !== null &&
    s.diffSeen
  );
}

/**
 * Whether the "ask AI" option may be pressed.
 *
 * `[PROPOSAL]` ADR §3.7.1: with no key the option is shown but disabled
 * and says why. `[PROPOSAL]` ADR §3.4.1: an option is only offered on a
 * provider that can stream.
 *
 * `[INFERENCE]` `keyStatus === 'unknown'` disables the option too. The
 * app has not asked the backend yet, and offering an option it cannot
 * honour is the same defect as hiding one that works. The check is
 * cheap and the wait is a round trip to the local process.
 */
export function canAskAi(s: ConflictState, streaming: boolean): boolean {
  return streaming && s.keyStatus === 'present' && !isBusy(s);
}

/**
 * The replacement text for a choice that needs no model.
 *
 * `[PROPOSAL]` ADR §3.7.4: "keep ours" and "keep theirs" are not put
 * through the suggestion checks — the user picked a side. A hand-written
 * edit *is*, because it replaces the region with text nobody checked.
 */
export function resolveChoice(
  region: ConflictRegion,
  choice: Exclude<ConflictChoice, 'ai'>,
  manual?: string
): { ok: true; text: string } | { ok: false; reason: string } {
  if (choice === 'ours') return { ok: true, text: region.ours };
  if (choice === 'theirs') return { ok: true, text: region.theirs };

  const text = manual ?? '';
  const marker = CONFLICT_MARKERS.find((m) => text.includes(m));
  if (marker !== undefined) {
    return { ok: false, reason: 'the suggestion carries a conflict marker' };
  }
  if (text.length > CONFLICT_MAX_TEXT_CHARS) {
    return { ok: false, reason: 'the suggestion is too long' };
  }
  return { ok: true, text };
}

/** Detaches the live subscription, if any. */
let detach: (() => void) | null = null;

/**
 * Whether a terminal event has been seen for the current session.
 *
 * `[FACT]` ADR §3.4.1: "a stream without a terminal event is a failure,
 * not a completion". The Rust side emits one on every exit path, but the
 * front-end does not lean on that.
 */
let terminalSeen = false;

/**
 * A `model` event that landed before the start command's promise resolved.
 *
 * `[FACT]` Providers report the served model in the first frame of the
 * stream, so across a real Tauri bridge this event routinely arrives while
 * `ai_conflict_start` is still in flight — `sessionId` is only written
 * after that promise settles.
 *
 * `[FACT]` `ConflictStarted.model` is "the model that was **requested**"
 * (`commands/ai.rs`), so writing it after a `model` event has already
 * arrived would replace the served model with the requested one. The Rust
 * suite forbids that in the review channel — "the served model must be
 * reported, not assumed" — and the state field here is documented the same
 * way. This is `$lib/stores/review.ts`'s `pendingModel`, carried over for
 * the same reason.
 */
let pendingModel: string | null = null;

function releaseSubscription(): void {
  if (detach) {
    detach();
    detach = null;
  }
}

/** Reset to the initial state and drop any subscription. */
export function resetArbitration(): void {
  releaseSubscription();
  terminalSeen = false;
  pendingModel = null;
  conflict.set({ ...INITIAL });
}

/**
 * Apply one event. Private: tests drive it through the subscription.
 */
function applyConflictEvent(e: ConflictEventDto): void {
  const s = get(conflict);
  // `[FACT]` A known session means anything else is a leftover from a
  // stream we already stopped. An unknown one means the start command has
  // not resolved yet, and this subscription can only carry the new stream.
  if (s.sessionId !== null && e.sessionId !== s.sessionId) return;
  // `[PROPOSAL]` ADR §3.7.2: an answer must be bound to the region it
  // was asked about. A token that names another region cannot land on
  // this one.
  if ('regionId' in e && s.regionId !== null && e.regionId !== s.regionId) return;

  switch (e.type) {
    case 'token': {
      if (s.status !== 'streaming' && s.status !== 'starting') return;
      const room = Math.max(0, CONFLICT_MAX_TEXT_CHARS - s.text.length);
      const delta = e.delta.length > room ? e.delta.slice(0, room) : e.delta;
      conflict.set({
        ...s,
        text: s.text + delta,
        tokenCount: s.tokenCount + 1,
        truncated: s.truncated || delta.length < e.delta.length,
      });
      return;
    }
    case 'model': {
      // `[FACT]` Held rather than dropped: see `pendingModel`. The session
      // id does not exist yet, which is exactly why this branch is
      // reachable, and the last line of `startArbitration` is where the
      // held value lands.
      if (s.sessionId === null) {
        pendingModel = e.model;
        return;
      }
      conflict.set({ ...s, model: e.model });
      return;
    }
    case 'done': {
      terminalSeen = true;
      const outcome = e.outcome;
      conflict.set({
        ...s,
        status: 'done',
        tokenCount: e.tokens,
        outcome: outcome.state,
        // `[FACT]` A refusal carries a reason and no text; an acceptance
        // carries the text and no reason. Neither is ever half-set.
        suggestion: outcome.state === 'accepted' ? outcome.suggestion : null,
        rejectionReason: outcome.state === 'rejected' ? outcome.reason : null,
        // `[FACT]` A new suggestion starts unseen: the accept button is
        // disabled until the user has opened this diff.
        diffSeen: false,
      });
      releaseSubscription();
      return;
    }
    case 'failed': {
      terminalSeen = true;
      conflict.set({
        ...s,
        status: 'error',
        errorKind: 'Gitgit',
        errorMessage: e.message,
      });
      releaseSubscription();
      return;
    }
    case 'cancelled': {
      terminalSeen = true;
      conflict.set({ ...s, status: 'cancelled' });
      releaseSubscription();
      return;
    }
  }
}

export interface StartOptions {
  provider?: string | null;
  model?: string | null;
  baseUrl?: string | null;
  /**
   * Event-subscription seam. Defaults to the Tauri event bridge; tests
   * pass their own so the store can be driven without a Rust backend.
   */
  subscribe?: typeof tauri.listenConflictEvents;
  /**
   * The key-status seam. Defaults to the Tauri command; tests pass
   * their own so the store can be driven without a Rust backend.
   */
  keyStatus?: typeof tauri.aiKeyStatus;
}

/**
 * Ask the backend whether a key can be resolved for `provider`.
 *
 * `[FACT]` The answer is a presence, never a value. The webview cannot
 * see the process environment, so it cannot work this out itself.
 */
export async function loadKeyStatus(
  provider: string,
  seam: typeof tauri.aiKeyStatus = tauri.aiKeyStatus
): Promise<KeyStatus> {
  try {
    const present = await seam(provider);
    const status: KeyStatus = present ? 'present' : 'absent';
    conflict.set({ ...get(conflict), keyStatus: status });
    return status;
  } catch {
    // `[INFERENCE]` A backend that cannot answer is not a reason to offer
    // an option that may not work.
    conflict.set({ ...get(conflict), keyStatus: 'absent' });
    return 'absent';
  }
}

/**
 * Start an arbitration for one region. Errors are folded into the store
 * rather than thrown, so a page can bind to one object and render every
 * state including failure.
 */
export async function startArbitration(
  region: ConflictRegion,
  opts: StartOptions = {}
): Promise<void> {
  if (isBusy(get(conflict))) return;
  const provider = opts.provider ?? null;
  releaseSubscription();
  terminalSeen = false;
  const before = get(conflict);
  conflict.set({
    ...INITIAL,
    status: 'starting',
    provider: provider ?? '',
    // `[FACT]` The page's own state survives a new arbitration: the
    // region list and the open region are the page's, not the stream's.
    regions: before.regions,
    selectedRegionId: before.selectedRegionId,
    keyStatus: before.keyStatus,
  });

  const subscribe = opts.subscribe ?? tauri.listenConflictEvents;
  try {
    detach = await subscribe(applyConflictEvent);
  } catch (e) {
    conflict.set({ ...get(conflict), status: 'error', ...normalizeError(e) });
    return;
  }

  let started: ConflictStartedDto;
  try {
    started = await tauri.aiConflictStart({
      region,
      provider,
      model: opts.model ?? null,
      baseUrl: opts.baseUrl ?? null,
    });
  } catch (e) {
    const err = normalizeError(e);
    releaseSubscription();
    // `[FACT]` A start that produced no session must not leave a held
    // model behind for the next arbitration to pick up.
    pendingModel = null;
    if (err.kind === 'AiReviewUnsupported') {
      conflict.set({
        ...get(conflict),
        status: 'unsupported',
        unsupportedProvider: provider,
        errorKind: err.kind,
        errorMessage: err.message,
      });
      return;
    }
    conflict.set({
      ...get(conflict),
      status: 'error',
      errorKind: err.kind,
      errorMessage: err.message,
    });
    return;
  }

  conflict.set({
    ...get(conflict),
    status: 'streaming',
    sessionId: started.sessionId,
    regionId: started.regionId,
    provider: started.provider,
    // `[FACT]` The provider's own answer wins over what we asked for,
    // whenever it got here first. `started.model` is the *requested*
    // model.
    model: pendingModel ?? started.model,
    redactions: started.redactions,
  });
  pendingModel = null;
}

/**
 * Stop the running arbitration.
 *
 * The Rust command aborts the consuming task, so this ends the request
 * rather than the rendering. If the stream had already reached a
 * terminal state, `ai_conflict_cancel` refuses with
 * `AiReviewNotRunning`; that refusal is not an error to show the user,
 * it means the terminal event already owns the state, so it is left
 * alone.
 */
export async function stopArbitration(): Promise<void> {
  if (!isBusy(get(conflict))) return;
  try {
    await tauri.aiConflictCancel();
  } catch (e) {
    const err = normalizeError(e);
    if (err.kind !== 'AiReviewNotRunning') {
      conflict.set({
        ...get(conflict),
        status: 'error',
        errorKind: err.kind,
        errorMessage: err.message,
      });
      releaseSubscription();
      return;
    }
    return;
  }
  terminalSeen = true;
  conflict.set({ ...get(conflict), status: 'cancelled' });
  releaseSubscription();
}

/**
 * Abandon the running arbitration — the user opened another region, or
 * left the page.
 *
 * `[FACT]` The `/review` discipline: "a channel that closes without a
 * terminal event is a failure, not a completion" (ADR §3.4.1). A session
 * that is closed while it is still busy, and that has produced no
 * terminal event, is recorded as a failure — never as a finished
 * answer. The region stays unresolved.
 */
export function abandonArbitration(): void {
  const s = get(conflict);
  if (isBusy(s) && !terminalSeen) {
    conflict.set({
      ...s,
      status: 'error',
      errorKind: 'Gitgit',
      errorMessage: CONFLICT_NO_TERMINAL_MESSAGE,
    });
  }
  releaseSubscription();
}

/**
 * Record that the user has opened the diff of the suggestion.
 *
 * `[PROPOSAL]` ADR §3.4.3: the accept button is disabled until the user
 * has looked at the diff.
 */
export function markDiffSeen(): void {
  const s = get(conflict);
  if (s.diffSeen) return;
  conflict.set({ ...s, diffSeen: true });
}

/**
 * Record the replacement the user chose.
 *
 * `[PROPOSAL]` ADR §3.4.4: nothing is written here. The resolution is
 * what the dialog shows; writing it back is the write path's job.
 */
export function setResolution(choice: ConflictChoice, text: string): void {
  conflict.set({ ...get(conflict), resolution: { choice, text } });
}

/**
 * Load the conflict regions of a repository.
 *
 * `[PROPOSAL]` ADR §3.1: the regions come from the Rust core's parse of
 * `git merge-file`'s output.
 */
export async function loadRegions(
  repo: string,
  seam: typeof tauri.conflictRegions = tauri.conflictRegions
): Promise<ConflictRegion[]> {
  try {
    const regions = await seam(repo);
    conflict.set({ ...get(conflict), regions, selectedRegionId: regions[0]?.id ?? null });
    return regions;
  } catch (e) {
    const err = normalizeError(e);
    conflict.set({
      ...get(conflict),
      regions: [],
      selectedRegionId: null,
      status: 'error',
      errorKind: err.kind,
      errorMessage: err.message,
    });
    return [];
  }
}

/** Open one region in the dialog. */
export function selectRegion(regionId: string): void {
  conflict.set({ ...get(conflict), selectedRegionId: regionId });
}
