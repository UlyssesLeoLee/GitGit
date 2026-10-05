# gm-desktop — test coverage for the stores, API layer and shared components

**Date:** 2026-10-05
**Branch:** `test/gmd-stores` (worktree `D:\GitGit\.worktrees\gmd-stores`)
**Base commit:** `d81f285`
**Lane:** the non-route half of the app's test debt — `src/lib/stores/**`,
`src/lib/api/**`, `src/lib/utils/clipboard.ts` and the shared chrome under
`src/lib/components/`. A second, disjoint lane owns `src/routes/**` and
`src/App.svelte`.

No file under `apps/gm-desktop/src/**` was modified. No coverage threshold was
lowered, deleted or relaxed; no `v8 ignore` / `istanbul ignore` comment and no new
`coverage.exclude` glob was added. No existing test was changed, skipped or
weakened. `--coverage` was **not** added to `pnpm test`.

---

## 1. Result

`corepack pnpm@9.15.9 test` → **17 files, 287 tests passed, 0 skipped.**
That is the previous 122 plus **165 new tests** in **8 new files**.

`corepack pnpm@9.15.9 test:coverage` still exits non-zero, as expected: the global
thresholds (lines/statements 70, functions 60, branches 55) are not met yet. That
is a whole-app gate and includes the route components the other lane is working
on; it is not this lane's to close.

### `All files`, before → after

`[FACT]` Both numbers were measured in this worktree at `d81f285 + this lane's
tests`. The "before" column was measured by moving the 8 new test files out of
`tests/unit/`, running the suite, and moving them back — not taken from a prior
run's log.

| Metric | Before | After | Delta |
|---|---|---|---|
| Statements | 36.95% (708/1916) | **49.42% (947/1916)** | +12.47 pts |
| Branches | 42.06% (294/699) | **54.22% (379/699)** | +12.16 pts |
| Functions | 31.96% (125/391) | **55.24% (216/391)** | +23.28 pts |
| Lines | 42.55% (552/1297) | **55.74% (723/1297)** | +13.19 pts |

The "before" figures reproduce the numbers in the task brief exactly
(36.95 / 42.06 / 31.96 / 42.55), which is a useful cross-check that the
measurement setup is the same one the baseline was taken with.

## 2. Per-file coverage, before → after

`[FACT]` Measured from `coverage/coverage-summary.json` in both runs.

| File | Stmts | Branch | Funcs | Lines |
|---|---|---|---|---|
| `src/lib/stores/graph.ts` | 0 → **100** | 0 → **100** | 0 → **100** | 0 → **100** |
| `src/lib/stores/theme.ts` | 0 → **95.45** | 0 → **94.11** | 0 → **100** | 0 → **100** |
| `src/lib/api/http.ts` | 0 → **100** | 0 → **100** | 0 → **100** | 0 → **100** |
| `src/lib/api/graph.ts` | 0 → **100** | 100 → **100** | 0 → **100** | 0 → **100** |
| `src/lib/api/tauri.ts` | 45.83 → **100** | 100 → **100** | 45.83 → **100** | 45.83 → **100** |
| `src/lib/api/ai.ts` | 50 → **100** | 100 → **100** | 40 → **100** | 57.14 → **100** |
| `src/lib/utils/clipboard.ts` | 0 → **95** | 0 → **83.33** | 0 → **100** | 0 → **100** |
| `src/lib/components/ErrorBoundary.svelte` | 0 → **53.33** | 0 → **75** | 0 → **66.66** | 0 → **50** |
| `src/lib/components/ServerStatusBar.svelte` | 0 → **100** | 0 → **80** | 0 → **100** | 0 → **100** |
| `src/lib/components/Sidebar.svelte` | 0 → **100** | 0 → **100** | 0 → **100** | 0 → **100** |
| `src/lib/components/ThemeToggle.svelte` | 0 → **100** | 100 → **100** | 0 → **100** | 0 → **100** |
| `src/lib/components/ToastHost.svelte` | 0 → **95.23** | 0 → **83.33** | 0 → **100** | 0 → **100** |
| `src/lib/components/Router.svelte` | 0 → **100** | 0 → **100** | 0 → **100** | 0 → **100** |

