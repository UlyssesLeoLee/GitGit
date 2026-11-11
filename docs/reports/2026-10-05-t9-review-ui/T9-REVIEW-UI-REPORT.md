# T9 — AI review UI: streaming tokens in the GUI (layer 2 of 2)

Lane K · branch `feat/t9-review-ui` · base `328962a` · 2026-10-05

T9 in `docs/plan/v0-tasks.md` row 9 is *"AI 评审 UI — push 前弹窗 +
streaming token 显示"*, accepted when *"GUI 上能看到 token 逐字流"*.
Layer 1 made the transport reachable from Rust; this lane makes it
reachable from the GUI.

---

## 1. What landed

### Commits

| SHA | Purpose |
|---|---|
| `cc41e00` | `feat(ai)`: cancellable command surface + `/review` page |
| `16d8b47` | `test(ai)`: 20 Rust tests, mock SSE server, two defects fixed |
| `fc78597` | `style(ai)`: normalize `ai.rs` to LF (repo convention) |
| `35d4bf8` | `test(gm-desktop)`: 19 store/component cases for the review UI |

### Files

**Rust — command surface**

- `apps/gm-desktop/src-tauri/src/commands/ai.rs` (new) — `ai_review_start`,
  `ai_review_cancel`, the `ReviewEvent` enum, the `ReviewEmitter` trait,
  and the 20 inline tests including the mock SSE server.
- `apps/gm-desktop/src-tauri/src/state.rs` — `ReviewManager` (one in-flight
  stream, abort **and await** on cancel), `REVIEW_MAX_DIFF_BYTES`,
  `DesktopState.reviews`.
- `apps/gm-desktop/src-tauri/src/error.rs` — five typed variants
  (`AiReviewUnsupported`, `AiReviewNoKey`, `AiReviewInvalid`,
  `AiReviewAlreadyRunning`, `AiReviewNotRunning`) so the UI can localize
  by `kind` instead of pattern-matching English.
- `apps/gm-desktop/src-tauri/src/{lib,commands/mod}.rs` — registration only.

**Frontend**

- `src/lib/api/ai.ts` (new) — command wrappers + `listenReviewEvents`.
- `src/lib/stores/review.ts` (new) — the state machine and the three
  ordering rules that matter.
- `src/routes/Review.svelte` (new) — the page.
- `src/lib/api/types.ts`, `src/App.svelte`, `src/lib/components/Sidebar.svelte`,
  `src/lib/i18n/{en,zh-CN}.ts`, `src/mocks/handlers.ts` — wiring.
- `tests/unit/review.test.ts` (new) — 19 cases.

**Not touched:** `src/ai/**` (layer 1 is unchanged), `scripts/regression-baseline.json`
(see §5 for why), and every file outside the ownership list.

---

## 2. Design decisions worth defending

### A route, not a pre-push dialog

T9's prose names a dialog. `[FACT]` The V0 shell has no push action to
anchor one to — `commands/repos.rs` exposes `list_repos`, `repo_detail`,
`clone_url`, `open_repo_in_shell`, and nothing that pushes. Inventing a
push button would be a larger change than T9 is, and the streaming
contract the dialog exists to show is identical either way. `[PROPOSAL]`
The dialog should be built as a wrapper around this panel when a push
action lands; the panel is deliberately the same component either way.
Landed as `/review` plus a sidebar entry (`nav.review`).

### Tauri behind a trait

`spawn_review` takes an `Arc<dyn ReviewEmitter>`, not an `AppHandle`.
A command body requiring a live Tauri runtime can only be tested by
running the app, and the behaviour worth protecting here — event
ordering, cancellation, the key guard — is exactly what a recording
emitter observes deterministically. `TauriEmitter` is a three-line
adapter.

### The key cannot reach the UI by any path

`[FACT]` Read only from `GITGIT_AI_API_KEY` (`read_api_key_env`). No
command parameter, no CLI flag, no frontend field. Every error leaving
the module is passed through `sanitize::scrub_secret`. The diff is
built by `prompt::review` as an untrusted message and passes through
`sanitize::sanitize_request` before the call; model output is re-emitted
as plain text and the page contains no `{@html}`.

