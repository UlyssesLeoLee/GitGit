/**
 * Shared harness for the route-component tests.
 *
 * Every route page reaches the backend the same way — through
 * `window.__TAURI_INTERNALS__.invoke` — so the useful thing a test can do
 * is replace that one function and record what the page asked for. That is
 * what `useInvoke` does, in the same shape the in-file copies in
 * `routes-graph.test.ts` and `review.test.ts` already use; it is factored
 * out here only because six more route files would otherwise each carry
 * their own copy.
 *
 * `resetRouteStores` matters just as much. The stores the pages read are
 * module singletons, so without it a selection (or a mock-side mutation,
 * which the shipped `handlers.ts` keeps in a file-lifetime `STORE`) leaks
 * from one case into the next one's assertions.
 */
import { get } from 'svelte/store';
import { cleanup } from '@testing-library/svelte';
import { installMock } from '../../src/mocks/handlers';
import { catalog, locale, t, tFor } from '../../src/lib/i18n';
import { setLocale } from '../../src/lib/stores/locale';
import { setTheme } from '../../src/lib/stores/theme';
import { server, serverBusy } from '../../src/lib/stores/server';
import { repos, repoDetails, reposLoading } from '../../src/lib/stores/repos';
import { vaultSecrets, vaultVersionsCache, vaultBusy } from '../../src/lib/stores/vault';
import { clearToasts } from '../../src/lib/stores/toasts';
import { clearWorktreeRoot } from '../../src/lib/stores/worktreeRoot';
import { resetWorktree } from '../../src/lib/stores/worktree';
import type { ServerStatus } from '../../src/lib/api/types';

export { t, tFor };

/**
 * The locale the app resolved to when this module was first imported.
 * jsdom reports `navigator.language === 'en-US'`, so this is `en`. Tests
 * compare against `t(...)` / `cat(...)` — i.e. against whatever is active
 * — rather than against a pinned language, following the house rule.
 */
export const INITIAL_LOCALE = get(locale);

/** The active locale's string for `key`, or the key itself. */
export function cat(key: string): string {
  return get(catalog)[key] ?? key;
}

export type Handler = (args: Record<string, unknown>) => unknown;

export interface Invocation {
  cmd: string;
  args: Record<string, unknown>;
}

/**
 * Record every command the page issues, answering from `overrides` where a
 * test cares and falling through to the shipped mock layer otherwise. The
 * recorded list is what proves *which* command a control issued — several
 * controls on these pages are indistinguishable by their label.
 */
export function useInvoke(overrides: Record<string, Handler> = {}): Invocation[] {
  installMock();
  const base = window.__TAURI_INTERNALS__?.invoke;
  if (!base) throw new Error('the mock layer did not install an invoke bridge');
  const seen: Invocation[] = [];
  window.__TAURI_INTERNALS__ = {
    invoke: (cmd: string, args?: Record<string, unknown>) => {
      seen.push({ cmd, args: args ?? {} });
      const over = overrides[cmd];
      return Promise.resolve(over ? over(args ?? {}) : base(cmd, args));
    },
  } as never;
  return seen;
}

/** Every recorded call to one command. */
export function callsTo(seen: Invocation[], cmd: string): Invocation[] {
  return seen.filter((c) => c.cmd === cmd);
}

export const STOPPED: ServerStatus = {
  handle: 'embedded',
  bind: '',
  pid: 0,
  uptime_secs: null,
  running: false,
};

export const RUNNING: ServerStatus = {
  handle: 'embedded',
  bind: '127.0.0.1:38080',
  pid: 4242,
  uptime_secs: 90,
  running: true,
};

/**
 * Svelte 5 implements every transition with the Web Animations API and
 * calls `element.animate(...)` directly. jsdom has no such method, so the
 * `out:` transition on a dismissed toast — and the `in:fade` on the page
 * host in `App.svelte` — throws a TypeError.
 *
 * `[FACT]` This is a copy of the stub in `components-chrome.test.ts`, not
 * an import: a test file cannot be imported without running its cases a
 * second time, and that file belongs to another lane. The stub completes
 * each animation on a timer instead of running real frames, so the removal
 * path the component actually implements is what gets observed. It only
 * fills in a method jsdom is missing.
 */
export function installAnimationStub(): void {
  if (typeof Element === 'undefined') return;
  // `[FACT]` The guard reads the property through an untyped view:
  // `Element.prototype.animate` is declared non-optional in the DOM lib
  // types, so TypeScript resolves it to a function type and reports
  // `if (Element.prototype.animate)` as always-true, which `svelte-check`
  // rejects — and `pnpm check` is a CI gate.
  const proto = Element.prototype as unknown as Record<string, unknown>;
  if (typeof proto['animate'] === 'function') return;
  Object.defineProperty(proto, 'animate', {
    configurable: true,
    writable: true,
    value: function animate(this: Element, _keyframes: unknown, options?: { duration?: number }) {
      const duration = Math.max(0, options?.duration ?? 0);
      const animation = {
        onfinish: null as (() => void) | null,
        // `playState` is read by the `tick` loop; reporting a finished
        // animation stops that loop from sampling a time never set.
        playState: 'finished',
        currentTime: duration,
        effect: {} as unknown,
        cancel: () => {
          animation.onfinish = null;
        },
        finish: () => animation.onfinish?.(),
      };
      setTimeout(() => animation.onfinish?.(), duration);
      return animation;
    },
  });
}

/**
 * Put every module singleton a route page reads back to its zero value,
 * unmount whatever the previous case rendered, and re-install the mock
 * invoke bridge (a case that replaced the bridge must not leak it).
 */
export function resetRouteStores(): void {
  cleanup();
  // `[FACT]` The `delete window.__TAURI_INTERNALS__` that used to sit here
  // was a workaround for `installMock()` identifying itself by grepping
  // its own source text. It is redundant now that the mock carries a
  // symbol, and a case that replaced the bridge still gets the mock back
  // from the plain call below.
  installMock();
  installAnimationStub();
  setTheme('auto');
  setLocale(INITIAL_LOCALE);
  clearToasts();
  server.set(STOPPED);
  serverBusy.set(false);
  repos.set([]);
  repoDetails.set({});
  reposLoading.set(false);
  vaultSecrets.set([]);
  vaultVersionsCache.set({});
  vaultBusy.set(false);
  clearWorktreeRoot();
  resetWorktree();
  window.location.hash = '#/';
}

/** Move the hash router and let jsdom deliver the `hashchange`. */
export async function gotoPath(hash: string): Promise<void> {
  window.location.hash = hash;
  await new Promise((r) => setTimeout(r, 0));
}

/** A promise a test resolves by hand, so a pending state is observable. */
export function deferred<T>(): {
  promise: Promise<T>;
  resolve: (v: T) => void;
  reject: (e: unknown) => void;
} {
  let resolve!: (v: T) => void;
  let reject!: (e: unknown) => void;
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}
