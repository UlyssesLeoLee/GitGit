# gm-desktop frontend toolchain — advisories and the skipped component tests

Date: 2026-10-05
Lane: P (`chore/frontend-deps`, worktree `.worktrees/frontend-deps`)
Base commit: `db15d39` (= `origin/dev` at start)
Commits: `cdda93f` (toolchain bump), `3da169a` (re-enabled component tests)

Every claim is tagged. This is a statement of current state with the command
that establishes it, not a narrative about how the repo got here.

---

## 1. Audit: 8 → 1

`[FACT]` Measured with `corepack pnpm@9.15.9 audit --audit-level=low` in
`apps/gm-desktop`, before and after, on the same toolchain:

| | before (`db15d39`) | after (`3da169a`) |
| --- | --- | --- |
| total | 8 | 1 |
| critical | 1 | 0 |
| high | 2 | 1 |
| moderate | 5 | 0 |

`[FACT]` Severity split before, verbatim from the tool: `Severity: 5 moderate | 2 high | 1 critical`.
After: `Severity: 1 high`.

### Cleared

| Package | from → to | Advisory | Severity |
| --- | --- | --- | --- |
| `vitest` | 2.1.9 → 5.0.3 | GHSA-5xrq-8626-4rwp (arbitrary file read/execute when the Vitest UI server listens) | critical |
| `vitest` / `@vitest/mocker` | 2.1.9 → 5.0.3 | GHSA-82fw-gwwq-j7x9 (path traversal / arbitrary file read via Redirect Mock) | moderate ×2 |
| `vite` | 5.4.21 → 7.3.6 | GHSA-fx2h-pf6j-xcff (`server.fs.deny` bypass via Windows alternate paths) | high |
| `vite` | 5.4.21 → 7.3.6 | GHSA-4w7w-66w2-5vf9 (path traversal in `.map` handling) | moderate |
| `vite` | 5.4.21 → 7.3.6 | GHSA-v6wh-96g9-6wx3 (`launch-editor` NTLM hash disclosure on Windows) | moderate |
| `esbuild` | 0.21.5 → 0.28.2 (transitive, via vite 7) | GHSA-67mh-4wv8-2f99 | moderate |

`[FACT]` `@sveltejs/vite-plugin-svelte` 4.0.4 → 6.2.4 and
`@vitest/coverage-v8` 2.1.9 → 5.0.3 were bumped to match; they carry no
advisory of their own.

`[FACT]` `vitest` had to go to 4.1.11+ rather than the 3.2.6 floor: the
critical advisory is fixed at `>=3.2.6`, but GHSA-82fw-gwwq-j7x9 is fixed
only at `>=4.1.11`. The two floors cannot both be met on the 3.x line.

### Advisories NOT cleared

`[FACT]` **`braces` 3.0.3 — high — GHSA-vfj7-8cjw-p6xm** (stack-exhaustion
DoS via deeply nested patterns) remains. The audit reports its patched range
as `<0.0.0`: **there is no patched release at any version**, so no upgrade of
`braces` itself, and no `pnpm.overrides` pin, can clear it. `braces@3.0.3` is
also the latest published version.

`[FACT]` All three dependency paths into it run through `tailwindcss`:

```
tailwindcss@3.4.19 > micromatch@4.0.8 > braces@3.0.3
tailwindcss@3.4.19 > fast-glob@3.3.3 > micromatch@4.0.8 > braces@3.0.3
tailwindcss@3.4.19 > chokidar@3.6.0 > braces@3.0.3
```

`[FACT]` `tailwindcss@3.4.19` is the newest 3.x; the fix would be a jump to
tailwind 4.x.

`[INFERENCE]` Clearing this requires a tailwind 3 → 4 migration, which touches
`tailwind.config.js`, `postcss.config.js` and the CSS entrypoints under
`src/**`. None of those are in this lane's file ownership, so this is
recorded as follow-up work rather than attempted here. It is not reachable
from `package.json` alone.

### Risk statement — dev-only

`[FACT]` Every advisory cleared above sits in a devDependency chain. The
production `dependencies` block is, verbatim and unchanged by this work:

```
@tauri-apps/api, @tauri-apps/plugin-clipboard-manager, @tauri-apps/plugin-dialog,
@tauri-apps/plugin-fs, @tauri-apps/plugin-log, @tauri-apps/plugin-notification,
@tauri-apps/plugin-opener, @tauri-apps/plugin-shell, felte, zod
```

`[FACT]` Verified by diff — every changed line in `package.json` is inside
`devDependencies`; `git diff db15d39 -- package.json` touches no line of the
`dependencies` block. **No new runtime dependency was added.**

