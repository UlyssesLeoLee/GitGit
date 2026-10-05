/**
 * Tests for the AI review bridge in `src/lib/api/ai.ts`.
 *
 * The uncovered half of this file is `listenReviewEvents` (lines
 * 65-71) — the function that fans one handler out across five event
 * names and hands back a single unlisten. That function is the whole
 * reason the review store has a `subscribe` seam at all, and the
 * `store/review` tests deliberately bypass it by injecting their own
 * subscription, so nothing exercised it.
 *
 * `ai.ts` gets 50% statement coverage from the existing review tests
 * (they drive `aiReviewStart` through the real bridge). The cases here
 * concentrate on the fan-out and on the failure mode the docstring
 * claims to handle but does not.
 */
import { beforeEach, describe, expect, it, vi } from 'vitest';
import {
  REVIEW_EVENT_CANCELLED,
  REVIEW_EVENT_DONE,
  REVIEW_EVENT_FAILED,
  REVIEW_EVENT_MODEL,
  REVIEW_EVENT_TOKEN,
  aiReviewCancel,
  aiReviewStart,
  listenReviewEvents,
} from '../../src/lib/api/ai';
import type { ReviewEventDto } from '../../src/lib/api/types';
import { installMock } from '../../src/mocks/handlers';

// `vi.mock` is hoisted above the imports, so the spy it closes over
// has to be hoisted too.
const { listenMock } = vi.hoisted(() => ({ listenMock: vi.fn() }));
vi.mock('@tauri-apps/api/event', () => ({ listen: listenMock }));

/** Make every `listen` resolve to its own unlisten spy. */
function fakeListen(): ReturnType<typeof vi.fn>[] {
  const unlistens: ReturnType<typeof vi.fn>[] = [];
  listenMock.mockImplementation(async () => {
    const unlisten = vi.fn();
    unlistens.push(unlisten);
    return unlisten;
  });
  return unlistens;
}

/**
 * Record the command names sent through the real mock backend, so a
 * case can check the name while still getting the mock's own answer
 * for the payload.
 */
function recordThroughMock(): string[] {
  installMock();
  const inner = window.__TAURI_INTERNALS__?.invoke as (
    cmd: string,
    args?: Record<string, unknown>
  ) => Promise<unknown>;
  const cmds: string[] = [];
  window.__TAURI_INTERNALS__ = {
    invoke: (cmd: string, args?: Record<string, unknown>) => {
      cmds.push(cmd);
      return inner(cmd, args);
    },
  };
  return cmds;
}

beforeEach(() => {
  // `[FACT]` This used to `delete window.__TAURI_INTERNALS__` first,
  // because `installMock()` grepped the installed function's own source
  // for the substring "mock" — and these tests genuinely contain that
  // word (a `model: 'mock'` payload, above), so the mock was not always
  // reinstalled. The mock now identifies itself with a symbol, so the
  // reinstall is unconditional. `mocks-handlers.test.ts` pins the defect
  // and the fix; the absence of the delete here is the proof for this
  // file, whose own fixtures are the ones that used to trigger it.
  installMock();
  listenMock.mockReset();
  fakeListen();
});

