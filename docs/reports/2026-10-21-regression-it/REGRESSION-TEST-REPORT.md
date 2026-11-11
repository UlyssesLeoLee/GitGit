# GitGit IT Tier Regression Test Report

**Lane**: `test/regression-it` (IT regression lane of a 3-lane parallel sweep)
**Worktree**: `D:\GitGit\.worktrees\reg-it`
**Base**: `dev` @ `3a9bbdc41041129dc1a42c6779ab7ad9716d8cee`
**Status**: PASS — 51/51 cases green, exit code 0, no defect found, no code fix required.
**Baseline**: `scripts/regression-baseline.json` → `it.endpoints`, captured `2026-09-20` at `151957d`
**Date**: 2026-10-21 JST

---

## 1. Scope

This report covers **only the IT tier** of the three-lane regression sweep
(UT / IT / ST). The UT and ST lanes ran concurrently in sibling worktrees
(`D:\GitGit\.worktrees\reg-ut`, `D:\GitGit\.worktrees\reg-st`) and are
reported separately.

The IT tier (`scripts/regression-it.ps1`) is the HTTP-level integration tier.
It differs from the in-process `axum` oneshot tests in `src/server/api.rs` by
crossing a **real bound TCP socket** and the full router stack — auth
middleware, error mapping, JSON serialization, path-extractor behaviour. It
also exercises routing distinctions a unit test cannot easily simulate
(400 vs 404), content-type handling, and JSON shape contracts end-to-end.

What this tier does, in order:

1. Builds the `gitgit` binary into a per-worktree `target-regression/` dir.
2. Seeds two bare repos (`alpha`, `beta`) in a scratch dir.
3. Boots a real `gitgit serve` instance on the given bind address.
4. Waits for `/api/health` to answer `200` (readiness gate).
5. Asserts the **auth layer** on its own, before the baseline runs:
   anonymous → 401, wrong credential → 401, `/api/health` → 200.
6. Runs the **18 baseline endpoints** from `scripts/regression-baseline.json`
   (`it.endpoints`) against `/api/*` via `curl` + `jq` — status code, JSON
   shape, array length, key containment and body-content assertions.
7. Tears the server down and writes a JSON report.

Exit-code contract: `0` = all assertions passed, `1` = an assertion failed,
`2` = setup error (build failed, server never became ready, jq missing, …).

**Out of scope / explicitly not touched:**

- `scripts/regression-baseline.json` — the shared cross-lane contract. It was
  **not modified** in this lane (verified: `git status --porcelain
  scripts/regression-baseline.json` is empty). Any proposed delta is written
  up in §10 instead of being committed.
- The integration target `D:\GitGit` (branch `dev`). No checkout, commit,
  merge, rebase or push was performed there. The parent integrator owns the
  rebase + merge.
- The sibling lanes' worktrees.

---

## 2. Deliverables

| Path                                                        | What it is                                                              |
|-------------------------------------------------------------|-------------------------------------------------------------------------|
| `docs/reports/2026-10-21-regression-it/REGRESSION-TEST-REPORT.md` | This report.                                                     |
| `docs/reports/2026-10-21-regression-it/it-report.json`      | Verbatim copy of the tier's own JSON report (the auditable artifact).   |

The tier's live report was written by the tier itself to
`target/regression-logs/it-20261021-074441.json` (gitignored build output;
left on disk). The committed copy is byte-identical to it:

```
sha256(it-20261021-074441.json) = D29F3D5450B0282F927B4CF34694C962E23D51252A68AA9496C85E9E2EA1FA13
```

**No code-fix commit accompanies this report.** The tier proved no defect,
so per the lane rules there is nothing to fix (§7, §8).

---

## 3. Environment

