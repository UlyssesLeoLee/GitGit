# GitGit ST Regression Test Report (2026-10-21)

**Lane**: `test/regression-st` (ST tier of a 3-lane parallel regression sweep)
**Worktree**: `D:\GitGit\.worktrees\reg-st`
**Base**: `dev` @ `3a9bbdc41041129dc1a42c6779ab7ad9716d8cee`
**Branch HEAD**: `d613105` (fix commit) — see §9
**Status**: **PASS** — the ST tier is green on the product behaviour, and one
tier-script defect was found, proved and fixed.

---

## 1. Scope

This lane runs **one** tier: **ST** (System Test).

| | |
|---|---|
| Tier script | `scripts/regression-st.ps1` |
| Wrapped script | `scripts/smoke.ps1` (unchanged by this lane) |
| What it exercises | The full stack end-to-end with a **real `git` client** over HTTP smart protocol: `info/refs` negotiation, `upload-pack` (clone), `receive-pack` (push), Basic-auth enforcement, branch propagation, and merge semantics across two independent clones. |
| What it does *not* exercise | The `/api/*` HTTP surface (IT lane) and the in-process unit tests (UT lane). |

`scripts/regression-st.ps1` does not reimplement the smoke test. It delegates
to `scripts/smoke.ps1`, captures its stdout, and asserts that a set of
**greppable milestone markers** are present in that output. Twelve milestones
plus three aggregate assertions make fifteen reported cases.

### Artefacts committed with this report

| File | What it is |
|---|---|
| `st-report.json` | The tier's JSON report for the canonical (passing) run — `target/regression-logs/st-20261021-075003.json`. |
| `smoke.log` | The captured `smoke.ps1` stdout for that same run — `target/regression-logs/smoke-20261021-074941.log`. |
| `st-report-run1-coldbuild-timeout.json` | JSON report of the **first** run, which failed (§5.1). |
| `smoke-run1-coldbuild-timeout.log` | That run's `smoke.ps1` stdout (48 bytes: the build banner and nothing else). |
| `smoke-run1-coldbuild-timeout.stderr.log` | That run's `smoke.ps1` stderr — the line `cargo build failed (exit=-1)` that is the whole root cause. |
| `st-report-skipbuild.json` | JSON report of the post-fix `-SkipBuild` run (§9.2). |
| `smoke-skipbuild.log` | Its `smoke.ps1` stdout — note the **absence** of the build banner. |
| `smoke-prefix-skipbuild-ignored.log` | The pre-fix `-SkipBuild` run's stdout — note the **presence** of the build banner. That contrast is the proof of the defect fixed in §9. |

---

## 2. Exact command

Run from the root of the worktree:

```
pwsh -NoProfile -File scripts/regression-st.ps1 -Bind 127.0.0.1:18198
```

`-Bind 127.0.0.1:18198` rather than the tier default `127.0.0.1:18098`, because
the UT and IT lanes were running concurrently in sibling worktrees and the two
`gitgit serve` instances must not collide on a port. The tier's default repo
name (`regression`) was used unchanged.

Environment preparation (the `cargo` on this box is not on the default PATH):

```powershell
$env:PATH        = 'E:\DevCache\cargo\bin;' + $env:PATH
$env:CARGO_BUILD_JOBS = '2'          # this box has ~2.4 GB free RAM
```

`CARGO_TARGET_DIR` was **not** set by the caller: `scripts/regression-st.ps1:37`
sets it itself to the per-worktree `target-regression/`, and that was left
alone. `CARGO_HOME=E:\DevCache\cargo` and `RUSTUP_HOME=E:\DevCache\rustup`
were inherited from the process environment.

---

## 3. Environment

| Item | Value |
|---|---|
| OS | Windows 11 (10.0.26300, x64) |
| Shell | PowerShell 7.6.6 (`pwsh -NoProfile`) |
| cargo | `cargo 1.98.1 (797e8a9bc 2026-08-05)` |
| rustc | `rustc 1.98.1 (48a229cea 2026-09-01)` |
| Toolchain pin | `rust-toolchain.toml` → `channel = "1.98.1"` (matches the toolchain above) |
| git | `git version 2.41.0.windows.1` (Git for Windows) |
| Bind address | `127.0.0.1:18198` |
| Repo name under test | `regression` (tier default) |
| `CARGO_TARGET_DIR` | `D:\GitGit\.worktrees\reg-st\target-regression` (set by the tier) |
| `CARGO_BUILD_JOBS` | `2` |
| Sibling lanes | UT and IT running concurrently in `.worktrees/reg-ut` and `.worktrees/reg-it` |