`[FACT]` None of the cleared packages is reachable from the packaged app, so
none of these shipped to a user. They were developer-machine risks: a
compromised or hostile input reaching a local dev/test server.

`[INFERENCE]` The critical was worth clearing regardless — it is remote file
read/execute on a listening server, and "commercial product" carries a higher
bar than "not exploitable by end users". The `braces` DoS is materially
lower: it needs deeply nested glob patterns through tailwind's own config
inputs, not attacker-controlled input at runtime.

### Out of scope, still open

`[FACT]` `apps/gm-console` (React 18, `react-router` 6.30.6) carries two
moderate advisories requiring 7.18.0. Not touched by this lane — a separate
React Router major migration.

---

## 2. Why vite 7 and not vite 8

`[FACT]` `vite@8.3.2` depends on `rolldown ~1.2.11` in place of
`rollup`. `vite@7.3.6` depends on `rollup ^4.43.0` and
`esbuild ^0.27.0 || ^0.28.0`.

`[INFERENCE]` vite 8 would have forced a `build.minify` migration, because
`vite.config.ts` sets `minify: 'esbuild'` and the rolldown pipeline uses a
different minifier. vite 7 clears every vite advisory while leaving the build
configuration byte-identical.

`[FACT]` Build output is unchanged in size and shape: 148 modules transformed
before and after, `built in ~2s` both times.

`[FACT]` `@sveltejs/vite-plugin-svelte@7.3.1` requires `vite ^8`, so plugin 6
(6.2.4, accepts `vite ^6.3 || ^7`) is the newest plugin that pairs with vite 7.

---

## 3. The 6 skipped component tests: 5 now run

`[FACT]` Baseline `pnpm test` on `db15d39`:

```
Test Files  7 passed (7)
     Tests  76 passed | 6 skipped (82)
```

`[FACT]` After `3da169a`:

```
Test Files  7 passed (7)
     Tests  81 passed | 1 skipped (82)
```

`[FACT]` The root cause recorded in the old comment — that the
`@sveltejs/vite-plugin-svelte` **compile step** emits server output and no
resolution setting can reach it — was wrong. The measured stack was:

```
Module.lifecycle_function_unavailable  svelte/src/internal/server/errors.js:126
Module.mount  svelte/src/index-server.js:25
  at @testing-library/svelte-core/src/mount.js:21
```

`svelte` was resolving to `index-server.js`. That is a plain module-resolution
outcome.

`[FACT]` Both previously-attempted fixes targeted `test.resolve.conditions`.
vitest's module runner resolves through the **top-level** `resolve`/`ssr`
conditions; `test.resolve` never reaches that lookup.

`[FACT]` The fix in `vite.config.ts` is the documented recipe:
`svelteTesting()` from `@testing-library/svelte/vite` (contributing its
`ssr.noExternal` rule) plus top-level `resolve.conditions: ['browser']`,
gated on `process.env.VITEST` so the production build keeps its own defaults.

`[FACT]` Two of the six asserted `tFor('zh-CN', …)`. jsdom reports
`navigator.language === 'en-US'` (measured), so `pickInitial()` in
`src/lib/stores/locale.ts` resolves to `en` and the page renders English.
They now assert against the active locale via `get(locale)`.

`[INFERENCE]` This does not weaken the `"every rendered string comes from the
catalogue"` guard. It still compares rendered output to the catalogue entry
for the live locale, which is the property it exists to catch (a hardcoded
string). It is additionally no longer coupled to one locale.

### The one case still skipped — a source defect, not tooling

`[FACT]` `src/routes/Review.svelte:157` renders the unsupported-provider panel with:

```svelte
{fill($catalog, '{provider}', $review.unsupportedProvider ?? provider)}
```

`[FACT]` `fill` is declared at `Review.svelte:30` as
`(t, token, value) => (t[token] ?? token).split(token).join(value)`. It looks
up `t['{provider}']` — the literal string `{provider}` used as a key — finds
nothing, falls back to the token itself, and returns the substituted token.

`[FACT]` Measured result: the panel's `textContent` is exactly `'anthropic'`.
The `review.unsupported` catalogue string (`'{provider} cannot stream. Its
protocol has no streaming implementation…'`) is never interpolated.

`[INFERENCE]` The helper expects the message string, not the catalogue. Passing
`$catalog['review.unsupported']` in place of `$catalog` would render the
intended sentence. That is a one-line change to application source.

