# T4 — Working-tree status and diff

`docs/plan/v0-tasks.md` row T4 recorded the repository view as
`[FACT] 部分落地`: list, refs and commit graph existed, while "working
tree status 视图与 diff 视图在 `apps/gm-desktop/src/` 未找到 → 该子项未落地".

This report records what now exists, what was measured, and what is not
verified. Every claim carries a tag: `[FACT]` (measured or read directly
from the code in this branch), `[UNVERIFIED-FACT]` (asserted by a
document or convention, not re-run here), `[INFERENCE]`, `[PROPOSAL]`,
`[TBD]`.

- Branch: `feat/t4-status-diff`, worktree `D:\GitGit\.worktrees\t4-status-diff`
- Base: `72dbb52` (= `origin/dev` at the time of the work)
- Date: 2026-10-05

---

## 1. What landed

| Area | File | What |
| --- | --- | --- |
| Library | `src/repo/status.rs` (new) | porcelain-v1 status parsing, three-way diff, untracked synthesis, path validation |
| Library | `src/repo/mod.rs` | re-exports the public surface |
| Desktop | `src-tauri/src/commands/repos.rs` | `repo_status`, `repo_diff`, `resolve_worktree_dir` |
| Desktop | `src-tauri/src/error.rs` | `NotAWorkTree`, `InvalidDiffTarget` error kinds |
| Desktop | `src-tauri/src/lib.rs` | both commands registered in `generate_handler!` |
| Frontend | `src/lib/components/RepoWorktree.svelte` (new) | status groups, diff target selector, clean / error states |
| Frontend | `src/lib/stores/worktree.ts` (new) | status + diff state, grouping helpers |
| Frontend | `src/routes/RepoDetail.svelte` | mounts `RepoWorktree`; fixes a broken interpolation |
| Frontend | `src/lib/api/{types,tauri}.ts` | typed wrappers |
| Frontend | `src/lib/i18n/{en,zh-CN}.ts` | 27 keys added to each catalogue |
| Frontend | `src/mocks/handlers.ts` | dev-mode fixtures for the two commands |
| Tests | `apps/gm-desktop/tests/unit/worktree.test.ts` (new) | 12 store + component + i18n cases |
| Plan | `docs/plan/v0-tasks.md` | T4 row updated |
| Baseline | `scripts/regression-baseline.json` | re-agreed, see §5 |

---

## 2. Design choices and why

### 2.1 Porcelain v1, and the quoting that makes it non-trivial

`[FACT]` The basis is `git status --porcelain=v1 --branch
--untracked-files=all`, invoked as
`git -c core.quotePath=false status --porcelain=v1 --branch
--untracked-files=all`.

`[FACT]` Measured on this machine: a path containing a space is emitted
**quoted, even with `core.quotePath=false`**:

```
## master
 M a.txt
A  staged.txt
?? "brand new.txt"
```

A parser that split the path field on whitespace would accept a format
git never emits and would report the filename as `"brand`. With
`core.quotePath` left at its default, a non-ASCII path arrives as a run
of octal byte escapes (`"\303\274n\303\257code.txt"`), so
`unquote_porcelain_path` decodes both forms. `[FACT]` The two-column `XY`
form is what makes staged and unstaged separable without a second pass;
`MM`, `M ` and ` M` are distinct inputs and all three are asserted.

`[FACT]` A rename quotes each side independently —
`R  "old name.txt" -> "new name.txt"` — so the arrow cannot be located
with a plain `split(" -> ")`. `split_rename_field` walks to the matching
close quote first.

### 2.2 The staged / unstaged / HEAD question

`[FACT]` These are three different comparisons and the UI must let the
user say which one they mean. `DiffTarget` has exactly three variants,
each mapped to a distinct git invocation, and each verified to return a
different answer on one repository that had both a staged and an unstaged
edit:

| Variant | Meaning | Command |
| --- | --- | --- |
| `staged` | what a commit would contain | `git diff --cached` |
| `worktree` | what is not yet staged | `git diff` |
| `head` | everything uncommitted | `git diff HEAD` |

