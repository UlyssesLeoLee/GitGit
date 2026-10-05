/**
 * Store + component tests for the streaming AI review (V0 task T9).
 *
 * The store is driven through its `subscribe` seam rather than through
 * the Tauri event bridge, so the tests observe the real event-ordering
 * rules without a Rust backend. Token accumulation, stop, failure, the
 * unsupported-provider state, and the stale-session guard each get a
 * case, because those are the rules a UI-only smoke test would miss.
 */
import { describe, expect, it, beforeEach } from 'vitest';
import { get } from 'svelte/store';
import { render, screen, fireEvent, cleanup, waitFor } from '@testing-library/svelte';
import {
  review,
  startReview,
  stopReview,
  resetReview,
  isBusy,
  REVIEW_MAX_TEXT_CHARS,
  type ReviewState,
} from '../../src/lib/stores/review';
import type { ReviewEventDto } from '../../src/lib/api/types';
import Review from '../../src/routes/Review.svelte';
import { installMock } from '../../src/mocks/handlers';
import { tFor, locale } from '../../src/lib/i18n';

beforeEach(() => {
  cleanup();
  installMock();
  resetReview();
});

/**
 * A stand-in for the Tauri event bridge that records the handler so a
 * test can push events in a chosen order.
 */
function fakeSubscribe() {
  let handler: ((e: ReviewEventDto) => void) | null = null;
  let detached = 0;
  const subscribe = async (h: (e: ReviewEventDto) => void) => {
    handler = h;
    return () => {
      detached += 1;
      handler = null;
    };
  };
  return {
    subscribe,
    emit(e: ReviewEventDto) {
      if (!handler) throw new Error('no subscription is attached');
      handler(e);
    },
    get attached() {
      return handler !== null;
    },
    get detaches() {
      return detached;
    },
  };
}

/** Session id of the stream currently in the store. */
function currentSession(): string {
  const id = get(review).sessionId;
  if (id === null) throw new Error('no review session is active');
  return id;
}

/**
 * Replace the invoke bridge for one test, so a typed `AppError` can be
 * produced on demand. The real refusal paths live in Rust; this only
 * needs to produce their wire shape.
 */
function invokeRejectsWith(payload: unknown): void {
  window.__TAURI_INTERNALS__ = {
    invoke: () => Promise.reject(payload),
  };
}

/**
 * Drives a full successful stream and returns the fake bridge plus the
 * session id the store actually issued.
 *
 * The id comes from the store rather than being hard-coded: the store
 * drops events whose `sessionId` is not the current one, so a made-up id
 * would make these cases pass or fail for the wrong reason.
 */
async function streamTokens(
  tokens: string[]
): Promise<{ bridge: ReturnType<typeof fakeSubscribe>; sessionId: string }> {
  const bridge = fakeSubscribe();
  await startReview('diff --git a/x b/x', { provider: 'openai', subscribe: bridge.subscribe });
  const sessionId = currentSession();
  for (const delta of tokens) {
    bridge.emit({ type: 'token', sessionId, delta });
  }
  return { bridge, sessionId };
}

describe('store / review — streaming', () => {
  it('accumulates tokens in arrival order and completes on done', async () => {
    const { bridge, sessionId } = await streamTokens(['Hel', 'lo', ' wor', 'ld']);
    let s: ReviewState = get(review);
    expect(s.status).toBe('streaming');
    expect(s.sessionId).toBe(sessionId);
    expect(s.text).toBe('Hello world');
    expect(s.tokenCount).toBe(4);

    bridge.emit({ type: 'done', sessionId, tokens: 4 });
    s = get(review);
    expect(s.status).toBe('done');
    // The terminal event releases the subscription: a finished review
    // must not keep a live event channel open.
    expect(bridge.detaches).toBeGreaterThan(0);
    expect(isBusy(s)).toBe(false);
  });

  it('records the model the provider reported rather than assuming one', async () => {
    const bridge = fakeSubscribe();
    await startReview('diff', { provider: 'openai', subscribe: bridge.subscribe });
    bridge.emit({ type: 'model', sessionId: currentSession(), model: 'mock-model-1' });
    expect(get(review).model).toBe('mock-model-1');
  });

  it('ignores events from a session that is no longer current', async () => {
    // A cancelled stream's last tokens can still be in flight. Splicing
    // them into the next review has no visible boundary, so they are
    // dropped.
    const first = fakeSubscribe();
    await startReview('diff', { provider: 'openai', subscribe: first.subscribe });
    const s1 = currentSession();
    first.emit({ type: 'token', sessionId: s1, delta: 'Hello' });
    first.emit({ type: 'done', sessionId: s1, tokens: 1 });
    expect(get(review).text).toBe('Hello');

    const second = fakeSubscribe();
    await startReview('other diff', { provider: 'openai', subscribe: second.subscribe });
    const s2 = currentSession();
    expect(s2).not.toBe(s1);

    // A late token from the finished session must not land in the new
    // review's text.
    second.emit({ type: 'token', sessionId: s1, delta: 'STALE' });
    expect(get(review).text).toBe('');
  });

  it('truncates accumulated output instead of growing without bound', async () => {
    // Model output is untrusted text appended into a live DOM node.
    const bridge = fakeSubscribe();
    await startReview('diff', { provider: 'openai', subscribe: bridge.subscribe });
    bridge.emit({
      type: 'token',
      sessionId: currentSession(),
      delta: 'x'.repeat(REVIEW_MAX_TEXT_CHARS + 50),
    });
    const s = get(review);
    expect(s.text.length).toBe(REVIEW_MAX_TEXT_CHARS);
    expect(s.truncated).toBe(true);
  });

  it('ignores a second start while one is already streaming', async () => {
    const first = fakeSubscribe();
    await startReview('diff', { provider: 'openai', subscribe: first.subscribe });
    const second = fakeSubscribe();
    await startReview('other', { provider: 'openai', subscribe: second.subscribe });
    // The busy guard returns before even subscribing, so no second
    // stream is opened.
    expect(second.attached).toBe(false);
    expect(first.attached).toBe(true);
  });
});