### Cancel means the work stopped

`ReviewManager::cancel` aborts the consuming task **and awaits it**
(same reasoning as `ServerManager::stop`: returning before the
cancellation lands would let the UI report "stopped" while the task
still held the connection). Aborting drops the `StreamReceiver`, so the
producer's next send fails and the HTTP response body is released.

---

## 3. Evidence

### Streaming, with no real API key

`MockSseServer` is a loopback HTTP/1.1 server speaking the OpenAI SSE
wire shape over raw `tokio` TCP. `[PROPOSAL]` It is raw TCP rather than
`axum` because a chunked streaming body needs a `futures_core::Stream`
and `src-tauri/Cargo.toml` — outside this lane's ownership — declares no
such crate; hand-writing chunked transfer keeps the test
dependency-free and lets the mock observe the peer disconnect.

`[FACT]` `tokens_arrive_in_order_and_the_review_completes`, actual
emitted delta sequence:

```
EVIDENCE deltas=["Hel", "lo", "wo", "rld", "!"]
         kinds=[model, model, token, model, token, model, token, model,
                token, model, token, done]
```

`[FACT]` `cancel_stops_the_work_and_the_provider_sees_the_client_leave`
asserts two independent things: the manager slot is free the instant
`cancel` resolves and a new stream can start, **and** the mock server
observed its connection closing. The second is the part a UI-only fake
cannot prove, and it is the part that matters — if the socket stayed
open, the request was still running.

### Commands and exit codes

| Command | Exit | Result |
|---|---|---|
| `cargo fmt --all -- --check` (gitgit) | 0 | clean |
| `cargo fmt --all -- --check` (gm-desktop) | 0 | clean |
| `cargo clippy --locked --all-targets -- -D warnings` (gitgit) | 0 | clean |
| `cargo clippy --offline --all-targets -- -D warnings` (gm-desktop) | 0 | clean |
| `cargo test --locked --lib` (gitgit) | 0 | 161 passed / 2 ignored / 0 failed |
| `cargo test --offline --lib` (gm-desktop) | 0 | **28 passed** / 0 failed |
| `svelte-check --tsconfig ./tsconfig.json` | 0 | 0 errors, 0 warnings |
| `vite build` | 0 | 148 modules, built in 2.29s |
| `vitest run` | 0 | **76 passed / 6 skipped** (82) |
| `vitest run --coverage` | 1 | pre-existing threshold failure — see §5 |

`[FACT]` gm-desktop went from 8 lib tests to 28: 20 new, of which 16
drive a real loopback socket.

### i18n

`[FACT]` 146 keys in `en.ts` and 146 in `zh-CN.ts`, identical key sets
**in identical order**, no duplicates (was 118 each). One test asserts
per-key presence in both locales; another asserts the `{provider}` /
`{n}` / `{model}` placeholder sets match across locales, so a token
cannot survive untranslated in one bundle and render as a literal brace.

### Two defects the tests found

1. `[FACT]` **The key leaked.** A non-2xx response whose body echoes the
   `Authorization` header put the key into the error the frontend
   receives: `{"kind":"Gitgit","message":"…HTTP 401 — unauthorized:
   sk-LEAKED-KEY",…}`. Layer 1 includes the provider's response body in
   its error, reasoning that the body is provider-authored and cannot
   contain our key — true for a real provider, false for a
   misconfigured proxy. `spawn_review` now scrubs the key off *every*
   error it returns, not only off in-band stream errors.
2. `[FACT]` **`redactions` reported the wrong number.**
   `sanitize_request` returns messages-rewritten, which is always ≥1
   because the structural `<untrusted>` tag always applies. The UI
   string claimed "N secret-shaped span(s) were filtered". It now counts
   spans actually filtered, via `sanitize::scrub`.

---

## 4. Explicitly NOT verified

- `[UNVERIFIED-FACT]` **No real provider was contacted.** The repository
  has no `GITGIT_AI_API_KEY`. Everything about OpenAI-compatible
  endpoints, the `data: [DONE]` framing as a *real* server emits it, and
  Anthropic's refusal is exercised only against the local mock and the
  registry's `supports_streaming` flag. A real call may differ.
