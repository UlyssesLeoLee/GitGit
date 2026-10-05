# Contributing to GitGit

## What this repository is

`gitgit` is a **single Rust crate**, not a workspace — `Cargo.toml` has no
`[workspace]` section by design. The archived
`docs_archive_rust_impl_2026_08_26/` tree describes a 14-crate design; that
design is historical reference material and is **not** a description of the code
in this repository. A sibling Tauri project (`apps/gm-desktop`) consumes the
crate as a path dependency.

Toolchain is pinned in `rust-toolchain.toml` to **1.98.1** with `rustfmt` and
`clippy`, deliberately, so a local `cargo fmt` / `cargo clippy` produces the
same verdict as CI. If you change that pin, change
`dtolnay/rust-toolchain` in `.github/workflows/rust-backend.yml` in the same
commit, or the two will disagree.

## Documentation that is authoritative

| Path | What it is |
| --- | --- |
| `docs/adr/` | Architecture decisions (ADR-0001, ADR-0020..0023) |
| `docs/plan/v0-tasks.md` | V0 task list, with per-task verified status |
| `docs/reports/<date>-<topic>/` | Dated verification reports |
| `.github/CI.md` | What CI does and does not cover, including known limits |

`README.md` is a programme-level document. It is not a feature reference and it
does not cover code or licensing, so do not treat it as a source of truth for
either.

## The rules that are not negotiable

**Tag every factual claim.** Use `[FACT]`, `[UNVERIFIED-FACT]`, `[INFERENCE]`,
`[PROPOSAL]`, `[TBD]`. A `[FACT]` you have not run is a fabrication, and it is
the single easiest way to damage this repository's credibility.

**No retrospective narrative.** Do not write "previously it was…", "originally…",
or "before the upgrade…". Describe the state as it is now, with the date the
measurement was taken.

**Report what a test actually did, including "nothing".** A filter or a target
selection that matches zero tests exits 0 and looks green. This repository has
been bitten by that more than once — `cargo test --bins` matching zero tests,
and a doc comment telling you to run the minIO e2e against the thin binary
target, where it matches nothing either. When you add a gate, assert that the
expected work actually happened, not just that the command succeeded.

**A number in `scripts/regression-baseline.json` is a contract, not a
measurement.** The UT tier compares passed/ignored counts per module, so adding
a legitimate test turns it red. Re-agree the numbers in the same commit that
adds the tests, and say what moved and why in the file's own `rationale` field.
Do not "fix" a red baseline by deleting or skipping tests.

**Do not weaken a gate to make it pass.** No lowering coverage thresholds, no
`istanbul ignore` / `v8 ignore` / `coverage.exclude`, no `@ts-ignore`, no
loosening a lint, no step-level `|| true`. A narrow `#[allow]` or
`// eslint-disable` with a comment stating the invariant is acceptable and has
precedent; a blanket suppression is not.

**Clippy is denied, not warned.** `unsafe_code = "forbid"`,
`unused_must_use = "deny"`, and `unwrap_used` / `expect_used` / `panic` are
denied at the crate level, which is why the code has small helpers like
`literal_regex` and `group` in `apps/gm-desktop/src-tauri/src/graph.rs` instead
of `Regex::new().unwrap()` at every call site.

## Running the gates locally

```text
cargo fmt --all -- --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --all-targets
cargo audit                       # pinned cargo-audit 0.22.2 in CI
```

The PowerShell tiers and end-to-end scripts, all of which are hard gates in CI:

```text
pwsh -NoProfile -File scripts/regression-ut.ps1    # count contract
pwsh -NoProfile -File scripts/regression-it.ps1    # HTTP-level /api/*
pwsh -NoProfile -File scripts/regression-st.ps1    # real git client over real HTTP
pwsh -NoProfile -File scripts/verify-gitai-e2e.ps1
pwsh -NoProfile -File scripts/verify-gitremote-e2e.ps1
```

The minIO credential-vault e2e needs a container runtime and is not in CI:

```text
pwsh -NoProfile -File scripts/verify-minio-vault.ps1
```

`cargo` must be on `PATH` before any of the above. If you use `rustup`, prepend
the toolchain's `bin` directory; a toolchain directory that lists but cannot be
entered is a broken install, and retrying it wastes time.

## Working in parallel

If several agents or shells are building at once, **give each one its own
`CARGO_TARGET_DIR`**. The default is shared machine-wide, and a shared target
directory makes one lane's verification depend on another's incremental state.

Give each lane its own git worktree and branch, one writer per file set, and
merge back to `dev` one lane at a time.

## Line endings

Line endings are load-bearing per file and are already mixed on purpose
(`README.md` is BOM + CRLF, `docs/plan/v0-tasks.md` is LF, the WiX templates are
CRLF, and most Rust sources are LF). Preserve what a file already has.

**Judge a formatting change with `git diff --numstat`, not with a byte-counting
script** — a PowerShell read/write pipeline rewrites line endings and will
report a rewrite that never happened, which is how a pure-formatting change
masquerades as a real one.

## Dependencies

New dependencies need a stated reason in the commit message. Enabling an extra
feature on an existing dependency is fine; adding a crate is not. `rust-s3` 0.37
was pinned deliberately (it does not expose `versionId` on `get_object`, which
is why `vault_versioned.rs` builds a presigned GET URL with a `versionId` query
for a real versioned restore) — read the comment in `Cargo.toml` before
bumping it, and re-run the minIO e2e afterwards, because that is the only thing
that exercises the S3 client against a real server rather than only compiling it.

## What is not done yet

`docs/plan/v0-tasks.md` carries the current per-task status, including the
tasks that are still `[TBD]`. Two of them are release blockers rather than
polish: the Windows installers are unsigned, and the HTTP API currently has no
authentication while the server binds all interfaces by default. See
[SECURITY.md](SECURITY.md) for the full description of the second one.
