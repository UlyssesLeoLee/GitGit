/**
 * Component tests for the shared chrome: the pieces every route is
 * wrapped in. `ErrorBoundary`, `Sidebar`, `ServerStatusBar`,
 * `ThemeToggle`, `ToastHost` and `Router` were all at 0% — the route
 * components had no tests either, so nothing under
 * `src/lib/components/` other than `RepoWorktree` had ever been
 * rendered.
 *
 * Two notes on what this file can and cannot assert, both measured
 * rather than assumed:
 *
 *   - `ErrorBoundary` cannot be given real children from a `.ts` file.
 *     A Svelte `Snippet` has to be produced by the compiler; passing a
 *     plain function renders nothing. Its `children` branch is
 *     therefore not exercised here, and neither is its error card —
 *     see the defect note on `lastError` below.
 *   - `Router`'s param hand-off cannot be observed without a fixture
 *     component that prints its props, which would mean adding a
 *     `.svelte` file outside this lane's scope. What is pinned is
 *     which component the router picks and that it swaps on a hash
 *     change; `matchRoute`'s own param extraction is covered by
 *     `router.test.ts`.
 *
 * Assertions compare against `tFor(get(locale), key)` rather than a
 * pinned language, following the existing house rule: jsdom reports
 * `navigator.language === 'en-US'`, so the active locale is `en`.
 */
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { get } from 'svelte/store';
import { render, screen, fireEvent, cleanup, waitFor } from '@testing-library/svelte';
import ErrorBoundary from '../../src/lib/components/ErrorBoundary.svelte';
import ThemeToggle from '../../src/lib/components/ThemeToggle.svelte';
import ToastHost from '../../src/lib/components/ToastHost.svelte';
import Sidebar from '../../src/lib/components/Sidebar.svelte';
import ServerStatusBar from '../../src/lib/components/ServerStatusBar.svelte';
import Router from '../../src/lib/components/Router.svelte';
import { theme, setTheme } from '../../src/lib/stores/theme';
import { toasts, clearToasts, pushToast } from '../../src/lib/stores/toasts';
import { server, serverBusy } from '../../src/lib/stores/server';
import { tFor, locale } from '../../src/lib/i18n';
import type { RouteMap } from '../../src/lib/router';
import type { ServerStatus } from '../../src/lib/api/types';
import { installMock } from '../../src/mocks/handlers';

const STOPPED: ServerStatus = {
  handle: 'embedded',
  bind: '',
  pid: 0,
  uptime_secs: null,
  running: false,
};

const RUNNING: ServerStatus = {
  handle: 'embedded',
  bind: '127.0.0.1:38080',
  pid: 4242,
  uptime_secs: 90,
  running: true,
};

/** Move the hash router and let jsdom deliver the `hashchange`. */
async function gotoPath(hash: string): Promise<void> {
  window.location.hash = hash;
  await new Promise((r) => setTimeout(r, 0));
}

/**
 * Svelte 5 implements every transition with the Web Animations API and
 * calls `element.animate(...)` directly. jsdom has no such method, so
 * the `out:` transition on a dismissed toast throws a TypeError and
 * the element is never removed from the DOM — the store updates, the
 * card stays. `[FACT]` Measured: without this stub,
 * `dismisses only the toast whose button was clicked` fails with
 * `TypeError: element.animate is not a function` from
 * `svelte/src/internal/client/dom/elements/transitions.js:415`.
 *
 * The stub completes each animation on a timer instead of running real
 * frames, so the removal path the component actually implements is what
 * gets observed. It only fills in a method jsdom is missing; it does
 * not change what the component does.
 */