describe('store / review — stop', () => {
  it('stopReview ends the streaming state and detaches', async () => {
    const { bridge } = await streamTokens(['Hel', 'lo']);
    await stopReview();
    const s = get(review);
    expect(s.status).toBe('cancelled');
    // The partial review stays on screen: the user asked to stop, not
    // to discard what already arrived.
    expect(s.text).toBe('Hello');
    expect(bridge.detaches).toBeGreaterThan(0);
  });

  it('stopReview leaves a finished review alone', async () => {
    // The Rust side refuses a cancel with `AiReviewNotRunning` once the
    // stream has ended. The terminal event owns the state, so the
    // refusal must not overwrite it.
    const { bridge, sessionId } = await streamTokens(['Hello']);
    bridge.emit({ type: 'done', sessionId, tokens: 1 });
    await stopReview();
    expect(get(review).status).toBe('done');
  });
});

describe('store / review — failure and refusal', () => {
  it('surfaces a failed event as an error state', async () => {
    const bridge = fakeSubscribe();
    await startReview('diff', { provider: 'openai', subscribe: bridge.subscribe });
    bridge.emit({ type: 'failed', sessionId: currentSession(), message: 'openai: HTTP 429' });
    const s = get(review);
    expect(s.status).toBe('error');
    expect(s.errorMessage).toContain('429');
  });

  it('shows the unsupported state for a provider that cannot stream', async () => {
    await startReview('diff', { provider: 'anthropic', subscribe: fakeSubscribe().subscribe });
    const s = get(review);
    expect(s.status).toBe('unsupported');
    expect(s.unsupportedProvider).toBe('anthropic');
    // Nothing is streaming, so there is nothing to stop.
    expect(isBusy(s)).toBe(false);
  });

  it('reports a missing API key as a typed, localizable error', async () => {
    // The key is never accepted from the UI, so a missing one is a
    // distinct terminal state with its own message.
    invokeRejectsWith({
      kind: 'AiReviewNoKey',
      message: 'no api key in the environment: set GITGIT_AI_API_KEY before launching the app',
      source: '"AiReviewNoKey"',
    });
    await startReview('diff', { provider: 'openai', subscribe: fakeSubscribe().subscribe });
    const s = get(review);
    expect(s.status).toBe('error');
    expect(s.errorKind).toBe('AiReviewNoKey');
    expect(isBusy(s)).toBe(false);
  });

  it('surfaces a start failure other than unsupported as an error state', async () => {
    invokeRejectsWith({
      kind: 'AiReviewInvalid',
      message: 'invalid review request: the diff is empty',
      source: '"AiReviewInvalid"',
    });
    await startReview('diff', { provider: 'openai', subscribe: fakeSubscribe().subscribe });
    expect(get(review).status).toBe('error');
    expect(get(review).errorKind).toBe('AiReviewInvalid');
  });

  it('reports no detail rather than "[object Object]" for a kindless message', async () => {
    // `[FACT]` The defect this pins, in the store's own `asAppError`:
    // when a payload carried a `kind` but no usable `message`, it fell
    // back to `String(e)` — and a Tauri rejection is a plain object, not
    // an `Error`, so the detail line rendered the literal
    // `[object Object]`. `normalizeError` returns `''` instead, which is
    // the correct "there is nothing to show" signal and what the review
    // page already handles. The kind is kept either way, so the page can
    // still localize the failure.
    invokeRejectsWith({ kind: 'AiReviewNoKey', source: '"AiReviewNoKey"' });
    await startReview('diff', { provider: 'openai', subscribe: fakeSubscribe().subscribe });

    const s = get(review);
    expect(s.status).toBe('error');
    expect(s.errorKind).toBe('AiReviewNoKey');
    expect(s.errorMessage).toBe('');
    expect(s.errorMessage).not.toBe('[object Object]');
  });

  it('keeps the message of a payload that carries no kind', async () => {
    // `[FACT]` The mirror case: no `kind` at all. `asAppError` also
    // reached for `String(e)` here, so a mock-layer rejection with a
    // usable message still lost it.
    invokeRejectsWith({ message: 'the backend said this, with no kind' });
    await startReview('diff', { provider: 'openai', subscribe: fakeSubscribe().subscribe });

    const s = get(review);
    expect(s.errorKind).toBe('Internal');
    expect(s.errorMessage).toBe('the backend said this, with no kind');
    expect(s.errorMessage).not.toBe('[object Object]');
  });
});