`[FACT]` Evidence that they differ, from one repo holding a staged new
file and an unstaged edit:

```
########## target=staged (git diff --cached)
diff --git a/staged.txt b/staged.txt
new file mode 100644
index 0000000..b478595
--- /dev/null
+++ b/staged.txt
@@ -0,0 +1 @@
+s

########## target=worktree (git diff)
diff --git a/a.txt b/a.txt
index ce01362..2227cdd 100644
--- a/a.txt
+++ b/a.txt
@@ -1 +1,2 @@
 hello
+more

########## target=head (git diff HEAD)   [both of the above, concatenated]
```

`[FACT]` The frontend never infers the target: it passes the chosen
`DiffTarget` through to `repo_diff`, and the Rust side **refuses** an
unrecognised value (`AppError::InvalidDiffTarget`) rather than silently
defaulting to one. A test asserts the wire target is forwarded verbatim
in call order.

`[FACT]` Clicking a status entry chooses a target only when the entry
implies one (a staged-only entry opens the staged comparison); untracked
entries always open their own file diff.

### 2.3 Untracked files

`[FACT]` `git diff` reports **nothing** for a file git has never seen.
Measured, scoped to the untracked path: output length 0, exit 0. A status
view that lists such a file with no way to read it is a half-feature, so
untracked entries get a real answer.

`[FACT]` The mechanism is `git diff --no-index` against an empty temp
file, then the header is rewritten into git's own new-file form. The
`index` line and every hunk line are git's output, untouched. The raw
`--no-index` output leaks the temp path, which is why the rewrite exists:

```
exit=1
diff --git "a/C:\\Users\\leo19\\AppData\\Local\\Temp\\empty-080397e6.txt" b/brand new.txt
index e69de29..fbbee86 100644
--- "a/C:\\Users\\leo19\\AppData\\Local\\Temp\\empty-080397e6.txt"
+++ b/brand new.txt	
@@ -0,0 +1,2 @@
+alpha
+beta
```

`[FACT]` Two details that are easy to get wrong and are pinned by tests:
`--no-index` **exits 1** whenever the two paths differ, so only 0 and 1
count as success; and the temp path must not survive into the UI (a test
asserts the output does not contain `gitgit-empty-`).

`[FACT]` The payload carries `untracked: true` and the view labels the
diff as synthesized, so the user is never told git produced something git
did not produce.

`[TBD]` One residual ambiguity: an untracked path has no staged content
and no `HEAD` content, so the synthesized diff is the same whatever
`target` was asked for. The payload echoes the requested `target` rather
than substituting one, which means a user who selects "staged" and then
clicks an untracked entry sees that file's full content under a "staged"
heading. The `untracked: true` label and its sentence are shown alongside,
which is the mitigation, not a fix. The alternative — silently rewriting
the target to `head` — would hide which comparison the user asked for.

`[PROPOSAL]` The synthesized diff is served when a specific untracked
path is requested. A whole-tree "unstaged" comparison does not append
every untracked file, because an unbounded append would need its own
truncation semantics and its own "and N more" state. The status list
makes each untracked file individually readable instead.

### 2.4 Bare repositories

`[FACT]` `git status` in a bare repository exits 128 with
`fatal: this operation must be run in a work tree`. This matters because
gitgit's own repositories **are** bare: `create_bare_repo` runs
`git init --bare`, and `list_repos` only discovers directories named
`<name>.git` containing a `HEAD` file.

`[FACT]` Rather than restate git's refusal, `is_bare_repo` classifies the
directory from the filesystem alone (`HEAD` is a file and there is no
`.git` child). The desktop returns a typed `NotAWorkTree`, and the UI
renders a localized remedy: point at a checkout. The raw git text is kept
but rendered small, never as the headline, because it can name a host
path.

`[FACT]` This is why the status view is a component that may fail on its
own: a bare repository still has refs and commits worth showing.

### 2.5 A clean tree is an answer

