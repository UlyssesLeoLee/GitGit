# gm-desktop — repairing and extending the route component tests

**Date:** 2026-10-05
**Branch:** `test/gmd-coverage` (worktree `D:\GitGit\.worktrees\gmd-coverage`)
**Base commit:** `d63b8c4` ("fix(gm-desktop): the graph detail pane never rendered, for any node")
**Lane:** the route half of the app's test debt — `src/routes/**` and `src/App.svelte`.
A disjoint lane (`test/gmd-stores`, report `2026-10-05-gmd-coverage-stores`) owns
`src/lib/stores/**`, `src/lib/api/**` and `src/lib/components/**`.

No file under `apps/gm-desktop/src/**` was modified. No coverage threshold was
lowered, deleted or relaxed; no `v8 ignore` / `istanbul ignore` comment and no new
`coverage.exclude` glob was added. No test was deleted, skipped, `.only`-ed or
weakened. `--coverage` was **not** added to `pnpm test`.

---

## 1. Result

`corepack pnpm@9.15.9 test` — **25 files, 378 tests passed, 0 failed, 0 skipped.**

| | files | tests |
|---|---|---|
| as handed over | 18 | 312 (304 passed / **8 failed**) |
| after this lane | 25 | **378** (+66) |

`corepack pnpm@9.15.9 check` — **0 errors, 0 warnings.**
`corepack pnpm@9.15.9 lint` — **clean, exit 0.**

`corepack pnpm@9.15.9 test:coverage` now **exits 0**. It did not before: the
thresholds declared in `vite.config.ts` (lines/statements 70, functions 60,
branches 55) were not met by any run of this suite. The gap the thresholds
described — a gate that no job evaluates — is now closable from the app side;
switching CI over to `test:coverage` is a one-line change in a workflow this lane
does not own, and is listed as follow-up work in §6.

---

## 2. The eight failures in `routes-graph.test.ts`

`[FACT]` All eight were defects in the test file. Five were wrong queries, one
was a wrong expectation about how many lines are drawn, one was a wrong DOM
scope, and one was a missing flush. The graph fix in `d63b8c4` is genuine and
the DOM it produces is correct; only the assertion aimed at it was wrong.

| # | Case | What it actually was | Fix |
|---|---|---|---|
| 1 | `reports a failure with the backend message and recovers on retry` | `findByText((el) => el.textContent?.includes(…))`. A Testing Library text matcher is invoked as `matcher(text, element)` — `el` is the node's **own text string**, so `el.textContent` is `undefined` and the predicate is `false` for every node in the document. The banner was rendered the whole time. | `findErrorBox()` matches on the node's own text (`${prefix}:`), which resolves to exactly one element. Both the prefix and the backend message are still asserted, and the retry is still exercised. |
| 2 | `survives a non-Error rejection by stringifying it` | Same query bug. | Same helper. |
| 3 | `colours each node from its kind and falls back for a kind it has no entry for` | jsdom normalises a hex colour written into an inline `style`: the component's `style="background: #3b82f6"` reads back as `"background: rgb(59, 130, 246);"`. | `rgbOf(hex)` normalises the *expectation* the same way, still derived from the palette, so a change to `nodeColor()` fails the case. Added `fill` assertions for the SVG circles, whose attribute spelling is not rewritten. |
| 4 | `gives a known edge kind its own colour and mutes one it does not name` | Expected 2 muted lines, measured 1. **A test bug, not a double render.** The payload carries four edges but only three are drawn: `supersedes` targets `GHOST-0001`, which is not in the node set, so the `{#if a && b}` guard in `Graph.svelte:217` drops it. Of the three drawn lines, `tolerates` is the only kind `edgeColor()` does not name. The old expectation counted the undrawn edge. | Asserts three lines, the two named hues, and exactly one default. |
| 5 | `fills the detail pane from a click in the master list and marks that row` | The body paragraph is a **sibling** of the `<header>` (`Graph.svelte:304-308`, header closes at 302), so `within(header)` cannot see it. The `d63b8c4` fix works; the assertion was scoped to the wrong element. | Body asserted against the detail pane, plus a negative assertion that the header did **not** absorb it — so a future refactor that merges the two blocks fails the case instead of passing quietly. |
| 6 | `navigates from a connection row to the node on the other end` | Two mistakes. `getByText(cat('graph.outgoing'))` never matches: the `<h4>`'s own text is `Outgoing (1)` (label plus count) and `getByText` with a string is an exact match. And `parentElement.parentElement` from the heading lands on the grid, which holds **both** columns, so the id query was ambiguous. | `connectionColumn(label)` locates the heading by predicate and returns its own column. |
| 7 | `puts the edge note in a connection row tooltip, and nothing when there is no note` | `getByText(REQ.id)` matched three elements: the master-list row, the SVG `<text>` label, and the connection row. | Scoped to the column, and each row is additionally checked for its edge-kind text (`implements` / `derived from`) so a wrong-row match cannot read green. |
| 8 | `re-renders the page in the other locale rather than caching a translation` | `setLocale` writes a store; the assertion read the DOM on the next line, before Svelte flushed. | `waitFor` on a re-queried heading, in both directions, so a cached (non-reactive) translation still fails. |

