/**
 * HTTP client used by the UI when it talks to the embedded server's
 * REST surface (e.g. for /api/repos in worker-A's parallel track).
 *
 * This file lives next to the Tauri wrappers so callers can opt
 * for either path. The V0.1 build of gm-desktop does not depend on
 * the REST API yet — the React/Vite UI for the Git smart-HTTP
 * endpoints goes through Tauri's fetch bridge, which sees the same
 * Content-Types the upstream `git` client expects.
 *
 * When worker-A's branch lands, REST endpoints will be reached via
 * `fetchJson<T>(path)` here.
 */

export interface ApiError {
  status: number;
  message: string;
}

const DEFAULT_BASE = '/api';

export async function fetchJson<T>(
  path: string,
  init: RequestInit = {},
  baseUrl: string = DEFAULT_BASE,
): Promise<T> {
  const url = path.startsWith('http') ? path : `${baseUrl}${path.startsWith('/') ? path : `/${path}`}`;
  // `[FACT]` Three header bugs meet here, and two of the obvious fixes
  // introduce new ones, so the shape of this code is load-bearing.
  //
  // 1. `...init` used to be spread *after* `headers`. A caller passing any
  //    `headers` key therefore replaced the whole merged object, so the
  //    `Accept` / `Content-Type` defaults were discarded and the inner
  //    `...(init.headers ?? {})` merge could never reach the wire. A
  //    caller that set one custom header silently stopped asking for JSON.
  //
  // 2. Moving `...init` first is not enough. `init.headers` is typed
  //    `HeadersInit`, so it may be a plain object, an entry array, or a
  //    `Headers` instance. A bare `{ ...init.headers }` expands a
  //    `Headers` instance to `{}` — its entries are not own enumerable
  //    properties — trading one silent header loss for another.
  //
  // 3. `Object.fromEntries(new Headers(...))` is *also* wrong, and it fails
  //    in a way that looks correct. `Headers` lower-cases every name, so
  //    the merged object ends up with both `'Accept'` (the default, set
  //    first) and `'accept'` (the caller's override) as separate keys.
  //    A lookup for `'Accept'` then hits the default and the caller's
  //    explicit value is silently ignored — the exact override semantic
  //    the merge is supposed to provide.
  //
  // `Headers.has()` is case-insensitive by spec, so seeding a `Headers`
  // from the caller and only filling in what is absent handles all three
  // `HeadersInit` forms, preserves the caller's casing, and lets a real
  // override win.
  const headers = new Headers(init.headers);
  if (!headers.has('Accept')) headers.set('Accept', 'application/json');
  if (init.body && !headers.has('Content-Type')) headers.set('Content-Type', 'application/json');
  const resp = await fetch(url, { ...init, headers });
  if (!resp.ok) {
    const text = await resp.text();
    throw {
      status: resp.status,
      message: text || resp.statusText,
    } satisfies ApiError;
  }
  if (resp.status === 204) {
    // @ts-expect-error -- callers handle the `null` shape they expect.
    return null;
  }
  return await resp.json() as T;
}