| Item                | Observed value                                          |
|---------------------|---------------------------------------------------------|
| OS                  | Microsoft Windows 11 Pro, `10.0.26300` (NT 10.0.26300.0), x64 |
| Shell               | PowerShell **7.6.6** Core (`pwsh`), `pwsh -NoProfile`    |
| Rust toolchain      | `cargo 1.98.1 (797e8a9bc 2026-08-05)` / `rustc 1.98.1 (48a229cea 2026-09-01)` — pinned by `rust-toolchain.toml` |
| jq                  | **`jq-1.7.1`** (verified with `jq --version` **before** the run) |
| curl                | `curl` from the system PATH (`C:\Windows\System32\curl.exe`) |
| git                 | `git version 2.41.0.windows.1`                           |
| Bind address        | `127.0.0.1:18199` (dedicated port for this lane)         |
| `CARGO_HOME`        | `E:\DevCache\cargo` (pre-set)                             |
| `RUSTUP_HOME`       | `E:\DevCache\rustup` (pre-set)                            |
| `CARGO_TARGET_DIR`  | `D:\GitGit\.worktrees\reg-it\target-regression` — set by the tier script itself, not overridden |
| `CARGO_BUILD_JOBS`  | `2` (box has ~2.4 GB free RAM; default `-j 24` would exhaust it) |

`jq` and `cargo` are **not** on the default PATH on this box; both were
prepended (`E:\DevCache\tools\jq;`, `E:\DevCache\cargo\bin;`) for the run.
jq is a hard requirement for this tier — `scripts/regression-it.ps1:88-91`
exits `2` when jq is absent, because jq gates the `expectJsonShape`,
`expectArrayLenEq` and `minArrayLen` assertion kinds and the tier refuses to
report a weaker pass with those assertions silently dropped.

---

## 4. Verified results

### 4.1 The run

```
Command : pwsh -NoProfile -File scripts/regression-it.ps1 -Bind 127.0.0.1:18199
Exit code: 0
Report  : target/regression-logs/it-20261021-074441.json
Started : 2026-10-21T07:44:30.8654039+09:00
Ended   : 2026-10-21T07:44:41.4481388+09:00
Duration: 10.583 s
```

The tier's own summary block, verbatim:

```
[step] binary: D:\GitGit\.worktrees\reg-it\target-regression\debug\gitgit.exe
[step] seeding bare repos in C:\Users\leo19\AppData\Local\Temp\gitgit-it-5cd206c314de48ce8ba4a73fd07c62c3\repos
[step] starting server on 127.0.0.1:18199 (log: ...\gitgit-it-5cd206c3.../server.log)
[step] server ready
[step] auth layer: anonymous / wrong credential / health
...
── IT summary ──
  regression cases    : pass=51 fail=0 skip=0
  baseline endpoints  : 18
  json report         : D:\GitGit\.worktrees\reg-it\target\regression-logs\it-20261021-074441.json
```

### 4.2 Verbatim results table

Counts are taken from the tier's JSON report, not estimated. Independently
re-derived with `jq` over the report's own `cases[]` array:

```
$ jq -r '"tier=\(.tier) pass=\(.pass) fail=\(.fail) skip=\(.skip) total=\(.total)"' it-20261021-074441.json
tier=it pass=51 fail=0 skip=0 total=51

$ jq -r '[.cases[].outcome] | group_by(.) | map({(.[0]): length}) | add' it-20261021-074441.json
{
  "pass": 51
}

$ jq -r '[.cases[] | select(.outcome != "pass")] | if length == 0 then "NONE" else . end' it-20261021-074441.json
NONE
```

| Outcome | Count |
|---------|-------|
| pass    | **51** |
| fail    | **0**  |
| skip    | **0**  |
| **Total** | **51** |

Assertion breakdown: **47** assertions derive from the 18 baseline endpoints
(multiple assertions per endpoint), plus **4** script-internal auth-layer
assertions that live in `scripts/regression-it.ps1` and are not part of the
baseline file. 47 + 4 = 51. The 2026-09-21 report's "IT 47 / 47" figure
matches the 47 baseline-derived half exactly; the +4 is explained in §7.1.

### 4.3 Per-endpoint results