`[FACT]` A ninth fix was needed in the same file, unrelated to the eight:
`pnpm check` failed on it, because the draft's `circles()` helper returned
`Element[]` typed as `SVGCircleElement[]` (`querySelectorAll` with a descendant
selector is not narrowed). The helper was also dead — and its filter kept both
circles of a node rather than dropping the backdrop. It was replaced with a typed
`nodeCircle(id)` helper, which is now used to pin the selected node's white
`stroke` / `stroke-width: 2` in the visualization.

---

## 3. Coverage

`[FACT]` Both columns were measured in this worktree with vitest 5.0.3 (verified
via `pnpm list vitest --depth 0`; a stale `node_modules` here silently runs
2.1.9 and produces different numbers). The "before" column is **not** a prior
run's log: it was measured by running the 18 pre-existing test files — the same
18 files as the full run, minus the 7 this lane added — with the eight failures
above already repaired. That isolates the effect of the coverage extension from
the effect of the repairs.

### `All files`

| Metric | Before | After | Delta | Threshold |
|---|---|---|---|---|
| Statements | 63.50% (1220/1921) | **93.23%** (1791/1921) | +29.73 pts | 70 ✅ |
| Branches | 65.62% (460/701) | **78.03%** (547/701) | +12.41 pts | 55 ✅ |
| Functions | 69.21% (272/393) | **93.63%** (368/393) | +24.42 pts | 60 ✅ |
| Lines | 67.10% (873/1301) | **93.69%** (1219/1301) | +26.59 pts | 70 ✅ |

Before, `test:coverage` exited non-zero with two threshold errors (lines,
statements). After, it exits 0.

### `src/routes/*` and `src/App.svelte`

| File | Before (stmts / branch / funcs / lines) | After (stmts / branch / funcs / lines) |
|---|---|---|
| `Home.svelte` | 0% (lines 21-137) | **98.98 / 94.11 / 100 / 98.33** |
| `Repos.svelte` | 0% (lines 17-93) | **100 / 100 / 100 / 100** |
| `RepoDetail.svelte` | 0% (lines 25-144) | **99.08 / 83.33 / 100 / 98.5** |
| `Settings.svelte` | 0% (lines 23-139) | **100 / 100 / 100 / 100** |
| `Vault.svelte` | 0% (lines 38-187) | **100 / 81.25 / 100 / 100** |
| `NotFound.svelte` | 0% (lines 11-16) | **100 / 100 / 100 / 100** |
| `Graph.svelte` | 96.7 / 66.66 / 98.18 / 94.11 *(15 of 23 cases passing)* | 96.7 / 66.66 / 98.18 / 94.11 *(23 of 23)* |
| `App.svelte` | 0% (lines 37-105) | 18.18 / 0 / 12.5 / 22.58 — **see §4.1** |

`[FACT]` `Graph.svelte`'s percentages are unchanged because the same 23 cases
cover the same lines; what changed is that 8 of them now run. Its remaining gaps
are `86-92, 94-95` and `263` as reported by the v8 text table.
`[INFERENCE]` `263` is the legend swatch, which is the one `nodeColor()` call
whose kind is not in the fixture set; `86-95` are the derived declarations. Not
measured line by line, so not claimed.

### What each new file covers

| File | Tests | Subject |
|---|---|---|
| `routes-harness.ts` | — | Shared invoke recorder, store reset, animation stub, `cat` / `t`, hash navigation. Not a test file. |
| `routes-home.test.ts` | 13 | start/stop exclusivity, the bind lock, pid/uptime/port and their em dashes, the three `friendlyError` shapes, log-tail re-reads |
| `routes-repos.test.ts` | 12 | grid rendering, the `{n}` count, client-side filtering, "no data" vs "no repositories", both per-card commands, refused copy / refused file-manager launch, a failed refresh keeping the list |
| `routes-repo-detail.test.ts` | 11 | loading / not-found / loaded, refs grouped by family with placeholders, commits, the branch-graph prefixes, the back link, the clone-URL button (and its no-op before load) |
| `routes-settings.test.ts` | 10 | admin-password status both ways, the empty-password refusal, save / clear with a status re-read, the about block present and absent, the three hosted controls |
| `routes-vault.test.ts` | 12 | empty state, the three row actions, the masked value field, the write refusal, expand/collapse, the default diff pair, restore, rotate, delete asked-then-confirmed |
| `routes-notfound.test.ts` | 5 | the 404 page itself, and reaching it through the real `Router`'s catch-all |
| `routes-app.test.ts` | 3 | documents the mount failure and its cause (§4.1) |

