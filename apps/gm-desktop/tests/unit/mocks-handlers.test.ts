/**
 * Installation-idempotence tests for `src/mocks/handlers.ts`.
 *
 * `[FACT]` `installMock()` used to decide "the mock is already
 * installed" by stringifying whatever `invoke` was currently installed
 * and searching the source text for the substring `"mock"`. An identity
 * test written as a text search has two failure modes, and both are
 * pinned here:
 *
 *   1. A foreign stub whose source contains that word — `model:
 *      'mock'`, a `mock-*` fixture name, a comment — was mistaken for
 *      the mock, so the reinstall was skipped and every later case
 *      talked to the stub instead. Six sibling test files carry a
 *      `delete window.__TAURI_INTERNALS__` in their `beforeEach`
 *      purely to work around this.
 *   2. It never matched its own function: the installed arrow is
 *      `(cmd, args) => handle(cmd, args)`, which contains no such
 *      substring, so every `installMock()` call replaced the bridge
 *      with a new object. The documented idempotence did not hold.
 *
 * The sharpest case is the first one below: a stub whose own source
 * genuinely contains `"mock"`, followed by a plain `installMock()`,
 * asserting the mock actually took over. That assertion fails against
 * the substring implementation and passes against a real identity test.
 *
 * `[FACT]` Every stub here is written inline rather than through a
 * shared helper. A helper takes a reply callback, which puts the
 * interesting text in a *different* function's source — and the whole
 * defect is that the check read the installed function's source. The
 * first version of this file had a `stubBridge()` helper and its
 * precondition assertion failed for exactly that reason.
 *
 * `[FACT]` These cases do not `delete window.__TAURI_INTERNALS__`
 * before reinstalling. That is the point: under the semantics chosen
 * here a plain `installMock()` restores the mock whatever is installed,
 * so a caller never has to know how the previous state was produced.
 * The sibling files' workarounds are now redundant rather than
 * load-bearing; a follow-up can drop them.
 */
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { installMock } from '../../src/mocks/handlers';
import type { VersionEntryDto } from '../../src/lib/api/types';

type InvokeBridge = NonNullable<Window['__TAURI_INTERNALS__']>['invoke'];

/**
 * The bridge currently on the window, with a failure that names the
 * thing under test instead of a type-level assertion.
 */
function bridge(): InvokeBridge {
  const invoke = window.__TAURI_INTERNALS__?.invoke;
  if (!invoke) throw new Error('no invoke bridge is installed on window');
  return invoke;
}

/** Repo names the shipped fixture store holds, in order. */
async function repoNames(): Promise<string[]> {
  const repos = (await bridge()('list_repos')) as { name: string }[];
  return repos.map((r) => r.name);
}

beforeEach(() => {
  // `[FACT]` Unconditional, with no `delete` first: under the
  // semantics chosen here this is enough on its own, whatever the
  // previous case left installed.
  installMock();
});

afterEach(() => {
  vi.unstubAllGlobals();
  installMock();
});

describe('mocks / handlers — installMock() identity', () => {
  it('takes over from a stub whose source contains the substring "mock"', async () => {
    let stubCalls = 0;
    window.__TAURI_INTERNALS__ = {
      invoke: (cmd: string) => {
        stubCalls += 1;
        return Promise.resolve({ cmd, model: 'mock' });
      },
    };
    // `[FACT]` The precondition the old check got wrong: the installed
    // function's own source really does contain "mock", so the
    // substring test read this stub as the mock. The `model: 'mock'`
    // payload is the realistic shape — an AI-review fixture, not a
    // mock of one.
    expect(bridge().toString().includes('mock')).toBe(true);

    installMock();

    // `[FACT]` The mock answers, not the stub: its own fixture repos
    // come back, and the stub was never called.
    expect(await repoNames()).toEqual(['demo', 'hello-world']);
    expect(stubCalls).toBe(0);
  });

  it('takes over from a stub that does not mention "mock" either', async () => {
    let stubCalls = 0;
    window.__TAURI_INTERNALS__ = {
      invoke: () => {
        stubCalls += 1;
        return Promise.resolve('stub answer');
      },
    };
    expect(bridge().toString().includes('mock')).toBe(false);

    installMock();

    // The other direction: replacement must depend on the installed
    // function not being the mock, and on nothing else.
    expect(await repoNames()).toEqual(['demo', 'hello-world']);
    expect(stubCalls).toBe(0);
  });

  it('answers with its own error for a command no stub ever saw', async () => {
    window.__TAURI_INTERNALS__ = {
      invoke: (cmd: string) => Promise.resolve({ cmd, model: 'mock' }),
    };
    installMock();

    // `[FACT]` The `default:` arm of `handle()` is only reachable
    // through the mock's own bridge, so this also proves the mock is
    // on the window rather than a stub that resolves anything.
    await expect(bridge()('no_such_command')).rejects.toThrow(
      'mock: command not implemented: no_such_command',
    );
  });

  it('is a no-op when its own bridge is already installed', () => {
    // `[FACT]` Seeded with a stub whose source does *not* contain
    // "mock", so the old check would have reinstalled over it too.
    // That isolates the second failure mode — the check never matched
    // its own function — from the cross-case leak in the case above.
    window.__TAURI_INTERNALS__ = {
      invoke: () => Promise.resolve('stub answer'),
    };
    installMock();
    const first = bridge();

    installMock();

    // `[FACT]` Same function object, not an equivalent one. Against
    // the substring check this failed: the installed arrow's own
    // source contains no "mock", so it was replaced every time.
    expect(bridge()).toBe(first);
  });

  it("keeps the mock's in-memory fixtures across a reinstall", async () => {
    // `[FACT]` `STORE` is module-lifetime, not installation-lifetime:
    // reinstalling restores the bridge, never the state behind it.
    // `vault_get` cannot witness this — it answers a fixed
    // `value-for-<key>` string for any key that has versions — so the
    // version list is the observable.
    const key = 'mock-idempotence.probe';
    await bridge()('vault_set', { key, value: 'written before the stub' });
    const before = (await bridge()('vault_versions', { key })) as VersionEntryDto[];
    expect(before).toHaveLength(1);

    window.__TAURI_INTERNALS__ = {
      invoke: () => Promise.resolve('stub answer'),
    };
    installMock();

    const after = (await bridge()('vault_versions', { key })) as VersionEntryDto[];
    expect(after).toHaveLength(1);
    expect(after[0]?.version).toBe(1);
  });

  it('does nothing when there is no window at all', () => {
    // `[FACT]` The SSR / worker guard is kept deliberately: with no
    // `window` there is nothing to patch, and the call must not throw.
    const before = bridge();
    vi.stubGlobal('window', undefined);

    expect(() => installMock()).not.toThrow();

    vi.unstubAllGlobals();
    expect(bridge()).toBe(before);
  });
});
