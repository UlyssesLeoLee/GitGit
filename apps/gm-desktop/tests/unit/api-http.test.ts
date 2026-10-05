/**
 * Tests for the REST client in `src/lib/api/http.ts`.
 *
 * `http.ts` was 0% covered. It is a thin wrapper, so the parts that
 * matter are not "does it call fetch" but the two things a thin
 * wrapper gets wrong: the URL it builds, and how it turns a
 * non-2xx into a throwable. Both are pinned here, along with the
 * 204 case — a 204 has no body, so a client that calls `.json()`
 * unconditionally turns every successful delete into a crash.
 *
 * `fetch` is stubbed with a hand-rolled Response because the wrapper
 * only reads four fields; a real `Response` would hide whether
 * `.json()` was actually called, which is one of the assertions.
 */
import { afterEach, describe, expect, it, vi } from 'vitest';
import { fetchJson } from '../../src/lib/api/http';

interface StubResponse {
  ok: boolean;
  status: number;
  statusText: string;
  text: () => Promise<string>;
  json: () => Promise<unknown>;
}

/**
 * A Response stand-in. `json` is a spy so a case can assert it was
 * *not* called, which is the whole point of the 204 case.
 */
function stubResponse(opts: {
  status: number;
  ok?: boolean;
  statusText?: string;
  textBody?: string;
  jsonBody?: unknown;
}): { response: StubResponse; json: ReturnType<typeof vi.fn>; text: ReturnType<typeof vi.fn> } {
  const text = vi.fn(async () => opts.textBody ?? '');
  const json = vi.fn(async () => {
    if (opts.jsonBody === undefined) throw new SyntaxError('Unexpected end of JSON input');
    return opts.jsonBody;
  });
  return {
    response: {
      ok: opts.ok ?? (opts.status >= 200 && opts.status < 300),
      status: opts.status,
      statusText: opts.statusText ?? '',
      text,
      json,
    },
    json,
    text,
  };
}

/** Install the stub and return the mock so a case can read the call args. */
function stubFetch(response: StubResponse): ReturnType<typeof vi.fn> {
  const fetchMock = vi.fn(async () => response);
  vi.stubGlobal('fetch', fetchMock);
  return fetchMock;
}

afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

describe('api / http — request shape', () => {
  it('builds a relative path against the default base', async () => {
    const { response } = stubResponse({ status: 200, jsonBody: { ok: true } });
    const fetchMock = stubFetch(response);

    await fetchJson('repos');

    // A path with no leading slash still gets exactly one separator.
    expect(fetchMock.mock.calls[0]?.[0]).toBe('/api/repos');
  });

  it('does not double the slash when the path already has one', async () => {
    const { response } = stubResponse({ status: 200, jsonBody: [] });
    const fetchMock = stubFetch(response);

    await fetchJson('/repos/alpha');

    expect(fetchMock.mock.calls[0]?.[0]).toBe('/api/repos/alpha');
  });

  it('bypasses the base for an absolute URL', async () => {
    // The embedded server hands out absolute clone URLs; prefixing
    // `/api` onto one would produce a path that resolves nowhere.
    const { response } = stubResponse({ status: 200, jsonBody: {} });
    const fetchMock = stubFetch(response);

    await fetchJson('http://127.0.0.1:38080/api/repos');

    expect(fetchMock.mock.calls[0]?.[0]).toBe('http://127.0.0.1:38080/api/repos');
  });

  it('honours an explicit base url', async () => {
    const { response } = stubResponse({ status: 200, jsonBody: {} });
    const fetchMock = stubFetch(response);

    await fetchJson('/repos', {}, 'http://127.0.0.1:38080/api');

    expect(fetchMock.mock.calls[0]?.[0]).toBe('http://127.0.0.1:38080/api/repos');
  });

  it('asks for JSON on every call', async () => {
    const { response } = stubResponse({ status: 200, jsonBody: {} });
    const fetchMock = stubFetch(response);

    await fetchJson('/repos');

    const init = fetchMock.mock.calls[0]?.[1] as RequestInit;
    // `[FACT]` `fetchJson` hands `fetch` a `Headers` instance rather than
    // a plain object, so the assertions in this file read through
    // `.get()`. That is deliberate: see the header-merge note in
    // `src/lib/api/http.ts` for why a plain-object merge cannot be made
    // correct here.
    const headers = init.headers as Headers;
    expect(headers.get('Accept')).toBe('application/json');
  });

  it('adds a JSON content type only when there is a body', async () => {
    // A GET with a content type is harmless but wrong; a POST without
    // one is refused by most servers. The branch keys off `init.body`,
    // so both halves matter.
    const withBody = stubResponse({ status: 200, jsonBody: {} });
    const m1 = stubFetch(withBody.response);
    await fetchJson('/repos', { method: 'POST', body: '{"name":"alpha"}' });
    const h1 = (m1.mock.calls[0]?.[1] as RequestInit).headers as Headers;
    expect(h1.get('Content-Type')).toBe('application/json');

    const noBody = stubResponse({ status: 200, jsonBody: {} });
    const m2 = stubFetch(noBody.response);
    await fetchJson('/repos', { method: 'GET' });
    const h2 = (m2.mock.calls[0]?.[1] as RequestInit).headers as Headers;
    expect(h2.get('Content-Type')).toBeNull();
  });

  it('forwards the caller method and body to fetch', async () => {
    const { response } = stubResponse({ status: 200, jsonBody: {} });
    const fetchMock = stubFetch(response);

    await fetchJson('/repos', { method: 'POST', body: '{"name":"alpha"}' });

    const init = fetchMock.mock.calls[0]?.[1] as RequestInit;
    expect(init.method).toBe('POST');
    expect(init.body).toBe('{"name":"alpha"}');
  });

  it('keeps the default Accept when the caller supplies its own headers', async () => {
    // `[FACT]` This case originally asserted the *opposite*, and the
    // assertion was correct at the time. `http.ts` built the request as
    // `{ headers: { Accept, ...ContentType, ...init.headers }, ...init }`
    // — the `...init` spread came last, so a caller passing a `headers`
    // key replaced the whole merged object and the defaults were
    // discarded. The inner `...(init.headers ?? {})` merge was dead code
    // whenever it could matter, and a caller that set one custom header
    // silently stopped asking for JSON.
    //
    // `http.ts` is fixed: `...init` is spread first and `init.headers` is
    // normalised through `Headers` before merging. Both halves matter —
    // moving the spread alone would have replaced a plain-object header
    // loss with a `Headers`-instance header loss. This case now pins the
    // merged result, and the next case pins the `Headers` form that the
    // normalisation exists for.
    const { response } = stubResponse({ status: 200, jsonBody: {} });
    const fetchMock = stubFetch(response);

    await fetchJson('/repos', { headers: { 'X-Trace': 'abc' } });

    const init = fetchMock.mock.calls[0]?.[1] as RequestInit;
    const headers = init.headers as Headers;
    expect(headers.get('Accept')).toBe('application/json');
    expect(headers.get('X-Trace')).toBe('abc');
  });

  it('keeps the default Accept when the caller passes a Headers instance', async () => {
    // `[FACT]` `HeadersInit` also admits a `Headers` instance, whose
    // entries are not own enumerable properties — so a bare
    // `{ ...init.headers }` expands it to `{}` and loses the caller's
    // header *and* the default. `fetchJson` normalises through `Headers`
    // for exactly this case. Measured in jsdom, where `Headers` iterates.
    const { response } = stubResponse({ status: 200, jsonBody: {} });
    const fetchMock = stubFetch(response);

    await fetchJson('/repos', { headers: new Headers({ 'X-Trace': 'abc' }) });

    const init = fetchMock.mock.calls[0]?.[1] as RequestInit;
    const headers = init.headers as Headers;
    expect(headers.get('Accept')).toBe('application/json');
    expect(headers.get('X-Trace')).toBe('abc');
  });

  it('lets a caller override the default Accept', async () => {
    // The merge is caller-first by design, so an explicit caller value
    // still wins. This distinguishes a correct merge from one that
    // unconditionally forces the default. It is also the case that
    // catches the case-normalisation trap: a plain-object merge built as
    // `{ 'Accept': default, ...caller }` would carry both `'Accept'` and
    // `'accept'` as separate keys, and a lookup for `'Accept'` would hit
    // the default and ignore the caller entirely.
    const { response } = stubResponse({ status: 200, jsonBody: {} });
    const fetchMock = stubFetch(response);

    await fetchJson('/repos', { headers: { 'Accept': 'text/plain' } });

    const init = fetchMock.mock.calls[0]?.[1] as RequestInit;
    const headers = init.headers as Headers;
    expect(headers.get('Accept')).toBe('text/plain');
  });
});