`[FACT]` `is_clean` is `entries.is_empty()`, and the store maps it to a
distinct `clean` state rather than an empty one. The view renders
"working tree is clean". Tests assert the clean state is reachable and
that it is not confused with the error state, in both the store and the
rendered DOM.

---

## 3. Security

### 3.1 The validation rule is reused, not reinvented

`[FACT]` Both commands take a repository **name** and validate it with
`crate::config::validate_repo_name` — the same rule the HTTP layer
applies, the one its `/api/repos/..%2Fescape` case tests — as the **first
statement**, before any filesystem access and before any subprocess is
spawned. `resolve_worktree` performs no I/O at all, which is what makes
the rejection testable in isolation:

```
########## traversal attempts, all refused by the canonical rule
../escape  ..  a/b  a\b  .hidden  ""  "with space"  ..git  demo..git  C:win
=> "invalid repo name: <name>"
```

`[FACT]` The `.git` suffix is stripped and re-validated exactly as
`Config::repo_path` does, so `..git` cannot smuggle a traversal past the
first check.

### 3.2 A real hole the canonical rule does not close

`[FACT]` `validate_repo_name` rejects `..`, both separators, a leading
`.` and whitespace, but it **permits a colon**. On Windows `C:` is a
drive *prefix*, so `Path::join` discards the root entirely. Measured on
this machine:

```
Path::new("/tmp/root").join("C:win")  ==  "C:win"     // root gone
resolved.parent()                     ==  Some("C:")
```

`[FACT]` That is exactly the failure the brief names — a repository name
reaching `git -C <path>` unvalidated — so `resolve_worktree` adds one
guard on top of the canonical rule rather than instead of it, rejecting a
colon. A colon is not a legal character in a Windows filename, so nothing
usable is lost on the platform that needs the guard.

`[FACT]` A companion test asserts the property that actually matters
rather than the rule's wording: every accepted name resolves to a **direct
child** of the root.

`[FACT]` The same reasoning applies one level down. A repo-relative path
argument (`path` on `repo_diff`) is rejected if it is absolute, contains
`..`, contains `\` or contains `:`. Git pathspecs are `/`-separated, so a
backslash is never legitimate. The primary boundary is an exact match
against git's own `??` list, which `../../etc/passwd` can never satisfy;
the guard is defence in depth and is asserted independently.

### 3.3 Process and output handling

`[FACT]` Arguments are passed as an argv vector to `tokio::process::Command`
with no shell anywhere in the path. `--no-ext-diff` and `--no-textconv`
are passed on every diff: a user-configured external diff driver or
textconv filter executes arbitrary commands, and a read-only viewer has
no business running one.

`[FACT]` Pathspecs are terminated with `--`, so a file named like a git
option cannot be read as one.

`[FACT]` Subprocess stderr is not forwarded to the frontend. Git failures
become `AppError::Git`, and the view renders that detail small and
non-bold, with the localized remedy as the headline.

`[FACT]` Diff text is rendered through a Svelte interpolation with no
`{@html}`. A test feeds `+<img src=x onerror=alert(1)>` as a diff line and
asserts it appears as characters and that no `img` element exists in the
DOM.

---

## 4. Evidence

All commands run from `D:\GitGit\.worktrees\t4-status-diff` on Windows 11
with `CARGO_TARGET_DIR=E:\DevCache\cargo\target-lane-q`, cargo 1.98.1.

### 4.1 The three required repository cases

`[FACT]` A clean repository — this is the literal porcelain text the
parser consumed:

```
$ git -c core.quotePath=false status --porcelain=v1 --branch --untracked-files=all
## master
exit=0

$ git -c core.quotePath=false diff --no-color --no-ext-diff --no-textconv
[]            # len=0, exit=0
```

→ `WorktreeStatus { is_clean: true, entries: [], branch: Some("master") }`,
and `DiffPayload { status: empty }` for every target.

`[FACT]` A dirty repository with one staged, one unstaged and one
untracked path — literal parser input:

```
$ git -c core.quotePath=false status --porcelain=v1 --branch --untracked-files=all
## master
 M a.txt