---

## 4. Results

### 4.1 Canonical run (final HEAD, `d613105`, default invocation)

Command: `pwsh -NoProfile -File scripts/regression-st.ps1 -Bind 127.0.0.1:18198`

**Tier exit code: `0`** · `smoke.ps1` exit code: `0` · tier duration: **22.1 s**
(the JSON report's own `started` → `ended` span, `07:49:41` → `07:50:03`;
the tier's own summary line for the equivalent pre-fix run reported
`duration 33.14s`).

| Outcome | Count |
|---|---|
| pass | **15** |
| fail | **0** |
| skip | **0** |
| **total** | **15** |

Milestones present: **12 / 12** (baseline floor `st.minAssertions = 7`).

### 4.2 Milestone-by-milestone (verbatim from `st-report.json`)

| # | Case | Outcome | Detail |
|---|---|---|---|
| 1 | `st.milestone.build` | pass | `pattern=cargo build --locked --quiet matched` |
| 2 | `st.milestone.init_repo` | pass | `pattern=init-repo matched` |
| 3 | `st.milestone.server_ready` | pass | `pattern=server ready matched` |
| 4 | `st.milestone.first_clone` | pass | `pattern=git clone \(first\) matched` |
| 5 | `st.milestone.default_branch` | pass | `pattern=default branch = matched` |
| 6 | `st.milestone.initial_commit` | pass | `pattern=initial commit on matched` |
| 7 | `st.milestone.push_main` | pass | `pattern=push -u origin matched` |
| 8 | `st.milestone.feature_branch` | pass | `pattern=checkout -b feature matched` |
| 9 | `st.milestone.feature_push` | pass | `pattern=git push.*origin feature\|push -u origin feature matched` |
| 10 | `st.milestone.second_clone` | pass | `pattern=git clone \(second\) for merge matched` |
| 11 | `st.milestone.merge_commit` | pass | `pattern=merge commit verified in log matched` |
| 12 | `st.milestone.negative_auth_push_rejected` | pass | `pattern=unauthenticated push\|received expected auth challenge\|push correctly rejected matched` |
| 13 | `st.verdict.pass_marker` | pass | `pattern=gitgit MVP smoke test: PASS matched` |
| 14 | `st.smoke_exit_code.is_zero` | pass | `expected=0 actual=0` |
| 15 | `st.milestone_count.meets_baseline` | pass | `present=12 baseline.min=7` |

### 4.3 What the smoke run actually did (from `smoke.log`)

Every step of the real `git` client flow is present in the captured log:

```
[smoke] cargo build --locked --quiet (debug)
[smoke] starting server on 127.0.0.1:18198 ...
[smoke] gitgit init-repo regression
[smoke] waiting for http://127.0.0.1:18198/repos/regression.git/info/refs?service=git-upload-pack
[smoke] server ready
[smoke] git clone (first)
[smoke]   Cloning into '...\clone1'...
[smoke]   warning: You appear to have cloned an empty repository.
[smoke] default branch = master
[smoke] create + commit hello.txt on master
[smoke]   [master (root-commit) e4fa614] initial commit on master
[smoke] git push -u origin master
[smoke]   branch 'master' set up to track 'origin/master'.
[smoke]    * [new branch]      master -> master
[smoke] git checkout -b feature
[smoke]   [feature 8226816] feature branch commit
[smoke] git push -u origin feature
[smoke]    * [new branch]      feature -> feature
[smoke] git clone (second) for merge verification
[smoke] fetch + merge origin/feature into master
[smoke]   Merge made by the 'ort' strategy.
[smoke] merge commit verified in log
[smoke] negative auth: push without creds must fail
[smoke]   push-output: error: RPC failed; HTTP 401 curl 22 The requested URL returned error: 401
[smoke]   push-output: send-pack: unexpected disconnect while reading sideband packet
[smoke]   push-output: fatal: the remote end hung up unexpectedly
[smoke]   push-output: Everything up-to-date
[smoke]   push correctly rejected (exit=1)
[smoke] stopping server (pid=31284)
========================================
  gitgit MVP smoke test: PASS
========================================
```

Two observations worth recording, neither a defect:

* **The default branch is `master`, not `main`.** `smoke.ps1` derives the
  branch name from `git symbolic-ref --short HEAD` rather than assuming one
  (`scripts/smoke.ps1:209`), so the flow adapts. The tier's
  `default_branch` milestone matches `default branch =` and is therefore
  branch-name agnostic.
