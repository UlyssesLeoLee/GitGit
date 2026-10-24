# GitGit Regression Test Report — UT tier

**Scope**: UT (unit-test) tier only — `scripts/regression-ut.ps1`.
**Status**: **Implementation green, contract stale.** Every test passes; the tier exits `1` because the committed count contract (`scripts/regression-baseline.json`) predates 28 legitimately-added tests.
**Branch**: `test/regression-ut` @ `3a9bbdc` (base `dev` @ `3a9bbdc41041129dc1a42c6779ab7ad9716d8cee`)
**Baseline contract**: `scripts/regression-baseline.json` — last re-agreed at `f690486` (2026-10-05 11:21 JST); `capturedFrom` still names `origin/dev @ 151957d`.
**Date**: 2026-10-21 JST

---

## 1. Scope

This report covers **only** the UT lane of a 3-lane parallel regression sweep
(UT / IT / ST). The IT and ST lanes ran concurrently in sibling worktrees and
are reported elsewhere; nothing in this document describes their results.

The UT tier is the fastest gate in the rig and the one that carries the
**count contract**: `scripts/regression-baseline.json` holds the known-good
number of unit tests per module, and the tier asserts that today's run matches
it exactly.

What the tier does:

1. Runs `cargo test` over the single `gitgit` crate at the repo root, with
   stdout/stderr captured to `target/regression-logs/ut-stdout.log` and
   `ut-stderr.log`.
2. Parses the captured stdout for `test result: ok|FAILED` summary lines and
   per-test `test <path> ... ok|FAILED|ignored` lines.
3. Buckets the per-test lines into modules by the **first two `::`-separated
   path components** (so `server::api::tests::foo` → `server::api`).
4. Asserts the totals and every per-module count against the baseline.
5. Writes a JSON report to `target/regression-logs/ut-<timestamp>.json`.

Exit-code contract: `0` = all pass and counts match; `1` = test failure **or**
count drift; `2` = setup error.

---

## 2. Exact command

Run from the root of the worktree `D:\GitGit\.worktrees\reg-ut`:

```powershell
$env:PATH = 'E:\DevCache\cargo\bin;' + $env:PATH
$env:CARGO_BUILD_JOBS = '2'
Set-Location D:\GitGit\.worktrees\reg-ut
pwsh -NoProfile -File scripts/regression-ut.ps1
```

The tier itself invokes cargo as (from `scripts/regression-ut.ps1:83`):

```
cargo test --color=never --no-fail-fast --offline
```

with `CARGO_TARGET_DIR` forced by the script to the per-worktree
`D:\GitGit\.worktrees\reg-ut\target-regression` (`scripts/regression-ut.ps1:62-68`).

**Pre-warm (not part of the tier).** The lane's target dir was cold, and a
cold build of this dependency tree (tokio `full`, axum, reqwest+rustls,
rust-s3) takes far longer than the script's internal 600 s timeout
(`scripts/regression-ut.ps1:25`). The cache was therefore warmed once, with
the same target dir and the same cargo flags, before the tier was run:

```
cargo test --color=never --no-fail-fast --offline --no-run
```

That pre-warm is **build-only**: it compiles and links the test binaries and
runs **no** test. Its exit code was `0` and it took `17m05s` cold. Every
number in this report comes from the tier run that followed, whose own cargo
invocation re-did the (now cached) build in `1.59s` and executed the tests.
No test result in this report originates from the pre-warm.

---

## 3. Environment