Every row below is one baseline endpoint (or one auth-layer group). All
observed values are quoted from the tier's JSON report `detail` fields.

| #   | Method | Path                                    | Assertions | Result | Observed (expected → actual) |
|-----|--------|-----------------------------------------|-----------|--------|------------------------------|
| A1  | GET    | `/api/repos` (no credentials)           | 2 | PASS | 401 → 401; body `{"error":"authentication required","code":"unauthenticated"}` |
| A2  | GET    | `/api/repos` (wrong credentials)        | 1 | PASS | 401 → 401 |
| A3  | GET    | `/api/health` (no credentials)          | 1 | PASS | 200 → 200 |
| 1   | GET    | `/api/health`                           | 4 | PASS | 200 → 200; body `{"status":"ok","version":"0.1.0","backend":"versioned","vault_online":true}` |
| 2   | GET    | `/api/repos`                            | 3 | PASS | 200 → 200; shape `array` → `array`; **length 2 → 2** |
| 3   | GET    | `/api/repos/does-not-exist`             | 1 | PASS | 404 → 404 |
| 4   | GET    | `/api/repos/..%2Fescape`                | 2 | PASS | 400 → 400; body `{"error":"invalid repo name: ../escape","code":"invalid_repo_name"}` |
| 5   | GET    | `/api/repos/alpha/refs`                 | 3 | PASS | 200 → 200; shape `array` → `array`; length 0 (≥ 0) |
| 6   | GET    | `/api/repos/alpha/log`                  | 2 | PASS | 200 → 200; shape `array` → `array` |
| 7   | GET    | `/api/repos/alpha`                      | 5 | PASS | 200 → 200; body contains `"name":"alpha"`, `"path":`, `"refs":`, `"log":` |
| 8   | GET    | `/api/vault/keys`                       | 3 | PASS | 200 → 200; shape `array` → `array`; **length 0 → 0** |
| 9   | POST   | `/api/vault/keys/openai/versions` `{"value":"sk-openai-abc"}` | 3 | PASS | 200 → 200; `"version":1`; `"key":"openai"` |
| 10  | POST   | `/api/vault/keys/openai/versions` `{"value":""}` | 2 | PASS | 400 → 400; body `{"error":"http error: value must not be empty","code":"bad_request"}` |
| 11  | POST   | `/api/vault/keys/openai/versions` `{"value":"sk-openai-def"}` | 2 | PASS | 200 → 200; `"version":2` |
| 12  | GET    | `/api/vault/keys/openai`                | 3 | PASS | 200 → 200; `"key":"openai"`; `"current_version":2` |
| 13  | GET    | `/api/vault/keys/openai/versions`       | 4 | PASS | 200 → 200; `"key":"openai"`; `"versions":`; shape `object` → `object` |
| 14  | GET    | `/api/vault/keys/openai/diff?base=1&head=2` | 4 | PASS | 200 → 200; `"base":`; `"head":`; `"object_changed":` |
| 15  | POST   | `/api/vault/keys/openai/restore` `{"target_version":1}` | 2 | PASS | 200 → 200; `"new_version":` present |
| 16  | POST   | `/api/vault/keys/openai/restore` `{"target_version":0}` | 2 | PASS | 400 → 400; body `{"error":"http error: target_version must be >= 1","code":"bad_request"}` |
| 17  | GET    | `/api/vault/keys/nope`                  | 1 | PASS | 200 → 200 |
| 18  | DELETE | `/api/vault/keys/openai`                | 1 | PASS | 204 → 204 |

Rows A1–A3 are the auth-layer group the tier asserts before the baseline;
they are **not** baseline endpoints. Rows 1–18 are the 18 baseline endpoints
in `scripts/regression-baseline.json` → `it.endpoints`, in file order.

The three numeric assertions, quoted verbatim from the report:

```
it.GET./api/repos.arrayLen            ::  expected=2 actual=2
it.GET./api/repos/alpha/refs.arrayLen>=0  ::  actual=0
it.GET./api/vault/keys.arrayLen       ::  expected=0 actual=0
```