* **The unauthenticated push was correctly rejected** with `HTTP 401` and
  `git push` exit code 1. The trailing `Everything up-to-date` is git's
  own summary of a push it never performed; the authoritative evidence is the
  401 and the non-zero exit.

---

## 5. Failures encountered, and their root cause

### 5.1 The first run failed — cold debug build exceeded the in-script build timeout

The **first** invocation of the tier in this lane exited **1**, with
`pass=1 fail=14 skip=0` and only **2 / 12** milestones present. The entire
smoke log was 48 bytes:

```
[smoke] cargo build --locked --quiet (debug)
```

and the stderr was:

```
[smoke] FAIL: cargo build failed (exit=-1)
```

**Root cause — traced through code, not guessed from the symptom.**

| Step | Evidence |
|---|---|
| `-1` is not a process exit code; it is `Invoke-External`'s timeout sentinel. | `scripts/lib/regression-common.ps1:186` — `ExitCode = if ($timedOut) { -1 } else { $proc.ExitCode }`; the timeout branch kills the process at lines 176–182. |
| The build is invoked with a hard-coded 600 s timeout. | `scripts/smoke.ps1:122` — `Invoke-External -FilePath $cargo -TimeoutSec 600`. |
| A non-zero build exit is fatal to the smoke script. | `scripts/smoke.ps1:125-128` — `Fail "cargo build failed (exit=$($build.ExitCode))"`. |
| The measured duration matches the timeout exactly. | The tier reported `duration 10m3s` (603 s) against a 600 s limit. |
| The build really was slower than 600 s because three lanes were compiling at once. | Observed during the run: 5 `cargo` and 10 `rustc` processes alive across the three lanes, and free physical memory fell to **674 MB**. |
| A cold build of this crate does fit in 600 s once the contention drops. | A subsequent standalone `cargo build --locked` with `CARGO_BUILD_JOBS=2` completed in **6 m 13 s (376.7 s)**, exit 0. |

So the first run failed in the **build step**, before a single line of the
product was exercised — which is why 14 of 15 cases were red with nothing but
the build banner in the log. It is a **capacity/concurrency failure of the
test harness, not a product defect**: no server was started, no `git` client
ran, and no product code path was reached.

Contributing factors, all environmental:

* **Shared `CARGO_HOME` package-cache lock.** The pre-warm build logged
  `Blocking waiting for file lock on package cache` for a period before it
  started compiling. The per-worktree `CARGO_TARGET_DIR` prevents a *target
  directory* collision, but `CARGO_HOME=E:\DevCache\cargo` is still shared
  across all three lanes, so the package cache is a shared lock.
* **RAM.** ~2.4 GB free with three concurrent debug builds.

### 5.2 The fix that made the run reproducible

With the build pre-warmed (the second run, §4.1), the tier completed in
**33.14 s** and went green. The fix commit `d613105` (§9) removes the need
for the pre-warm in the first place, by making `-SkipBuild` actually work.

### 5.3 No product failure was observed at any point

No milestone failed for a product reason in any of the seven tier runs
performed in this lane. The only red run was the cold-build timeout above,
and the only red *case* in the whole lane was `st.milestone.build` in the
intermediate post-fix state (§9.2), which was an artefact of the fix being
half-applied and was resolved before the fix commit.

---

## 6. Baseline observations

`scripts/regression-baseline.json` was **not modified** by this lane (it is a
shared contract and the sibling lanes are running against the same file).

### 6.1 Expected vs observed

| Baseline field | Value | Observed | Verdict |
|---|---|---|---|
| `st.minAssertions` | `7` | 12 / 12 milestones present (`present=12 baseline.min=7`) | **Met** (floor) |
| `st.description` | "End-to-end smoke: real git client against the HTTP smart protocol. Wraps `scripts/smoke.ps1` unchanged." | `scripts/smoke.ps1` is untouched by this lane | **Met** |
| `st.command` | `pwsh scripts/smoke.ps1` | The tier is invoked as `pwsh scripts/regression-st.ps1 -Bind 127.0.0.1:18198`; the baseline names the *inner* script | **Stale** (§6.3) |
| `st.timeoutSec` | `180` | Cold-build run: **603 s**. Warm run: **33.14 s**. The effective limit is `smoke.ps1`'s hard-coded **600 s** | **Stale** (§6.3) |
| `st.rationale` | "smoke.ps1 asserts: (1) build OK, … (7) unauth push rejected. 7 step-level checks plus negative-auth fallbacks." | The tier asserts **12** named milestones | **Stale wording** (§6.3) |

