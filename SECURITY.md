# Security Policy

## Status of this project

**No release has been published.** `Cargo.toml` declares `version = "0.1.0"`, the
repository is pre-1.0, and the desktop application has not been signed or
shipped to any distribution channel. Everything below describes unreleased
code that only exists on `dev` and `main`.

That matters for how you should read the "Known issues" section: those are not
published vulnerabilities, they are defects found and recorded while the
software is still being built. They are listed here so they are not discovered
by someone else first.

## Supported versions

| Version | Supported |
| --- | --- |
| `dev` / `main` (unreleased, 0.1.0) | Yes |
| Any tagged release | None exist yet |

Security fixes land on `dev`. There is no backport target, because there is no
released version to backport to.

## Reporting a vulnerability

**There is no private reporting channel configured yet.** GitHub's private
vulnerability reporting is not enabled for this repository, and the project has
no security contact address. That is a known gap, recorded here rather than
papered over with an address that does not exist.

Until a channel exists, open a regular GitHub issue. If the report itself would
expose a credential or let a third party in, say so in the issue body and
describe the reproduction in a way that does not require publishing the
exploit; the maintainer can pull the details into a private conversation from
there.

Treat the absence of a private channel as a real limitation of this project, not
as permission to publish an unfixed exploit against unreleased software.

## Known issues in unreleased code

These were found by reading the code and by running the test suites, and are
recorded so the state is not misrepresented. Where a fix has since landed, the
entry says so and describes only what is still true.

### The HTTP API is authenticated, but the credential travels in cleartext

> **Fixed (2026-10-05).** `/api/*` now requires HTTP Basic auth, and the
> credential is no longer compiled into the binary. The section below has been
> rewritten to describe what is true now. Read the **residual risks** list
> rather than assuming the fix was complete.

**What changed.** `/api/*` is mounted behind an auth layer
(`src/server/api.rs`, `build_api_router` applies
`route_layer(middleware::from_fn_with_state(state, require_api_auth))` to the
API sub-router, not per handler — a per-handler check is a check the next
endpoint forgets). The no-op `auth_optional` helper and the hardcoded
`admin` / `admin` pair in `src/config.rs` are deleted; `cargo metadata`
reports no `admin` constant in the binary. The credential is resolved at
start-up in this order:

1. `GITGIT_ADMIN_PASS` / `GITGIT_ADMIN_USER` from the environment,
2. the vault key `gitgit.password` (what the desktop Settings page writes),
3. a generated 128-bit random password, printed once to the console by the
   CLI.

A **non-loopback bind with a generated password is refused at start-up** in
both the CLI (`src/main.rs`) and the desktop's embedded server
(`apps/gm-desktop/src-tauri/src/commands/server.rs`), both calling
`enforce_exposure_policy`. Widening the bind therefore requires an operator
to choose a password, rather than producing a service protected by one nobody
knows.

`GET /api/health` is deliberately **outside** the auth layer. It is a
documented liveness probe and returns only a version string and a boolean; a
probe that needed the admin password would be unable to report that the
server is healthy in exactly the situation where the password is the problem.

The web console (`apps/gm-console`) has a sign-in gate
(`src/components/LoginGate.tsx`) and sends the credential on every request via
an axios request interceptor. It holds the credential in memory only
(`src/stores/credentials.ts`): there is deliberately no `localStorage` arm and
no `VITE_*` variable, because both would put the secret in a file or inline it
into the JavaScript bundle that ships to every browser.

**Residual risks, stated plainly:**

- **No TLS.** Basic auth is base64, not encryption. Over a widened bind, any
  host that can observe the network path can read the credential directly.
  Terminating TLS in front of the server is the operator's responsibility and
  is not something this codebase provides.
- **A generated password is not recoverable from the desktop shell.** The CLI
  prints it once to the console; the desktop has no console, so it appears only
  in the log. This fails closed — the server refuses a non-loopback bind while
  the password is generated — but it means the web console cannot be used
  against a widened bind until a password is set in Settings.
- **The credential is cached for the process lifetime.** It is resolved once at
  start-up. Changing it in the desktop Settings page takes effect at the next
  server start, and the page says so rather than claiming an immediate effect.

The MinIO backend is dev/test-only and is not part of any release.

### The Windows installers are unsigned

All four MSI variants (per-user and per-machine, each in `en-US` and `zh-CN`)
are produced without a code-signing certificate. Windows SmartScreen will warn
on install, and a commercial release needs a certificate. This is a purchasing
and release-process gap, not a code defect.

## Dependency advisories

`cargo audit` runs as a hard gate in the `audit` job of the
`rust-backend` workflow, over `Cargo.lock`, with `cargo-audit` pinned to
0.22.2 so the rule set cannot drift between runs. The three advisories present
on 2026-10-05 (`quick-xml` 0.38.4 twice, `rustls` 0.23.43) were cleared the
same day by moving `rust-s3` to 0.38.0, `quick-xml` to 0.41.0, and `rustls` to
0.23.45.

Note that the gate covers Rust dependencies only. Measured on 2026-10-05, the two
frontends carry their own advisories and have no equivalent CI gate:

| Frontend | Advisories | Severity split |
| --- | --- | --- |
| `apps/gm-desktop` | 8 | 1 critical, 2 high, 5 moderate |
| `apps/gm-console` | 10 | 1 critical, 2 high, 7 moderate |

In `gm-desktop` every advisory sits in a devDependency chain — `vitest`,
`@vitest/coverage-v8`, `vite`, `esbuild`, and `braces` under `tailwindcss` —
and the production dependency list is only `@tauri-apps/api`, five Tauri
plugins, `felte` and `zod`. None of it ships in the packaged application, so
these are developer-machine risks (a hostile dev server, a Vite `fs.deny`
bypass on Windows) rather than end-user exposure. `gm-console` additionally
carries two `react-router@6.30.6` advisories that are in its runtime tree.

The fixes are all major-version moves — `vitest` 2 -> 3 or 4, `vite` 5 -> 6,
`react-router` 6 -> 7 — so each needs the frontend test suite re-verified
rather than a lockfile edit, and `braces` has no patched 3.x release at all.

## Reporting something that is not a vulnerability

Ordinary bugs belong in ordinary issues. The repository's own conventions for
that — including the `[FACT]` / `[UNVERIFIED-FACT]` / `[INFERENCE]` /
`[PROPOSAL]` / `[TBD]` tagging discipline and the ban on retrospective
narrative — are written up in [CONTRIBUTING.md](CONTRIBUTING.md).
