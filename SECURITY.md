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

### The frontends have a gate now

`[FACT]` Until 2026-10-24 the two frontends had **no** dependency-advisory CI
gate: nothing in any workflow ran `pnpm audit`. The `frontend-audit` workflow
closes that gap. It is a **ratchet**, not a hard gate — `pnpm audit` exits
non-zero today, so a gate that failed on everything already present would have
been red on its first run and would then have been switched off. That is not a
hypothetical: the Rust-side unit-test count contract sat unenforced for 19 days
for exactly this reason. The gate fails on any advisory that is **not** in
`scripts/frontend-advisory-baseline.json`, and it names it.

`[FACT]` The baseline is a **record of what is known, not an ignore list**.
Every entry in it carries the advisory id, the package, the measured version
and the patched range, and the checker re-verifies each entry against the npm
advisory database on every run — an entry the database does not corroborate is
rejected and fails the build. There is no `pnpm audit --ignore` in this
repository, no waiver list, and no `pnpm.overrides` entry pinning a vulnerable
version. The checker parses the audit's JSON rather than grepping its text.

`[FACT]` An advisory that has disappeared is reported as informational and does
not fail the build — a fixed advisory is good news. The three verdicts are
*new* (in the audit, absent from the baseline — fails and is named), *resolved*
(in the baseline, gone from the audit — informational) and *fabricated* (in the
baseline, but not in the npm advisory database — fails and is named).

### Measured advisory state

`[FACT]` Measured 2026-10-24 with `corepack pnpm@9.15.9 audit --json` over the
committed lockfiles:

| Frontend | Before this change | After | Severity split after |
| --- | --- | --- | --- |
| `apps/gm-desktop` | 9 | 0 | — |
| `apps/gm-console` | 17 | 15 | 3 critical, 2 high, 9 moderate, 1 low |

`[FACT]` The "before" column is the same command on the tree at `255ab92`:
`gm-desktop` 9 (0 critical, 3 high, 5 moderate, 1 low), `gm-console` 17
(3 critical, 3 high, 10 moderate, 1 low). The 2026-10-05 figures in earlier
revisions of this file (8 and 10) were stale, and the counts below are
measurements rather than carried-over numbers.

### Which of these reach a user

`[FACT]` All 15 remaining `gm-console` advisories sit in a devDependency
chain, and `gm-desktop` has none. This is established from the audit's own
dependency `paths` rather than from the `package.json` label: `pnpm audit
--json` reports the chain each advisory arrives through, and the first hop
after the project root names the direct dependency. For all 15 the first hop is
`vite`, `vitest` or `@vitest/coverage-v8`, all devDependencies.

`[FACT]` **`package.json` says what is declared; only an import says what
ships.** A source-level import scan of `apps/gm-desktop/src` and
`apps/gm-desktop/tests` finds that of the declared production dependencies only
`@tauri-apps/api` (`core`, `event`) and `@tauri-apps/plugin-dialog` are
imported anywhere; the other `@tauri-apps/plugin-*` packages are imported
nowhere. The same scan of `apps/gm-console/src` and `apps/gm-console/tests`
finds `zod`, `react-hook-form` and `@hookform/resolvers` imported nowhere.
Vite bundles what is imported, so none of those packages ever reached a
packaged artefact.

`[FACT]` **`zod` was declared as a production dependency of both frontends and
carried a HIGH advisory (GHSA-mrhx-c7xc-m3w5, memory exhaustion while
validating oversized collections), but it was imported nowhere in either app's
source or tests, so Vite never bundled it and it never reached a packaged
artefact.** It was removed on 2026-10-24 — the fix was deleting a dead
dependency, not a `zod` 3 → 4 migration. The same removal took
`react-hook-form` and `@hookform/resolvers` in `gm-console` and `felte` in
`gm-desktop`; those carry no advisory, so their removal is hygiene rather than
a security fix.

`[FACT]` `gm-console`'s `react-router` is `7.18.4` (the `6.30.6` advisories
are gone — the 6 → 7 upgrade was done before this change), and it carries no
advisory.

### What was closed, and what still blocks

`[FACT]` Closed in `gm-desktop`: `vite` 7.3.6 → 7.3.7 (1 high, 4 moderate,
1 low), `yaml` 2.8.0 → 2.8.3 (1 moderate), `source-map-js` 1.2.1 → 1.2.2
(1 high), `postcss-selector-parser` 6.1.4 → 7.1.7 (1 moderate), and `zod`
removed (1 high). Each was backed by a green test suite — `gm-desktop` runs
35 test files / 533 tests.

`[FACT]` `postcss-selector-parser` needed a `pnpm.overrides` pin to reach its
patched range: `tailwindcss@3` declares `^6.0.10`, and every 6.x release is
inside the advisory's vulnerable range (`<7.1.6`), so no in-range version
clears it. The pin moves the package to `7.1.7`, which is inside the
advisory's **own patched range** (`>=7.1.6`). It is therefore not a
suppression — `pnpm audit` reports no finding because the version is genuinely
fixed, not because anything was ignored. It is a forced major substitution
(6.x → 7.x) under a parent that declares `^6.0.10`, so it was verified rather
than assumed: the generated CSS is byte-identical before and after (same
SHA-256), the build succeeds, and the test suite stays green.

`[FACT]` `braces` is no longer an open advisory. `GHSA-vfj7-8cjw-p6xm` — the
one this file previously cited as "no patched 3.x release at all" — was
**withdrawn on 2026-10-23** (published 2026-09-18). The advisory that still
exists, `GHSA-grv7-fg5c-xmjg`, applies to versions below 3.0.3, and this
repository pins `braces` 3.0.3, so it does not apply. The tailwind 3 → 4
migration that was justified by `braces` is therefore no longer required by any
advisory; it may still be wanted on other grounds.

`[FACT]` The 15 remaining `gm-console` advisories all need a major-version
move that this change deliberately did not make: `vitest` 2.1.9 → 3.2.6 or
4.1.11 (1 critical, 1 moderate), `tinypool` 1.1.1 → 2.1.2 (2 critical),
`vite` 5.4.21 → 6.4.4 (2 high, 5 moderate, 1 low), `@vitest/mocker` 2.1.9 →
4.1.11 or 4.1.12 (2 moderate) and `esbuild` 0.21.5 → 0.25.0 (1 moderate).
Each of those is the last release of its line, so no patch-level bump exists:
`vite` 5.4.21, `vitest` 2.1.9, `@vitest/coverage-v8` 2.1.9, `@vitest/mocker`
2.1.9, `tinypool` 1.1.1 and `esbuild` 0.21.5 are all the newest of their major
versions. Moving them is the toolchain upgrade the runbook at
`docs/plan/gm-console-toolchain-upgrade.md` describes, and it needs the full
test suite re-verified rather than a lockfile edit.

## Reporting something that is not a vulnerability

Ordinary bugs belong in ordinary issues. The repository's own conventions for
that — including the `[FACT]` / `[UNVERIFIED-FACT]` / `[INFERENCE]` /
`[PROPOSAL]` / `[TBD]` tagging discipline and the ban on retrospective
narrative — are written up in [CONTRIBUTING.md](CONTRIBUTING.md).