describe('api / http — responses', () => {
  it('returns the parsed body of a 200', async () => {
    const { response } = stubResponse({
      status: 200,
      jsonBody: { name: 'alpha', default_branch: 'main' },
    });
    stubFetch(response);

    await expect(fetchJson<{ name: string }>('/repos/alpha')).resolves.toEqual({
      name: 'alpha',
      default_branch: 'main',
    });
  });

  it('returns null for a 204 without reading a body', async () => {
    // A 204 has no body at all. Calling `.json()` on it rejects with
    // `Unexpected end of JSON input`, which would turn every
    // successful delete into a reported failure. The `json` spy is
    // what proves the wrapper skipped it.
    const { response, json } = stubResponse({ status: 204, ok: true });
    stubFetch(response);

    const out = await fetchJson('/repos/alpha');

    expect(out).toBeNull();
    expect(json).not.toHaveBeenCalled();
  });

  it('throws the status and the body text for a non-2xx', async () => {
    // The thrown value is a bare object, not an `Error`: callers match
    // on `.status`, and the message is the server's own text rather
    // than a generic "request failed".
    const { response } = stubResponse({ status: 500, textBody: 'repos table is corrupt' });
    stubFetch(response);

    await expect(fetchJson('/repos')).rejects.toMatchObject({
      status: 500,
      message: 'repos table is corrupt',
    });
  });

  it('falls back to statusText when the error body is empty', async () => {
    // An empty body is the common case for a 502 from a proxy, and
    // `'' || 'Bad Gateway'` is what keeps the message from being blank.
    const { response } = stubResponse({ status: 502, textBody: '', statusText: 'Bad Gateway' });
    stubFetch(response);

    await expect(fetchJson('/repos')).rejects.toMatchObject({
      status: 502,
      message: 'Bad Gateway',
    });
  });

  it('reports a 404 with its own status', async () => {
    const { response } = stubResponse({ status: 404, textBody: 'no such repo', statusText: 'Not Found' });
    stubFetch(response);

    await expect(fetchJson('/repos/nope')).rejects.toMatchObject({
      status: 404,
      message: 'no such repo',
    });
  });

  it('propagates a transport failure as-is', async () => {
    // `fetch` rejecting means the request never got a response; the
    // wrapper must not pretend it was a status error.
    const boom = new TypeError('Failed to fetch');
    vi.stubGlobal('fetch', vi.fn(async () => { throw boom; }));

    await expect(fetchJson('/repos')).rejects.toBe(boom);
  });
});
