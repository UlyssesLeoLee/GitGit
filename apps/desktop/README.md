# GitGit Desktop

> **Local-first engineering knowledge graph viewer** for the AI-Native Engineering Platform
> (`UlyssesLeoLee/GitGit` repo). Tauri 2 + Svelte 5 + Vite.

This is the desktop incarnation of the GitGit project — the same engineering knowledge graph
the platform's requirements book defines (Node / Edge / Event / Policy / View primitives),
but rendered in a native window that walks the repo's `docs/requirements/` directory directly,
no server required.

![GitGit Desktop](public/logo.svg)

## Why a desktop app

The `00-requirements-definition.md` document declares the platform **local-first** as a design
constraint checked from day one (§13 Principle 2). A desktop wrapper is the natural first
shippable surface: it has zero deployment cost for the user (download → run), survives
air-gapped use (no network round-trips), and is the lightest possible test of the platform's
"self-hostable, queryable graph" claim — at the cost of a single 19MB binary instead of a
full local stack (Postgres + Gitaly + Sidekiq + agent sandboxes, see Phase 11 red-team §6).

The Phase 11 red-team explicitly flags the platform's MVP-era self-hosting footprint as a gap
("MVP is permitted to ship self-hosted with *zero documented resource footprint claim at all*").
This desktop app is the answer to that gap *for the viewer surface specifically*: the graph
reader is a tiny Rust + WebView binary, not a server.

## What it does

1. **Walks the repo's `docs/requirements/` directory** at launch.
2. **Parses every Markdown file** extracting:
   - Requirement IDs: `REQ-NNN`, `PREFIX-REQ-NNN` (GRF / AGT / OPS / UX / CI / NFR / SEC / etc.)
   - ADR IDs: `ADR-NNNN`
   - RGS gate IDs: `RGS-IMPL-NNN`
   - Cross-doc references like `./phase10-architecture.md`
   - Section anchors (`## N. Title`)
   - Tag counts (TBD mentions, Phase markers)
3. **Builds the engineering knowledge graph** (Phase 6 primitives) — Nodes for docs /
   requirements / ADRs, Edges for `references` (doc→req, doc→doc) and explicit relational
   statements (`X implements Y`, `X depends on Y`, `X supersedes Y`, etc.).
4. **Renders** the graph in a 3-pane window: searchable list of nodes, SVG overview, and a
   detail pane with incoming/outgoing edges.
5. **Reads-only by design** — the platform itself owns write semantics. The desktop app is a
   viewer, not an editor, until the platform's write API lands.

## Live parse numbers (from the bundled docs)

```
requirements extracted: 432
adrs extracted:         0  (no ADR markers in the requirements text — they're referenced
                             by phase number instead)
rgs-impl extracted:     0
TBD mentions:           191

Graph:
  nodes: 137  (16 documents + 71 requirements + 35 policies + 15 agents)
  edges: 440  (all `references` — the markdown rarely uses explicit
              "X implements Y" statements; the platform's graph is
              reconstructed from cross-references instead)

Kinds (prefix → node kind):
  AGT-REQ-xxx  → agent
  OPS-REQ-xxx  → policy
  UX-REQ-xxx   → policy
  CDX-REQ-xxx  → policy
  CI-REQ-xxx   → policy
  NFR-REQ-xxx  → policy
  SEC-REQ-xxx  → policy
  REQ-xxx      → requirement
  ADR-xxxx     → adr
  RGS-IMPL-xxx → adr
  DOC:...      → document
```

## Tech stack

| Layer        | Choice                                  | Why                                                                                  |
| ------------ | --------------------------------------- | ------------------------------------------------------------------------------------ |
| Shell        | **Tauri 2.x**                           | Native WebView (no Electron weight), Rust backend, single binary                     |
| Frontend     | **Svelte 5** + **Vite 6**               | Tiny output, reactive stores (`$state`, `$derived`), excellent DX                     |
| Language     | **TypeScript** + **Rust 1.77+**         | Strict types across the IPC boundary                                                |
| Build time   | **Node 20+**, **Cargo 1.77+**           | Standard                                                                             |
| Distribution | Single `.exe` (~19MB debug, ~8MB release) | No installer required — perfect for the local-first principle                      |

### Why not Electron, native Rust, or a PWA?

- **Electron** ships a full Chromium (~150MB, ~80MB memory). Phase 11 flags this exact
  failure mode for the platform.
- **Pure native Rust UI** (egui / iced) would skip WebView but lose rapid iteration on the
  graph-rendering side, and Svelte's terse reactivity is genuinely a productivity win.
- **PWA** would require a browser, breaking the "single binary" principle.

Tauri splits the difference: a real native binary, a real WebView renderer, and Svelte's
ergonomics for the UI.

## Project layout

