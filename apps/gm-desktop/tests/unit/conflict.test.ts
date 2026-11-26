/**
 * Tests for the conflict arbitration (ADR-0024, Lane C).
 *
 * The acceptance list this file pins:
 *
 *  1. `/conflicts` is reachable.
 *  2. With no API key the dialog is still fully usable: keep ours, keep
 *     theirs and hand-edit all work, and "ask the AI" is *present but
 *     disabled* with an explanation.
 *  3. "Accept" is disabled until the user has looked at the diff.
 *  4. The suggestion is shown as a two-column diff.
 *  5. Model output reaches the DOM through plain text nodes.
 *  6. The four `/review` discipline rules each have an implementation and
 *     an assertion.
 *  7. `en.ts` and `zh-CN.ts` carry the same `conflict.*` key set.
 *  8. Every decision asks the backend for exactly one audit record
 *     (ADR-0024 §3.7.3), and a backend that cannot write the record does
 *     not undo the decision.
 *
 * The store is driven through its `subscribe` seam rather than through the
 * Tauri event bridge, so the cases observe the real event-ordering rules
 * without a Rust backend — the same shape `review.test.ts` uses.
 */
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { get } from 'svelte/store';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { readFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import {
  abandonArbitration,
  applyDecisions,
  canAccept,
  canApply,
  canAskAi,
  conflict,
  CONFLICT_MAX_TEXT_CHARS,
  CONFLICT_NO_TERMINAL_MESSAGE,
  CONFLICT_NO_REGIONS_MESSAGE,
  loadKeyStatus,
  loadRegions,
  markDiffSeen,
  resetArbitration,
  resolveChoice,
  selectRegion,
  setResolution,
  startArbitration,
  stopArbitration,
} from '../../src/lib/stores/conflict';
import type {
  ConflictEventDto,
  ConflictRegion,
  ConflictRegions,
  ConflictResolvePayload,
  ConflictResolveResult,
} from '../../src/lib/api/types';
import { supportsStreaming } from '../../src/lib/api/types';
import ConflictDialog from '../../src/lib/components/ConflictDialog.svelte';
import Conflicts from '../../src/routes/Conflicts.svelte';
import { installMock } from '../../src/mocks/handlers';
import { matchPath } from '../../src/lib/router';
import en from '../../src/lib/i18n/en';
import zh from '../../src/lib/i18n/zh-CN';

// tests/unit/ -> tests/ -> gm-desktop/
const APP_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..', '..');

const REGION: ConflictRegion = {
  id: 'a1b2c3d4e5f60718293a4b5c6d7e8f9',
  path: 'src/feature.ts',
  baseOid: '0123456789abcdef0123456789abcdef01234567',
  index: 0,
  ours: 'const timeout = 30;\n',
  base: 'const timeout = 10;\n',
  theirs: 'const timeout = 60;\n',
  contextBefore: 'export function connect() {\n',
  contextAfter: '  return retry(timeout);\n}\n',
  scopeLabel: 'connect',
  // `[FACT]` The backend's flag: the library's judgement that the region's
  // two sides carry the same text (ADR-0024 §3.7.4). These two sides
  // differ.
  sidesEquivalent: false,
};

/**
 * A stand-in for the Tauri event bridge that records the handler so a
 * test can push events in a chosen order.
 */
function fakeSubscribe() {
  let handler: ((e: ConflictEventDto) => void) | null = null;
  let detached = 0;
  const subscribe = async (h: (e: ConflictEventDto) => void) => {
    handler = h;
    return () => {
      detached += 1;
      handler = null;
    };
  };
  return {
    subscribe,
    emit(e: ConflictEventDto) {
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

/** Session id of the arbitration currently in the store. */
function currentSession(): string {
  const id = get(conflict).sessionId;
  if (id === null) throw new Error('no arbitration session is active');
  return id;
}

/** The region id the store believes it is arbitrating. */
function currentRegionId(): string {
  const id = get(conflict).regionId;
  if (id === null) throw new Error('no arbitration region is bound');
  return id;
}

/**
 * Replace the invoke bridge for one test, so a command can be answered
 * with a value the mock would not produce.
 */
function withInvoke(
  overrides: Record<string, (args: Record<string, unknown>) => unknown>
): void {
  const installed = window.__TAURI_INTERNALS__!.invoke;
  window.__TAURI_INTERNALS__ = {
    invoke: (cmd: string, args?: Record<string, unknown>) => {
      const over = overrides[cmd];
      if (over) return Promise.resolve(over(args ?? {}));
      return installed(cmd, args);
    },
  };
}

beforeEach(() => {
  cleanup();
  installMock();
  resetArbitration();
});

// ---------------------------------------------------------------------------
// The four `/review` discipline rules
// ---------------------------------------------------------------------------

describe('the conflict store — the /review discipline', () => {
  it('attaches the event subscription before the start command is issued', async () => {
    // `[FACT]` The Rust side spawns the stream and begins emitting
    // immediately, so a subscription attached after the start command
    // drops the first tokens — the one thing a user would definitely
    // notice.
    const order: string[] = [];
    const installed = window.__TAURI_INTERNALS__!.invoke;
    window.__TAURI_INTERNALS__ = {
      invoke: (cmd: string, args?: Record<string, unknown>) => {
        if (cmd === 'ai_conflict_start') order.push('start');
        return installed(cmd, args);
      },
    };
    const bridge = fakeSubscribe();
    const subscribe = async (h: (e: ConflictEventDto) => void) => {
      order.push('subscribe');
      return bridge.subscribe(h);
    };

    await startArbitration(REGION, { provider: 'openai', subscribe });
    expect(order).toEqual(['subscribe', 'start']);
    expect(bridge.attached).toBe(true);
  });

  it('reports the model the provider served, not the one that was requested', async () => {
    // `[FACT]` Providers report the served model in the first frame of the
    // stream, so across a real Tauri bridge the `model` event routinely
    // arrives while `ai_conflict_start` is still in flight — the session id
    // does not exist yet. `ConflictStarted.model` is the model that was
    // *requested* (`commands/ai.rs`), so writing it afterwards would
    // replace the served model with the requested one. The Rust suite
    // forbids that ("the served model must be reported, not assumed"), and
    // `$lib/stores/review.ts` already fixed it for the review channel.
    const bridge = fakeSubscribe();
    const subscribe = async (h: (e: ConflictEventDto) => void) => {
      const unlisten = await bridge.subscribe(h);
      // `[FACT]` The event lands before the start command has resolved.
      h({
        type: 'model',
        sessionId: 'not-known-yet',
        regionId: REGION.id,
        model: 'served-model',
      });
      return unlisten;
    };

    await startArbitration(REGION, {
      provider: 'openai',
      model: 'requested-model',
      subscribe,
    });
    expect(get(conflict).model).toBe('served-model');
  });

  it('ignores events from a session that is no longer current', async () => {
    // `[FACT]` A cancelled stream's last tokens can still be in flight.
    // Splicing them into the next arbitration has no visible boundary, so
    // they are dropped.
    const first = fakeSubscribe();
    await startArbitration(REGION, { provider: 'openai', subscribe: first.subscribe });
    const s1 = currentSession();
    first.emit({ type: 'token', sessionId: s1, regionId: currentRegionId(), delta: 'Hello' });
    first.emit({
      type: 'done',
      sessionId: s1,
      regionId: currentRegionId(),
      tokens: 1,
      outcome: { state: 'accepted', suggestion: 'Hello' },
    });
    expect(get(conflict).text).toBe('Hello');

    const second = fakeSubscribe();
    await startArbitration(REGION, { provider: 'openai', subscribe: second.subscribe });
    expect(currentSession()).not.toBe(s1);

    second.emit({ type: 'token', sessionId: s1, regionId: currentRegionId(), delta: 'STALE' });
    expect(get(conflict).text).toBe('');
  });

  it('ignores an event that names another region', async () => {
    // `[FACT]` ADR-0024 §3.7.2: an answer must be bound to the region
    // it was asked about. A token that names a different region cannot
    // land on this one.
    const bridge = fakeSubscribe();
    await startArbitration(REGION, { provider: 'openai', subscribe: bridge.subscribe });
    bridge.emit({
      type: 'token',
      sessionId: currentSession(),
      regionId: 'ffffffffffffffffffffffffffffffff',
      delta: 'OTHER REGION',
    });
    expect(get(conflict).text).toBe('');
  });

  it('records a failure when a busy session is closed without a terminal event', async () => {
    // `[FACT]` ADR-0024 §3.4.1: "a stream without a terminal event is
    // a failure, not a completion". The UI must not render a truncated
    // answer as a finished one.
    const bridge = fakeSubscribe();
    await startArbitration(REGION, { provider: 'openai', subscribe: bridge.subscribe });
    bridge.emit({ type: 'token', sessionId: currentSession(), regionId: currentRegionId(), delta: 'x' });
    expect(get(conflict).status).toBe('streaming');

    abandonArbitration();
    const s = get(conflict);
    expect(s.status).toBe('error');
    expect(s.errorMessage).toBe(CONFLICT_NO_TERMINAL_MESSAGE);
    // No suggestion, so nothing can be accepted.
    expect(s.suggestion).toBe(null);
    expect(canAccept(s)).toBe(false);
  });

  it('leaves a finished arbitration alone when it is closed', async () => {
    // The mirror of the case above: a session that *did* produce a
    // terminal event keeps its state.
    const bridge = fakeSubscribe();
    await startArbitration(REGION, { provider: 'openai', subscribe: bridge.subscribe });
    bridge.emit({
      type: 'done',
      sessionId: currentSession(),
      regionId: currentRegionId(),
      tokens: 1,
      outcome: { state: 'accepted', suggestion: 'done' },
    });
    abandonArbitration();
    expect(get(conflict).status).toBe('done');
  });

  it('truncates the accumulated answer instead of growing without bound', async () => {
    const bridge = fakeSubscribe();
    await startArbitration(REGION, { provider: 'openai', subscribe: bridge.subscribe });
    bridge.emit({
      type: 'token',
      sessionId: currentSession(),
      regionId: currentRegionId(),
      delta: 'x'.repeat(CONFLICT_MAX_TEXT_CHARS + 50),
    });
    const s = get(conflict);
    expect(s.text.length).toBe(CONFLICT_MAX_TEXT_CHARS);
    expect(s.truncated).toBe(true);
  });

  it('does not accept a suggestion the user has not looked at', async () => {
    const bridge = fakeSubscribe();
    await startArbitration(REGION, { provider: 'openai', subscribe: bridge.subscribe });
    bridge.emit({
      type: 'done',
      sessionId: currentSession(),
      regionId: currentRegionId(),
      tokens: 1,
      outcome: { state: 'accepted', suggestion: 'const timeout = 45;' },
    });
    // The suggestion arrived; the diff has not been opened.
    expect(get(conflict).status).toBe('done');
    expect(get(conflict).suggestion).toBe('const timeout = 45;');
    expect(canAccept(get(conflict))).toBe(false);

    markDiffSeen();
    expect(canAccept(get(conflict))).toBe(true);
  });

  it('never offers an accepted answer for a refused suggestion', async () => {
    const bridge = fakeSubscribe();
    await startArbitration(REGION, { provider: 'openai', subscribe: bridge.subscribe });
    bridge.emit({
      type: 'done',
      sessionId: currentSession(),
      regionId: currentRegionId(),
      tokens: 1,
      outcome: { state: 'rejected', reason: 'the suggestion carries a conflict marker' },
    });
    const s = get(conflict);
    expect(s.outcome).toBe('rejected');
    expect(s.rejectionReason).toBe('the suggestion carries a conflict marker');
    expect(s.suggestion).toBe(null);
    markDiffSeen();
    expect(canAccept(s)).toBe(false);
  });

  it('keeps the ask-AI option disabled while no key is known', async () => {
    const status = await loadKeyStatus('openai', async () => false);
    expect(status).toBe('absent');
    expect(canAskAi(get(conflict), true)).toBe(false);
  });

  it('stops a running arbitration and keeps what already arrived', async () => {
    // The user asked to stop, not to discard what already arrived.
    const bridge = fakeSubscribe();
    await startArbitration(REGION, { provider: 'openai', subscribe: bridge.subscribe });
    bridge.emit({
      type: 'token',
      sessionId: currentSession(),
      regionId: currentRegionId(),
      delta: 'Hello',
    });
    await stopArbitration();
    const s = get(conflict);
    expect(s.status).toBe('cancelled');
    expect(s.text).toBe('Hello');
  });

  it('enables the ask-AI option once a key is known', async () => {
    const status = await loadKeyStatus('openai', async () => true);
    expect(status).toBe('present');
    expect(canAskAi(get(conflict), true)).toBe(true);
  });

  it('clears the previous region\'s result when another region is opened', async () => {
    // `[FACT]` An arbitration result belongs to the region it was asked
    // about. Opening another region must clear it — otherwise the dialog
    // shows the first region's answer against the second region's text,
    // and can let the user accept it there.
    const bridge = fakeSubscribe();
    await startArbitration(REGION, { provider: 'openai', subscribe: bridge.subscribe });
    bridge.emit({
      type: 'done',
      sessionId: currentSession(),
      regionId: currentRegionId(),
      tokens: 1,
      outcome: { state: 'accepted', suggestion: 'const timeout = 45;' },
    });
    markDiffSeen();
    expect(canAccept(get(conflict))).toBe(true);

    selectRegion('ffffffffffffffffffffffffffffffff');
    const s = get(conflict);
    expect(s.status).toBe('idle');
    expect(s.suggestion).toBe(null);
    expect(s.outcome).toBe(null);
    expect(s.diffSeen).toBe(false);
    expect(s.text).toBe('');
    expect(s.resolution).toBe(null);
    expect(s.selectedRegionId).toBe('ffffffffffffffffffffffffffffffff');
    // Nothing may be accepted on the strength of the old region.
    expect(canAccept(s)).toBe(false);
  });

  it('does not let a closed region\'s stream write into the next one', async () => {
    // `[FACT]` A stream that outlives its arbitration must not write into
    // the next one: the session and region filters cannot catch it,
    // because a reset clears both.
    //
    // The handler is captured rather than driven through the bridge,
    // because the switch also detaches the subscription — the point of
    // the case is that the STORE drops the event, not that the event
    // never arrives.
    let captured: ((e: ConflictEventDto) => void) | null = null;
    const subscribe = async (h: (e: ConflictEventDto) => void) => {
      captured = h;
      return () => undefined;
    };
    await startArbitration(REGION, { provider: 'openai', subscribe });
    const session = currentSession();
    const region = currentRegionId();

    selectRegion('ffffffffffffffffffffffffffffffff');
    captured!({ type: 'token', sessionId: session, regionId: region, delta: 'STALE' });
    expect(get(conflict).text).toBe('');
  });

  it('carries a stop through a startup that was still in flight', async () => {
    // `[FACT]` The Rust side installs the session after the provider
    // call returns, so a cancel issued while the start is in flight
    // finds nothing to cancel — and the stream would carry on. The
    // intent has to be carried through startup.
    const bridge = fakeSubscribe();
    let cancelled = 0;
    let release: (() => void) | null = null;
    const gate = new Promise<void>((resolve) => {
      release = resolve;
    });
    withInvoke({
      ai_conflict_start: () => {
        // The start does not settle until the gate opens.
        return gate.then(() => ({
          sessionId: 'session-1',
          regionId: REGION.id,
          provider: 'openai',
          model: 'gpt-x',
          streaming: true,
          redactions: 0,
        }));
      },
      ai_conflict_cancel: () => {
        cancelled += 1;
        return 'session-1';
      },
    });

    const running = startArbitration(REGION, { provider: 'openai', subscribe: bridge.subscribe });
    // The start has not settled: there is no session to cancel yet.
    expect(get(conflict).status).toBe('starting');
    await stopArbitration();
    expect(cancelled).toBe(0);

    release!();
    await running;
    expect(cancelled).toBe(1);
    expect(get(conflict).status).toBe('cancelled');
  });

  it('refuses a hand-written replacement that carries a conflict marker', () => {
    // `[FACT]` ADR-0024 §3.7.4: the hand-edit gets the frontend's marker
    // check. It does **not** get the library's full validation — that half
    // of the rule is still a proposal (ADR §3.7.4's own row).
    const outcome = resolveChoice(REGION, 'manual', '<<<<<<< OURS\nconst timeout = 30;\n');
    expect(outcome.ok).toBe(false);
    if (!outcome.ok) expect(outcome.reason).toContain('conflict marker');
  });

  it('accepts a hand-written replacement that carries no marker', () => {
    const outcome = resolveChoice(REGION, 'manual', 'const timeout = 45;\n');
    expect(outcome.ok).toBe(true);
    if (outcome.ok) expect(outcome.text).toBe('const timeout = 45;\n');
  });
});

// ---------------------------------------------------------------------------
// The dialog component
// ---------------------------------------------------------------------------

describe('the conflict dialog — no API key', () => {
  it('offers keep ours, keep theirs and hand-edit, and disables "ask the AI"', async () => {
    // `[FACT]` ADR-0024 §3.7.1: the dialog does not degrade because
    // there is no AI. The three options that need no credential stay
    // available, and the fourth is shown but disabled with the reason.
    render(ConflictDialog, { props: { region: REGION } });

    expect(screen.getByTestId('conflict-keep-ours')).toBeTruthy();
    expect(screen.getByTestId('conflict-keep-theirs')).toBeTruthy();
    expect(screen.getByTestId('conflict-manual')).toBeTruthy();

    const ask = screen.getByTestId('conflict-ask-ai') as HTMLButtonElement;
    expect(ask).toBeTruthy();
    await waitFor(() => expect(ask.disabled).toBe(true));
    // The explanation says why, and where the key goes.
    const note = await waitFor(() => screen.getByTestId('conflict-no-key'));
    expect(note.textContent ?? '').not.toBe('');
    expect(note.textContent).toContain('Settings');
  });

  it('enables "ask the AI" when a key is configured', async () => {
    // The mirror of the case above: with a key the option is usable, and
    // the "no key" explanation is gone.
    withInvoke({ ai_key_status: () => true });
    render(ConflictDialog, { props: { region: REGION } });
    const ask = screen.getByTestId('conflict-ask-ai') as HTMLButtonElement;
    await waitFor(() => expect(ask.disabled).toBe(false));
    expect(screen.queryByTestId('conflict-no-key')).toBe(null);
  });

  it('keeps ours', async () => {
    render(ConflictDialog, { props: { region: REGION } });
    await fireEvent.click(screen.getByTestId('conflict-keep-ours'));
    await waitFor(() => expect(screen.getByTestId('conflict-resolution')).toBeTruthy());
    expect(screen.getByTestId('conflict-resolution-text').textContent).toBe(REGION.ours);
  });

  it('keeps theirs', async () => {
    render(ConflictDialog, { props: { region: REGION } });
    await fireEvent.click(screen.getByTestId('conflict-keep-theirs'));
    await waitFor(() => expect(screen.getByTestId('conflict-resolution')).toBeTruthy());
    expect(screen.getByTestId('conflict-resolution-text').textContent).toBe(REGION.theirs);
  });

  it('hand-edits, prefilled with the ours side', async () => {
    render(ConflictDialog, { props: { region: REGION } });
    await fireEvent.click(screen.getByTestId('conflict-manual'));

    const editor = screen.getByTestId('conflict-manual-text') as HTMLTextAreaElement;
    await waitFor(() => expect(editor).toBeTruthy());
    // `[FACT]` ADR-0024 §3.7.4: the editor is prefilled with the
    // ours side.
    expect(editor.value).toBe(REGION.ours);

    await fireEvent.input(editor, { target: { value: 'const timeout = 45;\n' } });
    await fireEvent.click(screen.getByTestId('conflict-manual-save'));

    await waitFor(() => expect(screen.getByTestId('conflict-resolution')).toBeTruthy());
    expect(screen.getByTestId('conflict-resolution-text').textContent).toBe('const timeout = 45;\n');
  });

  it('refuses a hand-edit that carries a conflict marker', async () => {
    render(ConflictDialog, { props: { region: REGION } });
    await fireEvent.click(screen.getByTestId('conflict-manual'));
    const editor = screen.getByTestId('conflict-manual-text') as HTMLTextAreaElement;
    await fireEvent.input(editor, { target: { value: '<<<<<<< OURS\n' } });
    await fireEvent.click(screen.getByTestId('conflict-manual-save'));
    await waitFor(() => expect(screen.getByTestId('conflict-manual-error')).toBeTruthy());
    expect(screen.queryByTestId('conflict-resolution')).toBe(null);
  });

  it('does not offer "ask the AI" on a provider that cannot stream', async () => {
    // `[FACT]` ADR-0024 §3.4.1: the option is only offered on a
    // provider that can stream, and the refusal is stated.
    expect(supportsStreaming('anthropic')).toBe(false);
    expect(supportsStreaming('openai')).toBe(true);

    render(ConflictDialog, { props: { region: REGION } });
    await fireEvent.change(screen.getByTestId('conflict-provider'), {
      target: { value: 'anthropic' },
    });
    await waitFor(() => expect(screen.getByTestId('conflict-unsupported')).toBeTruthy());
    expect(screen.queryByTestId('conflict-ask-ai')).toBe(null);
  });
});

// ---------------------------------------------------------------------------
// The two sides are the same (ADR-0024 §3.7.4)
// ---------------------------------------------------------------------------

describe('the conflict dialog — the two sides are the same', () => {
  /**
   * A region whose two sides carry the same text.
   *
   * `[FACT]` The flag is the **backend's**: the library's own judgement
   * (`ConflictRegion::sides_are_equivalent`, `src/ai/conflict.rs`). The
   * webview does not compare the two sides itself — it reads the flag.
   */
  const EQUIVALENT_REGION: ConflictRegion = {
    ...REGION,
    id: 'b7c8d9e0f1a2b3c4d5e6f708192a3b4',
    ours: 'const timeout = 30;\n',
    theirs: 'const timeout = 30;\n',
    sidesEquivalent: true,
  };

  it('does not offer "ask the AI", and says why', async () => {
    // `[FACT]` ADR-0024 §3.7.4: "the region's ours and theirs text are the
    // same — the region exists but the two sides are equivalent; the
    // default suggestion is keep ours". Asking the model such a region is
    // a question whose only possible answer is the text the user already
    // has.
    //
    // A key IS configured here, so the equivalence is the only reason the
    // option can be unavailable — and the option is present but disabled,
    // the same shape as the "no key" case.
    withInvoke({ ai_key_status: () => true });
    render(ConflictDialog, { props: { region: EQUIVALENT_REGION } });

    const ask = screen.getByTestId('conflict-ask-ai') as HTMLButtonElement;
    await waitFor(() => expect(ask.disabled).toBe(true));
    // And the reason is stated, so the user is not left guessing.
    const note = await waitFor(() => screen.getByTestId('conflict-sides-equivalent'));
    expect(note.textContent ?? '').not.toBe('');
  });

  it('names "keep ours" as the default', async () => {
    // `[FACT]` The ADR's default for such a region is "keep ours", and the
    // dialog says which option that is. The dialog does **not** make the
    // decision: a decision the user did not make is not a decision, and the
    // write-back needs one per region.
    render(ConflictDialog, { props: { region: EQUIVALENT_REGION } });

    const ours = screen.getByTestId('conflict-keep-ours');
    expect(ours.getAttribute('data-default')).toBe('true');
    expect(screen.getByTestId('conflict-keep-ours-default')).toBeTruthy();
  });

  it('offers "ask the AI" when the two sides differ', async () => {
    // The mirror of the case above: the feature is not taken away. A region
    // whose sides differ keeps the option, with a key configured.
    withInvoke({ ai_key_status: () => true });
    render(ConflictDialog, { props: { region: REGION } });

    const ask = screen.getByTestId('conflict-ask-ai') as HTMLButtonElement;
    await waitFor(() => expect(ask.disabled).toBe(false));
    // And nothing says the sides are equivalent, because they are not.
    expect(screen.queryByTestId('conflict-sides-equivalent')).toBe(null);
    expect(
      screen.getByTestId('conflict-keep-ours').getAttribute('data-default'),
    ).toBe('false');
  });
});

describe('the conflict dialog — the suggestion', () => {
  /** Drive a completed arbitration whose answer was accepted. */
  async function streamAccepted(suggestion: string): Promise<void> {
    const bridge = fakeSubscribe();
    await startArbitration(REGION, { provider: 'openai', subscribe: bridge.subscribe });
    bridge.emit({
      type: 'done',
      sessionId: currentSession(),
      regionId: currentRegionId(),
      tokens: 1,
      outcome: { state: 'accepted', suggestion },
    });
  }

  it('disables "accept" until the diff has been opened', async () => {
    render(ConflictDialog, { props: { region: REGION } });
    await streamAccepted('const timeout = 45;\n');

    await waitFor(() => expect(screen.getByTestId('conflict-accept')).toBeTruthy());
    const accept = screen.getByTestId('conflict-accept') as HTMLButtonElement;
    expect(accept.disabled).toBe(true);

    await fireEvent.click(screen.getByTestId('conflict-show-diff'));
    await waitFor(() => expect(accept.disabled).toBe(false));
  });

  it('shows the suggestion as a two-column diff, ours on the left', async () => {
    render(ConflictDialog, { props: { region: REGION } });
    await streamAccepted('const timeout = 45;\n');
    await fireEvent.click(screen.getByTestId('conflict-show-diff'));

    await waitFor(() => expect(screen.getByTestId('conflict-diff')).toBeTruthy());
    const left = screen.getByTestId('conflict-diff-left');
    const right = screen.getByTestId('conflict-diff-right');
    expect(left.textContent).toContain('const timeout = 30;');
    expect(right.textContent).toContain('const timeout = 45;');
    // Two columns, not one text and not a silent replacement.
    expect(left.textContent).not.toContain('const timeout = 45;');
    expect(right.textContent).not.toContain('const timeout = 30;');
  });

  it('shows the reason a suggestion was refused', async () => {
    render(ConflictDialog, { props: { region: REGION } });
    const bridge = fakeSubscribe();
    await startArbitration(REGION, { provider: 'openai', subscribe: bridge.subscribe });
    bridge.emit({
      type: 'done',
      sessionId: currentSession(),
      regionId: currentRegionId(),
      tokens: 1,
      outcome: { state: 'rejected', reason: 'the suggestion carries a conflict marker' },
    });

    await waitFor(() => expect(screen.getByTestId('conflict-rejected')).toBeTruthy());
    expect(screen.getByTestId('conflict-rejected').textContent).toContain('conflict marker');
    // A refused suggestion has nothing to accept.
    expect(screen.queryByTestId('conflict-accept')).toBe(null);
  });

  it('puts model output into the DOM as text, not as markup', async () => {
    // `[FACT]` ADR-0024 §3.4.3: model output is untrusted text and
    // goes into the DOM through a normal text node.
    render(ConflictDialog, { props: { region: REGION } });
    await streamAccepted('<script>globalThis.__pwned = 1;</script>\n');
    await fireEvent.click(screen.getByTestId('conflict-show-diff'));

    await waitFor(() => expect(screen.getByTestId('conflict-diff-right')).toBeTruthy());
    const right = screen.getByTestId('conflict-diff-right');
    expect(right.querySelector('script')).toBe(null);
    expect(right.textContent).toContain('<script>');
    expect((globalThis as Record<string, unknown>)['__pwned']).toBeUndefined();
  });
});

// ---------------------------------------------------------------------------
// The route
// ---------------------------------------------------------------------------

describe('the /conflicts route', () => {
  it('is registered in the app route table', () => {
    const app = readFileSync(join(APP_ROOT, 'src', 'App.svelte'), 'utf8');
    expect(app).toContain("'/conflicts'");
    expect(app).toContain('Conflicts');
    // The hash router matches it.
    expect(matchPath('/conflicts', '/conflicts')).toEqual({});
  });

  it('renders the region list and the dialog for the first region', async () => {
    render(Conflicts);
    await waitFor(() => expect(screen.getByTestId('conflicts-heading')).toBeTruthy());
    await waitFor(() => expect(screen.getByTestId('conflict-dialog')).toBeTruthy());
    expect(screen.getByTestId('conflict-regions').children.length).toBeGreaterThan(0);
  });

  it('shows the paths git has to resolve itself', async () => {
    // `[FACT]` ADR §3.7.4: a path the core refused to read is
    // reported, not dropped. A page that said "no conflict regions" while
    // git still had work to do would be lying — and the paths can sit
    // beside the regions, not only replace them.
    render(Conflicts);
    await waitFor(() => expect(screen.getByTestId('conflicts-needs-git')).toBeTruthy());
    expect(screen.getByTestId('conflicts-needs-git').textContent).toContain('assets/logo.png');
  });
});

// ---------------------------------------------------------------------------
// The source gates
// ---------------------------------------------------------------------------

describe('the conflict sources', () => {
  /**
   * The files that make up the conflict feature's front end.
   *
   * `[FACT]` ADR-0024 §3.4.3: model output must reach the DOM
   * through a normal text node, so none of these may contain a raw-HTML
   * block.
   */
  const FEATURE_FILES = [
    'src/routes/Conflicts.svelte',
    'src/lib/components/ConflictDialog.svelte',
    'src/lib/stores/conflict.ts',
    'src/lib/api/conflict.ts',
    'src/lib/utils/linediff.ts',
  ];

  it('contain no raw-HTML block', () => {
    for (const file of FEATURE_FILES) {
      const text = readFileSync(join(APP_ROOT, file), 'utf8');
      expect(text, `${file} renders model output as markup`).not.toContain('{@html');
    }
  });

  it('reference only i18n keys that exist', () => {
    // A typo'd key renders as the key itself, which is a hole in the
    // page rather than a visible error.
    const referenced = new Set<string>();
    for (const file of FEATURE_FILES) {
      const text = readFileSync(join(APP_ROOT, file), 'utf8');
      for (const match of text.matchAll(/'conflict\.[A-Za-z0-9.]+'/g)) {
        referenced.add(match[0].slice(1, -1));
      }
    }
    expect(referenced.size).toBeGreaterThan(0);
    for (const key of referenced) {
      expect(en, `${key} is missing from en.ts`).toHaveProperty(key);
      expect(zh, `${key} is missing from zh-CN.ts`).toHaveProperty(key);
    }
    // The four resolution labels are built from a template literal, so
    // the scan above cannot see them.
    for (const choice of ['ours', 'theirs', 'manual', 'ai']) {
      expect(en).toHaveProperty(`conflict.resolution.${choice}`);
      expect(zh).toHaveProperty(`conflict.resolution.${choice}`);
    }
  });
});

// ---------------------------------------------------------------------------
// The i18n catalogue
// ---------------------------------------------------------------------------

describe('the conflict i18n keys', () => {
  it('are the same set in both catalogues', () => {
    const enKeys = Object.keys(en).filter((k) => k.startsWith('conflict.')).sort();
    const zhKeys = Object.keys(zh).filter((k) => k.startsWith('conflict.')).sort();
    expect(enKeys.length).toBeGreaterThan(0);
    expect(enKeys).toEqual(zhKeys);
  });

  it('carry the four options, the diff labels and the three explanations', () => {
    const required = [
      // the heading
      'conflict.heading',
      // the four options, in the ADR's order
      'conflict.keepOurs',
      'conflict.keepTheirs',
      'conflict.manualEdit',
      'conflict.askAi',
      // the diff labels
      'conflict.diffOursLabel',
      'conflict.diffSuggestionLabel',
      // save / cancel
      'common.save',
      'common.cancel',
      // no key
      'conflict.noKey',
      // a provider that cannot stream
      'conflict.unsupported',
      // a refused suggestion
      'conflict.rejected',
    ];
    for (const key of required) {
      expect(en, `${key} is missing from en.ts`).toHaveProperty(key);
      expect(zh, `${key} is missing from zh-CN.ts`).toHaveProperty(key);
    }
  });

  it('has a nav entry in both catalogues', () => {
    expect(en).toHaveProperty('nav.conflicts');
    expect(zh).toHaveProperty('nav.conflicts');
  });
});

// ---------------------------------------------------------------------------
// The audit trail (ADR-0024 §3.7.3)
// ---------------------------------------------------------------------------

describe('the conflict audit trail', () => {
  /**
   * The audit writes the test observed.
   *
   * `[INFERENCE]` The webview sends the **region** and the **choice** and
   * nothing else: the choice is the only thing it knows that the backend
   * does not, and the backend re-derives the region's id, the model, the
   * redaction count and the digests. So the payload is the contract this
   * test pins.
   */
  interface AuditCall {
    choice: string;
    path: string;
    regionId: string;
  }

  /** Answer every audit write with a recorder instead of the dev mock. */
  function captureAudit(): AuditCall[] {
    const calls: AuditCall[] = [];
    withInvoke({
      conflict_audit_record: (args) => {
        const region = (args.region ?? {}) as { path?: string; baseOid?: string };
        calls.push({
          choice: String(args.choice),
          path: String(region.path ?? ''),
          regionId: REGION.id,
        });
        return null;
      },
    });
    return calls;
  }

  it('asks the backend for exactly one record when the user keeps ours', async () => {
    // `[FACT]` ADR §3.7.3: one arbitration, one record. The record is
    // written by the backend — the webview only says which choice was made.
    const calls = captureAudit();
    render(ConflictDialog, { props: { region: REGION } });
    await fireEvent.click(screen.getByTestId('conflict-keep-ours'));

    await waitFor(() => expect(calls.length).toBe(1));
    expect(calls[0].choice).toBe('ours');
    expect(calls[0].path).toBe(REGION.path);
  });

  it('asks the backend for exactly one record when the user keeps theirs', async () => {
    const calls = captureAudit();
    render(ConflictDialog, { props: { region: REGION } });
    await fireEvent.click(screen.getByTestId('conflict-keep-theirs'));

    await waitFor(() => expect(calls.length).toBe(1));
    expect(calls[0].choice).toBe('theirs');
  });

  it('records a hand-written edit as "manual"', async () => {
    const calls = captureAudit();
    render(ConflictDialog, { props: { region: REGION } });
    await fireEvent.click(screen.getByTestId('conflict-manual'));
    const editor = screen.getByTestId('conflict-manual-text') as HTMLTextAreaElement;
    await fireEvent.input(editor, { target: { value: 'const timeout = 45;\n' } });
    await fireEvent.click(screen.getByTestId('conflict-manual-save'));

    await waitFor(() => expect(calls.length).toBe(1));
    expect(calls[0].choice).toBe('manual');
  });

  it('records the model\'s suggestion as "ai"', async () => {
    const calls = captureAudit();
    const bridge = fakeSubscribe();
    render(ConflictDialog, { props: { region: REGION } });
    await startArbitration(REGION, { provider: 'openai', subscribe: bridge.subscribe });
    bridge.emit({
      type: 'done',
      sessionId: currentSession(),
      regionId: currentRegionId(),
      tokens: 1,
      outcome: { state: 'accepted', suggestion: 'const timeout = 45;\n' },
    });
    await waitFor(() => expect(screen.getByTestId('conflict-accept')).toBeTruthy());
    await fireEvent.click(screen.getByTestId('conflict-show-diff'));
    await fireEvent.click(screen.getByTestId('conflict-accept'));

    await waitFor(() => expect(calls.length).toBe(1));
    expect(calls[0].choice).toBe('ai');
  });

  it('writes no second record for the same decision', async () => {
    // `[INFERENCE]` One decision, one record: choosing the same side twice
    // is not two decisions, and the trail must not say it was.
    const calls = captureAudit();
    render(ConflictDialog, { props: { region: REGION } });
    await fireEvent.click(screen.getByTestId('conflict-keep-ours'));
    await waitFor(() => expect(calls.length).toBe(1));
    await fireEvent.click(screen.getByTestId('conflict-keep-ours'));

    // Give a second write the chance to arrive before asserting there is
    // none.
    await new Promise((resolve) => setTimeout(resolve, 20));
    expect(calls.length).toBe(1);
  });

  it('writes a record for a decision in ANOTHER region, even with the same text', async () => {
    // `[FACT]` The dedupe is keyed on the region as well as the choice and
    // the text. `loadRegions` replaces the region list WITHOUT clearing the
    // resolution, so a resolution from one region can still be sitting in
    // the state when the next region is arbitrated. Two regions whose
    // "ours" side is the same text are still two decisions, and a trail
    // that exists to say what was decided must say both.
    //
    // Without the region in the key, this test's second decision would be
    // silently dropped and the trail would name one decision where two
    // were made.
    const calls = captureAudit();
    const other: ConflictRegion = {
      ...REGION,
      id: 'f0e1d2c3b4a5968778695a4b3c2d1e0',
      path: 'src/other.ts',
    };

    setResolution('ours', REGION.ours, REGION);
    await waitFor(() => expect(calls.length).toBe(1));

    setResolution('ours', REGION.ours, other);
    await waitFor(() => expect(calls.length).toBe(2));
    expect(calls[0].path).toBe(REGION.path);
    expect(calls[1].path).toBe(other.path);
  });

  it('keeps the decision when the backend cannot write the record', async () => {
    // `[INFERENCE]` The trail must not be able to undo the arbitration: the
    // user has made a choice, it is shown, and a file that cannot be
    // written is a problem for the trail, not for the decision.
    const warn = vi.spyOn(console, 'warn').mockImplementation(() => undefined);
    try {
      withInvoke({
        conflict_audit_record: () => {
          throw { kind: 'Io', message: 'the audit file is not writable' };
        },
      });
      render(ConflictDialog, { props: { region: REGION } });
      await fireEvent.click(screen.getByTestId('conflict-keep-ours'));

      await waitFor(() => expect(screen.getByTestId('conflict-resolution')).toBeTruthy());
      expect(screen.getByTestId('conflict-resolution-text').textContent).toBe(REGION.ours);
      expect(warn).toHaveBeenCalled();
    } finally {
      warn.mockRestore();
    }
  });
});

// ---------------------------------------------------------------------------
// The write-back (ADR-0024 §3.6)
// ---------------------------------------------------------------------------

/**
 * A second region, in a second path.
 *
 * `[FACT]` The regions of a merge span several files: the write-back is
 * per path, so the store has to group the decisions by `region.path`
 * before it calls the command.
 */
const OTHER_REGION: ConflictRegion = {
  ...REGION,
  id: 'f0e1d2c3b4a5968778695a4b3c2d1e0',
  path: 'src/other.ts',
  index: 0,
};

/** The two regions, in two paths, as the backend would answer. */
function twoPathRegions(): ConflictRegions {
  return { regions: [REGION, OTHER_REGION], needsGit: [] };
}

describe('the conflict decisions — one per region', () => {
  it('keeps a decision when the user opens another region', () => {
    // `[FACT]` The decision is stored per region. Before this lane the
    // state held ONE resolution and `selectRegion` reset the whole state,
    // so a decision made in one region was gone the moment the user
    // opened the next one — and the write-back is per path, so every
    // region of the path has to be decided before it can be written.
    selectRegion(REGION.id);
    setResolution('ours', REGION.ours, REGION);
    expect(get(conflict).resolution).toEqual({
      choice: 'ours',
      text: REGION.ours,
      regionId: REGION.id,
    });

    selectRegion(OTHER_REGION.id);
    const s = get(conflict);
    // The region that was just opened has no decision of its own...
    expect(s.decisions[OTHER_REGION.id]).toBeUndefined();
    // ...and the first region's decision is still there.
    expect(s.decisions[REGION.id]).toEqual({ choice: 'ours', text: REGION.ours });
    // The resolution is a view of the OPEN region's decision, so it is
    // empty here — not the other region's answer.
    expect(s.resolution).toBe(null);

    // And switching back shows the decision again.
    selectRegion(REGION.id);
    expect(get(conflict).resolution).toEqual({
      choice: 'ours',
      text: REGION.ours,
      regionId: REGION.id,
    });
  });

  it('holds one decision per region, not one decision overall', () => {
    // `[FACT]` Two regions whose "ours" side is the same text are two
    // decisions, and the map has to keep both.
    setResolution('ours', REGION.ours, REGION);
    setResolution('theirs', REGION.theirs, OTHER_REGION);
    const decisions = get(conflict).decisions;
    expect(Object.keys(decisions).sort()).toEqual([REGION.id, OTHER_REGION.id].sort());
    expect(decisions[REGION.id]).toEqual({ choice: 'ours', text: REGION.ours });
    expect(decisions[OTHER_REGION.id]).toEqual({ choice: 'theirs', text: REGION.theirs });
  });
});

describe('the apply action', () => {
  it('is disabled until every arbitrable region has a decision', async () => {
    // `[FACT]` ADR-0024 §3.6: a path is written back only when EVERY one
    // of its regions has been decided, so the gate is "every arbitrable
    // region has a decision".
    await loadRegions('demo', null);
    const regions = get(conflict).regions;
    expect(regions.length).toBe(2);
    expect(canApply(get(conflict))).toBe(false);

    setResolution('ours', regions[0].ours, regions[0]);
    expect(canApply(get(conflict))).toBe(false);

    setResolution('theirs', regions[1].theirs, regions[1]);
    expect(canApply(get(conflict))).toBe(true);
  });

  it('writes nothing while a region is undecided', async () => {
    // `[FACT]` Not one byte may be written when a region has no decision:
    // the path could not be written back anyway, and a half-arbitrated
    // file in the work tree is worse than none.
    let calls = 0;
    const resolve = async (): Promise<ConflictResolveResult> => {
      calls += 1;
      return { path: 'src/feature.ts', regions: 0 };
    };

    await loadRegions('demo', null);
    const regions = get(conflict).regions;
    setResolution('ours', regions[0].ours, regions[0]);

    const outcome = await applyDecisions('demo', null, { resolve });
    expect(calls).toBe(0);
    expect(outcome.written).toEqual([]);
    expect(outcome.failed).toBe(null);
    expect(get(conflict).applyError).toContain('no decision yet');
  });

  it('writes nothing when there is no region to write', async () => {
    let calls = 0;
    const resolve = async (): Promise<ConflictResolveResult> => {
      calls += 1;
      return { path: '', regions: 0 };
    };
    const outcome = await applyDecisions('demo', null, { resolve });
    expect(calls).toBe(0);
    expect(outcome.written).toEqual([]);
    expect(get(conflict).applyError).toBe(CONFLICT_NO_REGIONS_MESSAGE);
  });

  it('groups the decisions by path and calls the command once per path', async () => {
    // `[FACT]` The command handles ONE path: a merge spans several
    // files, and each file is written and staged on its own.
    const calls: ConflictResolvePayload[] = [];
    const resolve = async (payload: ConflictResolvePayload): Promise<ConflictResolveResult> => {
      calls.push(payload);
      return { path: payload.path, regions: payload.choices.length };
    };

    withInvoke({ conflict_regions: () => twoPathRegions() });
    await loadRegions('demo', null);
    const regions = get(conflict).regions;
    setResolution('ours', regions[0].ours, regions[0]);
    setResolution('theirs', regions[1].theirs, regions[1]);

    const outcome = await applyDecisions('demo', null, { resolve });

    // One call per path — not one call with both paths' regions in it.
    expect(calls.length).toBe(2);
    expect(calls.map((c) => c.path)).toEqual([REGION.path, OTHER_REGION.path]);
    // And each call carries only its own path's regions.
    expect(calls[0].choices).toEqual([
      { index: REGION.index, choice: 'ours', text: REGION.ours },
    ]);
    expect(calls[1].choices).toEqual([
      { index: OTHER_REGION.index, choice: 'theirs', text: OTHER_REGION.theirs },
    ]);
    // The repository and the root travel with every call.
    expect(calls[0].repo).toBe('demo');
    expect(calls[0].root).toBe(null);
    expect(outcome.written).toEqual([REGION.path, OTHER_REGION.path]);
    expect(outcome.failed).toBe(null);
    expect(outcome.skipped).toEqual([]);
  });

  it('sends every region of a path in one call', async () => {
    // `[FACT]` A file can carry several regions, and the backend refuses
    // a path with an undecided region — so the regions of one path have
    // to travel together.
    const calls: ConflictResolvePayload[] = [];
    const resolve = async (payload: ConflictResolvePayload): Promise<ConflictResolveResult> => {
      calls.push(payload);
      return { path: payload.path, regions: payload.choices.length };
    };
    const second: ConflictRegion = { ...REGION, id: 'ffffffffffffffffffffffffffffffff', index: 1 };
    withInvoke({
      conflict_regions: () => ({ regions: [REGION, second], needsGit: [] }),
    });

    await loadRegions('demo', null);
    const regions = get(conflict).regions;
    setResolution('ours', regions[0].ours, regions[0]);
    setResolution('manual', 'hand written\n', regions[1]);

    await applyDecisions('demo', null, { resolve });
    expect(calls.length).toBe(1);
    expect(calls[0].path).toBe(REGION.path);
    expect(calls[0].choices).toEqual([
      { index: 0, choice: 'ours', text: REGION.ours },
      { index: 1, choice: 'manual', text: 'hand written\n' },
    ]);
  });

  it('reports which paths were written and which were not', async () => {
    // `[FACT]` A report that said "done" while some paths were untouched
    // would be a lie, and the user would commit a half-resolved merge.
    const resolve = async (payload: ConflictResolvePayload): Promise<ConflictResolveResult> => {
      if (payload.path === REGION.path) {
        throw { kind: 'Gitgit', message: 'git add failed: the index is locked' };
      }
      return { path: payload.path, regions: payload.choices.length };
    };

    withInvoke({ conflict_regions: () => twoPathRegions() });
    await loadRegions('demo', null);
    const regions = get(conflict).regions;
    setResolution('ours', regions[0].ours, regions[0]);
    setResolution('theirs', regions[1].theirs, regions[1]);

    const outcome = await applyDecisions('demo', null, { resolve });
    expect(outcome.written).toEqual([]);
    expect(outcome.failed?.path).toBe(REGION.path);
    expect(outcome.failed?.message).toContain('the index is locked');
    // The path after the failure was not attempted.
    expect(outcome.skipped).toEqual([OTHER_REGION.path]);
    // And the failure is in the state, for the page to show.
    expect(get(conflict).applyError).toContain('the index is locked');
  });

  it('refreshes the region list after the write-back', async () => {
    // `[FACT]` A path that was written back is no longer unmerged, so the
    // region list has to be read again — otherwise the page would still
    // show regions that git has already resolved.
    let loads = 0;
    const load = async (): Promise<ConflictRegions> => {
      loads += 1;
      return twoPathRegions();
    };
    const resolve = async (payload: ConflictResolvePayload): Promise<ConflictResolveResult> => ({
      path: payload.path,
      regions: payload.choices.length,
    });

    withInvoke({ conflict_regions: () => twoPathRegions() });
    await loadRegions('demo', null);
    const regions = get(conflict).regions;
    setResolution('ours', regions[0].ours, regions[0]);
    setResolution('theirs', regions[1].theirs, regions[1]);

    await applyDecisions('demo', null, { resolve, load });
    expect(loads).toBe(1);
  });
});

describe('the /conflicts page — the write-back', () => {
  it('shows on each region whether it has been decided', async () => {
    render(Conflicts);
    await waitFor(() => expect(screen.getByTestId('conflict-regions')).toBeTruthy());
    const regions = get(conflict).regions;
    expect(regions.length).toBe(2);
    for (const region of regions) {
      expect(screen.getByTestId(`conflict-region-${region.index}`).getAttribute('data-decided')).toBe(
        'false',
      );
    }
    setResolution('ours', regions[0].ours, regions[0]);
    await waitFor(() =>
      expect(
        screen.getByTestId(`conflict-region-${regions[0].index}`).getAttribute('data-decided'),
      ).toBe('true'),
    );
  });

  it('disables the apply button until every region is decided', async () => {
    render(Conflicts);
    await waitFor(() => expect(screen.getByTestId('conflict-apply')).toBeTruthy());
    const apply = screen.getByTestId('conflict-apply') as HTMLButtonElement;
    expect(apply.disabled).toBe(true);

    const regions = get(conflict).regions;
    setResolution('ours', regions[0].ours, regions[0]);
    await waitFor(() => expect((screen.getByTestId('conflict-apply') as HTMLButtonElement).disabled).toBe(
      true,
    ));
    setResolution('theirs', regions[1].theirs, regions[1]);
    await waitFor(() =>
      expect((screen.getByTestId('conflict-apply') as HTMLButtonElement).disabled).toBe(false),
    );
  });

  it('shows a failed write-back to the user, not only to the console', async () => {
    // `[FACT]` The write-back is the step that changes the work tree, so
    // its failure has to reach the user.
    withInvoke({
      conflict_resolve: () => {
        throw { kind: 'Gitgit', message: 'git add failed: the index is locked' };
      },
    });
    render(Conflicts);
    await waitFor(() => expect(screen.getByTestId('conflict-apply')).toBeTruthy());

    const regions = get(conflict).regions;
    setResolution('ours', regions[0].ours, regions[0]);
    setResolution('theirs', regions[1].theirs, regions[1]);
    await waitFor(() =>
      expect((screen.getByTestId('conflict-apply') as HTMLButtonElement).disabled).toBe(false),
    );

    await fireEvent.click(screen.getByTestId('conflict-apply'));
    await waitFor(() => expect(screen.getByTestId('conflicts-apply-error')).toBeTruthy());
    expect(screen.getByTestId('conflicts-apply-error').textContent).toContain('the index is locked');
  });

  it('shows which paths were written back', async () => {
    render(Conflicts);
    await waitFor(() => expect(screen.getByTestId('conflict-apply')).toBeTruthy());
    const regions = get(conflict).regions;
    setResolution('ours', regions[0].ours, regions[0]);
    setResolution('theirs', regions[1].theirs, regions[1]);
    await waitFor(() =>
      expect((screen.getByTestId('conflict-apply') as HTMLButtonElement).disabled).toBe(false),
    );

    await fireEvent.click(screen.getByTestId('conflict-apply'));
    await waitFor(() => expect(screen.getByTestId('conflicts-apply-written')).toBeTruthy());
    expect(screen.getByTestId('conflicts-apply-written').textContent).toContain(REGION.path);
  });
});
