/**
 * Tests for the `/` route component (`src/routes/Home.svelte`).
 *
 * This file was 0% covered. The dashboard is the one page whose controls
 * change the state of the whole backend, so the cases below pin:
 *
 *  - the two mutually exclusive controls (start / stop) and the bind
 *    field that locks while a server owns the port,
 *  - the three facts the status block reports (pid, uptime, port) and
 *    the em dashes it shows for a server that is not running — a stale
 *    port on a stopped server is a claim the page cannot back up,
 *  - the three shapes a failed start can take. `friendlyError` is the
 *    interesting function on this page: a typed `AppError` with a
 *    localized template, a typed one without, and a rejection that is
 *    not an `AppError` at all,
 *  - and that the log tail is re-read after every action that could
 *    change it.
 *
 * `[FACT]` Toast *content* is asserted through the `toasts` store rather
 * than through a rendered card: these pages push into the store, and
 * `ToastHost` — which turns a store entry into a card — is covered by
 * `components-chrome.test.ts`. Asserting here keeps the test about what
 * this page decided to tell the user.
 */
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { get } from 'svelte/store';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import Home from '../../src/routes/Home.svelte';
import { server, serverBusy } from '../../src/lib/stores/server';
import { toasts } from '../../src/lib/stores/toasts';
import type { ServerStatus } from '../../src/lib/api/types';
import {
  RUNNING,
  STOPPED,
  callsTo,
  cat,
  deferred,
  resetRouteStores,
  t,
  useInvoke,
} from './routes-harness';

const LOG_LINE = '[mock] 2026-09-19T14:00:00Z INFO mock_server listening';

function startButton(): HTMLElement {
  return screen.getByTestId('home-start');
}

function bindInput(): HTMLInputElement {
  return screen.getByTestId('bind-input') as HTMLInputElement;
}

function statusValue(label: string): string {
  // The status block is a `<dl>` of label / value pairs; the value is the
  // `<dd>` that follows the label, so the pair is read structurally
  // rather than by position in the page.
  const dt = screen.getByText(label).closest('div');
  return dt?.querySelector('dd')?.textContent?.trim() ?? '';
}

beforeEach(() => {
  resetRouteStores();
});

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

describe('route / home — a stopped server', () => {
  it('offers to start, on the documented default bind, and claims nothing about a port', async () => {
    const seen = useInvoke();
    render(Home);
    await screen.findByTestId('home-start');

    // The default comes from ADR-0020 §2.2, and the field says so.
    expect(bindInput().value).toBe('127.0.0.1:38080');
    expect(bindInput().hasAttribute('disabled')).toBe(false);
    expect(screen.getByText(cat('dashboard.defaultBindHint'))).toBeTruthy();
    expect(startButton().textContent?.trim()).toBe(t('dashboard.start'));
    // Only one lifecycle control may be live at a time.
    expect(screen.queryByRole('button', { name: cat('dashboard.stop') })).toBeNull();
    // A stopped server has no pid, no uptime and no port. Em dashes, not
    // a leftover value from the last run.
    expect(statusValue(t('dashboard.pid'))).toBe('—');
    expect(statusValue(t('dashboard.uptime'))).toBe('—');
    expect(statusValue(t('dashboard.port'))).toBe('—');
    // …and the status was actually asked for, not just left at its zero
    // value. `refreshServerStatus` runs after the log read resolves, so
    // this is a wait, not a synchronous read.
    await waitFor(() => expect(callsTo(seen, 'server_status').length).toBeGreaterThan(0));
  });

  it('reads the log tail on mount and names the active locale', async () => {
    const seen = useInvoke({ server_logs: () => [LOG_LINE, 'second line'] });
    render(Home);

    const pane = await waitFor(() => {
      const el = screen.getByTestId('logs-pane');
      expect(el.textContent).toContain(LOG_LINE);
      return el;
    });
    expect(pane.textContent).toContain('second line');
    expect(callsTo(seen, 'server_logs')[0]?.args).toMatchObject({ limit: 200 });
    // Shown raw, not translated: it is subprocess output.
    expect(screen.getByText((text) => text.includes('locale:'))).toBeTruthy();
  });

  it('refetches the log tail when asked, and locks the button meanwhile', async () => {
    const gate = deferred<string[]>();
    let call = 0;
    const seen = useInvoke({
      server_logs: () => {
        call += 1;
        return call === 1 ? Promise.resolve([LOG_LINE]) : gate.promise;
      },
    });
    render(Home);
    await waitFor(() => expect(callsTo(seen, 'server_logs')).toHaveLength(1));

    const refresh = screen.getByRole('button', { name: cat('common.refresh') });
    // Let the mount-time read finish first. `[FACT]` `refreshing` is one
    // flag shared by every `refreshLogs` call, so two *overlapping* reads
    // clear each other's busy state — the older call's `finally` runs
    // last. Clicking while the first is still in flight therefore cannot
    // be used to observe the lock; the user cannot click a disabled
    // button either.
    await waitFor(() => expect(refresh.hasAttribute('disabled')).toBe(false));
    await fireEvent.click(refresh);
    // A second click while the first is in flight would race two reads.
    await waitFor(() => expect(refresh.hasAttribute('disabled')).toBe(true));

    gate.resolve([LOG_LINE, 'fresh line']);
    await waitFor(() => expect(refresh.hasAttribute('disabled')).toBe(false));
    expect(screen.getByTestId('logs-pane').textContent).toContain('fresh line');
  });
});