function installAnimationStub(): void {
  if (typeof Element === 'undefined') return;
  // `[FACT]` The guard has to read the property through an untyped view.
  // `Element.prototype.animate` is declared non-optional in the DOM lib
  // types, so TypeScript resolves `Element.prototype.animate` to a
  // function type and reports `if (Element.prototype.animate)` as a
  // condition that "will always return true" — which `svelte-check`
  // rejects as an error, and the `gm-desktop` CI job runs
  // `svelte-check` as a gate. The runtime check is still the correct
  // one; a real browser has the method and jsdom does not, so the stub
  // must stay conditional. Going through `Record<string, unknown>`
  // keeps the runtime semantics and drops the false positive.
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
        // animation stops that loop from sampling a time we never set.
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

beforeEach(() => {
  cleanup();
  installAnimationStub();
  delete window.__TAURI_INTERNALS__;
  installMock();
  setTheme('auto');
  clearToasts();
  server.set(STOPPED);
  serverBusy.set(false);
  window.location.hash = '#/';
});

afterEach(() => {
  cleanup();
  window.location.hash = '#/';
  vi.restoreAllMocks();
});

/* ---------- ErrorBoundary ---------- */

describe('component / ErrorBoundary', () => {
  it('renders a bare container when it has no children', () => {
    render(ErrorBoundary);
    const el = screen.getByTestId('error-boundary');
    expect(el).toBeTruthy();
    // Mount key 0: the counter the retry affordance would bump.
    expect(el.getAttribute('data-eb-key')).toBe('0');
  });

  it('renders no error card in the normal case', () => {
    // The card and its retry button only appear when `lastError` is
    // set, so their absence here is the state the app is actually in
    // for every route that renders successfully.
    render(ErrorBoundary);
    expect(screen.queryByRole('button')).toBeNull();
    expect(screen.queryByText(tFor(get(locale), 'errors.routeTitle'))).toBeNull();
  });

  it('contains an exception from its child, and offers a working retry', async () => {
    // `[FACT]` Measured: passing a snippet that throws makes `render`
    // itself throw, and nothing is left in the document. This is the
    // opposite of what the component's own header comment promises
    // ("Catches uncaught render exceptions in its default slot").
    //
    // The cause is visible in the source: `lastError` is a `$state`
    // that is only ever assigned `null`, in `reset()`. No code path
    // sets it to an `Error`, so the `{#if lastError}` branch and the
    // `reset` handler behind it are unreachable. The inline comment
    // from line 26 onwards says as much — "Svelte 5 has no built-in
    // error boundary yet … let the route-level try/catch decide" —
    // which contradicts the header comment above it.
    //
    // Asserted as the behaviour it is today. A real fix belongs in
    // `ErrorBoundary.svelte`; this case then documents the new one.
    //
    // `[FACT]` The fix has landed, so the expectation is inverted: the
    // boundary is built on `<svelte:boundary>` (available since Svelte
    // 5.3.0; this repo pins 5.57.1) and now contains the throw. The card
    // and the retry affordance — dead code while `lastError` was only ever
    // assigned `null` — are reachable, and `data-eb-key` advances when
    // the child is retried.
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const throwing = (() => { throw new Error('boom'); }) as any;

    render(ErrorBoundary, { props: { children: throwing } });

    // The mount point survives; the failure is reported inside it.
    const el = screen.getByTestId('error-boundary');
    expect(el.getAttribute('data-state')).toBe('failed');
    expect(screen.getByTestId('error-boundary-failed')).toBeTruthy();
    expect(document.body.textContent).toContain('boom');
    expect(screen.getByRole('button', { name: tFor(get(locale), 'errors.routeRetry') })).toBeTruthy();

    // Retrying remounts the subtree, which is the only way a child that
    // threw during render can recover.
    await fireEvent.click(screen.getByRole('button', { name: tFor(get(locale), 'errors.routeRetry') }));
    await waitFor(() =>
      expect(screen.getByTestId('error-boundary').getAttribute('data-eb-key')).toBe('1')
    );
  });

  it('does not render "[object Object]" when a child throws an AppError payload', () => {
    // `[FACT]` The defect this pins: `toMessage` was
    // `e instanceof Error ? e.message : String(e)`, with no `AppError`
    // awareness. Tauri rejects with the serialized `AppError` from
    // `src-tauri/src/error.rs` — a plain `{ kind, message, source }`
    // object, not an `Error` — so the `instanceof` arm never matched and
    // `String(e)` rendered the literal `[object Object]`. A render
    // failure is precisely when a command underneath is most likely to
    // have rejected, so this was the worst possible place to be blind to
    // the payload shape. This is the same defect class that was already
    // fixed in four other files; the boundary was missed.
    //
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const throwing = (() => {
      throw { kind: 'NotAWorkTree', message: 'demo is a bare repository', source: '"NotAWorkTree"' };
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
    }) as any;

    render(ErrorBoundary, { props: { children: throwing } });

    expect(screen.getByTestId('error-boundary-failed')).toBeTruthy();
    // The kind wins, so the user is told what to do about it in their
    // own language rather than shown raw git output.
    expect(document.body.textContent).toContain(
      tFor(get(locale), 'errors.kind.NotAWorkTree')
    );
    expect(document.body.textContent).not.toContain('[object Object]');
  });

  it('shows the localized Internal label when the throw carries nothing usable', () => {
    // `[FACT]` A payload with neither a usable kind nor a message leaves
    // nothing to say. `friendlyError`'s floor is the localized
    // `errors.kind.Internal` label, which is why the boundary uses it
    // rather than `normalizeError(...).message` — the latter returns
    // `''` here and the card would render a blank line under its heading.
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const throwing = (() => {
      throw { kind: '' };
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
    }) as any;

    render(ErrorBoundary, { props: { children: throwing } });

    expect(document.body.textContent).toContain(tFor(get(locale), 'errors.kind.Internal'));
    expect(document.body.textContent).not.toContain('[object Object]');
  });
});

/* ---------- ThemeToggle ---------- */

describe('component / ThemeToggle', () => {
  it('offers exactly the three modes, labelled from the catalogue', () => {
    render(ThemeToggle);
    // The test id is `theme-<mode>`; the catalogue key is
    // `settings.theme<Mode>` in camelCase. Pinned separately because
    // the two spellings are unrelated and a mismatch shows an empty
    // button rather than an error.
    for (const [id, key] of [
      ['theme-light', 'settings.themeLight'],
      ['theme-auto', 'settings.themeAuto'],
      ['theme-dark', 'settings.themeDark'],
    ] as const) {
      expect(screen.getByTestId(id).textContent?.trim(), id).toBe(tFor(get(locale), key));
    }
    expect(screen.getByTestId('theme-toggle')).toBeTruthy();
  });

  it('marks exactly one mode as pressed', () => {
    render(ThemeToggle);
    const pressed = ['theme-light', 'theme-auto', 'theme-dark'].filter(
      (id) => screen.getByTestId(id).getAttribute('aria-pressed') === 'true'
    );
    // A segmented control with two pressed buttons is ambiguous; the
    // user cannot tell which theme is active.
    expect(pressed).toEqual(['theme-auto']);
  });

  it('moves the pressed state when a mode is clicked', async () => {
    render(ThemeToggle);
    await fireEvent.click(screen.getByTestId('theme-dark'));

    expect(get(theme)).toBe('dark');
    expect(screen.getByTestId('theme-dark').getAttribute('aria-pressed')).toBe('true');
    expect(screen.getByTestId('theme-auto').getAttribute('aria-pressed')).toBe('false');
  });

  it('changes the store, the document and the persisted value together', async () => {
    // All three are the same decision. If the document attribute did
    // not follow, the buttons would say "dark" while the page stayed
    // light — the exact class of bug the store's subscribe side effect
    // exists to prevent.
    render(ThemeToggle);
    await fireEvent.click(screen.getByTestId('theme-light'));

    expect(get(theme)).toBe('light');
    expect(document.documentElement.dataset.theme).toBe('light');
    expect(localStorage.getItem('gm-desktop.theme')).toBe('light');
  });

  it('highlights the active mode and not the others', async () => {
    // `auto` is the default the store starts on, so it is the
    // highlighted one before any click.
    render(ThemeToggle);
    expect(screen.getByTestId('theme-auto').className).toContain('bg-accent-500');
    expect(screen.getByTestId('theme-light').className).toContain('bg-slate-200');

    await fireEvent.click(screen.getByTestId('theme-light'));

    expect(screen.getByTestId('theme-light').className).toContain('bg-accent-500');
    expect(screen.getByTestId('theme-auto').className).toContain('bg-slate-200');
  });

  it('keeps the pressed state in step with the store, not just with clicks', async () => {
    // The control is a projection of the store. A settings page that
    // sets the theme directly must light the matching button.
    render(ThemeToggle);
    setTheme('dark');
    await waitFor(() =>
      expect(screen.getByTestId('theme-dark').getAttribute('aria-pressed')).toBe('true')
    );
  });
});

/* ---------- ToastHost ---------- */

describe('component / ToastHost', () => {
  it('renders the host with nothing in it when there are no toasts', () => {
    render(ToastHost);
    expect(screen.getByTestId('toast-host')).toBeTruthy();
    expect(screen.queryByTestId('toast-info')).toBeNull();
    expect(screen.queryByTestId('toast-success')).toBeNull();
  });

  it('renders one card per toast, in push order', () => {
    // `ttl_ms: 0` keeps them sticky so the auto-dismiss timer cannot
    // remove a card mid-assertion.
    pushToast('info', 'first message', 0);
    pushToast('warn', 'second message', 0);
    render(ToastHost);

    const cards = Array.from(document.querySelectorAll('[role="status"]'));
    expect(cards).toHaveLength(2);
    expect(cards[0]?.textContent).toContain('first message');
    expect(cards[1]?.textContent).toContain('second message');
  });

  it('gives each kind its own colour', () => {
    // The kind is the only signal a user gets at a glance; four
    // identical grey cards would make the distinction invisible.
    for (const [kind, marker] of [
      ['success', 'bg-emerald-500/95'],
      ['warn', 'bg-amber-500/95'],
      ['error', 'bg-red-500/95'],
      ['info', 'bg-slate-800/95'],
    ] as const) {
      cleanup();
      clearToasts();
      pushToast(kind, `${kind} message`, 0);
      render(ToastHost);
      expect(screen.getByTestId(`toast-${kind}`).className, kind).toContain(marker);
    }
  });

  it('dismisses only the toast whose button was clicked', async () => {
    const first = pushToast('info', 'keep me', 0);
    pushToast('error', 'dismiss me', 0);
    render(ToastHost);

    const cards = screen.getAllByRole('status');
    await fireEvent.click(cards[1]!.querySelector('button') as HTMLElement);

    // Dismissing is per-toast: the other one has to survive, keyed by
    // its own id rather than by position.
    expect(get(toasts).map((t) => t.id)).toEqual([first]);
    await waitFor(() => expect(screen.queryByTestId('toast-error')).toBeNull());
    expect(screen.getByTestId('toast-info').textContent).toContain('keep me');
  });

  it('removes the last toast and leaves an empty host', async () => {
    pushToast('success', 'all done', 0);
    render(ToastHost);

    // Scoped to this toast's own button: the host also carries
    // `role="region"`, so a role query alone is not specific enough.
    const dismiss = screen.getByTestId('toast-success').querySelector('button') as HTMLElement;
    await fireEvent.click(dismiss);

    expect(get(toasts)).toEqual([]);
    await waitFor(() => expect(screen.queryByTestId('toast-success')).toBeNull());
    expect(screen.getByTestId('toast-host')).toBeTruthy();
  });

  it('renders an untrusted toast message as text, not as markup', () => {
    // Messages can carry subprocess or provider output.
    pushToast('error', '<img src=x onerror=alert(1)>', 0);
    render(ToastHost);

    const card = screen.getByTestId('toast-error');
    expect(card.textContent).toContain('<img src=x onerror=alert(1)>');
    expect(card.querySelector('img')).toBeNull();
  });
});

/* ---------- Sidebar ---------- */

describe('component / Sidebar', () => {
  // The test ids are `nav-${labelKey}`, and the label keys already
  // start with `nav.` — so the rendered id is `nav-nav.repos`, not
  // `nav-repos`. `[FACT]` That doubled prefix is what the template
  // produces; the cases below use the ids as they really are rather
  // than the tidier spelling someone would guess.
  it('links every destination through the hash router', async () => {
    await gotoPath('#/');
    render(Sidebar);

    // The `#` prefix is the whole contract with the router: an anchor
    // without it triggers a full page load and throws the app state
    // away. This is the assertion that catches a plain `href="/repos"`.
    for (const [id, path] of [
      ['nav-nav.dashboard', '/'],
      ['nav-nav.repos', '/repos'],
      ['nav-nav.vault', '/vault'],
      ['nav-nav.graph', '/graph'],
      ['nav-nav.review', '/review'],
      ['nav-nav.settings', '/settings'],
    ] as const) {
      expect(screen.getByTestId(id).getAttribute('href'), id).toBe(`#${path}`);
    }
  });

  it('labels every link from the catalogue', async () => {
    await gotoPath('#/');
    render(Sidebar);
    // Not the raw key: a missing translation must show up as a failed
    // assertion here rather than as `{nav.repos}` on screen.
    expect(screen.getByTestId('nav-nav.repos').textContent).toContain(tFor(get(locale), 'nav.repos'));
    expect(screen.getByTestId('nav-nav.repos').textContent).not.toContain('nav.repos');
  });

  it('marks the current destination as the page', async () => {
    await gotoPath('#/vault');
    render(Sidebar);

    const current = Array.from(document.querySelectorAll('[aria-current="page"]'));
    expect(current).toHaveLength(1);
    expect(current[0]?.getAttribute('data-testid')).toBe('nav-nav.vault');
  });

  it('keeps a parent lit while a detail view is open', async () => {
    // The prefix check: `/repos/foo` is still "Repositories", or the
    // sidebar goes dark on every detail page.
    await gotoPath('#/repos/alpha');
    render(Sidebar);

    expect(screen.getByTestId('nav-nav.repos').getAttribute('aria-current')).toBe('page');
    expect(screen.getByTestId('nav-nav.vault').getAttribute('aria-current')).toBeNull();
  });

  it('does not light the dashboard for a nested path', async () => {
    // The inverse of the same rule: `/` must match only exactly, or
    // "Dashboard" would be highlighted on every page.
    await gotoPath('#/settings');
    render(Sidebar);

    expect(screen.getByTestId('nav-nav.dashboard').getAttribute('aria-current')).toBeNull();
    expect(screen.getByTestId('nav-nav.settings').getAttribute('aria-current')).toBe('page');
  });

  it('lights the dashboard at the root only', async () => {
    await gotoPath('#/');
    render(Sidebar);
    expect(screen.getByTestId('nav-nav.dashboard').getAttribute('aria-current')).toBe('page');
  });

  it('highlights the active link with the active classes', async () => {
    await gotoPath('#/graph');
    render(Sidebar);

    expect(screen.getByTestId('nav-nav.graph').className).toContain('bg-accent-50');
    expect(screen.getByTestId('nav-nav.repos').className).not.toContain('bg-accent-50');
  });

  it('follows a hash change to a different destination', async () => {
    await gotoPath('#/');
    render(Sidebar);
    expect(screen.getByTestId('nav-nav.dashboard').getAttribute('aria-current')).toBe('page');

    await gotoPath('#/review');
    await waitFor(() =>
      expect(screen.getByTestId('nav-nav.review').getAttribute('aria-current')).toBe('page')
    );
    expect(screen.getByTestId('nav-nav.dashboard').getAttribute('aria-current')).toBeNull();
  });

  it('carries the theme and locale controls', async () => {
    await gotoPath('#/');
    render(Sidebar);
    // The sidebar is where the theme toggle lives, so a Sidebar without
    // it is a page with no way to change theme.
    expect(screen.getByTestId('theme-toggle')).toBeTruthy();
    expect(screen.getByTestId('locale-switcher')).toBeTruthy();
  });
});

/* ---------- ServerStatusBar ---------- */

describe('component / ServerStatusBar', () => {
  it('offers to start a server that is stopped', () => {
    server.set(STOPPED);
    render(ServerStatusBar);

    expect(screen.getByTestId('server-status-bar').textContent).toContain(
      tFor(get(locale), 'dashboard.stopped')
    );
    expect(screen.getByTestId('start-server')).toBeTruthy();
    // Only one of the two controls may be live at a time.
    expect(screen.queryByTestId('stop-server')).toBeNull();
  });

  it('hides the port and uptime while the server is stopped', () => {
    // A stopped server has no port; showing the last one would imply it
    // is still reachable.
    server.set(STOPPED);
    render(ServerStatusBar);

    expect(screen.getByTestId('server-status-bar').textContent).not.toContain(
      tFor(get(locale), 'dashboard.port')
    );
    expect(screen.getByTestId('server-status-bar').textContent).not.toContain('38080');
  });

  it('shows the bind and the formatted uptime while running', () => {
    server.set(RUNNING);
    render(ServerStatusBar);

    const bar = screen.getByTestId('server-status-bar');
    expect(bar.textContent).toContain(tFor(get(locale), 'dashboard.running'));
    expect(bar.textContent).toContain('127.0.0.1:38080');
    // 90s renders as "1m 30s", not as raw seconds.
    expect(bar.textContent).toContain('1m 30s');
    expect(screen.getByTestId('stop-server')).toBeTruthy();
    expect(screen.queryByTestId('start-server')).toBeNull();
  });

  it('colours the indicator by state', () => {
    server.set(STOPPED);
    const { unmount } = render(ServerStatusBar);
    expect(screen.getByTestId('server-indicator').className).toContain('bg-slate-400');
    unmount();

    server.set(RUNNING);
    render(ServerStatusBar);
    expect(screen.getByTestId('server-indicator').className).toContain('bg-accent-500');
  });

  it('starts the server and flips the control to stop', async () => {
    server.set(STOPPED);
    render(ServerStatusBar);

    await fireEvent.click(screen.getByTestId('start-server'));
    await waitFor(() => expect(get(server).running).toBe(true));

    expect(screen.getByTestId('server-status-bar').textContent).toContain(
      tFor(get(locale), 'dashboard.running')
    );
    expect(screen.getByTestId('stop-server')).toBeTruthy();
  });

  it('stops a running server and flips the control back', async () => {
    server.set(RUNNING);
    render(ServerStatusBar);

    await fireEvent.click(screen.getByTestId('stop-server'));
    await waitFor(() => expect(get(server).running).toBe(false));

    expect(screen.getByTestId('start-server')).toBeTruthy();
    expect(screen.getByTestId('server-status-bar').textContent).toContain(
      tFor(get(locale), 'dashboard.stopped')
    );
  });

  it('disables the control while a call is already in flight', () => {
    // A double-clicked start would otherwise fire two `start_server`
    // commands and race the two answers.
    server.set(STOPPED);
    serverBusy.set(true);
    render(ServerStatusBar);

    expect((screen.getByTestId('start-server') as HTMLButtonElement).disabled).toBe(true);
  });

  it('re-enables the control once the call settles', async () => {
    server.set(STOPPED);
    render(ServerStatusBar);

    await fireEvent.click(screen.getByTestId('start-server'));
    await waitFor(() =>
      expect((screen.getByTestId('stop-server') as HTMLButtonElement).disabled).toBe(false)
    );
  });
});

/* ---------- Router ---------- */

describe('component / Router', () => {
  // Two components that render a distinctive test id and take no
  // props, used as stand-ins so a case can tell which one the router
  // picked without depending on a route component's own behaviour.
  const routes: RouteMap = {
    '/': { component: ThemeToggle },
    '/panel/:id': { component: ToastHost },
  };

  it('renders the component matching the current path', async () => {
    await gotoPath('#/');
    render(Router, { props: { routes } });

    expect(screen.getByTestId('theme-toggle')).toBeTruthy();
    expect(screen.queryByTestId('toast-host')).toBeNull();
  });

  it('matches a parameterised route', async () => {
    await gotoPath('#/panel/alpha');
    render(Router, { props: { routes } });

    expect(screen.getByTestId('toast-host')).toBeTruthy();
    expect(screen.queryByTestId('theme-toggle')).toBeNull();
  });

  it('swaps the component when the path changes', async () => {
    await gotoPath('#/');
    render(Router, { props: { routes } });
    expect(screen.getByTestId('theme-toggle')).toBeTruthy();

    await gotoPath('#/panel/alpha');
    await waitFor(() => expect(screen.getByTestId('toast-host')).toBeTruthy());
    expect(screen.queryByTestId('theme-toggle')).toBeNull();
  });

  it('renders nothing when no route matches and there is no catch-all', async () => {
    await gotoPath('#/nowhere');
    const { container } = render(Router, { props: { routes } });

    // The `{#if match}` guard: an unmatched path renders an empty shell
    // rather than crashing on a missing component.
    expect(container.textContent).toBe('');
  });

  it('prefers the catch-all over rendering nothing', async () => {
    await gotoPath('#/nowhere');
    const withFallback: RouteMap = { ...routes, '*': { component: ToastHost } };
    render(Router, { props: { routes: withFallback } });

    expect(screen.getByTestId('toast-host')).toBeTruthy();
  });
});