### 6.2 The milestone set did **not** legitimately change

The brief asked for a finding if `scripts/smoke.ps1`'s milestone set had
changed since the 2026-09-20 baseline capture. It did not:

```
git log --oneline -S "Name='" -- scripts/regression-st.ps1
  d36d8c4 feat(test): UT/IT/ST regression suite + baseline contract
```

The milestone list has been **12 entries since the tier's very first
commit** (`d36d8c4`, 2026-09-21). The only milestone ever touched is the
`build` pattern, whose *text* was corrected in `cf46021`
(`fix(scripts): correct the ST build-milestone pattern`) to follow
`smoke.ps1`'s banner change in `40d8f67` (`cargo build --quiet` →
`cargo build --locked --quiet`). The **set** is unchanged.

`git log --oneline -S 'minAssertions' -- scripts/regression-baseline.json`
returns the same single commit, `d36d8c4` — i.e. `st.minAssertions = 7` was
authored *together with* the 12-milestone tier. It is a **floor**, not a
count, and `scripts/regression-st.ps1:119-123` says so in as many words:
*"Total milestone count check (defensive against future drift)."*

### 6.3 Proposed baseline delta — **reported, not applied**

```diff
   "st": {
     "description": "End-to-end smoke: real git client against the HTTP smart protocol. Wraps scripts/smoke.ps1 unchanged.",
-    "command": "pwsh scripts/smoke.ps1",
-    "timeoutSec": 180,
+    "command": "pwsh scripts/regression-st.ps1 -Bind 127.0.0.1:<port>",
+    "timeoutSec": 600,
     "minAssertions": 7,
-    "rationale": "smoke.ps1 asserts: (1) build OK, (2) server ready, (3) first clone, (4) push to main, (5) feature branch push, (6) second clone + merge, (7) unauth push rejected. 7 step-level checks plus negative-auth fallbacks."
+    "rationale": "The tier asserts 12 greppable milestones individually (build, init-repo, server ready, first clone, default branch, initial commit, push main, feature branch, feature push, second clone, merge commit, negative-auth push rejected) plus 3 aggregate cases (PASS banner, smoke exit code, milestone count) -- 15 cases in total. minAssertions = 7 is a FLOOR, deliberately below the milestone count, and is not the number of milestones: the tier already asserts each milestone individually, so the floor cannot hide a missing milestone. Keep it at 7: with -SkipBuild the build milestone is recorded as a skip and is excluded from the count, so a floor of 12 would make the -SkipBuild path red. timeoutSec 600 mirrors the build timeout smoke.ps1 actually applies (Invoke-External -TimeoutSec 600); the tier's own $TimeoutSec parameter is not wired to anything."
   }
```

Rationale for each change:

* `st.command` — names the script a reader would actually type. The old value
  names the tier's *inner* script, and `scripts/regression.ps1`'s own
  documented entry point is `pwsh scripts/regression.ps1`.
* `st.timeoutSec` `180` → `600` — the old value understates reality by ~3.3×:
  the measured cold-build run took **603 s**. The effective limit is
  `smoke.ps1`'s `Invoke-External -TimeoutSec 600`. Note that **no script reads
  `st.timeoutSec`** — it is documentation only, so this change has no
  behavioural effect. Likewise `scripts/regression-st.ps1:26` declares
  `[int]$TimeoutSec = 240` and never reads it (contrast `regression-ut.ps1:89`
  and `:136`, which both use theirs). That dead parameter is reported as a
  finding in §8, not fixed here.
* `st.rationale` — the current text says "7 step-level checks", which reads
  as "the tier asserts 7 things". It asserts 12. The revised wording states
  the milestone list explicitly and explains why the floor stays at 7.
* `st.minAssertions` — **deliberately left at 7.** Raising it to 12 would
  look tidier but would make the `-SkipBuild` path red, because the build
  milestone is a skip there and is excluded from `$presentMilestones`
  (`scripts/regression-st.ps1:121`). A floor of 7 is the correct contract.

---

## 7. Artefact checksums of the committed evidence

The committed `smoke.log` and `st-report.json` are byte-for-byte copies of
the tier's own outputs under `target/regression-logs/`. The two artefacts
that carry the failure evidence are the run-1 pair, and the one that carries
the defect proof is the pair with the contrasting build banners.

