/**
 * Streaming AI review store (V0 task T9).
 *
 * All review state lives here rather than in the page component so the
 * event-ordering rules are testable without rendering. The rules that
 * matter:
 *
 * - The event subscription is attached **before** `ai_review_start` is
 *   invoked. The Rust side spawns the stream and begins emitting
 *   immediately, so subscribing afterwards drops the first tokens — the
 *   one thing a user would definitely notice.
 * - Events whose `sessionId` is not the current one are dropped. A
 *   cancelled stream's last events can still be in flight, and appending
 *   them to the next review would splice two reviews together with no
 *   visible boundary. The one exception is a stream whose id is not yet
 *   known because the start command has not resolved; only the stream we
 *   just started can be talking on that subscription.
 * - A channel that closes without a terminal event is a failure, not a
 *   completion. The UI must not render a truncated review as a finished
 *   one.
 */

import { get, writable } from 'svelte/store';
import * as tauri from '$lib/api/ai';
import { normalizeError } from '$lib/utils/errors';
import type { ReviewEventDto } from '$lib/api/types';

/**
 * Hard cap on accumulated review text, in characters.
 *
 * Model output is untrusted: it is appended verbatim into a live DOM
 * text node. Bounded accumulation keeps a looping or hostile provider
 * from growing the webview without limit. Svelte escapes the text on
 * render, so this is a memory bound, not an XSS defence.
 */
export const REVIEW_MAX_TEXT_CHARS = 200_000;

export type ReviewStatus =
  | 'idle'
  | 'starting'
  | 'streaming'
  | 'done'
  | 'cancelled'
  | 'error'
  | 'unsupported';

export interface ReviewState {
  status: ReviewStatus;
  sessionId: string | null;
  provider: string;
  model: string;
  /** Review text accumulated so far, exactly as the model produced it. */
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
  /** True once `REVIEW_MAX_TEXT_CHARS` was reached. */
  truncated: boolean;
}

const INITIAL: ReviewState = {
  status: 'idle',
  sessionId: null,
  provider: '',
  model: '',
  text: '',
  tokenCount: 0,
  errorKind: null,
  errorMessage: null,
  unsupportedProvider: null,
  redactions: 0,
  truncated: false,
};

export const review = writable<ReviewState>({ ...INITIAL });

/** True while a start is in flight or a stream is open. */
export function isBusy(s: ReviewState): boolean {
  return s.status === 'starting' || s.status === 'streaming';
}

export interface StartOptions {
  provider?: string | null;
  model?: string | null;
  baseUrl?: string | null;
  /**
   * Event-subscription seam. Defaults to the Tauri event bridge; tests
   * pass their own so the store can be driven without a Rust backend.
   */
  subscribe?: typeof tauri.listenReviewEvents;
}

/** Detaches the live subscription, if any. */
let detach: (() => void) | null = null;

function releaseSubscription(): void {
  if (detach) {
    detach();
    detach = null;
  }
}

/** Reset to the initial state and drop any subscription. */
export function resetReview(): void {
  releaseSubscription();
  review.set({ ...INITIAL });
}

/*
 * `[FACT]` This store used to carry its own `asAppError`, a near-duplicate
 * of `normalizeError` with a different fallback order. The difference was
 * user-visible: when a payload had a `kind` but no usable `message`, that
 * function fell back to `String(e)`, so the detail line rendered
 * `[object Object]`. `normalizeError` returns `''` in that case, which is
 * the correct "there is no detail to show" signal and is what the review
 * page already handles.
 */

/** Apply one event. Private: tests drive it through the subscription. */
function applyReviewEvent(e: ReviewEventDto): void {
  const s = get(review);
  // A known session means anything else is a leftover from a stream we
  // already stopped. An unknown one means the start command has not
  // resolved yet, and this subscription can only carry the new stream.
  if (s.sessionId !== null && e.sessionId !== s.sessionId) return;

  switch (e.type) {
    case 'token': {
      if (s.status !== 'streaming' && s.status !== 'starting') return;
      const room = Math.max(0, REVIEW_MAX_TEXT_CHARS - s.text.length);
      const delta = e.delta.length > room ? e.delta.slice(0, room) : e.delta;
      review.set({
        ...s,
        text: s.text + delta,
        tokenCount: s.tokenCount + 1,
        truncated: s.truncated || delta.length < e.delta.length,
      });
      return;
    }
    case 'model': {
      if (s.sessionId === null) return; // start has not resolved yet
      review.set({ ...s, model: e.model });
      return;
    }
    case 'done': {
      review.set({ ...s, status: 'done', tokenCount: e.tokens });
      releaseSubscription();
      return;
    }
    case 'failed': {
      review.set({ ...s, status: 'error', errorKind: 'Gitgit', errorMessage: e.message });
      releaseSubscription();
      return;
    }
    case 'cancelled': {
      review.set({ ...s, status: 'cancelled' });
      releaseSubscription();
      return;
    }
  }
}

/**
 * Start a streaming review. Errors are folded into the store rather
 * than thrown, so a page can bind to one object and render every state
 * including failure.
 */
export async function startReview(diff: string, opts: StartOptions = {}): Promise<void> {
  if (isBusy(get(review))) return;
  const provider = opts.provider ?? null;
  releaseSubscription();
  review.set({
    ...INITIAL,
    status: 'starting',
    provider: provider ?? '',
  });

  const subscribe = opts.subscribe ?? tauri.listenReviewEvents;
  try {
    detach = await subscribe(applyReviewEvent);
  } catch (e) {
    review.set({ ...get(review), status: 'error', ...normalizeError(e) });
    return;
  }

  let started;
  try {
    started = await tauri.aiReviewStart({
      diff,
      provider,
      model: opts.model ?? null,
      baseUrl: opts.baseUrl ?? null,
    });
  } catch (e) {
    const err = normalizeError(e);
    releaseSubscription();
    if (err.kind === 'AiReviewUnsupported') {
      review.set({
        ...get(review),
        status: 'unsupported',
        unsupportedProvider: provider,
        errorKind: err.kind,
        errorMessage: err.message,
      });
      return;
    }
    review.set({
      ...get(review),
      status: 'error',
      errorKind: err.kind,
      errorMessage: err.message,
    });
    return;
  }

  review.set({
    ...get(review),
    status: 'streaming',
    sessionId: started.session_id,
    provider: started.provider,
    model: started.model,
    redactions: started.redactions,
  });
}

/**
 * Stop the running review.
 *
 * The Rust command aborts the consuming task, so this ends the request
 * rather than the rendering. If the stream had already reached a
 * terminal state, `ai_review_cancel` refuses with
 * `AiReviewNotRunning`; that refusal is not an error to show the user,
 * it means the terminal event already owns the state, so it is left
 * alone.
 */
export async function stopReview(): Promise<void> {
  if (!isBusy(get(review))) return;
  try {
    await tauri.aiReviewCancel();
  } catch (e) {
    const err = normalizeError(e);
    if (err.kind !== 'AiReviewNotRunning') {
      review.set({
        ...get(review),
        status: 'error',
        errorKind: err.kind,
        errorMessage: err.message,
      });
      releaseSubscription();
      return;
    }
    return;
  }
  review.set({ ...get(review), status: 'cancelled' });
  releaseSubscription();
}