Every owned file is now meaningfully non-zero. The two that are not 100% are
explained in §4 and §5; neither is hidden.

### A note on the file list in the brief

`[FACT]` `src/lib/components/StatusBar.svelte` does not exist. The only
status-bar component in the app is `src/lib/components/ServerStatusBar.svelte`
(54 lines, was 0% covered), and that is the file covered above. `[INFERENCE]` The
brief's entry was a stale or abbreviated name for it.

## 3. The tests, and what each one pins

Conventions follow the existing house style: a JSDoc block stating the intent of
the file, `respondWith` / `rejectsWith`-style bridge helpers defined inside the
test file, behaviour-named cases, and rendered copy compared against
`tFor(get(locale), key)` rather than a pinned language (jsdom reports
`navigator.language === 'en-US'`, so the active locale is `en`).

`tests/setup.ts` was **not** modified. Every helper stayed inside the test file
that uses it, so there is nothing for the other lane to conflict with.

### `api-http.test.ts` (14 tests) — `src/lib/api/http.ts`
- URL building: relative path, leading-slash path (no doubled separator),
  absolute URL bypassing the base, explicit `baseUrl`.
- Header shape: `Accept: application/json` always; `Content-Type` only when there
  is a body; caller `method`/`body` forwarded.
- 204 returns `null` **without** calling `resp.json()` — asserted with a spy,
  because a 204 has no body and an unconditional `.json()` turns every successful
  delete into a reported failure.
- Non-2xx throws a bare `{status, message}` carrying the response body text;
  falls back to `statusText` when the body is empty; a transport-level `fetch`
  rejection propagates unchanged rather than being reported as a status error.
- One case pins a **defect** — see §5.4.

### `api-tauri.test.ts` (30 tests) — `src/lib/api/tauri.ts`
The wrappers a store never happens to call in a test run were exactly the
uncovered ones, and the risky part of a thin wrapper is the *argument names*:
Serde matches named fields, so renaming `serverBind` to `server_bind` compiles
cleanly and then silently delivers `None` at runtime.

- Exact `{cmd, args}` pairs for the ten camelCase / nullable / numeric contracts:
  `startServer` (`{bind}`), `serverLogs` (`{limit}`), `cloneUrl` (`{name,
  serverBind}`), `vaultRestore` (`{key, targetVersion}`), `repoDiff` (`{name,
  target, path, root}` — target passed verbatim, never defaulted), `repoStatus`
  (`{name, root}`, explicit `null`), `vaultDiff` (`{key, base, head}`, with
  `typeof === 'number'` asserted so a stringified integer is caught),
  `repoDetail` (`{name, limit}`).
- All ten argument-less command names in one ordered case.
- Refusals propagate with `kind` intact (the module deliberately does not catch,
  so stores can read it), and a bare `Error` propagates by identity.
- Round trips against the real `src/mocks/handlers.ts` fixtures, so the wrappers
  are checked against the documented wire shape end to end: server lifecycle,
  repos, working-tree status and all three diff targets, the vault read/write/
  rotate/versions/diff/restore/delete cycle, and the admin-password cycle.

### `api-graph.test.ts` (12 tests) — `src/lib/api/graph.ts`
- The seven command names in order; `graph_get_node` sends `id`; `docs_read`
  sends `path`; the four argument-less commands send no arguments.
- `graph_get_node` and `docs_read` return `null` for a miss — a real distinction
  from a failure, since the page can show "no such doc" without an error path.
- Typed refusal propagates with `kind` intact.
- The unvalidated cast at the boundary is checked rather than assumed: every
  returned node carries the fields `GraphNode` promises, and every edge endpoint
  resolves to a node that exists. `graph.ts`'s docstring asserts the cast is safe
  because the wire JSON conforms; these two cases are what would notice if it
  stopped conforming.