### 4.4 Post-run hygiene

- Scratch dir cleaned: the tier removes its scratch root on a clean run
  (`scripts/regression-it.ps1:402-406`). Verified: no `gitgit-it-*` directory
  remains under `%TEMP%`.
- Server torn down: `stopping server (pid=27708)` was logged, and
  `Get-Process gitgit` afterwards returned nothing.
- Only `it-20261021-074441.json` is present in
  `target/regression-logs/` — no `it-build-failed.json` / `it-setup-error.json`
  / `it-build-timeout.json`, i.e. the run was **not** a setup error.

---

## 5. Baseline observations (expected vs observed)

**The IT baseline did not drift.** Every one of the 51 assertions matched its
recorded expectation exactly; there is no expected-vs-observed gap to report.

The three assertions most exposed to drift — and their outcomes:

| Assertion                                    | Baseline expects | Observed | Drift? |
|----------------------------------------------|------------------|----------|--------|
| `GET /api/repos` array length                | `expectArrayLenEq: 2` | `2`  | No |
| `GET /api/vault/keys` array length           | `expectArrayLenEq: 0` | `0`  | No |
| `POST /api/vault/keys/openai/versions` → v1  | `"version":1`    | `1`    | No |
| `POST /api/vault/keys/openai/versions` → v2  | `"version":2`    | `2`    | No |
| `GET /api/vault/keys/openai` current version | `"current_version":2` | `2` | No |
| `GET /api/vault/keys/openai/diff?base=1&head=2` | `"base":`,`"head":`,`"object_changed":` | all present | No |

This is worth stating explicitly, because the lane briefing warned that drift
was likely: **the contract survived the drift.** The IT baseline was captured
`2026-09-20` at `151957d`; `dev` has since advanced **211 commits**
(`git rev-list --count 151957d..3a9bbdc` = 211; `git merge-base --is-ancestor
151957d 3a9bbdc` exits 0, so the capture point is on this line of history) —
and none of the 18 endpoint expectations moved.

Two structural checks explain why the contract held:

1. **The `it` baseline block was never edited.** The whole `it` object is
   byte-identical to the one first committed in `d36d8c4`
   ("feat(test): UT/IT/ST regression suite + baseline contract", 2026-09-21):

   ```
   $ jq -cS '.it' <(git show d36d8c4:scripts/regression-baseline.json)   # 2462 chars
   $ jq -cS '.it' scripts/regression-baseline.json                       # 2462 chars
   RESULT: IT BASELINE BLOCK IS IDENTICAL to the original capture (d36d8c4)
   ```

   Later commits touched the file (`46b07de`, `328962a`, `f690486` on
   2026-10-05) but only re-agreed the **`ut`** count contract; the `it`
   section was left alone.