| Artefact | Source file in `target/regression-logs/` | Bytes |
|---|---|---|
| `st-report.json` | `st-20261021-075003.json` | 2802 |
| `smoke.log` | `smoke-20261021-074941.log` | 2368 |
| `st-report-run1-coldbuild-timeout.json` | `st-20261021-073709.json` | 3641 |
| `smoke-run1-coldbuild-timeout.log` | `smoke-20261021-072705.log` | 48 |
| `smoke-run1-coldbuild-timeout.stderr.log` | `smoke-20261021-072705.log.err` | 468 |
| `st-report-skipbuild.json` | `st-20261021-074940.json` | 2837 |
| `smoke-skipbuild.log` | `smoke-20261021-074917.log` | 2322 |
| `smoke-prefix-skipbuild-ignored.log` | `smoke-20261021-074557.log` | 2368 |

---

## 8. Findings

| # | Severity | Finding | Evidence |
|---|---|---|---|
| F1 | **Defect — fixed** | The ST tier declared `[switch]$SkipBuild` and never used it. | §9 |
| F2 | **Risk** | `scripts/smoke.ps1`'s build has a hard-coded 600 s timeout that the tier cannot influence, and the tier's own `$TimeoutSec` is dead. A cold build under concurrent lanes blew through it (§5.1). | `scripts/smoke.ps1:122`; `scripts/regression-st.ps1:26` (never read) |
| F3 | **Risk** | The `negative_auth_push_rejected` milestone pattern is loose: `unauthenticated push` also matches smoke.ps1's *failure* text `unauthenticated push unexpectedly succeeded` (`scripts/smoke.ps1:306`). The tier is not blind — that path exits 1 and `st.smoke_exit_code.is_zero` catches it — but the milestone itself can go green on a failing log. Proposed tightening: `'received expected auth challenge\|push correctly rejected'`. **Not changed by this lane** — it alters the milestone contract, and the brief reserves `scripts/regression-baseline.json` for the parent. | `scripts/smoke.ps1:298-313`; `scripts/regression-st.ps1:101` |
| F4 | **Observation** | The `push_main` milestone pattern (`push -u origin`) is also satisfied by the `git push -u origin feature` line, so it is not a strictly independent check of the `main` push. The tier's other milestones and the smoke script's own `$LASTEXITCODE` gates still cover the main push. | `scripts/smoke.ps1:220-222` vs `:234-236` |
| F5 | **Environment** | The three lanes share `CARGO_HOME=E:\DevCache\cargo`, so the **package-cache lock** is still shared even though each lane has its own `CARGO_TARGET_DIR`. The pre-warm build logged `Blocking waiting for file lock on package cache`. | observed during the pre-warm build |
| F6 | **Documentation** | `scripts/regression-baseline.json`'s `st.timeoutSec`, `st.command` and `st.rationale` are stale (§6.3). Only `st.minAssertions` is read by any script. | §6.3 |

---

## 9. What changed and why

### 9.1 The fix — `d613105` `fix(test): forward -SkipBuild from the ST tier to smoke.ps1`

**The defect.** `scripts/regression-st.ps1` declared `[switch]$SkipBuild` at
line 23 and then never used it. The switch appeared **exactly once** in the
whole file — in the param block:

```
$ grep -n 'SkipBuild' scripts/regression-st.ps1
  23:    [switch]$SkipBuild,
```

The argv handed to `smoke.ps1` was a fixed list that did not contain it
(`scripts/regression-st.ps1:71`, before the fix):