### `api-ai.test.ts` (14 tests) — `src/lib/api/ai.ts`
The store tests deliberately inject their own subscription, so
`listenReviewEvents` — the function the review store's seam exists to call — had
never run. `listen` is mocked via `vi.hoisted` + `vi.mock`.

- All five event names subscribed, in order; the literals are pinned to the Rust
  strings (`ai-review://token` …) because a rename there is not mirrored silently
  — the review just never streams.
- The handler receives the event **payload**, not the Tauri envelope; without
  this, `e.type` is `undefined` at the store's first branch.
- Complete fan-out: each of the five channels delivers all four event kinds.
- The single returned unlisten detaches all five, and a second call detaches
  nothing further.
- One case pins a **defect** — see §5.3.
- `aiReviewStart` argument shape including `baseUrl`, unset options sent as
  explicit `null` rather than omitted, and typed refusals propagated.

### `stores-graph.test.ts` (27 tests) — `src/lib/stores/graph.ts`
- `loadGraph` hydration and `source: 'tauri'`; `loading` raised in flight and
  lowered after; fresh `buildAt`; a previous error cleared on retry; **`loading`
  not stuck** after a failure; and a failed refresh keeping the already-loaded
  graph on screen rather than blanking it.
- A typed backend refusal being **lost** — see §5.2.
- `filteredNodes`: empty and whitespace-only queries both return everything; a
  query ranks best-match-first and drops zero-score nodes; id matching; case
  insensitivity; no match at all.
- `stats`: node/edge counts and the per-kind breakdown; zeroed for an empty graph.
- `selectNode` / `getSelected`: set, clear with `null`, and `null` for a
  selected id that a reload has removed.
- `neighbors`: `out` / `in` / `both`; a self-loop counts as **both** an incoming
  and an outgoing edge (the `if` arm wins for `both`, the `else if` matches on
  `e.to` for `in`); `null` id; an isolated node; and a dangling edge reported as
  an edge while contributing no node.

### `stores-theme.test.ts` (18 tests) — `src/lib/stores/theme.ts`
The only store that reads `localStorage` at module-evaluation time, so each case
uses `vi.resetModules()` + a dynamic import for a genuinely fresh module;
importing it once would let the first case's persisted value leak into the rest.

- Persisted `dark` / `light` restored instead of falling back to `auto`; an
  unrecognised persisted value ignored; a storage that **cannot be read** falls
  back to `auto` rather than crashing before first render; the key is pinned to
  the documented `gm-desktop.theme` (it is the on-disk contract with the
  previous launch).
- `setTheme` moves the store, the `data-theme` document attribute Tailwind reads,
  and the persisted value together; a storage that **refuses the write** still
  switches the theme in memory; `initTheme` does not reset a user's choice.
- `isEffectivelyDark`: explicit `dark` / `light` do not consult the system at
  all (asserted with a `matchMedia` spy that must not be called); `auto` mirrors
  `(prefers-color-scheme: dark)`; `auto` degrades to light when `matchMedia` is
  absent.

### `utils-clipboard.test.ts` (13 tests) — `src/lib/utils/clipboard.ts`
`navigator.clipboard` and `document.execCommand` are both absent from jsdom, so
the fallback path is the natural one to test here rather than a contrived one.

- Async path: copies, passes the text through byte for byte (a command with a
  newline and quotes), and copies an empty string without treating it as failure.
- Fallback triggers: clipboard present but rejecting, no clipboard at all, and a
  clipboard object with no `writeText`.
- Honest failure: the fallback refusing reports `false`; `execCommand` throwing
  resolves to `false` rather than rejecting; and with neither mechanism
  available the helper still returns `false` instead of throwing a `TypeError`.
- The scratch textarea: value verbatim, `readonly`, `position: absolute`,
  `left: -9999px`, `select()` called; removed after success **and** after a
  throw (the `finally` is the reason a failed copy cannot leak a node).

