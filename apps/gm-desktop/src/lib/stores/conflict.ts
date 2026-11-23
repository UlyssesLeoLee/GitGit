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
import type {
  ConflictChoice,
  ConflictEventDto,
  ConflictRegion,
  ConflictResolveChoice,
  ConflictStartedDto,
} from '$lib/api/types';

/**
 * The four options the dialog offers (ADR §3.4.3).
 *
 * `[INFERENCE]` The union itself lives in `$lib/api/types`, because the
 * bridge needs it too and the bridge cannot import from the store without
 * a cycle. It is re-exported here so the store's callers keep one import
 * for the arbitration's vocabulary.
 */
export type { ConflictChoice };

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

/** Whether a key for the chosen provider could be resolved. */
export type KeyStatus = 'unknown' | 'present' | 'absent';

/**
 * The decision the user made about one region.
 *
 * `[FACT]` The replacement text travels with the choice: `ours` and
 * `theirs` carry the region's own text, and `ai` and `manual` carry the
 * text the user or the model produced. The core maps all four to
 * `RegionChoice`, and `ai` / `manual` to `RegionChoice::Custom`.
 */
export interface ConflictDecision {
  /** Which side won. */
  choice: ConflictChoice;
  /** The replacement text, verbatim. */
  text: string;
}

/**
 * What a write-back did, path by path.
 *
 * `[FACT]` The paths are separate because the write-back is per path
 * (ADR §3.6): a failure on one path says nothing about the others, and a
 * report that said "done" while half the paths were untouched would be
 * a lie.
 */
export interface ConflictApplyOutcome {
  /** The paths that were written back and staged. */
  written: string[];
  /** The path that failed, and what the backend said about it. */
  failed: { path: string; message: string } | null;
  /** The paths that were not attempted, because an earlier path failed. */
  skipped: string[];
}

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
   *
   * `[FACT]` The resolution is a **view**: it is derived from `decisions`
   * and the open region, and it is never the truth. The truth is the
   * map below — one decision per region, kept across region switches,
   * because the write-back is per path and a path can carry several
   * regions (ADR §3.6).
   *
   * `[FACT]` The resolution carries the **region it belongs to**. It has
   * to: "the same choice with the same text" would mean "the same
   * decision" for two different regions — and the second region's audit
   * record would be silently dropped (ADR §3.7.3: one decision, one
   * record — and a decision in another region IS another decision).
   */
  resolution: { choice: ConflictChoice; text: string; regionId: string } | null;
  /**
   * The decision for each region, by the region's id.
   *
   * `[FACT]` Before this lane the state held ONE resolution and
   * `selectRegion` reset the whole state, so a decision made in one
   * region was gone the moment the user opened the next one — and there
   * was nowhere to keep the decisions for a file that carries several
   * regions. The map is what makes "decide every region, then write the
   * path back" possible at all.
   */
  decisions: Record<string, ConflictDecision>;
  /**
   * True while a write-back is in flight.
   */
  applying: boolean;
  /**
   * The message from the last write-back, for the user to read.
   */
  applyError: string | null;
  /**
   * What the last write-back did, path by path.
   */
  applyOutcome: ConflictApplyOutcome | null;
  /** The regions of the selected repository. */
  regions: ConflictRegion[];
  /**
   * The paths the core leaves to git.
   *
   * `[PROPOSAL]` They are reported, not dropped: an add/delete or rename
   * conflict has no base stage, and binary content is not text
   * (ADR §3.7.4). A page that said "no conflict regions" while git still
   * had work to do would be lying.
   */
  needsGit: string[];
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
  decisions: {},
  applying: false,
  applyError: null,
  applyOutcome: null,
  regions: [],
  needsGit: [],
  selectedRegionId: null,
};

export const conflict = writable<ConflictState>({ ...INITIAL });

/**
 * The resolution of the open region, derived from the decisions.
 *
 * `[FACT]` The resolution is a view of `decisions[selectedRegionId]`, so
 * it cannot drift from the map: there is one function that writes the
 * state and it derives the view on the way out. A region with no
 * decision has no resolution.
 */