describe('route / home — a running server', () => {
  it('offers to stop instead, and reports what is actually running', async () => {
    server.set(RUNNING);
    useInvoke();
    render(Home);
    const stop = await screen.findByRole('button', { name: cat('dashboard.stop') });

    expect(screen.queryByTestId('home-start')).toBeNull();
    // The port is bound now, so the field is read-only.
    expect(bindInput().hasAttribute('disabled')).toBe(true);
    expect(stop.hasAttribute('disabled')).toBe(false);
    // 90s as "1m 30s", not as raw seconds, and not an em dash.
    expect(statusValue(t('dashboard.pid'))).toBe('4242');
    expect(statusValue(t('dashboard.uptime'))).toBe('1m 30s');
    expect(statusValue(t('dashboard.port'))).toBe('127.0.0.1:38080');
  });

  it('stops the server and reports the new state', async () => {
    server.set(RUNNING);
    const seen = useInvoke();
    render(Home);
    await screen.findByRole('button', { name: cat('dashboard.stop') });

    await fireEvent.click(screen.getByRole('button', { name: cat('dashboard.stop') }));

    await waitFor(() => expect(get(server).running).toBe(false));
    expect(callsTo(seen, 'stop_server')).toHaveLength(1);
    // `stopServer` follows the stop with a fresh `server_status`: the
    // command returns the *prior* snapshot, so trusting it would leave
    // the page claiming the server is still up.
    expect(callsTo(seen, 'server_status').length).toBeGreaterThan(1);
    expect(get(toasts).map((t) => [t.kind, t.message])).toEqual([['info', t('dashboard.stopped')]]);
    // The control set follows the state, in both directions.
    expect(screen.getByTestId('home-start')).toBeTruthy();
    expect(screen.queryByRole('button', { name: cat('dashboard.stop') })).toBeNull();
    expect(statusValue(t('dashboard.pid'))).toBe('—');
  });

  it('disables both controls and says "starting" while a start is in flight', async () => {
    // `serverBusy` is what the top bar and this page share, so the label
    // has to admit that something is happening rather than leaving a
    // clickable button that will be ignored.
    serverBusy.set(true);
    useInvoke();
    render(Home);

    const button = await screen.findByTestId('home-start');
    expect(button.hasAttribute('disabled')).toBe(true);
    expect(button.textContent?.trim()).toBe(t('dashboard.starting'));
    // The port is not owned yet, so the bind field stays editable.
    expect(bindInput().hasAttribute('disabled')).toBe(false);
  });
});