### `components-chrome.test.ts` (37 tests) — the shared chrome
- **ErrorBoundary** (3): bare container with mount key `0`; no error card in the
  normal case; and an exception from a child **escaping** `render()` — see §5.1.
- **ThemeToggle** (6): the three modes labelled from the catalogue; exactly one
  `aria-pressed`; the pressed state following a click; store + document +
  persistence moving together; the active/inactive classes; and the control
  staying in step when the store is set directly.
- **ToastHost** (6): one card per toast in push order; a distinct colour per
  kind; dismissing one toast leaving the others (keyed by id, not position);
  emptying the host; and an untrusted message rendered as text, not markup.
- **Sidebar** (9): every href carrying the `#` prefix (the assertion that catches
  a plain `href="/repos"`, which would reload the page and discard app state);
  labels from the catalogue and never the raw key; exactly one `aria-current`;
  a parent staying lit on a detail path; `/` matching only exactly; the active
  classes; following a hash change; and carrying the theme and locale controls.
- **ServerStatusBar** (8): stopped vs running copy, port and uptime hidden while
  stopped, uptime formatted (`90s` → `1m 30s`), indicator colour by state,
  start/stop round trips through the mock, and the control disabled while a call
  is in flight.
- **Router** (5): the matched component per path, a parameterised route, swapping
  on a hash change, an empty render when nothing matches and there is no
  catch-all, and the catch-all winning over that empty render.

## 4. What I deliberately did not cover, and why

- **`theme.ts:29` and `clipboard.ts:21`** — the `typeof document === 'undefined'`
  SSR guards. `[FACT]` jsdom always provides `document`, so covering them means
  tearing down a DOM global, which would break every other case in the same file.
  `[PROPOSAL]` If these are ever wanted, the honest home is a separate
  `@vitest-environment node` test file, not a hack in the jsdom ones.
- **`ErrorBoundary.svelte`'s error card and `children` branch** — the card is
  unreachable (§5.1). The `children` branch additionally cannot be exercised from
  a `.ts` file at all: `[FACT]` a Svelte `Snippet` must be produced by the
  compiler, and passing a plain function renders nothing (measured). Covering it
  would need a `.svelte` fixture, which is outside this lane's file scope.
- **`Router.svelte`'s param hand-off** — `[FACT]` `Router` passes
  `params={match.params}`, but observing that requires a fixture component that
  prints its props, i.e. a new `.svelte` file outside this lane's scope. What is
  pinned is *which* component the router picks and that it swaps on a hash
  change. `matchRoute`'s own param extraction is already covered by the existing
  `router.test.ts`, and the route components' use of the prop is the other lane's
  slice.
- **Branch-only residue** in `ServerStatusBar.svelte` (line 30) and
  `ToastHost.svelte` (line 29): `[INFERENCE]` these are the transition/format
  helper branches inside the template that no reachable input distinguishes.
  I did not manufacture an input to move a number; they are left visible in the
  report rather than hidden behind an ignore comment.

### An environment trap worth knowing

`[FACT]` Svelte 5 implements every transition with the Web Animations API and
calls `element.animate(...)` directly. jsdom has no such method, so the `out:`
transition on a dismissed toast threw `TypeError: element.animate is not a
function` and **the element was never removed from the DOM** — the store updated
correctly while the card stayed on screen. `components-chrome.test.ts`
installs a small `Element.prototype.animate` stub that completes each animation
on a timer. It only supplies a method jsdom is missing; it does not change what
the component does, and it is what makes the real removal path observable
instead of assumed. Anyone writing the route-component tests will hit this too.

## 5. Defects found in `src/` — reported, not fixed

`src/**` is read-only for this lane, so all four are left for the owner.

### 5.1 `ErrorBoundary.svelte` cannot catch anything
`[FACT]` The file's header comment says it "Catches uncaught render exceptions
in its default slot". It does not. `lastError` is a `$state` that is only ever
assigned `null`, in `reset()`; no code path sets it to an `Error`. The
`{#if lastError}` card and the `reset` handler behind it are therefore
unreachable, which is why the file sits at 50% line coverage. Measured: passing
a child that throws makes `render()` itself throw, and nothing is left in the
document.