```powershell
-ArgumentList @('-NoProfile','-File',$smokePath,'-Bind',$Bind,'-RepoName',$RepoName) `
```

**The history.** `git log --oneline -S 'SkipBuild' -- scripts/regression-st.ps1`
returns a single commit:

```
bf79d05 fix(st): accept -SkipBuild param so orchestrator's -SkipBuild flag works
```

The commit's stated purpose was exactly the wiring that was never completed.
Meanwhile `scripts/regression.ps1:66` *does* forward the flag to every tier
(`if ($SkipBuild) { $argsForTier += '-SkipBuild' }`), and `scripts/smoke.ps1:42`
and `:115` *do* honour it (`if (-not $SkipBuild) { ...build... }`). The gap
was in the middle link only.

**Proof, by differential test.** Running the tier with `-SkipBuild` before
the fix produced a smoke log whose **first line was still the build banner**:

```
$ pwsh -NoProfile -File scripts/regression-st.ps1 -SkipBuild -Bind 127.0.0.1:18198
[smoke] cargo build --locked --quiet (debug)      <-- the flag was ignored
```

The committed `smoke-prefix-skipbuild-ignored.log` is that log; the run
reported `pass=15 fail=0 skip=0` **because it built anyway** — the flag had
no effect whatsoever.

**Why it matters.** `smoke.ps1` owns the build, and that build carries the
hard-coded 600 s timeout that actually failed this lane's first run (§5.1).
`-SkipBuild` was the only available escape hatch, and it did not work.

**The fix** (`scripts/regression-st.ps1`, 3 hunks):

1. Forward the switch to `smoke.ps1`.
2. Make the `build` milestone conditional. This is **not** optional: the
   `build` milestone matches the literal `cargo build --locked --quiet` in
   the smoke log, and with `-SkipBuild` there is no build step in that log.
   Asserting it would fail a run that was told not to build; calling it a
   pass would claim a build that never happened. It is recorded as a
   **skip**, the same shape the IT tier already uses for a deliberate
   no-build (`scripts/regression-it.ps1:98-103`).
3. Document the parameter with a `.PARAMETER SkipBuild` block, matching the
   IT tier's style (`scripts/regression-it.ps1:30-32`).

### 9.2 Verification of the fix

Both paths were run on this tree, after the fix:

| Invocation | Exit | pass | fail | skip | Milestones | `cargo build` in the smoke log? |
|---|---|---|---|---|---|---|
| `regression-st.ps1 -SkipBuild -Bind 127.0.0.1:18198` | **0** | 14 | 0 | **1** | 11 / 11 | **no** |
| `regression-st.ps1 -Bind 127.0.0.1:18198` | **0** | 15 | 0 | 0 | 12 / 12 | **yes** |

The default path is unchanged: same 15 cases, same 12 milestones, same
`build` milestone result as before the fix. The `-SkipBuild` path now
actually skips the build and reports the build milestone as a skip with the
reason `-SkipBuild was set: the tier did not build, so there is no build step
to assert`.

An intermediate state was also observed and is recorded here for honesty: with
only the forwarding change applied and the milestone still unconditional, the
`-SkipBuild` run reported `pass=14 fail=1` with `st.milestone.build` failing.
That is what motivated change 2 above; it was never committed.

### 9.3 Commits on this branch

| Commit | Subject | Contents |
|---|---|---|
| `d613105` | `fix(test): forward -SkipBuild from the ST tier to smoke.ps1` | the code fix (§9.1) — **separate from the report commit, as required** |
| *(this commit)* | `test(regression): ST 回归结果 — PASS` | this report + the eight evidence artefacts |

The canonical run in §4.1 was executed **after** the fix was applied to the
working tree and **before** it was committed, and `git diff HEAD --
scripts/regression-st.ps1` is empty at `d613105` — so the tree the canonical
run exercised is byte-identical to the committed tree.

---

## 10. Reproducing

```powershell
$env:PATH = 'E:\DevCache\cargo\bin;' + $env:PATH
$env:CARGO_BUILD_JOBS = '2'
Set-Location D:\GitGit\.worktrees\reg-st
pwsh -NoProfile -File scripts/regression-st.ps1 -Bind 127.0.0.1:18198
```

On a cold `target-regression/`, **pre-build first** or pass `-SkipBuild`
(after `d613105`) — otherwise the cold build can exceed `smoke.ps1`'s
600 s in-script timeout on a loaded box:

```powershell
$env:CARGO_TARGET_DIR = 'D:\GitGit\.worktrees\reg-st\target-regression'
cargo build --locked
pwsh -NoProfile -File scripts/regression-st.ps1 -SkipBuild -Bind 127.0.0.1:18198
```

Tier JSON reports and captured smoke logs land in `target/regression-logs/`.

---

## 11. Handoff notes for the integrator

* The product behaviour under test is **green**: 15/15 cases, 12/12
  milestones, real `git` client over HTTP smart protocol, including the
  negative-auth check.
* `scripts/regression-baseline.json` is **untouched**. The proposed delta is
  in §6.3 for the parent to apply.
* `d613105` changes `scripts/regression-st.ps1` only. It does **not** change
  the default-path result of the tier (15/15 before and after). If the
  parent prefers to keep the ST tier script frozen for this sweep, this
  commit can be dropped and the defect recorded as a finding instead — the
  report stands without it, with the caveat that the canonical run in §4.1
  was executed on the post-fix tree.
* F3 (loose `negative_auth_push_rejected` pattern) and F5 (shared
  `CARGO_HOME` package-cache lock) are left for the parent to decide on.
