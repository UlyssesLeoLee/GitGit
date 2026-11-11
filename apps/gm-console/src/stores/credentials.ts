import { create } from 'zustand';

/**
 * The admin credential the console presents to `/api/*`.
 *
 * `[FACT]` `GET /api/*` requires HTTP Basic auth as of 2026-10-05
 * (`src/server/api.rs`, `build_api_router`). Before that the API was
 * open, so this store did not exist; `ApiError.isUnauthenticated` was
 * already in `api/errors.ts` with a comment describing a `/login`
 * redirect that had never been built.
 */
export interface AdminCredential {
  user: string;
  password: string;
}

interface CredentialState {
  credential: AdminCredential | null;
  /** Record a credential the user just typed. In-memory only — see below. */
  signIn: (next: AdminCredential) => void;
  /** Drop the credential; the next request goes out unauthenticated. */
  signOut: () => void;
}

/**
 * Admin credential store.
 *
 * `[FACT]` This store deliberately has **no** `localStorage` /
 * `sessionStorage` / IndexedDB arm, and that omission is the design, not
 * an oversight. The `theme` and `locale` stores persist on purpose: they
 * hold preferences. This one holds a secret.
 *
 * The server-side change that made this necessary removed a hardcoded
 * `admin` / `admin` pair from the compiled binary
 * (`src/config.rs`, `ADMIN_USER` / `ADMIN_PASS`, deleted). Persisting
 * the replacement in `localStorage` would move the same secret from the
 * binary into a file any script on the origin can read, and Vite's
 * `import.meta.env` was rejected for the same reason one layer over —
 * anything in a `VITE_*` variable is inlined into the built JavaScript
 * and ships to every browser that loads the page.
 *
 * The cost of this choice is that a page reload asks for the password
 * again. That is the intended trade: an operator-backed admin console
 * re-authenticating on reload is a small price for a secret that is not
 * sitting in a file.
 */
export const useCredentialsStore = create<CredentialState>((set) => ({
  credential: null,
  signIn: (next) => set({ credential: next }),
  signOut: () => set({ credential: null }),
}));

/**
 * Build the `Authorization` header value for a credential.
 *
 * `[FACT]` `btoa` is byte-oriented, not character-oriented: it throws
 * `InvalidCharacterError` on any code point above U+00FF. A password is
 * arbitrary user input, so a non-Latin-1 one (any CJK character, an
 * emoji, an accented Latin letter) would otherwise throw *inside the
 * request interceptor* and take down the request rather than
 * authenticating it. RFC 7617 encodes the credential as UTF-8 before
 * base64, so the bytes have to be converted the same way here.
 */
export function toBasicAuthHeader(credential: AdminCredential): string {
  const bytes = new TextEncoder().encode(`${credential.user}:${credential.password}`);
  let binary = '';
  for (const byte of bytes) binary += String.fromCharCode(byte);
  return `Basic ${btoa(binary)}`;
}

/** The header for the currently-held credential, or `null` if signed out. */
export function currentAuthHeader(): string | null {
  const { credential } = useCredentialsStore.getState();
  return credential ? toBasicAuthHeader(credential) : null;
}