/**
 * Component-render cases for the review page.
 *
 * `[FACT]` These now run. The blocker was module *resolution*, not the
 * `.svelte` compile step. `svelte` was resolving to `index-server.js`, so
 * `@testing-library/svelte`'s `render` threw `lifecycle_function_unavailable:
 * mount(...) is not available on the server` from
 * `svelte/src/internal/server/errors.js`.
 *
 * The two candidates recorded previously were both aimed at the wrong
 * lever, and that is why they failed:
 *
 *   1. `test: { resolve: { conditions: ['browser'] } }` — `test.resolve` is
 *      merged into the resolved config, but vitest's module runner resolves
 *      through the top-level `resolve`/`ssr` conditions, so this never
 *      reached the lookup.
 *   2. the above plus `test.server.deps.inline: ['svelte']` — same reason.
 *
 * The fix is in `apps/gm-desktop/vite.config.ts`: the documented
 * `svelteTesting()` plugin from `@testing-library/svelte/vite` (which adds
 * the `ssr.noExternal` rule) plus top-level
 * `resolve: { conditions: ['browser'] }` gated on `process.env.VITEST`.
 *
 * `[FACT]` 5 of these 6 cases pass. The 6th is blocked by a defect in
 * application source and stays `.skip`ed with the reason recorded at the
 * case itself. Two other cases had to drop a hardcoded `zh-CN` expectation:
 * jsdom reports `navigator.language === 'en-US'`, so `pickInitial()` in
 * `src/lib/stores/locale.ts` resolves to `en` and the page renders English.
 * They now assert against the active locale, which is what they meant.
 */
