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
recorded so the state is not misrepresented. None of them is fixed yet.

### The HTTP API has no authentication, and the server binds all interfaces by
### default

`gitgit serve` defaults to `0.0.0.0:8080` (`src/config.rs`, `DEFAULT_BIND`). On
that router, `/api/*` is mounted with no authentication layer at all
(`src/server/api.rs`): the only auth helper in the crate, `auth_optional`, is a
no-op that is never called and is kept alive with `#[allow(dead_code)]`. The
single place real authentication is enforced is the git push path
(`require_basic` in `src/server/http.rs`), and that one credential pair is
hardcoded as `admin` / `admin` (`src/config.rs`).

`GET /api/vault/keys/:key` returns the stored credential value in plaintext
(`src/server/api.rs`, `get_vault_key`), and `DELETE /api/vault/keys/:key` plus
`POST /api/vault/keys/:key/restore` are equally unauthenticated.

**Impact:** on a default configuration, any host that can reach the machine on
that port can read, delete, and roll back stored API credentials, and can clone
any repository. The desktop shell adds a second path to the same outcome: the
bind string reaches `TcpListener::bind` unvalidated from the frontend
(`apps/gm-desktop/src-tauri/src/state.rs` and `commands/server.rs`), so the
WebView can request a non-loopback bind.

**Status: open, awaiting a decision on the fix shape.** The obvious fix — a
loopback default, enforced auth on `/api/*`, and credentials sourced from the
environment instead of compiled in — changes what local development and demos
can do without credentials, so it is not being changed unilaterally.

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

Note that the gate covers Rust dependencies only. The two frontends
(`apps/gm-desktop`, `apps/gm-console`) are audited separately and are not yet
covered by an equivalent CI gate.

## Reporting something that is not a vulnerability

Ordinary bugs belong in ordinary issues. The repository's own conventions for
that — including the `[FACT]` / `[UNVERIFIED-FACT]` / `[INFERENCE]` /
`[PROPOSAL]` / `[TBD]` tagging discipline and the ban on retrospective
narrative — are written up in [CONTRIBUTING.md](CONTRIBUTING.md).