function deriveResolution(
  decisions: Record<string, ConflictDecision>,
  selectedRegionId: string | null
): ConflictState['resolution'] {
  if (selectedRegionId === null) return null;
  const decision = decisions[selectedRegionId];
  if (decision === undefined) return null;
  return { choice: decision.choice, text: decision.text, regionId: selectedRegionId };
}

/**
 * Write the state, with the resolution derived from the decisions.
 *
 * `[FACT]` Every write goes through here, so the resolution cannot be
 * set independently of the map it is a view of.
 */
function commit(patch: Partial<ConflictState>): void {
  const next = { ...get(conflict), ...patch };
  next.resolution = deriveResolution(next.decisions, next.selectedRegionId);
  conflict.set(next);
}

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

/**
 * Whether an arbitration is currently allowed to write events.
 *
 * `[FACT]` The status alone cannot say this: a stream that outlived its
 * arbitration — because the user opened another region, or because the
 * page went away — carries events that belong to an arbitration which is
 * no longer running. The session and region filters cannot catch that
 * either, because a reset clears both, and an event with no session id
 * to compare against would pass. This flag is the ground truth: only an
 * arbitration that is running owns events.
 */
let armed = false;

/**
 * A stop that was asked for while the start was still in flight.
 *
 * `[FACT]` The Rust side installs the session after the provider call
 * returns, so a cancel issued while `ai_conflict_start` is still in
 * flight can find nothing to cancel and answers `AiReviewNotRunning` —
 * while the start goes on to open a stream the user believes they
 * stopped. The intent is therefore carried through startup and issued
 * the moment the session exists.
 */
let cancelRequested = false;

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
  armed = false;
  cancelRequested = false;
  commit({ ...INITIAL });
}

/**
 * Apply one event. Private: tests drive it through the subscription.
 */