The inline comment at lines 26–35 actually states the opposite of the header —
"Svelte 5 has no built-in error boundary yet … let the route-level try/catch
decide". `[INFERENCE]` The component is a container with a dead error branch, and
the header comment is the misleading part. Either the header should stop
claiming to catch, or the boundary should be implemented (e.g. by catching in
the route component and rendering this fallback on demand). The first is a
comment fix; the second is a design decision.

### 5.2 `stores/graph.ts` discards the typed refusal kind
`[FACT]` `loadGraph`'s catch does `e instanceof Error ? e.message : String(e)`.
A Tauri `AppError` is not an `Error`, so a refusal such as
`{kind: 'GraphNotBuilt', ...}` becomes the string `"[object Object]"` and `kind`
is lost. The `worktree` store keeps `errorKind` and the `review` store keeps
`errorKind` and `errorMessage` precisely so the page can pick a localized remedy;
the graph page cannot. `[PROPOSAL]` add `errorKind: string | null` alongside
`error` and set it from `e.kind` when present.

### 5.3 `api/ai.ts` `listenReviewEvents` leaks listeners on a mid-flight failure
`[FACT]` Its docstring claims "a mid-flight failure still detaches what was
already open". It does not. The `await listen(...)` inside the `for` loop means a
rejection escapes before the `return`, so the closure that would have detached
the already-attached listeners is never created, and nothing else holds a
reference to them. The review store is the only caller and treats a rejected
subscribe as a fatal error state, so the leak lasts for the life of the page.
`[PROPOSAL]` wrap the loop in `try/catch` and call the same detach logic before
re-throwing.

### 5.4 `api/http.ts` discards its default headers when the caller supplies any
`[FACT]` The call is built as
`{ headers: { Accept, ...ContentType, ...init.headers }, ...init }` — the
`...init` spread comes **last**, so when the caller passes a `headers` key it
replaces the whole merged object and the `Accept: application/json` and
`Content-Type` are dropped. The inner `...(init.headers ?? {})` merge is
therefore dead code in exactly the case it was written for. A caller that sets
one custom header silently stops asking for JSON. `[PROPOSAL]` spread `...init`
first and let the explicit `headers` property override it.

### 5.5 (minor) `installMock()`'s idempotence check is fragile
`[FACT]` `src/mocks/handlers.ts` decides it is already installed by testing
whether the installed `invoke`'s own source text contains the substring
`"mock"`. A test-written stub whose body happens to contain that word — a
`model: 'mock'` fixture, say — is mistaken for the real mock, so the mock is
never reinstalled and every later case in the file silently talks to the stub.
This cost real debugging time here. The three API test files work around it with
`delete window.__TAURI_INTERNALS__` before `installMock()`, with a comment at
each site. `[PROPOSAL]` a module-level `let installed` flag, or a symbol
property on the installed function.

### 5.6 (cosmetic) Sidebar test ids carry a doubled prefix
`[FACT]` `Sidebar.svelte` renders `data-testid={`nav-${item.labelKey}`}` while
the label keys already begin with `nav.`, so the real ids are `nav-nav.repos`,
`nav-nav.vault` and so on. It works, but it is surprising to anyone writing a
test against it, and the cases here use the ids as they actually are rather than
the tidier spelling one would guess. Not worth a source change on its own.

## 6. How to reproduce

```powershell
cd D:\GitGit\.worktrees\gmd-stores\apps\gm-desktop
$env:PATH="E:\DevCache\node\bin;$env:PATH"
corepack pnpm@9.15.9 test            # 17 files, 287 tests, 0 skipped
corepack pnpm@9.15.9 test:coverage   # exits 1 on the global thresholds (expected)
```

Per-file numbers come from `coverage/coverage-summary.json`, produced by adding
`--coverage.reporter=json-summary` to the same vitest invocation. That flag was
used for reporting only; it changes no committed config and adds nothing to
`pnpm test`.