A  staged.txt
?? "brand new.txt"
```

→ three entries; `staged = ["staged.txt"]`,
`unstaged = ["a.txt"]`, `untracked = ["brand new.txt"]` (unquoted). The
diff texts received are the three in §2.2.

`[FACT]` A repository with an untracked file — the classification call
and the synthesis, quoted in full in §2.3.

### 4.2 Test counts and gates

`[FACT]` Every figure below is from a run in this worktree, not from a
document. Exit codes are cargo's / the tool's own, not a pipeline's.

| Gate | Command | Result |
| --- | --- | --- |
| fmt, root | `cargo fmt --all -- --check` | exit 0 |
| fmt, gm-desktop | `cargo fmt --manifest-path apps/gm-desktop/src-tauri/Cargo.toml -- --check` | exit 0 |
| clippy, root | `cargo clippy --all-targets --locked -- -D warnings` | exit 0 |
| clippy, gm-desktop | `cargo clippy -p gm-desktop --all-targets --locked --manifest-path apps/gm-desktop/src-tauri/Cargo.toml -- -D warnings` | exit 0 |
| test, root | `cargo test --locked` | exit 0 |
| test, gm-desktop | `cargo test -p gm-desktop --locked --manifest-path apps/gm-desktop/src-tauri/Cargo.toml` | exit 0 |
| typecheck | `corepack pnpm@9.15.9 check` | `svelte-check found 0 errors and 0 warnings` |
| frontend tests | `corepack pnpm@9.15.9 test` | see below |
| frontend build | `corepack pnpm@9.15.9 build` | `✓ built in 2.42s`, 150 modules |

`[FACT]` Note on `cargo fmt -p gm-desktop`: that form fails with
`package 'gm-desktop' is not a member of the workspace`, because the root
is a single crate rather than a workspace. `--manifest-path` is the
correct invocation.

`[FACT]` Root crate, `cargo test --locked`, cargo's own summary line:

```
running 188 tests
test result: ok. 186 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out
```

`[FACT]` Frontend, `corepack pnpm@9.15.9 test`:

```
 Test Files  8 passed (8)
      Tests  97 passed (97)
```

`[FACT]` The 97 comprise 85 pre-existing and 12 new. No test is skipped,
no `describe.skip` was added, no threshold was lowered, and no
`istanbul ignore` / `v8 ignore` / `coverage.exclude` was added.

`[FACT]` i18n parity, counted directly from the two catalogues:

```
en keys  = 173
zh keys  = 173
identical key sets = true
duplicates en = []
duplicates zh = []
```

173 = the 146 recorded in the brief plus the 27 added here (25 `repos.*`
plus 2 `errors.kind.*`). A test asserts every new key resolves in both
locales and that `{placeholder}` sets match across locales.

`[FACT]` `gm-desktop` Rust tests, `cargo test -p gm-desktop --locked`,
cargo's own summary line:

```
running 28 tests
test result: ok. 28 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

`[FACT]` All 28 are pre-existing (`state::tests::*` and
`commands::ai::tests::*`) and this change adds **zero** Rust tests to that
crate. It is not covered by `scripts/regression-baseline.json`, which
tracks the root crate only, so its count needs no re-agreement. The
desktop command surface is covered from the frontend instead, by the 12
vitest cases.

`[FACT]` The 97 comprise 85 pre-existing and 12 new. No test is skipped,
no `describe.skip` was added, no threshold was lowered, and no
`istanbul ignore` / `v8 ignore` / `coverage.exclude` was added.

`[FACT]` i18n parity, counted directly from the two catalogues:

```
en keys  = 173
zh keys  = 173
identical key sets = true
duplicates en = []
duplicates zh = []
```

173 = the 146 recorded in the brief plus the 27 added here (25 `repos.*`
plus 2 `errors.kind.*`). A test asserts every new key resolves in both
locales and that `{placeholder}` sets match across locales.


### 4.3 What is explicitly NOT verified