```
apps/desktop/
├── package.json             # Svelte/Vite/Tauri CLI dependencies
├── vite.config.ts           # Tauri-aware Vite config (port 5174, strict, envPrefix)
├── tsconfig.json            # strict TS, ESNext target
├── index.html               # SPA entry
├── public/
│   ├── logo.svg             # App icon
│   └── requirements/        # Bundled fixtures (web-build only)
│       ├── 00-requirements-definition.md
│       ├── phase{1-15}-*.md
├── scripts/
│   └── smoke.mjs            # Node smoke test — runs the parser against live docs
├── src/
│   ├── main.ts              # mount(App)
│   ├── App.svelte           # top-level layout
│   ├── styles.css           # dark / light theme
│   ├── components/
│   │   ├── Topbar.svelte    # brand, search, stats, source/os/version badges
│   │   ├── Sidebar.svelte   # scrollable node list (filtered by query)
│   │   ├── GraphView.svelte # SVG overview, kind-coloured nodes, hover-to-focus
│   │   └── Detail.svelte    # selected node + incoming/outgoing edges
│   └── lib/
│       ├── api.ts           # Tauri runtime detection + dynamic invoke import
│       ├── store.ts         # Svelte writable store + derived stores + loadGraph()
│       ├── types.ts         # GraphNode / GraphEdge / NodeKind / EdgeKind
│       └── parser.ts        # parseDoc + docsToGraph + kindOf + scoreNode
└── src-tauri/
    ├── Cargo.toml           # Tauri 2.x, walkdir, regex, chrono, anyhow
    ├── tauri.conf.json      # product name, window 1280x820, identifier, security CSP
    ├── build.rs             # tauri_build::build()
    ├── capabilities/
    │   └── default.json     # default Tauri 2.x capability set
    ├── icons/
    │   ├── icon.png         # 512x512 generated PNG
    │   ├── icon.ico         # Windows .ico (PNG-in-ICO format)
    │   ├── 32x32.png        # platform icon variants
    │   ├── 128x128.png
    │   └── 128x128@2x.png
    ├── scripts/
    │   └── gen-icons.cjs    # generates the icon set from a single PNG
    └── src/
        ├── main.rs          # exe entry, sets windows_subsystem on release
        ├── lib.rs           # Tauri Builder + invoke_handler registration
        ├── commands.rs      # Tauri IPC commands (graph_load, docs_list, etc.)
        └── graph.rs         # Rust equivalent of parser.ts (parses the same docs)
```

## Web build vs desktop build

This project intentionally compiles to **both** a web bundle and a Tauri desktop binary from
the same source. The trick lives in `src/lib/api.ts`:

```ts
declare global {
  interface Window { __TAURI_INTERNALS__?: unknown; }
}

export const isTauri = (): boolean =>
  typeof window !== 'undefined' && typeof window.__TAURI_INTERNALS__ !== 'undefined';

export async function webInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (isTauri()) {
    const mod = await import('@tauri-apps/api/core');
    return mod.invoke<T>(cmd, args);
  }
  // Web fallback: bundled mock data, fetched from /requirements/
  return mockFor(cmd, args);
}
```

The desktop branch uses a **dynamic** `import('@tauri-apps/api/core')` — Rollup never sees it
on the web branch (which has `@tauri-apps/api` in `optionalDependencies` so the dependency is
optional). This is the same pattern that keeps the codebase shipping as both Vercel-static
and Tauri-native without breaking either target.

## Getting started

### Prereqs

| Tool   | Min version | Check                |
| ------ | ----------- | -------------------- |
| Node   | 20.x        | `node -v`            |
| Rust   | 1.77+       | `rustc --version`    |
| WebView2 | any recent | pre-installed on Win 10+ |

### Install

```bash
cd apps/desktop
npm install --proxy <corp-proxy> --https-proxy <corp-proxy> --strict-ssl false
```

> ⚠️ If you're behind a corp HTTPS proxy that npm doesn't pick up automatically, pass it
> explicitly via `--proxy` / `--https-proxy`, or set it in `~/.npmrc`. EPEM/ENOTEMPTY errors
> on this step usually mean an AV scanner is locking `node_modules` — `rm -rf node_modules`
> and retry.

### Run (desktop)

```bash
# Debug binary that opens a real window
cargo build --manifest-path src-tauri/Cargo.toml
../src-tauri/target/debug/gitgit-desktop.exe   # Windows
../src-tauri/target/debug/gitgit-desktop       # Unix

# Or with HMR (dev server + Tauri shell):
npx tauri dev
```

### Run (web preview)

```bash
npm run build           # produces dist/
npm run preview         # serves dist/ on http://localhost:5174/
```

The web build uses a bundled fixture (the same Markdown files from `docs/requirements/`,
copied into `public/requirements/`). The desktop build reads the real repo path.

### Run tests

```bash
# Rust unit tests
cargo test --manifest-path src-tauri/Cargo.toml

# JS smoke test — runs the parser against the live docs and prints graph stats
node scripts/smoke.mjs
```

## Where it reads from (desktop runtime)