function applyConflictEvent(e: ConflictEventDto): void {
  const s = get(conflict);
  // `[FACT]` Only an arbitration that is running owns events. A stream
  // that outlived its arbitration — the user opened another region, or
  // left the page — must not write into the next one, and the session
  // and region filters cannot catch it, because a reset clears both.
  if (!armed) return;
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
      commit({
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
      commit({ ...s, model: e.model });
      return;
    }
    case 'done': {
      terminalSeen = true;
      armed = false;
      const outcome = e.outcome;
      commit({
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
      armed = false;
      commit({
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
      armed = false;
      commit({ ...s, status: 'cancelled' });
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
    commit({ ...get(conflict), keyStatus: status });
    return status;
  } catch {
    // `[INFERENCE]` A backend that cannot answer is not a reason to offer
    // an option that may not work.
    commit({ ...get(conflict), keyStatus: 'absent' });
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
  cancelRequested = false;
  const before = get(conflict);
  commit({
    ...INITIAL,
    status: 'starting',
    provider: provider ?? '',
    // `[FACT]` The page's own state survives a new arbitration: the
    // region list and the open region are the page's, not the stream's.
    regions: before.regions,
    selectedRegionId: before.selectedRegionId,
    keyStatus: before.keyStatus,
    // `[FACT]` The user's decisions survive too: an arbitration is about
    // one region, and it must not throw away what the user has decided
    // for the others.
    decisions: before.decisions,
  });

  const subscribe = opts.subscribe ?? tauri.listenConflictEvents;
  // `[FACT]` Armed before the subscription is attached: the Rust side
  // emits immediately, and a provider reports its served model in the
  // first frame of the stream — which can arrive while the subscribe
  // call is still returning.
  armed = true;
  try {
    detach = await subscribe(applyConflictEvent);
  } catch (e) {
    armed = false;
    commit({ ...get(conflict), status: 'error', ...normalizeError(e) });
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
    armed = false;
    // `[FACT]` A start that produced no session must not leave a held
    // model behind for the next arbitration to pick up.
    pendingModel = null;
    if (err.kind === 'AiReviewUnsupported') {
      commit({
        ...get(conflict),
        status: 'unsupported',
        unsupportedProvider: provider,
        errorKind: err.kind,
        errorMessage: err.message,
      });
      return;
    }
    commit({
      ...get(conflict),
      status: 'error',
      errorKind: err.kind,
      errorMessage: err.message,
    });
    return;
  }

  commit({
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

  // `[FACT]` A stop that was asked for while this start was in flight is
  // issued now: the session exists from here on, so the cancel can find
  // it. Without this, "stop" during the provider handshake would answer
  // `AiReviewNotRunning` and the stream would carry on.
  if (cancelRequested) {
    cancelRequested = false;
    armed = false;
    try {
      await tauri.aiConflictCancel();
    } catch {
      // `[INFERENCE]` A session that is already gone needs no complaint:
      // the user asked for it to stop, and it is not running.
    }
    commit({ ...get(conflict), status: 'cancelled' });
    releaseSubscription();
  }
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
  const s = get(conflict);
  if (!isBusy(s)) return;
  if (s.sessionId === null) {
    // `[FACT]` The Rust side installs the session after the provider
    // call returns, so a cancel issued now finds nothing to cancel and
    // answers `AiReviewNotRunning` — while the start goes on to open a
    // stream the user believes they stopped. The intent is carried
    // through startup instead: `startArbitration` issues the cancel the
    // moment the session exists.
    cancelRequested = true;
    return;
  }
  try {
    await tauri.aiConflictCancel();
  } catch (e) {
    const err = normalizeError(e);
    if (err.kind !== 'AiReviewNotRunning') {
      commit({
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
  armed = false;
  commit({ ...get(conflict), status: 'cancelled' });
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
    commit({
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
  commit({ ...s, diffSeen: true });
}

/**
 * Record the replacement the user chose.
 *
 * `[PROPOSAL]` ADR §3.4.4: nothing is written here. The decision is
 * recorded; writing it back is the write path's job
 * (`applyDecisions`).
 *
 * `[PROPOSAL]` ADR §3.7.3: the same decision is what produces the
 * **audit record**, and the record is written by the backend — the
 * webview only says *which* choice was made. The write is kicked off here
 * rather than from the dialog's four handlers so that no choice can be
 * made without one: the two live in the same function.
 *
 * `[INFERENCE]` The write is **not** awaited and its failure is not
 * allowed to reach the user: the decision has already been made and shown,
 * and a trail that cannot be written must not undo the arbitration. A
 * failed write is reported on the console, which is where the app's other
 * non-fatal failures go (`App.svelte`).
 *
 * `[INFERENCE]` Re-choosing the *same* replacement **for the same region**
 * is not a new decision, so it does not write a second record: one
 * decision, one record.
 *
 * `[FACT]` The decision is stored **per region**, in the `decisions` map,
 * keyed on the region's id. Two regions whose "ours" side happens to be
 * the same text are two decisions, and the second one's audit record
 * would be dropped without a word — a hole in a trail that exists to say
 * what was decided. The map is also what makes the write-back possible:
 * a path is written back only when every one of its regions has been
 * decided (ADR §3.6).
 */
export function setResolution(choice: ConflictChoice, text: string, region: ConflictRegion): void {
  const s = get(conflict);
  const previous = s.decisions[region.id];
  const unchanged = previous !== undefined && previous.choice === choice && previous.text === text;
  commit({
    decisions: { ...s.decisions, [region.id]: { choice, text } },
  });
  if (unchanged) return;
  void writeAuditRecord(region, choice);
}

/**
 * Ask the backend to write the audit record for a decision.
 *
 * `[INFERENCE]` The record names the region it is about, so the region
 * travels with it. The backend recomputes the region's id from that
 * content, so a record cannot be written about a region the caller did
 * not actually arbitrate.
 */
async function writeAuditRecord(region: ConflictRegion, choice: ConflictChoice): Promise<void> {
  try {
    await tauri.conflictAuditRecord(region, choice);
  } catch (e) {
    console.warn('could not write the conflict audit record', e);
  }
}

/**
 * Load the conflict regions of a repository.
 *
 * `[PROPOSAL]` ADR §3.1: the regions come from the Rust core's parse of
 * `git merge-file`'s output.
 */
export async function loadRegions(
  repo: string,
  root: string | null = null,
  seam: typeof tauri.conflictRegions = tauri.conflictRegions
): Promise<ConflictRegion[]> {
  try {
    // `[FACT]` The root travels with the name: the desktop keeps its
    // repositories as bare `<name>.git` directories, and the checkouts live
    // under the root the user picked. Without it the command would resolve
    // a bare repository, where `git` answers `NotAWorkTree`.
    const payload = await seam(repo, root);
    const regions = payload.regions;
    commit({
      ...get(conflict),
      regions,
      needsGit: payload.needsGit,
      selectedRegionId: regions[0]?.id ?? null,
    });
    return regions;
  } catch (e) {
    const err = normalizeError(e);
    commit({
      ...get(conflict),
      regions: [],
      needsGit: [],
      selectedRegionId: null,
      status: 'error',
      errorKind: err.kind,
      errorMessage: err.message,
    });
    return [];
  }
}

/**
 * Open one region in the dialog.
 *
 * `[FACT]` An arbitration result belongs to the region it was asked
 * about. Opening another region therefore clears it: the status, the
 * accumulated text, the suggestion and the diff-seen flag all describe
 * the region that was just closed, and leaving them in place would show
 * one region's answer against another's text — and would let the user
 * accept it there.
 *
 * `[FACT]` The **decisions** are not cleared. They are the user's work,
 * they belong to their own regions, and ADR §3.6 writes a path back only
 * when every one of its regions has been decided — so a decision that
 * disappeared when the user looked at the next region would make the
 * write-back impossible to reach.
 *
 * `[FACT]` A stream that is still running belongs to the closed
 * region, so it is cancelled and its events are no longer accepted.
 * The cancellation is not awaited: the state is replaced immediately,
 * and the `armed` flag is what stops the old stream from writing into
 * the new one.
 */
export function selectRegion(regionId: string): void {
  const s = get(conflict);
  if (s.selectedRegionId === regionId) return;
  if (isBusy(s)) {
    armed = false;
    releaseSubscription();
    void tauri.aiConflictCancel().catch(() => undefined);
  }
  terminalSeen = false;
  pendingModel = null;
  cancelRequested = false;
  const before = get(conflict);
  commit({
    ...INITIAL,
    regions: before.regions,
    selectedRegionId: regionId,
    keyStatus: before.keyStatus,
    decisions: before.decisions,
  });
}

/**
 * The message recorded when a write-back is asked for before every region
 * has been decided.
 *
 * `[FACT]` The count is a placeholder, the way the rest of the catalogue
 * spells its placeholders.
 */
export const CONFLICT_UNDECIDED_MESSAGE = '{n} conflict region(s) have no decision yet';

/**
 * The message recorded when a write-back is asked for with no region to
 * write.
 */
export const CONFLICT_NO_REGIONS_MESSAGE = 'there is no conflict region to write back';

/**
 * Whether the "apply" action may be pressed.
 *
 * `[FACT]` ADR-0024 §3.6: a path is written back only when **every** one
 * of its regions has been decided, so the gate is "every arbitrable
 * region has a decision". A path git has to resolve itself is not
 * arbitrable and does not count — it is not in `regions` at all.
 */
export function canApply(s: ConflictState): boolean {
  if (s.regions.length === 0) return false;
  return s.regions.every((region) => s.decisions[region.id] !== undefined);
}

/**
 * The seams `applyDecisions` talks to.
 *
 * `[FACT]` The store's other seams work the same way (`loadRegions`,
 * `loadKeyStatus`): the default is the Tauri command and a test passes
 * its own, so the store can be driven without a Rust backend.
 */
export interface ApplyOptions {
  /** The write-back seam. Defaults to the Tauri command. */
  resolve?: typeof tauri.conflictResolve;
  /** The region-list seam, used to refresh the list after a write-back. */
  load?: typeof tauri.conflictRegions;
}

/**
 * Write the decided regions back into the work tree and stage them
 * (ADR-0024 §3.6).
 *
 * `[FACT]` The application does **not** drive the merge itself: it writes
 * the resolved file back and `git add`s the path, and the user commits.
 *
 * `[FACT]` Two phases:
 *
 * 1. **Decide.** Every arbitrable region must have a decision. A region
 *    without one stops the whole write-back, and not one byte is written
 *    anywhere — the path could not be written back anyway, and a
 *    half-arbitrated file in the work tree would be worse than none.
 * 2. **Write.** The decisions are grouped by path and the command is
 *    called **once per path**, because the command handles one path: a
 *    merge spans several files and each file is written and staged on its
 *    own.
 *
 * `[FACT]` A failure on one path stops the loop, and the outcome says
 * which paths were written, which one failed and which were not
 * attempted. A report that said "done" while some paths were untouched
 * would be a lie, and the user would commit a half-resolved merge.
 */
export async function applyDecisions(
  repo: string,
  root: string | null = null,
  opts: ApplyOptions = {}
): Promise<ConflictApplyOutcome> {
  const resolve = opts.resolve ?? tauri.conflictResolve;
  const load = opts.load ?? tauri.conflictRegions;
  const s = get(conflict);
  commit({ applying: true, applyError: null, applyOutcome: null });

  // --- phase 1: every arbitrable region has a decision. ---
  if (s.regions.length === 0) {
    const outcome: ConflictApplyOutcome = { written: [], failed: null, skipped: [] };
    commit({ applying: false, applyError: CONFLICT_NO_REGIONS_MESSAGE, applyOutcome: outcome });
    return outcome;
  }
  const undecided = s.regions.filter((region) => s.decisions[region.id] === undefined);
  if (undecided.length > 0) {
    const outcome: ConflictApplyOutcome = { written: [], failed: null, skipped: [] };
    commit({
      applying: false,
      applyError: CONFLICT_UNDECIDED_MESSAGE.replace('{n}', String(undecided.length)),
      applyOutcome: outcome,
    });
    return outcome;
  }

  // --- group the decisions by path. ---
  const byPath = new Map<string, ConflictResolveChoice[]>();
  for (const region of s.regions) {
    const decision = s.decisions[region.id];
    // `[INFERENCE]` Unreachable: phase 1 returned for an undecided region.
    // The guard keeps the map's value type honest without a non-null
    // assertion.
    if (decision === undefined) continue;
    const choices = byPath.get(region.path) ?? [];
    choices.push({ index: region.index, choice: decision.choice, text: decision.text });
    byPath.set(region.path, choices);
  }

  // --- phase 2: one command call per path. ---
  const written: string[] = [];
  const skipped: string[] = [];
  let failed: { path: string; message: string } | null = null;
  for (const path of byPath.keys()) {
    if (failed !== null) {
      // `[INFERENCE]` The write-back stops at the first failure: a locked
      // index or a read-only work tree is not a fault of one path, and
      // the paths after it would fail the same way.
      skipped.push(path);
      continue;
    }
    try {
      await resolve({ repo, root, path, choices: byPath.get(path) ?? [] });
      written.push(path);
    } catch (e) {
      const err = normalizeError(e);
      failed = { path, message: err.message };
    }
  }

  const outcome: ConflictApplyOutcome = { written, failed, skipped };
  commit({
    applying: false,
    applyError: failed === null ? null : failed.message,
    applyOutcome: outcome,
  });

  // `[FACT]` A path that was written back is no longer unmerged, so the
  // region list has to be read again — otherwise the page would still
  // show regions that git has already resolved.
  if (written.length > 0) {
    await loadRegions(repo, root, load);
  }
  return outcome;
}