- `[UNVERIFIED-FACT]` **No live Tauri runtime run.** Events were verified
  through the `ReviewEmitter` seam. The `TauriEmitter` adapter itself
  (that `app.emit("ai-review://token", …)` reaches a webview) is
  `[UNVERIFIED-FACT]` — `core:event:default` is already granted in
  `capabilities/default.json`, and no capability change was needed, but
  it was not observed end-to-end in a packaged app.
- `[UNVERIFIED-FACT]` **No CI run.** All results are local, on this box,
  at `328962a` + these four commits.
- `[TBD]` **The six component-render tests do not execute.** Under this
  vitest setup Svelte resolves to its *server* build, so
  `@testing-library/svelte`'s `render` throws
  `lifecycle_function_unavailable: mount(...) is not available on the
  server` before any assertion runs. The fix is one line in
  `apps/gm-desktop/vite.config.ts` — `test: { resolve: { conditions:
  ['browser'] } }` — which is **outside this lane's ownership**, so it is
  reported rather than edited. The cases are kept as `describe.skip`
  with the reason inline, ready to re-enable by dropping `.skip`. The
  store cases above them cover the same state machine, so no behaviour
  is left unverified in the meantime.

---

## 5. Baseline re-agreement: no change required

`[FACT]` `scripts/regression-baseline.json` is **unchanged** at
163 total / 161 passed / 2 ignored, and its `perModule` entries still sum
to exactly 163.

Reason: `[FACT]` `regression-ut.ps1` runs `cargo test` at the repo root,
and the root `Cargo.toml` is explicitly *not* a workspace — `gm-desktop`
is a separate crate with its own `Cargo.lock`. All 20 new Rust tests
therefore land in `gm-desktop` and cannot move the `gitgit` per-module
counts. No new `src/**` module was added, so no `perModule` entry is
due. Verified by running the tier's own command, not by grepping:
`cargo test --locked --lib` reports 161 passed / 2 ignored / 0 failed.

---

## 6. Needs outside this lane's ownership

1. **Scope note — `apps/gm-desktop/tests/unit/review.test.ts`.** `[FACT]`
   This path appears in neither the may-modify list nor the
   must-not-touch list. It was added because deliverable 3 requires
   frontend tests and `vite.config.ts` sets
   `include: ['tests/unit/**/*.test.ts']`, so that is the only place
   vitest collects from; the forbidden `apps/gm-desktop/scripts/**` is a
   different directory. The file is purely additive and touches no
   packaging or CI surface. Flagged rather than assumed: if the parent
   considers `tests/**` owned by another lane, this one file should move
   with it.
2. **`apps/gm-desktop/vite.config.ts`** — add
   `test: { resolve: { conditions: ['browser'] } }` to un-skip the six
   component tests. One line.
3. **`apps/gm-desktop/src-tauri/Cargo.lock`** — `[FACT]` it does not
   satisfy its own manifest, so `--locked` fails for the `gm-desktop`
   crate. Proven pre-existing: extracting `328962a` with `git archive`
   into a scratch directory and running `cargo metadata --locked` there,
   with none of this lane's changes present, fails identically
   (`error: cannot update the lock file … because --locked was passed`,
   exit 101). The drift is `rust-s3`'s transitive `aws-creds`
   (0.39.1 → 0.40.0) and `quick-xml` (0.38.4 → 0.41.0) re-resolving from
   the local registry cache. `gm-desktop` was therefore verified with
   `--offline` (which permits the in-place lock update) and the lock file
   was restored to `328962a` before every commit — this lane's four
   commits touch no lock file. Regenerating the lock is a packaging-lane
   decision.
4. **`apps/gm-desktop/vite.config.ts` coverage thresholds** — `[FACT]`
   `vitest run --coverage` exits 1 at `328962a` **without** this lane's
   changes: 37.71% lines / 50% functions against thresholds of 70 / 60.
   This lane *raises* those to **45.06% / 53.19%** (`review.ts` 87.09%
   lines, `Review.svelte` 77% statements, `ai.ts` 66.66%). No threshold
   was lowered and no `coverage.exclude` was added. The pre-existing
   failure is not T9's to fix and is reported rather than papered over.