describe('api / ai — event fan-out', () => {
  it('subscribes to all five event names in order', async () => {
    await listenReviewEvents(() => {});
    expect(listenMock.mock.calls.map((c) => c[0])).toEqual([
      REVIEW_EVENT_TOKEN,
      REVIEW_EVENT_MODEL,
      REVIEW_EVENT_DONE,
      REVIEW_EVENT_FAILED,
      REVIEW_EVENT_CANCELLED,
    ]);
  });

  it('keeps the event names the Rust side emits', async () => {
    // These strings are duplicated in `commands/ai.rs`. A rename there
    // that is not mirrored here fails silently: the review just never
    // streams. Pin the literals, not the constants' relationship.
    expect(REVIEW_EVENT_TOKEN).toBe('ai-review://token');
    expect(REVIEW_EVENT_MODEL).toBe('ai-review://model');
    expect(REVIEW_EVENT_DONE).toBe('ai-review://done');
    expect(REVIEW_EVENT_FAILED).toBe('ai-review://failed');
    expect(REVIEW_EVENT_CANCELLED).toBe('ai-review://cancelled');
  });

  it('hands the handler the payload, not the event envelope', async () => {
    const seen: ReviewEventDto[] = [];
    await listenReviewEvents((e) => seen.push(e));

    // The Tauri callback receives `{ event, id, payload }`; the store
    // only ever wants the payload. Passing the envelope through would
    // make `e.type` undefined at the first `if`.
    const handler = listenMock.mock.calls[0]?.[1] as (e: unknown) => void;
    handler({
      event: 'ai-review://token',
      id: 7,
      payload: { type: 'token', sessionId: 's1', delta: 'Hel' },
    });

    expect(seen).toEqual([{ type: 'token', sessionId: 's1', delta: 'Hel' }]);
  });

  it('routes every event name to the same handler', async () => {
    const seen: ReviewEventDto[] = [];
    await listenReviewEvents((e) => seen.push(e));

    const payloads: ReviewEventDto[] = [
      { type: 'model', sessionId: 's1', model: 'gpt-x' },
      { type: 'done', sessionId: 's1', tokens: 12 },
      { type: 'failed', sessionId: 's1', message: 'HTTP 429' },
      { type: 'cancelled', sessionId: 's1' },
    ];
    for (const call of listenMock.mock.calls) {
      const handler = call[1] as (e: unknown) => void;
      for (const payload of payloads) handler({ payload });
    }

    // Every one of the five channels delivers every payload, so each
    // of the four event kinds arrives five times: the fan-out is
    // complete, not just the first channel.
    expect(seen).toHaveLength(20);
    const kinds = seen.reduce<Record<string, number>>((acc, e) => {
      acc[e.type] = (acc[e.type] ?? 0) + 1;
      return acc;
    }, {});
    expect(kinds).toEqual({ model: 5, done: 5, failed: 5, cancelled: 5 });
  });

  it('detaches all five listeners through the one returned unlisten', async () => {
    const unlistens = fakeListen();
    const unlisten = await listenReviewEvents(() => {});
    expect(unlistens.every((u) => u.mock.calls.length === 0)).toBe(true);

    unlisten();

    // One closure, all five. A caller cannot leak a subscription by
    // forgetting to detach four of them.
    expect(unlistens.map((u) => u.mock.calls.length)).toEqual([1, 1, 1, 1, 1]);
  });

  it('detaches nothing the second time the unlisten runs', async () => {
    const unlistens = fakeListen();
    const unlisten = await listenReviewEvents(() => {});

    unlisten();
    unlisten();

    // `attached.splice(0)` empties the list, so the second call is a
    // no-op rather than a double-detach of already-detached channels.
    expect(unlistens.map((u) => u.mock.calls.length)).toEqual([1, 1, 1, 1, 1]);
  });

  it('detaches the listeners it already opened when a later listen fails', async () => {
    // `[FACT]` This case used to assert the opposite — `[0, 0]`, zero
    // unlisten calls — because that was the measured behaviour while it
    // was a bug. The `await` inside the `for` loop meant a rejection
    // escaped before the `return`, so the closure that would have
    // detached the first listeners never existed and nothing else held a
    // reference to them: two live event channels with no handle anyone
    // could ever use, for the life of the page. `ai.ts` now tracks each
    // unlisten as it arrives and detaches the lot before rethrowing, so
    // the same two listeners are each released exactly once.
    const unlistens = fakeListen();
    const failure = new Error('event bridge is not available');
    let call = 0;
    listenMock.mockImplementation(async () => {
      if (call++ < 2) {
        const unlisten = vi.fn();
        unlistens.push(unlisten);
        return unlisten;
      }
      throw failure;
    });

    // The caller still sees the original failure, not a detach error.
    await expect(listenReviewEvents(() => {})).rejects.toBe(failure);
    // The two that did open are both released; the three that never
    // opened have no unlisten to call.
    expect(unlistens.map((u) => u.mock.calls.length)).toEqual([1, 1]);
  });

  it('still detaches the rest when one unlisten throws on the failure path', async () => {
    // `[FACT]` A detached listener is user code as far as this module is
    // concerned. If the first unlisten throws, a naive cleanup would
    // abandon the loop and leave the second listener attached — trading
    // one leak for another, plus a second error thrown over the failure
    // the caller actually needs. The loop therefore releases every
    // handle it collected and swallows the teardown error.
    const unlistens: ReturnType<typeof vi.fn>[] = [];
    const failure = new Error('event bridge is not available');
    let call = 0;
    listenMock.mockImplementation(async () => {
      if (call++ < 2) {
        const unlisten = vi.fn(() => {
          if (unlistens.length === 1) {
            throw new Error('channel already gone');
          }
        });
        unlistens.push(unlisten);
        return unlisten;
      }
      throw failure;
    });

    await expect(listenReviewEvents(() => {})).rejects.toBe(failure);
    // The thrower did not stop the second detach.
    expect(unlistens.map((u) => u.mock.calls.length)).toEqual([1, 1]);
  });
});

