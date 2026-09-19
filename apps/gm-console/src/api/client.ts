import axios, { AxiosError, type AxiosInstance, type AxiosRequestConfig } from 'axios';
import { ApiError, NetworkError } from './errors';
import type { ApiErrorBody } from './types';

/**
 * Build the absolute base URL the axios client should hit.
 *
 * - Production / preview: `import.meta.env.VITE_API_BASE_URL` (same
 *   origin when empty → relative URLs, useful behind a reverse proxy).
 * - Dev (Vite): rely on the proxy → `/api` (the proxy forwards to
 *   `VITE_API_PROXY_TARGET`).
 *
 * The caller is responsible for ensuring the trailing `/api` segment.
 */
export function resolveApiBaseUrl(): string {
  const explicit = import.meta.env.VITE_API_BASE_URL;
  if (explicit && explicit.length > 0) return trimTrailingSlash(explicit);
  return '/api';
}

function trimTrailingSlash(value: string): string {
  return value.endsWith('/') ? value.slice(0, -1) : value;
}

/**
 * Extract a typed `ApiError` from any thrown axios/network error.
 *
 * Behaviour:
 * - 2xx with malformed JSON → `ApiError(500, …)`.
 * - non-2xx with `{error, code}` body → `ApiError(status, message, code)`.
 * - non-2xx with HTML / empty body → `ApiError(status, "<status> <statusText>")`.
 * - no response (network/DNS/CORS) → `NetworkError`.
 */
export function toApiError(err: unknown): ApiError | NetworkError {
  if (err instanceof ApiError) return err;
  if (err instanceof NetworkError) return err;
  if (axios.isAxiosError(err)) {
    const ax = err as AxiosError<unknown>;
    if (!ax.response) {
      return new NetworkError(ax.message || 'network error', err);
    }
    const status = ax.response.status;
    const data = ax.response.data as unknown;
    if (data && typeof data === 'object' && 'error' in (data as Record<string, unknown>)) {
      const body = data as ApiErrorBody;
      const code = typeof body.code === 'string' ? body.code : null;
      return new ApiError(status, body.error || ax.message, code, body);
    }
    return new ApiError(status, `${status} ${ax.response.statusText || 'request failed'}`);
  }
  if (err instanceof Error) return new NetworkError(err.message, err);
  return new NetworkError('unknown error', err);
}

let instance: AxiosInstance | null = null;

/**
 * Lazily-built singleton axios client. The first call wires up the
 * base URL + interceptors; subsequent calls return the same instance.
 */
export function getClient(): AxiosInstance {
  if (instance) return instance;
  instance = axios.create({
    baseURL: resolveApiBaseUrl(),
    timeout: 15000,
    headers: { 'Content-Type': 'application/json', Accept: 'application/json' },
    // Don't auto-throw on non-2xx; we want to surface every error envelope
    // through `toApiError` consistently.
    validateStatus: () => true,
  });
  instance.interceptors.response.use(
    (resp) => resp,
    (err) => Promise.reject(toApiError(err)),
  );
  return instance;
}

/**
 * Apply a one-off config patch (mainly used by tests to inject an
 * adapter). Returns the client so callers can chain.
 */
export function configureClient(config: AxiosRequestConfig): AxiosInstance {
  const c = getClient();
  Object.assign(c.defaults, config);
  return c;
}

/** Reset the singleton; intended for tests. */
export function __resetClientForTests(): void {
  instance = null;
}