describe('route / home — starting a server', () => {
  it('starts on the bind the user typed, then offers to stop it', async () => {
    // `[FACT]` `start_server` is answered from a fixture here rather
    // than from the shipped mock. The mock's `STORE` lives for the whole
    // test file, so really starting a server would leave every later
    // case's `server_status` answering `running: true` — and this page
    // reads that on mount. The fixture is the same wire shape.
    const started: ServerStatus = {
      handle: 'embedded',
      bind: '10.0.0.5:9999',
      pid: 4242,
      uptime_secs: 0,
      running: true,
    };
    /** A distinguishable stopped snapshot, so "the status read landed"
     *  is observable rather than assumed. */
    const probed: ServerStatus = { ...STOPPED, pid: 1 };
    const seen = useInvoke({ start_server: () => started, server_status: () => probed });
    render(Home);
    // Wait for the mount-time status read to land before starting. The
    // page fires it without awaiting, so a click in the same tick would
    // race it and the stale snapshot would win.
    await waitFor(() => expect(get(server).pid).toBe(probed.pid));

    await fireEvent.input(bindInput(), { target: { value: '10.0.0.5:9999' } });
    await fireEvent.click(startButton());

    await waitFor(() => expect(get(server).running).toBe(true));
    // The typed value is what reached the backend — the default must not
    // win over what the user entered.
    expect(callsTo(seen, 'start_server')[0]?.args).toMatchObject({ bind: '10.0.0.5:9999' });
    expect(get(server).bind).toBe('10.0.0.5:9999');
    expect(get(server).pid).toBe(4242);
    // …and the log tail is re-read, because a server that just started
    // has just written to it.
    expect(callsTo(seen, 'server_logs').length).toBeGreaterThan(1);
    expect(get(toasts).map((t) => [t.kind, t.message])).toEqual([['success', t('dashboard.running')]]);
    expect(bindInput().hasAttribute('disabled')).toBe(true);
    expect(screen.getByRole('button', { name: cat('dashboard.stop') })).toBeTruthy();
  });

  it('reports a refused start with the localized reason for that kind', async () => {
    // The typed-error path: `errors.kind.Bind` has a template, so the
    // user gets "Could not bind port" rather than a raw backend string.
    useInvoke({
      start_server: () => Promise.reject({ kind: 'Bind', message: 'address already in use' }),
    });
    render(Home);
    await screen.findByTestId('home-start');

    await fireEvent.click(startButton());

    await waitFor(() => expect(get(toasts)).toHaveLength(1));
    expect(get(toasts)[0]?.kind).toBe('error');
    expect(get(toasts)[0]?.message).toBe(t('errors.kind.Bind'));
    expect(get(toasts)[0]?.message).not.toContain('address already in use');
  });

  it('falls back to the backend message for a kind the catalogue does not name', async () => {
    // The i18n lookup returns the key when it misses, so the raw message
    // is the only useful thing left to show.
    useInvoke({
      start_server: () => Promise.reject({ kind: 'KernelPanic', message: 'thread pool exhausted' }),
    });
    render(Home);
    await screen.findByTestId('home-start');

    await fireEvent.click(startButton());

    await waitFor(() => expect(get(toasts)).toHaveLength(1));
    expect(get(toasts)[0]?.message).toBe('thread pool exhausted');
    expect(get(toasts)[0]?.message).not.toContain('errors.kind');
  });

  it('stringifies a rejection that is not an AppError at all', async () => {
    useInvoke({ start_server: () => Promise.reject('socket closed') });
    render(Home);
    await screen.findByTestId('home-start');

    await fireEvent.click(startButton());

    await waitFor(() => expect(get(toasts)).toHaveLength(1));
    expect(get(toasts)[0]?.message).toBe('socket closed');
  });

  it('leaves the start control usable after a failure', async () => {
    // A failed start must not strand the user: the button comes back, the
    // busy flag is cleared, and the port is still not owned.
    useInvoke({
      start_server: () => Promise.reject({ kind: 'Bind', message: 'address already in use' }),
    });
    render(Home);
    await screen.findByTestId('home-start');

    await fireEvent.click(startButton());
    await waitFor(() => expect(get(toasts)).toHaveLength(1));

    expect(get(serverBusy)).toBe(false);
    expect(get(server).running).toBe(false);
    const button = screen.getByTestId('home-start');
    await waitFor(() => expect(button.hasAttribute('disabled')).toBe(false));
    expect(button.textContent?.trim()).toBe(t('dashboard.start'));
    expect(bindInput().hasAttribute('disabled')).toBe(false);
  });
});

describe('route / home — the log controls', () => {
  it('clears the log tail and re-reads it', async () => {
    const seen = useInvoke();
    render(Home);
    await waitFor(() => expect(callsTo(seen, 'server_logs')).toHaveLength(1));

    await fireEvent.click(screen.getByRole('button', { name: cat('common.delete') }));

    await waitFor(() => expect(callsTo(seen, 'clear_logs')).toHaveLength(1));
    // The pane is re-read rather than blanked locally, so what it shows
    // is what the backend still holds.
    expect(callsTo(seen, 'server_logs').length).toBeGreaterThan(1);
  });

  it('empties the log tail when the backend reports no lines', async () => {
    // Asserted as a *transition*: the pane held a line first, so an
    // assertion that merely found the pane empty could not pass on a
    // page that never rendered logs at all. A page that kept the old
    // lines after the backend cleared them would show them here.
    let call = 0;
    useInvoke({
      server_logs: () => {
        call += 1;
        return Promise.resolve(call === 1 ? [LOG_LINE] : []);
      },
    });
    render(Home);
    await waitFor(() =>
      expect(screen.getByTestId('logs-pane').textContent).toContain(LOG_LINE)
    );

    await fireEvent.click(screen.getByRole('button', { name: cat('common.refresh') }));

    await waitFor(() => expect(screen.getByTestId('logs-pane').textContent?.trim()).toBe(''));
  });
});