---

## 4. Product defects found, reported and not fixed

`src/**` is read-only for this lane, so each of these is left in place with the
evidence that found it. Every one was reproduced by a test, and each of those
tests pins today's behaviour with a comment saying a fix has to be a deliberate
change to it.

### 4.1 `src/App.svelte:70-72` — the root component cannot mount at all

```svelte
<svelte:head>
  <html lang={$locale} data-theme={$theme}></html>
</svelte:head>
```

`[FACT]` `render(App)` throws `TypeError: Cannot read properties of null
(reading 'cloneNode')`. The chain, each step measured:

1. `svelte/compiler` 5.57.1 compiles that block to
   `var root = $.from_html(`<html></html>`)`.
2. `from_html` does `node = create_fragment_from_html('<html></html>')`, which is
   `template.innerHTML = '<html></html>'`, then
   `node = get_first_child(node)`.
3. `template.innerHTML = '<html lang="en"></html>'` produces **0** child nodes
   (`<div lang="en"></div>` produces 1). An `<html>` start tag inside template
   content is *ignored* by the HTML parser — "in template" insertion mode, HTML
   spec, not a jsdom quirk.
4. `node` is `null`, so `node.cloneNode(true)` throws.

`[FACT]` This is not specific to jsdom: the Tauri WebView runs the same parser,
so the shell fails to mount in the packaged app too. Nothing is left in the
document when it throws (`routes-app.test.ts` asserts that), but there is no UI
and no error message.

`[FACT]` Consequence for this lane: `App.svelte` cannot be covered above 22.58%
of lines, because no test can mount it. Untested as a result: the boot spinner,
the order of the three pre-warm calls, the route table, and the boot-failure
card. `matchRoute`'s own fallback and param capture are already covered by
`router.test.ts`, and `NotFound` is covered through the real `Router` in
`routes-notfound.test.ts`.

`[PROPOSAL]` The fix belongs in `App.svelte`: drop the `<html>` element from
`<svelte:head>` and set `lang` / `data-theme` on `document.documentElement` from
the existing theme subscription (`theme.ts:33-35` already does the `data-theme`
half). A `<svelte:head>` block cannot carry `<html>` in any Svelte 5 version.

### 4.2 `src/routes/RepoDetail.svelte:75` — a stray `}` in the not-found message

`[FACT]` The expression is closed twice:

```
{$catalog['repos.notFound'].replace('{name}', params?.name ?? '')}}
```

Svelte compiles the first `}` as the end of the expression and emits the second
as literal text, so the page reads **"Repository ghost not found}"**.

### 4.3 `src/routes/RepoDetail.svelte:61-63` — "Loading…" and "not found" at once

`[FACT]` The `<h1>` is `{#if detail}{detail.name}{:else}{$catalog['common.loading']}{/if}`.
`detail` is `null` on the not-found path, so the heading keeps reading
"Loading…" while the card below says the repository was not found.

### 4.4 `src/routes/RepoDetail.svelte:41` — the clone URL hardcodes a port