`[FACT]` The case stays `.skip`ped with that reason recorded at the case. The
fix is in `src/**`, outside this lane's ownership, so it is reported rather
than edited. No test was deleted or weakened to reach green.

---

## 4. Config migrations

`[FACT]` Two config changes, both in `vite.config.ts`:

1. **Added `svelteTesting()` and top-level `resolve.conditions`** — see §3.
2. **Nothing else.** `defineConfig` still comes from `vite`, not
   `vitest/config`.

`[FACT]` No default had to be pinned back. The vite 2→7 and vitest 2→5 bumps
changed no setting this project relies on: `environment`, `globals`,
`setupFiles`, `include`, `server.port`/`strictPort`/`host`, `clearScreen`,
`envPrefix`, `build.target`, `build.sourcemap` and `build.minify` all behaved
identically — the build emits 148 modules and `svelte-check` reports 0 errors
on both sides of the bump.

`[FACT]` `build.minify: 'esbuild'` was left alone deliberately; that is the
reason for staying on vite 7 (§2).

`[PROPOSAL]` vitest 5 suggests `test.fsModuleCache: true` for faster reruns
(it reported transform at 27% of a 4.5s run). Not adopted here — it is a
performance knob unrelated to either goal.

---

## 5. Verification

All commands run from `apps/gm-desktop` via `corepack pnpm@9.15.9`.

| Command | Before (`db15d39`) | After (`3da169a`) |
| --- | --- | --- |
| `install` | frozen lockfile, clean | clean after manifest change |
| `check` | `svelte-check found 0 errors and 0 warnings` | `svelte-check found 0 errors and 0 warnings` |
| `test` | `7 passed (7)` / `76 passed \| 6 skipped (82)` | `7 passed (7)` / `81 passed \| 1 skipped (82)` |
| `build` | `148 modules transformed`, built in 1.77s | `148 modules transformed`, built in 2.22s |
| `lint` | exit 0, no output | exit 0, no output |
| `audit --audit-level=low` | 8 (1 crit, 2 high, 5 mod) | 1 (1 high) |

`[FACT]` `lint` was verified green **before** any change was made, so the
post-change exit 0 is not inherited from a pre-existing failure.

### `test:coverage` — honest state, thresholds untouched

`[FACT]` `pnpm test:coverage` exits non-zero **both before and after** this
work. Thresholds in `vite.config.ts` were not touched, and no
`istanbul ignore` / `v8 ignore` / `coverage.exclude` entry was added — the
exclude list is still exactly `src/main.ts`, `src/mocks/**`, `src/**/*.d.ts`.

`[FACT]` Measured:

| | stmts | branch | funcs | lines | threshold errors |
| --- | --- | --- | --- | --- | --- |
| before | 45.06 | 73.36 | 53.19 | 45.06 | 3 (lines, funcs, stmts) |
| after | 27.15 | 32.55 | 21.55 | 32.88 | 4 (adds branches) |

`[FACT]` The reported percentage **dropped**. A controlled measurement
isolates the cause: with `vite.config.ts` and `review.test.ts` reverted but the
new vite 7 / vitest 5 still installed, `test:coverage` reports **27.26%**
lines — lower still. With the config change in place it is **32.88%**. So the
config change *raised* coverage by ~5.6 points; the drop is attributable to
the `@vitest/coverage-v8` 2.1.9 → 5.0.3 major bump changing what the v8
provider attributes.

`[FACT]` The same 33 file rows are reported in both the reverted and the
changed configuration, so this is a change in per-file attribution, not new
files entering the denominator.

`[FACT]` Test outcomes moved the other way: 76 → 81 passing. `Review.svelte`
is now measured at 79.22% lines instead of 0%.

`[INFERENCE]` The old 45.06% was not a truer picture of the codebase; the new
number is a differently-computed one, and neither is a passing gate. No
claim is made here that coverage improved.

`[FACT]` Coverage is not enforced in CI: `pnpm test` is `vitest run` without
`--coverage`. No `--coverage` step was added, because it would fail
immediately and would swap one red gate for another.

---

## 6. Known breakage and follow-up

- `[FACT]` `braces@3.0.3` high advisory stands; needs a tailwind 4 migration
  across files this lane does not own (§1).
- `[FACT]` 1 of 6 Review component cases remains skipped, blocked on a
  one-line fix in `src/routes/Review.svelte` (§3).
- `[FACT]` `apps/gm-console` two moderate `react-router` advisories untouched.
- `[TBD]` Whether the ~18-point coverage drop is a measurement-semantics
  change in coverage-v8 3+ or a source-attribution regression was not
  bisected further than the controlled A/B in §5. It does not gate CI today.