`[TBD]` The **HTTP layer was not touched.** There is no
`/api/repos/:name/status` or `/api/repos/:name/diff` endpoint; only the
GUI path was delivered. The library core is path-based and directly
reusable, but no axum handler wraps it and no `it` tier endpoint row was
added.

`[TBD]` **No test invokes the two Tauri commands themselves.**
`#[tauri::command]` functions take `State<'_, DesktopState>`, which
cannot be constructed outside a Tauri runtime, and the crate has no
`src-tauri/tests/` directory to add an integration test to. What is
tested is `resolve_worktree_dir`'s validation path (it is the first
statement of both commands and delegates to the library's
`resolve_worktree`, which has 2 traversal tests) and the library
subprocess layer. The gap is the Tauri plumbing itself.

`[TBD]` **The feature was not exercised through a running Tauri
application.** No window was opened and no real `invoke` round trip was
observed. The store and component cases drive the store directly with
the wire payloads the Rust side produces.

`[TBD]` **The default install still lists bare repositories**, so out of
the box every repo in the desktop's own `repos` directory answers
`NotAWorkTree`. Pointing at a checkout requires passing the optional
`root` argument; no folder-picker UI was built in this change, so a user
of the shipped app cannot yet set that root from the interface. The
`root` parameter exists, is validated identically, and is used by the
store and the dev mock, but wiring a picker is outstanding work.

`[FACT]` The webview already holds `fs:allow-read-dir` over `$HOME/**`,
`$DOCUMENT/**` and `$DOWNLOAD/**` plus `dialog:allow-open`
(`src-tauri/capabilities/default.json`), so accepting a caller-supplied
worktree root introduces no privilege the webview did not already have.

`[TBD]` **Windows-only behaviour is untested on other platforms.** The
`C:` drive-prefix hole is a Windows finding; the guard is unconditional,
so it also refuses a colon in a repository name on Unix, where a colon
would have been legal.

`[TBD]` **The `--no-index` synthesis was not tested for a binary file.**
The header rewrite preserves the `index` line, so a binary untracked file
should render correctly, but no test covers it.

`[UNVERIFIED-FACT]` The 2 `#[ignore]`d minIO tests in the root crate are
unchanged and were not re-run; they need a running minIO.

---

## 5. Baseline

`[FACT]` Which target the tests land in: **the root `gitgit` crate.** The
25 new Rust tests are `src/repo/status.rs::tests`, so they run under
`cargo test --locked` in the root crate and are counted by
`scripts/regression-baseline.json`. The 12 frontend tests are vitest and
are a different target entirely; `gm-desktop` contributes no Rust tests
of its own. The brief asked which crate to attribute the count to before
touching numbers — it is the root crate, and only the root crate.

`[FACT]` Re-agreed from 163 / 161 / 2 to **188 / 186 / 2**, with
`"repo::status": 25` added to `perModule`. Verified after editing:

```
JSON parses            = true
expectedTotal          = 188
expectedPassed         = 186
expectedIgnored        = 2
perModule sum          = 188
sum === total          = true
passed+ignored===total = true
```

`[FACT]` The diff is `4` insertions and `3` deletions on that file, and
the edited file still has **0** LF-only line breaks, so its CRLF endings
are intact.

`[FACT]` The rationale field was extended rather than rewritten, keeping
the file's own convention of recording each re-agreement in place, and it
names this branch, the base commit, the exact cargo summary line, and the
fact that the frontend tests are not counted here.

`[FACT]` The two `#[ignore]`d minIO tests are untouched and still need a
running minIO; they were not re-run.


## 6. Pre-existing defect found and fixed

`[FACT]` `src/routes/RepoDetail.svelte:75` called `.replace()` on the
*interpolated* catalogue string:

```svelte
{$catalog['repos.notFound']}.replace('{name}', params?.name ?? '')
```

so the page displayed the literal text `.replace('{name}', 'demo')`
instead of the repository name. This is the same defect class the
review-page test recorded for `Review.svelte` and it was fixed there. It
is fixed here as a one-line correction in a file this change already
touches.