`[FACT]` `const url = \`http://127.0.0.1:38080/repos/${detail.name}.git\`` is
built in the component, while `Repos.svelte` asks the backend through
`clone_url` (which takes the server's real bind address). The detail page's copy
button is therefore wrong for any install not bound to 38080.

### 4.5 `src/lib/stores/vault.ts:35` — the version column is a list position

`[FACT]` `refreshVault` builds each entry with `version: out.length + 1`, i.e.
the key's index in `vault_list`, not the vault's latest version for it. The
seed credential has versions 1 and 2 in the mock and the collapsed cell reads
`v1`.

### 4.6 `src/routes/Vault.svelte:79,86` — restore and rotate do not refresh the list

`[FACT]` Both handlers call `vaultVersions(key)` after the write, and that
function answers from `vaultVersionsCache` when the key is already cached
(`vault.ts:53-54`) — which expanding the row just did. The version list
therefore keeps showing the pre-change history until the page is remounted.
Measured: after restoring a one-version key, the expanded list still shows one
row.

### 4.7 `src/routes/Home.svelte:29-36` — one busy flag for every log read

`[FACT]` `refreshLogs` sets `refreshing = true` and clears it in a `finally`.
Two overlapping calls share the one flag, so the older call's `finally` — which
runs last — clears the lock the newer call is relying on. Measured: clicking
Refresh while the mount-time read is still in flight re-enables the button while
the second read is pending.

### 4.8 Hardcoded English strings that bypass the catalogue

`[FACT]` `[TBD]` owner decision needed on whether these are bugs or accepted
debt. Present in: `Vault.svelte:40` (`key + value required`), `:78`
(`restored {key} → v{n}`), `:85` (`rotated {key} → v{n}`), `:93`
(`deleted {key}`); `Settings.svelte:41` (`empty password`), `:49` (`updated`),
`:57` (`cleared`). Separately, `NotFound.svelte:13` reuses `common.empty`
("No data") as the 404 message — the tests assert that, since it is what the
component really renders, but it is the one string on that page that tells the
user nothing about what went wrong.

---

## 5. Notes on how the tests were written

`[FACT]` Four traps in this suite produced false failures, and each is now
commented at the point of use so the next agent does not rediscover it:

- **A text matcher takes text, not an element** (case 1 above). `matcher(text,
  element)`.
- **`<script>`-side `t()` is not reactive**, but a template expression that
  *reads* a store is — `t('common.close')` inside `{#if openKey === s.key}` does
  update, because `openKey` is the tracked dependency.
- **The shipped mock's `STORE` lives for the whole test file.** A case that
  really starts a server leaves every later case's `server_status` answering
  `running: true`, and one that really adds a credential changes what the table
  shows. The affected cases answer from a local fixture instead, with a comment
  saying why.
- **jsdom normalises colours in `style` but not in attributes.** `style="…#3b82f6"`
  reads back as `rgb(59, 130, 246)`; `fill="#3b82f6"` does not.

`[FACT]` `Element.prototype.animate` is stubbed in `routes-harness.ts` with the
same approach as `components-chrome.test.ts` (a copy, not an import — a test
file cannot be imported without running its cases twice, and that file belongs
to another lane). Svelte 5 implements every transition with the Web Animations
API, which jsdom lacks.

`[FACT]` Two behaviours are pinned **as defects** rather than as intent, with
the reason in the case: the stray `}` in the not-found message (§4.2) and the
stale version list after a restore (§4.6). Both will fail when the underlying
defect is fixed, which is the intended signal.

---

## 6. Follow-up (not done here)

1. `[PROPOSAL]` Fix `App.svelte`'s `<svelte:head>` (§4.1), then add the shell
   tests this lane had to leave out: boot spinner, pre-warm order, route table,
   boot-failure card.
2. `[PROPOSAL]` Fix §4.2-§4.4 in `RepoDetail.svelte` (one line, one template
   branch, one call to the existing `clone_url` helper).
3. `[PROPOSAL]` Fix §4.5-§4.6 in the vault store: take the version from the
   backend, and let `vaultVersions` re-read after a write.
4. `[PROPOSAL]` Switch the `gm-desktop` CI job from `pnpm test` to
   `pnpm test:coverage`. `vite.config.ts` records that the thresholds were never
   evaluated by any job; they are now met with margin (lowest: branches 78.03%
   against a 55 threshold), so the gate can be made real. That workflow file is
   outside this lane.
5. `[TBD]` Whether the hardcoded English strings in §4.8 are accepted debt.

---

## 7. Validation

Every command below was run in `D:\GitGit\.worktrees\gmd-coverage\apps\gm-desktop`
with `corepack pnpm@9.15.9`, after `pnpm list vitest --depth 0` confirmed
**vitest 5.0.3**.

| Command | Result |
|---|---|
| `pnpm list vitest --depth 0` | `vitest 5.0.3` |
| `pnpm test` | 25 files, **378 passed**, 0 failed, 0 skipped |
| `pnpm check` | 0 errors, 0 warnings |
| `pnpm lint` | clean, exit 0 |
| `pnpm test:coverage` | exit **0**; All files 93.23 / 78.03 / 93.63 / 93.69 |
| `pnpm test:coverage` (18 pre-existing files only) | exit non-zero, 2 threshold errors; All files 63.50 / 65.62 / 69.21 / 67.10 |

The two scratch files left by the previous agent (`tests/unit/zz-scratch.test.ts`,
`tests/unit/zz-probe.test.ts`) and its other untracked scratch output
(`.tmp-graph.txt`, `.tmp-scratch.txt`, `.cov-base.txt`, `.pnpm-install.txt`) were
removed with the recoverable-delete path; `git status` now lists only the files
this lane owns.