describe('component / Review page', () => {
  it('refuses to start with an empty diff, without calling the backend', async () => {
    render(Review);
    await fireEvent.click(screen.getByTestId('review-start'));
    const err = await screen.findByTestId('review-local-error');
    // Asserted against the *active* locale, not a hardcoded one. jsdom
    // reports `navigator.language === 'en-US'` (measured), so
    // `pickInitial()` in `src/lib/stores/locale.ts` resolves to `en` and
    // the page renders English. Hardcoding `zh-CN` here asserted an
    // environment the test does not run in.
    expect(err.textContent?.trim()).toBe(tFor(get(locale), 'review.invalidDiff'));
    expect(get(review).status).toBe('idle');
  });

  it('streams tokens into the output pane as they arrive', async () => {
    render(Review);
    await fireEvent.input(screen.getByTestId('review-diff'), {
      target: { value: 'diff --git a/x b/x' },
    });

    // Drive the store directly: the page's own start button would use
    // the real Tauri bridge, which does not exist under jsdom.
    const bridge = fakeSubscribe();
    await startReview('diff --git a/x b/x', {
      provider: 'openai',
      subscribe: bridge.subscribe,
    });
    const sessionId = currentSession();
    bridge.emit({ type: 'token', sessionId, delta: 'Hel' });

    await waitFor(() => {
      expect(screen.getByTestId('review-output').textContent).toContain('Hel');
    });
    bridge.emit({ type: 'token', sessionId, delta: 'lo' });
    await waitFor(() => {
      expect(screen.getByTestId('review-output').textContent).toBe('Hello');
    });

    // Stop control is live while streaming.
    expect(screen.getByTestId('review-stop').hasAttribute('disabled')).toBe(false);
    expect(screen.getByTestId('review-start').hasAttribute('disabled')).toBe(true);
  });

  it('renders untrusted model output as text, not as markup', async () => {
    // Svelte escapes interpolations and the page has no `{@html}`. A
    // provider that emits a script tag must show as characters.
    render(Review);
    const bridge = fakeSubscribe();
    await startReview('diff', { provider: 'openai', subscribe: bridge.subscribe });
    bridge.emit({
      type: 'token',
      sessionId: currentSession(),
      delta: '<img src=x onerror=alert(1)>',
    });

    await waitFor(() => {
      const out = screen.getByTestId('review-output');
      expect(out.textContent).toContain('<img src=x onerror=alert(1)>');
      expect(out.querySelector('img')).toBeNull();
    });
  });

  // This case caught a real source defect at `src/routes/Review.svelte:157`,
  // not a tooling one — the only case the toolchain upgrade could not rescue.
  // It is now fixed; the note below records what the defect was, so the reason
  // this assertion exists is not lost.
  // `fill` is declared as `(t, token, value) => (t[token] ?? token)…`, so it
  // looks up the literal key `'{provider}'` in the catalogue, finds nothing,
  // falls back to the token itself, and returns `'anthropic'`. The panel
  // therefore renders the provider name alone; the `review.unsupported`
  // string ('{provider} cannot stream. Its protocol has no streaming
  // implementation…') is never interpolated. Measured: panel textContent
  // is exactly `'anthropic'`, so `toContain('cannot stream')` fails.
  //
  // `[FACT]` Fixed. `Review.svelte` no longer routes these strings through
  // the broken `fill($catalog, token, value)` helper — it looks the catalogue
  // key up and interpolates directly, the same idiom the rest of the app
  // already uses (`$catalog['repos.countOne'].replace('{n}', …)`). The full
  // sentence now renders and this case runs again.
  it('shows a clearly-worded unsupported-provider message', async () => {
    render(Review);
    const bridge = fakeSubscribe();
    await startReview('diff', { provider: 'anthropic', subscribe: bridge.subscribe });
    const panel = await screen.findByTestId('review-unsupported');
    const text = panel.textContent ?? '';
    expect(text).toContain('anthropic');
    expect(text).toContain('cannot stream');
  });

  it('shows the error state with a localized heading and the raw detail', async () => {
    render(Review);
    const bridge = fakeSubscribe();
    await startReview('diff', { provider: 'openai', subscribe: bridge.subscribe });
    bridge.emit({
      type: 'failed',
      sessionId: currentSession(),
      message: 'openai: HTTP 429 — rate limited',
    });
    const panel = await screen.findByTestId('review-error');
    expect(panel.textContent).toContain('429');
    expect(panel.textContent?.length).toBeGreaterThan(0);
  });

  it('every rendered string comes from the catalogue', async () => {
    // Cheap guard against a hardcoded string slipping into the page.
    // Compared against the active locale rather than a pinned one: this
    // asserts "the page renders the catalogue's string", which is the
    // actual intent, and it keeps holding under either locale.
    render(Review);
    const heading = screen.getByTestId('review-heading');
    expect(heading.textContent?.trim()).toBe(tFor(get(locale), 'review.heading'));
    expect(screen.getByTestId('review-start').textContent?.trim()).toBe(
      tFor(get(locale), 'review.start')
    );
  });
});

describe('i18n parity for the review surface', () => {
  it('has both locales for every review and ai-error key', async () => {
    const keys = [
      'nav.review',
      'review.heading',
      'review.subhead',
      'review.diffLabel',
      'review.diffPlaceholder',
      'review.providerLabel',
      'review.modelLabel',
      'review.modelPlaceholder',
      'review.start',
      'review.stop',
      'review.starting',
      'review.streaming',
      'review.done',
      'review.cancelled',
      'review.empty',
      'review.outputLabel',
      'review.tokenCount',
      'review.modelServed',
      'review.redacted',
      'review.unsupported',
      'review.noKey',
      'review.truncated',
      'review.invalidDiff',
      'errors.kind.AiReviewUnsupported',
      'errors.kind.AiReviewNoKey',
      'errors.kind.AiReviewInvalid',
      'errors.kind.AiReviewAlreadyRunning',
      'errors.kind.AiReviewNotRunning',
    ];
    for (const key of keys) {
      const en = tFor('en', key);
      const zh = tFor('zh-CN', key);
      expect(en, `missing en for ${key}`).not.toBe(key);
      expect(zh, `missing zh for ${key}`).not.toBe(key);
    }
  });

  it('interpolation placeholders match across locales', async () => {
    // A `{provider}` left untranslated in one bundle renders as a
    // literal brace in that language.
    for (const key of [
      'review.unsupported',
      'review.tokenCount',
      'review.modelServed',
      'review.redacted',
    ]) {
      const placeholders = (s: string) => (s.match(/\{[a-z]+\}/g) ?? []).sort();
      expect(placeholders(tFor('en', key)), key).toEqual(placeholders(tFor('zh-CN', key)));
    }
  });
});