| Item | Value |
|------|-------|
| OS | Microsoft Windows 11 Enterprise, 10.0.26300 (build 26300), x64 |
| Shell | PowerShell 7.6.6 (Core) driving the tier; GNU bash 5.2.15(1)-release (x86_64-pc-msys) used by the script for cargo output redirection |
| Rust toolchain | `cargo 1.98.1 (797e8a9bc 2026-08-05)`, `rustc 1.98.1 (48a229cea 2026-09-01)` |
| Toolchain pin | `rust-toolchain.toml` → `channel = "1.98.1"` (matches the baseline's `capturedFrom.rustToolchain`) |
| `CARGO_HOME` / `RUSTUP_HOME` | `E:\DevCache\cargo` / `E:\DevCache\rustup` |
| `CARGO_TARGET_DIR` | `D:\GitGit\.worktrees\reg-ut\target-regression` (per-lane; set by the script) |
| `CARGO_BUILD_JOBS` | `2` (the box has ~2.4 GB free RAM; cargo's default `-j 24` exhausts it) |
| Crate under test | `gitgit` v0.1.0 — one crate, one `[lib]` target (`src/lib.rs`) + one `[[bin]]` (`src/main.rs`) |
| HEAD under test | `3a9bbdc41041129dc1a42c6779ab7ad9716d8cee` (`Merge branch 'test/gm-console-act-warnings' into dev`) |

---

## 4. Results

### 4.1 Tier outcome

| Metric | Value |
|--------|-------|
| Tier script exit code | **`1`** (count drift) |
| `cargo test` exit code | **`0`** (all targets succeeded) |
| Tier regression cases | **pass = 20, fail = 5, skip = 0, total = 25** |
| JSON report | `target/regression-logs/ut-20261021-074615.json` (copied to `ut-report.json` in this directory) |

### 4.2 Test outcome (verbatim)

cargo's own summary lines, copied character-for-character from
`target/regression-logs/ut-stdout.log`:

```
test result: ok. 214 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 48.84s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
```

The three lines are the three compiled targets cargo ran: the `[lib]` unit-test
binary (214 tests), the `[[bin]]` test harness (`src/main.rs`, no tests), and
the doc-test target (no tests).

| Result | Count |
|--------|-------|
| Passed | **214** |
| Failed | **0** |
| Skipped (`#[ignore]`) | **2** |
| **Total** | **216** |

The tier's own parsed totals agree with cargo's summary line exactly:
`passed / failed : 214 / 0`, `ignored : 2`, and a sum-of-modules of `216`.

> Note on a false positive to avoid: `Select-String 'FAILED'` over the stdout
> log returns **3** hits, but all three are the word *"failed"* inside the
> `test result:` summary lines above. There are **zero** failed tests.

### 4.3 The two ignored tests

Both remain deliberately `#[ignore]`d and require a live minIO server. They
were neither un-ignored nor run:

| Test | Source | Cargo output |
|------|--------|--------------|
| `server::vault::tests::minio_vault_e2e_roundtrip` | `src/server/vault.rs:994` — `#[ignore = "requires a running minIO server (see comment in test)"]` | `... ignored, requires a running minIO server (see comment in test)` |
| `server::vault_versioned::tests::minio_vault_versioned_e2e_roundtrip` | `src/server/vault_versioned.rs:1229` — `#[ignore = "requires a real minIO server with bucket versioning enabled (see comment)"]` | `... ignored, requires a real minIO server with bucket versioning enabled (see comment)` |

These are exercised instead by `scripts/verify-minio-vault.ps1`, which is
outside this lane.

---

## 5. Per-case failures and root cause

All **5** failing cases are the *same* finding — the count contract is stale.
None of them indicates a failing test.

| # | Regression case | Expected | Observed | Root cause |
|---|-----------------|----------|----------|------------|
| 1 | `ut.total.passed.matches_baseline` | `186` | `214` | 28 tests added to the crate since the contract was last re-agreed |
| 2 | `ut.total_count.matches_sum_of_modules` | `188` | `216` | same 28 tests; the contract's `expectedTotal` no longer equals the sum of real modules |
| 3 | `ut.module.server::api.count_matches_baseline` | `17` | `25` | `1621bc5` added 8 `#[test]` fns to `src/server/api.rs` |
| 4 | `ut.module.server::auth.count_matches_baseline` | `4` | `15` | `1621bc5` added 11 net `#[test]` fns to `src/server/auth.rs` (4 at baseline → 15 today) |
| 5 | `ut.module.server::http.count_matches_baseline` | `7` | `9` | `1621bc5` added 2 `#[test]` fns to `src/server/http.rs` |

**Root cause, with the evidence.** The two commits that moved the counts are
found by `git log -S '<test symbol>' -- <file>`, not by reading test names:

```
git log --oneline -S'every_api_route_except_health_requires_authentication' -- src/server/api.rs
  1621bc5 fix(security): /api requires auth, the credential is external, and the console can sign in (#19)

git log --oneline -S'non_loopback_bind_with_a_generated_password_is_refused' -- src/server/auth.rs
  1621bc5 fix(security): /api requires auth, the credential is external, and the console can sign in (#19)

git log --oneline -S'receive_pack_accepts_the_configured_credential' -- src/server/http.rs
  1621bc5 fix(security): /api requires auth, the credential is external, and the console can sign in (#19)

git log --oneline -S'serve_defaults_to_loopback_rather_than_every_interface' -- src/cli.rs
  e2553c5 fix(security): the CLI no longer exposes itself to the network by default

git log --oneline -S'no_compiled_source_hard_codes_the_admin_password' -- src/config.rs
  1621bc5 fix(security): /api requires auth, the credential is external, and the console can sign in (#19)
```

A cross-check by diffing the `#[test]`-annotated function sets confirms the
same attribution numerically — at the last re-agreement point `f690486` the
counts were exactly the baseline's, and at HEAD they are exactly today's:

| File | `#[test]` fns at `f690486` | `#[test]` fns at HEAD | Baseline `perModule` | Observed |
|------|--------------------------|-----------------------|----------------------|----------|
| `src/server/api.rs` | 17 | 25 | `server::api` = 17 | `server::api` = 25 |
| `src/server/auth.rs` | 4 | 15 | `server::auth` = 4 | `server::auth` = 15 |
| `src/server/http.rs` | 7 | 9 | `server::http` = 7 | `server::http` = 9 |
| `src/cli.rs` | 0 | 3 | *(absent)* | `cli::tests` = 3 |
| `src/config.rs` | 0 | 4 | *(absent)* | `config::tests` = 4 |

**Verdict: the implementation is green, the contract is stale.** No defect was
found in `src/`, so **no code fix was committed** in this lane.

---

## 6. Baseline observations

### 6.1 Expected vs observed

| Baseline key | Expected | Observed | Delta |
|--------------|----------|----------|-------|
| `ut.expectedTotal` | 188 | 216 | **+28** |
| `ut.expectedPassed` | 186 | 214 | **+28** |
| `ut.expectedFailed` | 0 | 0 | 0 |
| `ut.expectedIgnored` | 2 | 2 | 0 |

`expectedFailed = 0` and `expectedIgnored = 2` both still hold — no test
regressed and no `#[ignore]` was removed.

### 6.2 Per-module, expected vs observed

| Module | Baseline | Observed | Delta | Attributed to |
|--------|----------|----------|-------|---------------|
| `ai::anthropic` | 3 | 3 | 0 | — |
| `ai::diff` | 6 | 6 | 0 | — |
| `ai::openai` | 5 | 5 | 0 | — |
| `ai::prompt` | 3 | 3 | 0 | — |
| `ai::registry` | 6 | 6 | 0 | — |
| `ai::sanitize` | 20 | 20 | 0 | — |
| `ai::stream` | 11 | 11 | 0 | — |
| `ai::tests` | 3 | 3 | 0 | — |
| **`cli::tests`** | *(absent)* | **3** | **+3 (new module)** | `e2553c5` |
| **`config::tests`** | *(absent)* | **4** | **+4 (new module)** | `1621bc5` |
| `remote::sync` | 5 | 5 | 0 | — |
| `remote::tests` | 15 | 15 | 0 | — |
| `repo::refs` | 7 | 7 | 0 | — |
| `repo::status` | 25 | 25 | 0 | — |
| `repo::store` | 6 | 6 | 0 | — |
| **`server::api`** | 17 | **25** | **+8** | `1621bc5` |
| **`server::auth`** | 4 | **15** | **+11** | `1621bc5` |
| **`server::http`** | 7 | **9** | **+2** | `1621bc5` |
| `server::smart` | 4 | 4 | 0 | — |
| `server::subprocess` | 5 | 5 | 0 | — |
| `server::vault` | 17 | 17 | 0 | — |
| `server::vault_versioned` | 19 | 19 | 0 | — |
| **Sum** | **188** | **216** | **+28** | |

`server::vault` (17) and `server::vault_versioned` (19) still match the
baseline even though only 16 and 18 of them respectively *pass*: the tier's
per-module counter counts `ok` + `FAILED` + `ignored`, and each of those two
modules holds one `#[ignore]`d minIO test. The baseline's own `rationale`
documents this convention ("Per-module values are passed+ignored counts as
the tier reports them"), and it is applied consistently below.

### 6.3 Proposed baseline delta (for the integrator — **not applied here**)

`scripts/regression-baseline.json` was **not** modified by this lane: it is a
shared contract owned by the parent integrator, and three lanes ran in
parallel. The proposed change, for the integrator to apply:

```jsonc
"ut": {
  "expectedTotal": 216,        // was 188
  "expectedPassed": 214,       // was 186
  "expectedFailed": 0,         // unchanged
  "expectedIgnored": 2,        // unchanged
  "perModule": {
    "ai::anthropic": 3,
    "ai::diff": 6,
    "ai::openai": 5,
    "ai::prompt": 3,
    "ai::registry": 6,
    "ai::sanitize": 20,
    "ai::stream": 11,
    "ai::tests": 3,
    "cli::tests": 3,           // NEW — was absent from the map
    "config::tests": 4,        // NEW — was absent from the map
    "remote::sync": 5,
    "remote::tests": 15,
    "repo::refs": 7,
    "repo::status": 25,
    "repo::store": 6,
    "server::api": 25,         // was 17
    "server::auth": 15,        // was 4
    "server::http": 9,         // was 7
    "server::smart": 4,
    "server::subprocess": 5,
    "server::vault": 17,
    "server::vault_versioned": 19
  }
}
```

The `perModule` values sum to exactly `216`, which equals
`expectedPassed + expectedIgnored` (`214 + 2`) and equals the sum of the
tier's own observed module totals. All 22 values were re-derived from
cargo's own captured per-test output in `ut-stdout.log`, using the tier's own
bucketing rule — **not** hand-counted with `grep` over the source.

Two further metadata corrections belong in the same edit:

| Field | Current | Proposed | Why |
|-------|---------|----------|-----|
| `capturedAt` | `"2026-09-20"` | `"2026-10-21"` | the numbers are being re-agreed now |
| `capturedFrom.head` | `"151957d"` | `"3a9bbdc"` | `151957d` is 211 commits behind HEAD; the contract has in fact tracked `f690486` since 2026-10-05 |
| `capturedFrom.branch` | `"origin/dev"` | `"dev"` | the number was measured from a local worktree of `dev`, not from `origin/dev` |
| `ignoredTest` | `"... server::vault_versioned::tests::minio_vault_versioned_e2e (both require ...)"` | `"... server::vault_versioned::tests::minio_vault_versioned_e2e_roundtrip (both require ...)"` | the real test name carries a `_roundtrip` suffix (`src/server/vault_versioned.rs:1229`); the baseline names a test that does not exist |
| `ut.command` | `"cargo test --quiet"` | `"cargo test --color=never --no-fail-fast --offline"` | the tier actually runs the latter (`scripts/regression-ut.ps1:83`); `--quiet` also suppresses the per-test lines the tier's per-module parser depends on |

---

## 7. What changed and why

### 7.1 Why the contract went stale

The baseline was **last re-agreed at `f690486` on 2026-10-05 at 11:21 JST**.
Two security commits landed on the *same day*, afterwards:

| Time (JST) | Commit | Subject |
|-------------|--------|---------|
| 2026-10-05 16:16:57 | `e2553c5` | `fix(security): the CLI no longer exposes itself to the network by default` |
| 2026-10-05 19:41:15 | `1621bc5` | `fix(security): /api requires auth, the credential is external, and the console can sign in (#19)` |

These are the **only** two commits that touched `src/` between `f690486` and
`HEAD` (`git log --oneline f690486..HEAD -- src/` returns exactly these two).
Both added authentication tests, which is why the drift is concentrated in
`server::auth`, `server::api`, `server::http`, and the two brand-new modules
`cli::tests` and `config::tests`.

In total `dev` advanced 96 commits past `f690486` (211 past the
`capturedFrom` commit `151957d`), but only those two moved the test counts.

### 7.2 Why no fix was committed

Rule 4 of this lane requires a fix to be proven by the tier's own output. The
tier's output proves a **contract** problem, not an implementation problem:
all 216 tests pass, cargo exits `0`, and the 5 failing assertions are all
count comparisons. Editing the implementation to make a count contract agree
would be wrong, and editing the contract is explicitly out of scope for this
lane. So this lane commits **report only**.

### 7.3 Risk: the tier cannot see a brand-new module

`scripts/regression-ut.ps1:223` iterates over the **baseline's** `perModule`
keys only:

```powershell
foreach ($prop in $perModule.PSObject.Properties) { ... }
```

A module that appears for the first time — such as `cli::tests` and
`config::tests` here — is therefore **never asserted on**, and its tests are
absorbed into the total with no per-module attribution. The gate is not blind
(the two total assertions still catch the drift and the tier still exits `1`),
but the failure message does not say *which* new module appeared. Adding the
two new modules to `perModule` (as proposed in §6.3) closes this for the
current state and restores per-module attribution for future additions.

---

## 8. Reproducing locally

```powershell
$env:PATH = 'E:\DevCache\cargo\bin;' + $env:PATH
$env:CARGO_BUILD_JOBS = '2'
Set-Location D:\GitGit\.worktrees\reg-ut

# one-time build warm-up (compiles and links only; runs no tests)
$env:CARGO_TARGET_DIR = 'D:\GitGit\.worktrees\reg-ut\target-regression'
cargo test --color=never --no-fail-fast --offline --no-run

# the tier itself
pwsh -NoProfile -File scripts/regression-ut.ps1
```

Per-tier JSON reports and raw cargo captures land in `target/regression-logs/`.
`target/` and `target-regression/` are gitignored and were left on disk.

---

## 9. Findings summary

1. **Stale count contract (primary).** `scripts/regression-baseline.json`
   still says 188 tests / 186 passed; reality today is 216 / 214. The tier
   exits `1` on 5 count assertions. Proposed delta in §6.3.
2. **No implementation defect.** 214 passed, 0 failed, 2 ignored; cargo exits
   `0`. No code fix was warranted, so none was committed.
3. **New modules are invisible to the per-module assertions**
   (`cli::tests`, `config::tests`). The gate still fails correctly via the
   total assertions, but per-module attribution is lost until the contract is
   re-agreed. See §7.3.
4. **Baseline metadata is stale in four places** — `capturedAt`,
   `capturedFrom.head`, `capturedFrom.branch`, `ignoredTest` (it names
   `minio_vault_versioned_e2e`, which does not exist; the real test is
   `minio_vault_versioned_e2e_roundtrip`) — plus `ut.command`, which records
   `cargo test --quiet` while the tier runs
   `cargo test --color=never --no-fail-fast --offline`. See §6.3.
5. **Both minIO tests remain `#[ignore]`d**, as required, and were not run.
6. **Build-cost note for future lanes.** A cold build of this dependency tree
   took 17m05s at `-j2` on this box, which exceeds the tier's internal 600 s
   timeout. Any lane that starts from a cold `target-regression/` should
   warm the cache first, or the tier will report a spurious setup error
   (exit `2`).
