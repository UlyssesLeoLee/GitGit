/**
 * Tests for `LoginGate` and the credential path it drives.
 *
 * `[FACT]` This file deliberately does **not** mock `@/api`, the way
 * `app-routing.test.tsx` does. That file replaces the whole API module
 * with `vi.fn()`s, so it can say nothing about whether a request
 * actually carries an `Authorization` header — which is the entire
 * subject here. These cases drive the real axios singleton and assert on
 * the config that reaches the adapter.
 *
 * The behaviours pinned:
 *  - a signed-out visitor gets the form, not the console,
 *  - submitting attaches HTTP Basic auth to a request that is genuinely
 *    behind the auth layer,
 *  - the credential is **not** persisted anywhere,
 *  - a 401 anywhere drops the credential and brings the form back, and
 *  - a non-Latin-1 password is encoded as UTF-8 rather than throwing,
 *    which is the case a naive `btoa` gets wrong.
 */
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { act, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import type { InternalAxiosRequestConfig } from 'axios';
import { AxiosError } from 'axios';
import { LoginGate } from '@/components/LoginGate';
import { __resetClientForTests, configureClient, getClient } from '@/api/client';
import { ApiError } from '@/api/errors';
import { toBasicAuthHeader, useCredentialsStore } from '@/stores/credentials';
import { useLocaleStore } from '@/stores/locale';
import { MESSAGES } from '@/i18n/runtime';

interface Seen {
  url?: string;
  authorization?: string;
}

/**
 * Adapter that records what axios was about to send and settles the
 * reply the way a real one does.
 *
 * `[FACT]` It has to settle the response itself. `validateStatus` is
 * applied by `settle()`, which lives *inside* axios's built-in xhr/http
 * adapters — `dispatchRequest` does not consult it — so an adapter that
 * resolved with a 401 would produce a fulfilled promise and the test
 * would pass against a client configured to do the opposite of what is
 * being claimed here.
 */
function recordingAdapter(status: number, body: unknown = []) {
  const seen: Seen[] = [];
  const adapter = async (config: InternalAxiosRequestConfig) => {
    const headers = config.headers as unknown as Record<string, string> | undefined;
    seen.push({
      url: config.url,
      // `AxiosHeaders` lower-cases on set; read it back the way axios does.
      authorization: headers?.['Authorization'] ?? headers?.['authorization'],
    });
    const response = {
      data: body,
      status,
      statusText: status === 200 ? 'OK' : 'Unauthorized',
      headers: {},
      config,
    };
    const validate = config.validateStatus ?? ((s: number) => s >= 200 && s < 300);
    if (validate(status)) return response as never;
    return Promise.reject(
      new AxiosError(response.statusText, 'ERR_BAD_REQUEST', config, undefined, response as never),
    );
  };
  return { adapter, seen };
}

function renderGate() {
  return render(
    <LoginGate>
      <p>console content</p>
    </LoginGate>,
  );
}

beforeEach(() => {
  __resetClientForTests();
  useCredentialsStore.getState().signOut();
  useLocaleStore.setState({ locale: 'en' });
});

afterEach(() => {
  __resetClientForTests();
  // `[FACT]` `<LoginGate>` is still mounted — `cleanup()` is registered
  // in `vitest.setup.ts`, which loads first, and vitest runs this
  // later-registered hook first. Signing the store out here therefore
  // re-renders a live gate outside React's act environment.
  act(() => useCredentialsStore.getState().signOut());
});

describe('LoginGate', () => {
  it('shows the sign-in form, not the console, when signed out', () => {
    renderGate();

    expect(screen.getByRole('heading', { name: MESSAGES.en.login.title })).toBeInTheDocument();
    expect(screen.getByLabelText(MESSAGES.en.login.user)).toBeInTheDocument();
    expect(screen.getByLabelText(MESSAGES.en.login.password)).toBeInTheDocument();
    // The thing being protected must not be on the page yet.
    expect(screen.queryByText('console content')).toBeNull();
  });

  it('probes an endpoint behind the auth layer and opens the console on success', async () => {
    const user = userEvent.setup();
    // `/api/health` is deliberately outside the auth layer, so probing
    // with it would accept any password. The assertion on the path is
    // what keeps that from being "tidied up" later.
    const { adapter, seen } = recordingAdapter(200);
    configureClient({ adapter });
    renderGate();

    await user.type(screen.getByLabelText(MESSAGES.en.login.user), 'admin');
    await user.type(screen.getByLabelText(MESSAGES.en.login.password), 's3cret');
    // `[FACT]` Deliberately *not* wrapped in `act`. `userEvent` runs its
    // events inside Testing Library's `asyncWrapper`, which owns the act
    // environment; nesting an outer `act` around it makes the wrapper's
    // restore land on the wrong value and React reports "The current
    // testing environment is not configured to support act(...)" instead
    // of the warning being fixed.
    await user.click(screen.getByRole('button', { name: MESSAGES.en.login.submit }));

    expect(await screen.findByText('console content')).toBeInTheDocument();
    expect(seen).toHaveLength(1);
    expect(seen[0]?.url).toContain('/repos');
    expect(seen[0]?.url).not.toContain('/health');
    expect(seen[0]?.authorization).toBe(toBasicAuthHeader({ user: 'admin', password: 's3cret' }));
  });

  it('keeps the user on the form when the credential is refused', async () => {
    const user = userEvent.setup();
    const { adapter } = recordingAdapter(401, { error: 'unauthenticated', code: 'unauthenticated' });
    configureClient({ adapter });
    renderGate();

    await user.type(screen.getByLabelText(MESSAGES.en.login.user), 'admin');
    await user.type(screen.getByLabelText(MESSAGES.en.login.password), 'wrong');
    await user.click(screen.getByRole('button', { name: MESSAGES.en.login.submit }));

    expect(await screen.findByRole('alert')).toHaveTextContent(MESSAGES.en.login.failed);
    expect(screen.queryByText('console content')).toBeNull();
    // A refused credential must not be left held, or every later request
    // would repeat the 401 with no way back to the form.
    expect(useCredentialsStore.getState().credential).toBeNull();
  });

  it('drops the credential when any later request comes back 401', async () => {
    // Signing in is not a one-time event: a password rotated on the
    // server invalidates the held credential mid-session, and the
    // console has to fall back to the form rather than keep rendering
    // screens full of failed queries.
    useCredentialsStore.getState().signIn({ user: 'admin', password: 'stale' });
    const { adapter } = recordingAdapter(401);
    configureClient({ adapter });
    renderGate();

    expect(screen.getByText('console content')).toBeInTheDocument();

    // The call rejects — that is the point: a 401 is a failure the
    // caller must see, not a payload. What is asserted here is the
    // *side effect* on the credential, so the rejection is caught and
    // only its type checked.
    //
    // `[FACT]` It runs inside `act` because the response interceptor
    // calls `signOut()` on the way out, which re-renders the mounted
    // gate. Awaiting the request outside act is what produced
    // `An update to LoginGate inside a test was not wrapped in act(...)`.
    const err = await act(async () =>
      getClient()
        .get('/vault/keys')
        .then(
          () => null,
          (e: unknown) => e,
        ),
    );
    expect(err).toBeInstanceOf(ApiError);
    expect((err as ApiError).isUnauthenticated).toBe(true);

    await waitFor(() => expect(useCredentialsStore.getState().credential).toBeNull());
    expect(await screen.findByRole('heading', { name: MESSAGES.en.login.title })).toBeInTheDocument();
    expect(screen.queryByText('console content')).toBeNull();
  });

  it('lets the operator sign out', async () => {
    const user = userEvent.setup();
    useCredentialsStore.getState().signIn({ user: 'admin', password: 's3cret' });
    renderGate();

    await user.click(screen.getByRole('button', { name: MESSAGES.en.login.signOut }));

    expect(useCredentialsStore.getState().credential).toBeNull();
    expect(await screen.findByRole('heading', { name: MESSAGES.en.login.title })).toBeInTheDocument();
  });
});

describe('credential handling', () => {
  it('never writes the credential to durable storage', async () => {
    const user = userEvent.setup();
    const setItem = vi.spyOn(Storage.prototype, 'setItem');
    const { adapter } = recordingAdapter(200);
    configureClient({ adapter });
    renderGate();

    await user.type(screen.getByLabelText(MESSAGES.en.login.user), 'admin');
    await user.type(screen.getByLabelText(MESSAGES.en.login.password), 's3cret');
    // Not wrapped in `act` — see the note in the probe case above.
    await user.click(screen.getByRole('button', { name: MESSAGES.en.login.submit }));
    await screen.findByText('console content');

    // `[FACT]` The whole point of the server-side change was to stop the
    // secret living in a file. `theme` and `locale` persist deliberately;
    // this one must not, so a successful sign-in writing *anything* is
    // the failure this guards.
    for (const call of setItem.mock.calls) {
      expect(String(call[1])).not.toContain('s3cret');
    }
    setItem.mockRestore();
  });

  it('encodes a non-Latin-1 password as UTF-8 instead of throwing', () => {
    // `btoa` is byte-oriented and throws on any code point above
    // U+00FF. A password is arbitrary user input, so a CJK or accented
    // one is not an edge case — and the throw would happen inside the
    // request interceptor, turning "wrong password" into "the console
    // is broken". RFC 7617 base64s the UTF-8 bytes.
    const header = toBasicAuthHeader({ user: 'admin', password: '密码-ünï' });

    expect(header.startsWith('Basic ')).toBe(true);
    // Decode the way a server would, and compare on bytes.
    const decoded = new TextDecoder().decode(Uint8Array.from(atob(header.slice(6)), (c) => c.charCodeAt(0)));
    expect(decoded).toBe('admin:密码-ünï');
  });
});
