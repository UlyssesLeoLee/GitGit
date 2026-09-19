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
  const resp = await fetch(url, {
    headers: {
      'Accept': 'application/json',
      ...(init.body ? { 'Content-Type': 'application/json' } : {}),
      ...(init.headers ?? {}),
    },
    ...init,
  });
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
