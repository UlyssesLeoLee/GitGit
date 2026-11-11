# gm-console

Web admin UI for the **gitgit-server** RGS product (V0.1 commercial-grade
preview). Lists bare repositories, exposes the versioned credential vault,
and surfaces the clone URL for each repo.

This package sits under `apps/gm-console/` in the RGS repository. The
companion REST surface (`/api/*`) lives in `src/server/api.rs` on the
`feature/gm-console-v0.1` branch and ships together with this UI.

## Tech stack

- **Vite 5** + **React 18** + **TypeScript** (`strict`).
- **Tailwind CSS** for styling; **clsx** for class composition.
- **React Router v6** for client routing (lazy-loaded per route).
- **TanStack Query v5** for server-state caching.
- **Zustand** for cross-component UI state (theme, locale, toasts).
- **MSW (Mock Service Worker)** to allow demo without a live backend.
- **vitest** + **@testing-library/react** for unit tests.

## Local development

```sh
# from this directory
pnpm install         # or npm install / yarn install
pnpm dev             # start Vite on http://127.0.0.1:5173
```

`pnpm dev` starts MSW automatically (driven by `__ENABLE_MOCKS__`,
default `true` in `.env.development`). To run against a real
gitgit-server instead, point the dev proxy at it:

```sh
VITE_ENABLE_MOCKS=false \
VITE_API_PROXY_TARGET=http://127.0.0.1:8080 \
  pnpm dev
```

`gitgit-server` defaults to bind `127.0.0.1:8080`; the Vite proxy
forwards every request under `/api/*` to that origin.

## Build for production

```sh
pnpm build           # type-check + emit dist/
pnpm preview         # serve dist/ on http://127.0.0.1:4173
```

The static bundle in `dist/` is plain HTML/CSS/JS — deploy behind any
HTTP reverse proxy (envoy recommended per the V0.1 platform decision,
nginx only as a fallback when envoy is unavailable).

## Tests

```sh
pnpm test                  # vitest run
pnpm test:coverage         # with v8 coverage, threshold lines >= 70
pnpm typecheck             # tsc --noEmit
pnpm lint                  # eslint, max-warnings 0
pnpm format:check          # prettier --check
```

The unit suite covers the format helpers, i18n runtime, toasts store,
and the mock seed consistency. Component / e2e suites land in V0.2.

## Project layout

```
src/
  api/         axios client + per-endpoint helpers + DTO types
  components/  generic UI: Loading / Empty / Error / ErrorBoundary / Toast / ...
  routes/      page-level components, one per URL
  stores/      zustand stores (theme, locale, toasts)
  i18n/        zh-CN + en messages and the translator
  lib/         format / clipboard / query-client
  mocks/       MSW handlers + seed data (DEV only)
tests/unit/    vitest specs
```

## Routes

| Path | Component | Purpose |
| --- | --- | --- |
| `/` | `Home` | repo grid |
| `/repos/:name` | `RepoDetail` | refs + log + clone URL card |
| `/vault` | `Vault` | every user-visible key |
| `/vault/:key` | `VaultKeyDetail` | current value (masked) + timeline |
| `/vault/:key/diff?base=&head=` | `VaultDiff` | version-to-version diff |
| `/vault/:key/restore?target=` | `VaultRestore` | restore to a chosen version |
| `/settings` | `Settings` | backend health + `gitgit.password` rotation |
| `/404` | `NotFound` | catch-all |

## Environment variables

| Name | Default | Notes |
| --- | --- | --- |
| `VITE_API_BASE_URL` | `/api` | absolute URL prefix when not behind the Vite proxy |
| `VITE_API_PROXY_TARGET` | `http://127.0.0.1:8080` | dev proxy upstream |
| `VITE_ENABLE_MOCKS` | `true` (dev), `false` (prod) | toggles MSW |

## CI

See `.github-workflow-snippet.md` for a copy-pasteable GitHub Actions
workflow: install, typecheck, lint, test, build on every push. CI is
wired by the platform team — this file documents the expected pipeline.

## API contract gotchas

These are real differences between the wire shape the front-end types
advertise and what the back-end `VersionedVault` trait actually
implements today. They are not bugs but are noted here so the operator
isn't surprised:

| Endpoint | Field | Behaviour |
| --- | --- | --- |
| `POST /api/vault/keys/:key/versions` | `change_note` in body | Accepted by the API handler and stored in the request log, but the underlying `VersionedVault::set_with_version` trait does not thread the note into the sidecar `change_note` field of the new version entry. The audit trail today is the `created_at_unix_ms` of the new entry plus the success toast rendered by the UI. A follow-up commit can extend the trait without breaking the wire shape. |
| `POST /api/vault/keys/:key/versions` | empty `value` | Rejected with 400 + `code: "bad_request"`. |
| `POST /api/vault/keys/:key/restore` | `target_version < 1` | Rejected with 400 + `code: "bad_request"`. |

## Known gaps (per守门 #1 缺标比错标)

- Component tests / Playwright e2e land in V0.2.
- WebAuthn / OIDC for production `/api/*` (currently local-only).
- Server-side filtering of internal vault sidecars (`_attachments`,
  `_versions`) was added in `283302c`; the client does not duplicate
  that filter.
- The MSW seed is illustrative — values reset on every page reload.

## Authoring

Code style: TypeScript strict, ESLint 0 errors, Prettier defaults,
Tailwind utility-first. Every public component ships with a JSDoc
header. New translations go in both `i18n/zh-CN.ts` and `i18n/en.ts`
and are keyed by dotted path under the `Messages` type.

Author / approver / reviser follow the per-project代签 policy (see
the RGS repo's `AGENTS.md` for the canonical form).