2. **The server-side behaviour the `it` block pins did not drift either.**
   `git log --full-history 151957d..3a9bbdc -- src/server/api.rs` lists the
   auth layer — landed from two parallel branches as `1621bc5` ("/api
   requires auth, the credential is external, and the console can sign in")
   and `e67b27c` ("/api requires auth and the credential is no longer in the
   binary") — plus a clippy fix (`de35233`) and a `cargo fmt` (`eef2a7a`); the
   vault commits (`d3d184e`, `46b07de`) did not alter the response shapes the
   `it` block asserts on. Verified against the implementation source:
   `src/server/api.rs:243-266` (route table), `src/server/api.rs:293-305`
   (`health` → `status`/`backend`/`vault_online`), `src/server/api.rs:386-411`
   (`list_vault_keys` → array), `src/server/api.rs:449-488`
   (`post_vault_version` → `version`/`key`), `src/server/api.rs:501-524`
   (`post_vault_restore` → `new_version`, `target_version` guard),
   `src/server/vault_versioned.rs:124-130` (`VaultVersionDiff` → `base`,
   `head`, `object_changed`), `src/server/config.rs:179-188`
   (`validate_repo_name` → the `"invalid repo name"` body).

---

## 6. Failures

**None.** `fail=0`, `skip=0`, exit code `0`. There is no failed endpoint, no
setup error, and no partially-degraded assertion set — jq was present, so all
four assertion kinds (status code, JSON shape, array length, key/body
containment) were actually evaluated.

---

## 7. What changed and why

### 7.1 Why 51 cases and not the 47 of the 2026-09-21 report

The 4 extra cases are the **auth-layer** group the tier asserts *before* the
baseline runs (`scripts/regression-it.ps1:219-250`). They were added together
with the auth layer itself, on 2026-10-05, by **two parallel commits** that
both landed in `dev` — neither is an ancestor of the other:

```
$ git log -S 'it.auth.anon.repos.status' --format='%h %ad %s' --date=short -- scripts/regression-it.ps1
1621bc5 2026-10-05 fix(security): /api requires auth, the credential is external, and the console can sign in (#19)

$ git show af6b9c8:scripts/regression-it.ps1 | Select-String 'it.auth.anon.repos.status'
af6b9c8 2026-10-05 test(regression): the tiers now authenticate, and the IT tier checks that
```

`git merge-base --is-ancestor` confirms both are ancestors of `3a9bbdc`
(exit 0 each) while `af6b9c8` is **not** an ancestor of `1621bc5` (exit 1) —
two independent branches that implemented the same auth work and were merged
into `dev`. The merged `scripts/regression-it.ps1` carries the block.

The auth layer made `/api/*` require HTTP Basic auth, so the baseline
requests had to start sending `-u` and the tier gained the three auth-layer
assertions (anonymous → 401, wrong credential → 401, `/api/health` → 200
outside the auth layer). The 47 baseline-derived assertions are unchanged.
**This is an addition, not a regression.**

### 7.2 Why the tier ran on `127.0.0.1:18199`

The tier's default bind is `127.0.0.1:18099` (`scripts/regression-it.ps1:150`).
The UT and ST lanes run concurrently in sibling worktrees, and each lane
boots its own `gitgit serve`; sharing the default port would let two servers
collide on the same socket and produce a false result in whichever lane lost
the race. Port **18199** was reserved for this lane so the two `gitgit serve`
instances cannot interfere.

### 7.3 Why the build was pre-warmed before the tier ran

`scripts/regression-it.ps1:116-119` builds with a **hard-coded 600-second
timeout**. This box has ~2.4 GB free RAM and three lanes were compiling
concurrently (`Blocking waiting for file lock on package cache` appears in the
build log — the shared `CARGO_HOME` registry cache, not a fault). A cold
build under that contention can plausibly exceed 600 s, which would make the
tier exit `2` with `it-build-timeout.json` — a setup error that says nothing
about the contract under test.

So the binary was built first, into the tier's **own** target dir, with the
tier's **exact** flags:

```
$env:CARGO_BUILD_JOBS = '2'
$env:CARGO_TARGET_DIR = 'D:\GitGit\.worktrees\reg-it\target-regression'
cargo build --locked --color=never
PREWARM_EXIT=0
BINARY_OK
```

The tier's own `Resolve-Binary` (`scripts/lib/regression-common.ps1:375-427`)
then found that binary fresh and skipped its internal rebuild. Per-worktree
target dir, no shared cache, `CARGO_BUILD_JOBS=2` — all as required. The tier
script itself still produced the report and the exit code, and the binary it
tested is the one built from this worktree's `3a9bbdc` source.

### 7.4 Why jq and cargo were prepended to PATH

Neither is on the default PATH on this box. jq specifically is a hard gate
for this tier (`scripts/regression-it.ps1:88-91`): without it the
`expectJsonShape` / `expectArrayLenEq` / `minArrayLen` assertions would be
silently dropped and the tier would report a pass it never earned, so it
exits `2` instead. `jq --version` → **`jq-1.7.1`** was verified **before** the
run.

---

## 8. Findings and risks

- **No defect.** The IT contract holds exactly as written against
  `dev @ 3a9bbdc`. No code fix was needed, so no fix commit is included.
- **Baseline drift risk is lower than the briefing assumed for this tier.**
  211 commits have landed since the capture and the `it` block still matches
  byte-for-byte *and* semantically. The one place drift *could* bite is
  `expectArrayLenEq: 2` on `/api/repos`: it counts the repos the tier itself
  seeds (`alpha`, `beta`) in a scratch dir, so it is self-consistent and
  immune to repo-count changes elsewhere in the product.
- **Residual coupling in the vault version assertions.** The `"version":1` /
  `"version":2` / `"current_version":2` expectations pin
  `FileVault::set_with_version` version numbering
  (`src/server/vault_versioned.rs:470-491`). That numbering is currently
  1-indexed and contiguous, but a future change to `append_version` /
  `bump_timeline` semantics — e.g. making the restore snapshot bump that the
  current `restore_to_version` performs
  (`src/server/vault_versioned.rs:522-570`) land *before* the `current_version`
  read — would silently invalidate the baseline. The baseline currently
  passes only because the `GET /api/vault/keys/openai` read happens before
  the restore in the baseline's endpoint order. Worth a guard comment in the
  baseline if that ordering is ever reshuffled.
- **The 600 s in-script build timeout is a latent flake source on contended
  boxes** (§7.3). It is not a failure of this tier today, but on a 3-lane
  sweep it is the most likely way the IT tier produces a `2` that has
  nothing to do with the contract.
- **The IT tier does not cover the frontend or the desktop app.** This lane
  exercised the Rust HTTP surface only; `apps/gm-console` and
  `apps/gm-desktop` are out of scope for the IT tier as defined.

---

## 9. Reproducing locally

```powershell
# The tier needs jq and cargo on PATH on this box.
$env:PATH = 'E:\DevCache\cargo\bin;E:\DevCache\tools\jq;' + $env:PATH
$env:CARGO_BUILD_JOBS = '2'

cd D:\GitGit\.worktrees\reg-it

# Optional: pre-warm the build so the tier's internal 600 s build timeout
# cannot fire under lane contention. Same target dir, same flags as the tier.
$env:CARGO_TARGET_DIR = (Join-Path (Get-Location) 'target-regression')
cargo build --locked --color=never

# The tier itself (override -Bind when running lanes concurrently).
pwsh -NoProfile -File scripts/regression-it.ps1 -Bind 127.0.0.1:18199
Write-Output "exit=$LASTEXITCODE"
```

The tier's JSON report lands in `target/regression-logs/it-<timestamp>.json`;
on a setup failure it writes `it-build-failed.json`, `it-build-timeout.json`
or `it-setup-error.json` instead.

---

## 10. Proposed baseline delta

**None.** No change to `scripts/regression-baseline.json` is proposed or
needed — every baseline assertion matched its expectation, so there is no
stale number to re-agree.

For the record, the following were checked and found **not** to be drift
(neither needs updating):

| Baseline entry                          | Expected | Observed | Action |
|-----------------------------------------|----------|----------|--------|
| `GET /api/repos` `expectArrayLenEq`     | 2        | 2        | none   |
| `GET /api/vault/keys` `expectArrayLenEq`| 0        | 0        | none   |
| `POST .../versions` `"version":1`       | 1        | 1        | none   |
| `POST .../versions` `"version":2`       | 2        | 2        | none   |
| `GET /api/vault/keys/openai` `"current_version":2` | 2 | 2 | none |

If the parent integrator prefers to record the auth-layer assertions in the
baseline file rather than only in the script, that would be an **addition** of
4 new endpoint entries (anonymous `GET /api/repos` → 401,
`GET /api/repos` with a wrong credential → 401, and the `/api/health`
outside-the-auth-layer check) — but it is optional, and it is a
cross-lane contract change, so it is left to the parent decision rather than
committed here.
