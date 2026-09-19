import type { ApiErrorBody } from './types';

/**
 * Typed error envelope for every /api/* call. Carries the parsed
 * `code` (e.g. `unauthenticated`, `vault_error`) so the UI layer can
 * branch on stable identifiers instead of fragile strings.
 */
export class ApiError extends Error {
  readonly status: number;
  readonly code: string | null;
  readonly body: ApiErrorBody | null;

  constructor(status: number, message: string, code: string | null = null, body: ApiErrorBody | null = null) {
    super(message);
    this.name = 'ApiError';
    this.status = status;
    this.code = code;
    this.body = body;
  }

  /**
   * True when the server indicated the caller must re-authenticate.
   * The UI uses this to redirect to `/login` or surface a re-auth toast.
   */
  get isUnauthenticated(): boolean {
    return this.status === 401 || this.code === 'unauthenticated';
  }

  /** True when the request was malformed (validation failure on the server). */
  get isBadRequest(): boolean {
    return this.status === 400 || this.code === 'bad_request';
  }

  /** True when the upstream vault backend reported an error (502). */
  get isVaultError(): boolean {
    return this.status === 502 || this.code === 'vault_error';
  }

  /** True when the response was a network / 5xx level failure. */
  get isServer(): boolean {
    return this.status >= 500;
  }
}

/**
 * Network-level failure (DNS, refused, timeout). Distinct from
 * ApiError so the UI can decide between "retry" and "show details".
 */
export class NetworkError extends Error {
  readonly cause: unknown;
  constructor(message: string, cause: unknown) {
    super(message);
    this.name = 'NetworkError';
    this.cause = cause;
  }
}