The Tauri backend (`src-tauri/src/graph.rs`) looks for the requirements folder in this order:

1. `$GITGIT_DOCS` env var (explicit override)
2. `<cwd>/../../../docs/requirements` (when launched from `src-tauri/`)
3. `<exe-dir>/docs/requirements` (when launched from an installed binary)
4. `$HOME/GitGit/docs/requirements` (default user profile clone)

To point the app at a different requirements folder:

```bash
GITGIT_DOCS=D:/my-fork/requirements gitgit-desktop.exe
```

## Tauri IPC commands

The Svelte frontend calls these Tauri commands; the Rust handlers live in
`src-tauri/src/commands.rs`:

| Command          | Args           | Returns                                | Purpose                                  |
| ---------------- | -------------- | -------------------------------------- | ---------------------------------------- |
| `graph:load`     | —              | `{nodes, edges, docs}`                 | Full graph from `docs/requirements/`     |
| `graph:list_nodes` | —            | `GraphNode[]`                          | Nodes only (for fast paint)              |
| `graph:list_edges` | —            | `GraphEdge[]`                          | Edges only                               |
| `graph:get_node` | `{id}`         | `GraphNode \| null`                    | Single-node lookup                       |
| `graph:stats`    | —              | `{nodes, edges, by_type}`              | Counts                                   |
| `docs:list`      | —              | `DocMeta[]`                            | List of parsed docs                       |
| `docs:read`      | `{path}`       | `{path, title, content}`               | Raw markdown (for the future read pane)  |
| `app:platform`   | —              | `string`                               | e.g. `windows` / `darwin`                |
| `app:version`    | —              | `{name, version}`                      | For the topbar badge                     |

## Architecture notes

### Dual-implementation rule

Every primitive operation (parse a doc, list nodes, list edges, get a node, stats) exists
**twice**: once in TypeScript (`src/lib/parser.ts`) and once in Rust (`src-tauri/src/graph.rs`).
They use the same regexes, the same kind-classification table, and the same edge rules.
This is intentional: the web build needs to render without a Tauri runtime (preview on
Vercel, on a CI screenshot, on a colleague's laptop without Rust installed), and the desktop
build needs to do the same work in Rust so a 200MB Markdown corpus doesn't have to round-trip
through the WebView.

Both implementations are kept in sync by tests:

- `src-tauri/src/graph.rs::tests` — Rust unit tests.
- `scripts/smoke.mjs` — runs the TS parser against the live docs and asserts basic graph
  invariants.

If the numbers diverge between TS and Rust, that's a bug — both should report the same node /
edge counts for the same input.

### Why a flat graph (no `commit` / `pr` / `issue` nodes yet)?

The platform's full data model includes Commit / PR / Issue nodes (Phase 6 §3.1, all first-class
subtypes of Node). The desktop app currently only renders nodes that exist in the *requirements
book*. Wiring the app to a live Git repo (parsing `git log` for commits, parsing
`.github/ISSUE/` for issues, etc.) is left for a follow-up issue — it's a clean extension point
because every new node type is just another kind + regex, no schema migration.

### The graph-view visualisation

The SVG in `GraphView.svelte` uses a **deterministic grid layout** keyed by index, not a
physics simulation. For the 100s-of-nodes scale of the requirements book, this is more
predictable and faster than d3-force; a follow-up can swap in a force layout when the platform
itself adds 10000+ node repos.

## Known issues

- **`tauri build` (release) fails on Windows** with `STATUS_STACK_BUFFER_OVERRUN` in the `syn`
  crate during `tauri::generate_context!` compilation. This is a known Tauri 2.x +
  Windows-specific issue with large `tauri.conf.json` + capability sets + many icons; it
  crashes during macro expansion of the generated context, not in our code.
  Workaround: use `cargo build --release --bin gitgit-desktop` directly with
  `RUST_MIN_STACK=33554432`, or build with `cargo build` (debug) which works fine and
  produces a ~19MB binary that's already usable.
- **Web build vs parser parity**: the TS parser has slightly more permissive regex matching
  than the Rust one (e.g. surrounding whitespace). Numbers can diverge by ±2-3 nodes in
  pathological inputs.
- **No write API yet**: the platform's `add_node` / `update_node` / `delete_node` IPC stubs
  are wired but only echo args back. Real persistence will land with the platform's local
  storage layer.

## Roadmap

- [ ] Wire up `git log` parsing → live Commit nodes (and `requires` edges from commit messages)
- [ ] Add a force-directed graph layout option (toggle in the topbar)
- [ ] Read pane: render a doc's full markdown in the detail column
- [ ] SQLite-backed local node annotations (per-user tags, comments) via `tauri-plugin-sql`
- [ ] IPC bridge to the platform's eventual backend service (when it lands)
- [ ] Resolve the `tauri build` Windows stack-overrun (try newer `tauri-codegen` versions)

## License

Inherits the parent repo's AGPL-3.0 license.