describe('api / ai — start and cancel', () => {
  it('sends the request under the keys the Rust command expects', async () => {
    const calls: { cmd: string; args: Record<string, unknown> | undefined }[] = [];
    window.__TAURI_INTERNALS__ = {
      invoke: (cmd, args) => {
        calls.push({ cmd, args });
        return Promise.resolve({
          session_id: 's1', provider: 'openai', model: 'gpt-x', streaming: true, redactions: 0,
        });
      },
    };

    await aiReviewStart({ diff: 'diff --git a/x b/x', provider: 'openai', model: 'gpt-x', baseUrl: 'https://api.openai.com' });

    // `baseUrl` is the one camelCase key here; `base_url` would
    // compile and then be dropped by the deserializer.
    expect(calls).toEqual([
      {
        cmd: 'ai_review_start',
        args: {
          diff: 'diff --git a/x b/x',
          provider: 'openai',
          model: 'gpt-x',
          baseUrl: 'https://api.openai.com',
        },
      },
    ]);
  });

  it('sends unset options as null rather than omitting them', async () => {
    const calls: Record<string, unknown>[] = [];
    window.__TAURI_INTERNALS__ = {
      invoke: (_cmd, args) => {
        calls.push(args ?? {});
        return Promise.resolve({
          session_id: 's1', provider: 'openai', model: 'mock', streaming: true, redactions: 0,
        });
      },
    };

    await aiReviewStart({ diff: 'diff', provider: null, model: null, baseUrl: null });

    expect(calls[0]).toEqual({ diff: 'diff', provider: null, model: null, baseUrl: null });
  });

  it('returns the started session as-is', async () => {
    const started = await aiReviewStart({ diff: 'diff', provider: 'openai', model: null, baseUrl: null });
    expect(started.session_id).toMatch(/^mock-session-/);
    expect(started.streaming).toBe(true);
  });

  it('propagates a typed refusal for an empty diff', async () => {
    // The store turns `kind` into a localized message, so the wrapper
    // must let it through untouched.
    await expect(
      aiReviewStart({ diff: '   ', provider: 'openai', model: null, baseUrl: null })
    ).rejects.toMatchObject({ kind: 'AiReviewInvalid' });
  });

  it('propagates the unsupported refusal for a provider that cannot stream', async () => {
    await expect(
      aiReviewStart({ diff: 'diff', provider: 'anthropic', model: null, baseUrl: null })
    ).rejects.toMatchObject({ kind: 'AiReviewUnsupported' });
  });

  it('returns the session id a cancel stopped', async () => {
    const started = await aiReviewStart({ diff: 'diff', provider: 'openai', model: null, baseUrl: null });
    const cmds = recordThroughMock();

    expect(await aiReviewCancel()).toBe(started.session_id);
    expect(cmds).toEqual(['ai_review_cancel']);
  });

  it('propagates the typed refusal when no review is streaming', async () => {
    // One case for start-then-cancel-twice: the mock backend tracks a
    // single active session, so "nothing is running" is only reachable
    // from inside the same case that started one.
    await aiReviewStart({ diff: 'diff', provider: 'openai', model: null, baseUrl: null });
    const first = await aiReviewCancel();
    expect(first).toMatch(/^mock-session-/);
    await expect(aiReviewCancel()).rejects.toMatchObject({ kind: 'AiReviewNotRunning' });
  });
});
