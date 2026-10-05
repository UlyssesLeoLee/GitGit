/**
 * Tests for the `/graph` route component (`src/routes/Graph.svelte`).
 *
 * This is the largest uncovered file in the app: 315 lines, three
 * panes, and — at the base commit — zero tests. The gaps the cases
 * below close are the ones a smoke test cannot see:
 *
 *  - the four mutually exclusive render states (loading / error /
 *    empty / loaded) and the fact that an error is recoverable
 *    without a reload,
 *  - the two independent selection surfaces (master list and SVG),
 *    including the keyboard path and the guard that stops a
 *    non-Enter key from selecting,
 *  - the edges: which ones become a `<line>` at all, the palette
 *    fallbacks for a kind the component has no entry for, and the
 *    two directional connection lists,
 *  - the search box, which filters the master list but *not* the
 *    visualization (they read different store fields), and the
 *    500-row cap.
 *
 * `[FACT]` The backend is reached only through `graph_load`, so
 * every case drives the whole store → component path by replacing
 * the invoke bridge. The graph store is reset between cases because
 * it is a module singleton: without that, a selection made by one
 * case would leak into the next one's assertions.
 */
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { get } from 'svelte/store';
import { cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import Graph from '../../src/routes/Graph.svelte';
import { installMock } from '../../src/mocks/handlers';
import { graph, type GraphState } from '../../src/lib/stores/graph';
import { catalog, locale, tFor } from '../../src/lib/i18n';
import type { GraphEdge, GraphLoadResult, GraphNode, NodeKind } from '../../src/lib/api/types';

/* ---------- fixtures ---------- */

const AT = '2026-09-19T00:00:00.000Z';

function node(id: string, kind: NodeKind, title: string, extra: Partial<GraphNode> = {}): GraphNode {
  return { id, kind, title, tags: [], created_at: AT, updated_at: AT, ...extra };
}

const REQ = node('REQ-GRF-001', 'requirement', 'Queryable cross-object engineering graph', {
  body: 'Master requirements book. 55 sections.',
  tags: ['graph', 'P0'],
  source: 'docs/requirements/00-requirements-definition.md',
});
const ADR = node('ADR-0001', 'adr', 'Graph substrate: Node/Edge/Event', {
  tags: ['phase6'],
  source: 'docs/requirements/phase6-primitives.md',
});
const PR = node('PR-001', 'pr', 'feat: ingest requirement IDs into graph');
/** No edges, no tags, no source, no body — the four `{#if}` guards
 *  in the detail header all take their false branch for this one. */
const ISO = node('ISO-001', 'issue', 'Isolated seed node');

const NODES = [REQ, ADR, PR, ISO];

const E_IMPLEMENTS: GraphEdge = {
  id: 'e1',
  kind: 'implements',
  from: PR.id,
  to: REQ.id,
  note: 'the PR that built it',
  created_at: AT,
};
const E_DERIVED: GraphEdge = {
  id: 'e2',
  kind: 'derived_from',
  from: REQ.id,
  to: ADR.id,
  created_at: AT,
};
/** `to` names a node that is not in the payload. A real graph can
 *  reach this when a doc disappears between two loads. */
const E_DANGLING: GraphEdge = {
  id: 'e3',
  kind: 'supersedes',
  from: ADR.id,
  to: 'GHOST-0001',
  created_at: AT,
};

const EDGES = [E_IMPLEMENTS, E_DERIVED, E_DANGLING];

function payload(over: Partial<GraphLoadResult> = {}): GraphLoadResult {
  return { nodes: NODES, edges: EDGES, docs: [], ...over };
}

const PALETTE: Record<string, string> = {
  requirement: '#3b82f6',
  adr: '#ec4899',
  pr: '#8b5cf6',
  issue: '#f59e0b',
};

/* ---------- harness ---------- */

type Handler = (args: Record<string, unknown>) => Promise<unknown>;

/**
 * Record every command and answer from `overrides`, falling through to
 * the shipped mock layer for anything a test does not care about.
 * The recorded list is what proves *which* command a control issued.
 */
function useInvoke(overrides: Record<string, Handler> = {}): { cmd: string; args: Record<string, unknown> }[] {
  installMock();
  const base = window.__TAURI_INTERNALS__?.invoke;
  if (!base) throw new Error('the mock layer did not install an invoke bridge');
  const seen: { cmd: string; args: Record<string, unknown> }[] = [];
  window.__TAURI_INTERNALS__ = {
    invoke: (cmd: string, args?: Record<string, unknown>) => {
      seen.push({ cmd, args: args ?? {} });
      const over = overrides[cmd];
      return over ? over(args ?? {}) : base(cmd, args);
    },
  } as never;
  return seen;
}

/** A promise a test resolves by hand, so a pending state is observable. */
function deferred<T>(): {
  promise: Promise<T>;
  resolve: (v: T) => void;
  reject: (e: unknown) => void;
} {
  let resolve!: (v: T) => void;
  let reject!: (e: unknown) => void;
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}

/** The store's zero state, built here because the module does not
 *  export its `initial` value. */
function emptyGraph(): GraphState {
  return {
    nodes: [],
    edges: [],
    docs: [],
    selected: null,
    query: '',
    loading: false,
    error: null,
    source: 'cache',
    buildAt: AT,
  };
}

/** Copy of the page under test. */
function cat(key: string): string {
  return get(catalog)[key] ?? key;
}

/** The title in the detail pane, or `null` when nothing is selected.
 *  The page header is always the first `h1`, so the detail title is
 *  the second one — and is absent entirely in the empty state. */
function detailTitle(): string | null {
  const h1s = screen.getAllByRole('heading', { level: 1 });
  return h1s.length > 1 ? (h1s[1]?.textContent ?? '').trim() : null;
}

/** The detail pane's `<header>`, which holds the selected id, kind,
 *  title, source and tags. `null` when no node is selected. */
function detailHeader(): HTMLElement | null {
  const h1s = screen.getAllByRole('heading', { level: 1 });
  return h1s.length > 1 ? (h1s[1]?.parentElement as HTMLElement) : null;
}

function listRows(): HTMLElement[] {
  return within(screen.getByRole('listbox')).queryAllByRole('option');
}

function svg(): SVGSVGElement {
  return screen.getByTestId('graph-route').querySelector('svg') as SVGSVGElement;
}

/**
 * The node's own circle in the visualization.
 *
 * `[FACT]` Each node draws two circles: a translucent backdrop at
 * `r + 2` and the node itself. `nth-of-type(2)` is the node's, so a
 * query cannot accidentally read the backdrop's fill.
 */
function nodeCircle(id: string): SVGCircleElement | null {
  return svg().querySelector(`g[aria-label="${id}"] circle:nth-of-type(2)`);
}

/**
 * `[FACT]` jsdom normalises a hex colour written into an inline `style`
 * attribute: the component's `style="background: #3b82f6"` reads back
 * as `"background: rgb(59, 130, 246);"`. The palette entry is still what
 * the component rendered — only the spelling in the attribute changed —
 * so the expectation is normalised the same way rather than dropped.
 * A colour in an *attribute* (`fill`, `stroke`) is not rewritten, so
 * those are still compared as hex.
 */
function rgbOf(hex: string): string {
  const n = parseInt(hex.slice(1), 16);
  return `rgb(${(n >> 16) & 255}, ${(n >> 8) & 255}, ${n & 255})`;
}

/** The hue `nodeColor()` falls back to for a kind it has no entry for. */
const UNKNOWN_KIND = '#94a3b8';

/**
 * The red error banner, as the node the message is asserted on.
 *
 * `[FACT]` A Testing Library text matcher is invoked as
 * `matcher(text, element)`, where `text` is the node's *own* text — not
 * the element. A predicate written as `(el) => el.textContent?...` is
 * therefore handed a string, reads `undefined` off it, and is `false` for
 * every node in the document; the banner was on screen the whole time.
 * Matching on the node's own text is also the honest query here: the
 * banner is the only element whose own text joins the prefix and the
 * message, so this resolves to exactly one element.
 */
async function findErrorBox(): Promise<HTMLElement> {
  return (await screen.findByText((text) =>
    text.includes(`${cat('graph.errorPrefix')}:`)
  )) as HTMLElement;
}

/**
 * The detail pane: the scroll container holding the detail header, the
 * body paragraph and the connections. `null` when nothing is selected.
 * `[FACT]` The body is a *sibling* of the `<header>`, not a child, so a
 * query scoped to the header cannot see it.
 */
function detailPane(): HTMLElement | null {
  const h1s = screen.getAllByRole('heading', { level: 1 });
  return h1s.length > 1 ? (h1s[1]?.parentElement?.parentElement as HTMLElement) : null;
}

/**
 * One column of the connections grid — the `<div>` under the
 * "Outgoing (n)" / "Incoming (n)" heading.
 *
 * `[FACT]` The heading's own text carries the count, so
 * `getByText(label)` (an exact match) never finds it, and walking
 * `parentElement` twice from the heading lands on the grid, which holds
 * *both* columns and so made a node-id query ambiguous. The heading is
 * located by predicate and scoped to its own column.
 */
function connectionColumn(label: string): HTMLElement {
  const heading = screen
    .getAllByRole('heading', { level: 4 })
    .find((h) => h.textContent?.trim().startsWith(cat(label)));
  if (!heading) throw new Error(`no connection column headed "${cat(label)}"`);
  return heading.parentElement as HTMLElement;
}

beforeEach(() => {
  cleanup();
  installMock();
  graph.set(emptyGraph());
});

afterEach(() => {
  cleanup();
  graph.set(emptyGraph());
});

/* ---------- the four render states ---------- */

describe('route / graph — loading, failure, empty', () => {
  it('shows the loading state until the backend answers, then the nodes', async () => {
    const gate = deferred<GraphLoadResult>();
    useInvoke({ graph_load: () => gate.promise });

    render(Graph);
    // The pending state must be visible *before* anything resolves:
    // a page that renders "no data" while the query is in flight
    // would tell the user their requirements folder is empty.
    expect(screen.getByText(cat('common.loading'))).toBeTruthy();
    expect(screen.queryByTestId('graph-node-REQ-GRF-001')).toBeNull();

    gate.resolve(payload());
    await screen.findByTestId('graph-node-REQ-GRF-001');

    expect(screen.queryByText(cat('common.loading'))).toBeNull();
    expect(listRows()).toHaveLength(NODES.length);
  });

  it('reports a failure with the backend message and recovers on retry', async () => {
    // First call fails, second succeeds. If the retry button were
    // removed, or if `loadGraph` left `error` set behind, the nodes
    // would never appear and this case fails.
    let attempt = 0;
    useInvoke({
      graph_load: () => {
        attempt += 1;
        return attempt === 1
          ? Promise.reject(new Error('graph backend unreachable'))
          : Promise.resolve(payload());
      },
    });

    render(Graph);
    const box = await findErrorBox();
    // The backend's own message, verbatim, behind the catalogue prefix.
    // A banner that dropped it, or rendered a generic apology instead,
    // fails both assertions.
    expect(box.textContent).toContain(`${cat('graph.errorPrefix')}:`);
    expect(box.textContent).toContain('graph backend unreachable');
    // An error is not an empty state: nothing may claim "no data".
    expect(screen.queryByText(cat('graph.empty'))).toBeNull();
    expect(screen.queryByRole('listbox')).toBeNull();

    await fireEvent.click(
      screen.getByRole('button', { name: cat('common.refresh') })
    );
    await screen.findByTestId('graph-node-REQ-GRF-001');
    expect(listRows()).toHaveLength(NODES.length);
  });

  it('survives a non-Error rejection by stringifying it', async () => {
    // `loadGraph` falls back to `String(e)` for a rejection that is
    // not an `Error`. A backend that rejects with a bare payload
    // (the `AppError` shape does) must still produce a message.
    useInvoke({ graph_load: () => Promise.reject({ kind: 'Graph', message: 'boom' }) });

    render(Graph);
    const box = await findErrorBox();
    expect(box.textContent).toContain('[object Object]');
  });

  it('renders a graph with no nodes as the empty state, not as a failure', async () => {
    useInvoke({ graph_load: () => Promise.resolve(payload({ nodes: [], edges: [] })) });

    render(Graph);
    expect(await screen.findByText(cat('graph.empty'))).toBeTruthy();
    // The error box and the three-pane layout are both absent.
    expect(screen.queryByText(cat('graph.errorPrefix'))).toBeNull();
    expect(screen.queryByRole('listbox')).toBeNull();
    expect(detailTitle()).toBeNull();
  });
});

/* ---------- rendering a loaded graph ---------- */

describe('route / graph — the loaded graph', () => {
  it('renders the counts, one row per node, and the per-kind legend', async () => {
    useInvoke({ graph_load: () => Promise.resolve(payload()) });
    render(Graph);
    await screen.findByTestId('graph-node-REQ-GRF-001');

    // Header counts come from the `stats` derived store, not from a
    // hardcoded number.
    expect(cat('graph.nodes')).toBeTruthy();
    expect(screen.getByTestId('graph-route').textContent).toContain(
      `${NODES.length} ${cat('graph.nodes')}`
    );
    expect(screen.getByTestId('graph-route').textContent).toContain(
      `${EDGES.length} ${cat('graph.edges')}`
    );
    // The legend is built from `stats.by_type`, so it must name each
    // kind exactly once with its count.
    const legend = screen.getByTestId('graph-route');
    for (const [kind, count] of Object.entries({ requirement: 1, adr: 1, pr: 1, issue: 1 })) {
      expect(legend.textContent, kind).toContain(`${kind} (${count})`);
    }
    expect(listRows()).toHaveLength(NODES.length);
    // Each row shows the id, the kind and the title.
    const row = screen.getByTestId('graph-node-PR-001');
    expect(row.textContent).toContain('PR-001');
    expect(row.textContent).toContain('pr');
    expect(row.textContent).toContain('feat: ingest requirement IDs into graph');
  });

  it('draws a line only for an edge whose two endpoints are both loaded', async () => {
    useInvoke({ graph_load: () => Promise.resolve(payload()) });
    render(Graph);
    await screen.findByTestId('graph-node-REQ-GRF-001');

    // Three edges are in the payload but only two have both ends in
    // the node set, so the dangling `supersedes` edge must not be
    // drawn against `undefined` coordinates.
    const lines = svg().querySelectorAll('line');
    expect(lines).toHaveLength(2);
    // And every drawn line has real coordinates on both ends.
    for (const line of Array.from(lines)) {
      for (const attr of ['x1', 'y1', 'x2', 'y2']) {
        expect(Number(line.getAttribute(attr)), attr).not.toBeNaN();
      }
    }
    // One `g` per node: the dangling edge must not add a phantom one.
    expect(svg().querySelectorAll('g[role="button"]')).toHaveLength(NODES.length);
  });

  it('colours each node from its kind and falls back for a kind it has no entry for', async () => {
    // `sprint` is not a `NodeKind`. The Rust side is not obliged to
    // agree with the TypeScript union, so the component must render a
    // neutral swatch rather than `undefined` into a style attribute.
    const odd = node('SPRINT-1', 'sprint' as NodeKind, 'Iteration 1');
    useInvoke({ graph_load: () => Promise.resolve(payload({ nodes: [...NODES, odd] })) });
    render(Graph);
    await screen.findByTestId('graph-node-SPRINT-1');

    // Known kinds use their declared hue. `[FACT]` Compared in the
    // `rgb()` spelling jsdom normalises an inline `style` to — see
    // `rgbOf`. The value is still derived from the palette, so a change
    // to the mapping in `nodeColor()` fails here.
    const badge = screen
      .getByTestId('graph-node-REQ-GRF-001')
      .querySelector('span[style]') as HTMLElement;
    expect(badge.getAttribute('style')).toContain(rgbOf(PALETTE.requirement!));
    // The unknown kind gets the documented neutral fallback in both
    // the row swatch and the SVG circle. The `fill` attribute is not
    // CSS and keeps its hex spelling.
    const oddRowSwatch = screen
      .getByTestId('graph-node-SPRINT-1')
      .querySelector('span[style]') as HTMLElement;
    expect(oddRowSwatch.getAttribute('style')).toContain(rgbOf(UNKNOWN_KIND));
    expect(nodeCircle('SPRINT-1')?.getAttribute('fill')).toBe(UNKNOWN_KIND);
    // …and the node's own circle, not the translucent backdrop, carries
    // the hue. Both are asserted so a swap cannot pass.
    expect(nodeCircle('REQ-GRF-001')?.getAttribute('fill')).toBe(PALETTE.requirement);
    expect(nodeCircle('ADR-0001')?.getAttribute('fill')).toBe(PALETTE.adr);
  });

  it('gives a known edge kind its own colour and mutes one it does not name', async () => {
    const odd: GraphEdge = {
      id: 'e4',
      kind: 'tolerates' as GraphEdge['kind'],
      from: ISO.id,
      to: PR.id,
      created_at: AT,
    };
    useInvoke({ graph_load: () => Promise.resolve(payload({ edges: [...EDGES, odd] })) });
    render(Graph);
    await screen.findByTestId('graph-node-ISO-001');

    const strokes = Array.from(svg().querySelectorAll('line')).map((l) => l.getAttribute('stroke'));
    // `[FACT]` Three lines are drawn here, and only three. The shared
    // payload contributes `implements` (PR-001 → REQ-GRF-001) and
    // `derived_from` (REQ-GRF-001 → ADR-0001); this case adds
    // `tolerates` (ISO-001 → PR-001), whose ends are both loaded. The
    // fourth edge, `supersedes`, is *not* drawn at all — its target
    // GHOST-0001 is not in the node set — so it cannot contribute a
    // muted line. The previous expectation of two muted lines counted
    // that undrawn edge: a test bug, not a double render.
    expect(strokes).toHaveLength(3);
    // `implements` and `derived_from` are both named in the switch.
    expect(strokes).toContain('#22c55e');
    expect(strokes).toContain('#8b5cf6');
    // `tolerates` is not named, so it must take the default — and be
    // the *only* line that does.
    expect(strokes.filter((s) => s === 'rgba(100,116,139,0.45)')).toHaveLength(1);
  });

  it('truncates a long id in the SVG label but not in the list', async () => {
    const long = node('REQ-VERY-LONG-IDENTIFIER-01', 'requirement', 'Long id');
    useInvoke({ graph_load: () => Promise.resolve(payload({ nodes: [long, REQ], edges: [] })) });
    render(Graph);
    await screen.findByTestId('graph-node-REQ-VERY-LONG-IDENTIFIER-01');

    const label = (id: string): string =>
      (svg().querySelector(`g[aria-label="${id}"] text`)?.textContent ?? '').trim();
    // Over 14 characters: first 13 plus an ellipsis.
    expect(label('REQ-VERY-LONG-IDENTIFIER-01')).toBe('REQ-VERY-LONG…');
    // At or under the limit: shown in full.
    expect(label('REQ-GRF-001')).toBe('REQ-GRF-001');
    // The master list is not truncated — the id must stay copyable.
    expect(
      screen.getByTestId('graph-node-REQ-VERY-LONG-IDENTIFIER-01').textContent
    ).toContain('REQ-VERY-LONG-IDENTIFIER-01');
  });
});

/* ---------- selection ---------- */

describe('route / graph — selecting a node', () => {
  it('starts with the detail pane asking for a selection', async () => {
    useInvoke({ graph_load: () => Promise.resolve(payload()) });
    render(Graph);
    await screen.findByTestId('graph-node-REQ-GRF-001');

    expect(detailTitle()).toBeNull();
    expect(screen.getByText(cat('graph.detailEmptyTitle'))).toBeTruthy();
    expect(screen.getByText(cat('graph.detailEmptyHint'))).toBeTruthy();
  });

  it('fills the detail pane from a click in the master list and marks that row', async () => {
    useInvoke({ graph_load: () => Promise.resolve(payload()) });
    render(Graph);
    await screen.findByTestId('graph-node-REQ-GRF-001');

    await fireEvent.click(screen.getByTestId('graph-node-REQ-GRF-001'));

    expect(detailTitle()).toBe(REQ.title);
    const header = detailHeader()!;
    // The id and the kind are shown next to the title.
    expect(within(header).getByText(REQ.id)).toBeTruthy();
    expect(within(header).getByText('requirement')).toBeTruthy();
    // Source and tags render in the header for a node that has them.
    expect(within(header).getByText(REQ.source!)).toBeTruthy();
    expect(within(header).getByText('graph')).toBeTruthy();
    expect(within(header).getByText('P0')).toBeTruthy();
    // The body is a *sibling* of the header, so the pane is its scope.
    // `[FACT]` Measured: `within(header)` cannot see it, which is what
    // the previous assertion tripped over — the body did render.
    const pane = detailPane()!;
    expect(within(pane).getByText(REQ.body!)).toBeTruthy();
    // …and the header must not have absorbed it, or the two would be
    // one block and the layout above it would grow.
    expect(within(header).queryByText(REQ.body!)).toBeNull();

    // Only the selected row is `aria-selected`; the rest are not.
    expect(screen.getByTestId('graph-node-REQ-GRF-001').getAttribute('aria-selected')).toBe('true');
    expect(screen.getByTestId('graph-node-PR-001').getAttribute('aria-selected')).toBe('false');
  });

  it('reaches the same selection from the visualization', async () => {
    // A second, independent click surface. If the `g` handler were
    // dropped, the SVG would be decorative and this case would fail
    // with the detail pane still empty.
    useInvoke({ graph_load: () => Promise.resolve(payload()) });
    render(Graph);
    await screen.findByTestId('graph-node-ADR-0001');

    await fireEvent.click(svg().querySelector('g[aria-label="ADR-0001"]') as Element);
    expect(detailTitle()).toBe(ADR.title);
    expect(screen.getByTestId('graph-node-ADR-0001').getAttribute('aria-selected')).toBe('true');
  });

  it('selects on Enter and ignores every other key', async () => {
    useInvoke({ graph_load: () => Promise.resolve(payload()) });
    render(Graph);
    await screen.findByTestId('graph-node-REQ-GRF-001');

    // A non-Enter key must not select. If the guard were dropped, the
    // selection below would move to `ISO-001` and fail.
    await fireEvent.keyDown(screen.getByTestId('graph-node-REQ-GRF-001'), { key: 'Tab' });
    expect(detailTitle()).toBeNull();

    await fireEvent.keyDown(screen.getByTestId('graph-node-REQ-GRF-001'), { key: 'Enter' });
    expect(detailTitle()).toBe(REQ.title);
  });

  it('splits the connections of a node into outgoing and incoming', async () => {
    useInvoke({ graph_load: () => Promise.resolve(payload()) });
    render(Graph);
    await screen.findByTestId('graph-node-REQ-GRF-001');
    await fireEvent.click(screen.getByTestId('graph-node-REQ-GRF-001'));

    // REQ-GRF-001 has one edge each way: it derives from ADR-0001 and
    // is implemented by PR-001.
    expect(screen.getByText(`${cat('graph.connections')} (2)`)).toBeTruthy();
    expect(screen.getByText(`${cat('graph.outgoing')} (1)`)).toBeTruthy();
    expect(screen.getByText(`${cat('graph.incoming')} (1)`)).toBeTruthy();
  });

  it('says so plainly when the selected node has no edges at all', async () => {
    // The four `{#if selected.*}` guards all take their false branch
    // here: no source, no tags, no body, no connections.
    useInvoke({ graph_load: () => Promise.resolve(payload()) });
    render(Graph);
    await screen.findByTestId('graph-node-ISO-001');
    await fireEvent.click(screen.getByTestId('graph-node-ISO-001'));

    expect(detailTitle()).toBe(ISO.title);
    expect(screen.getByText(cat('graph.noConnections'))).toBeTruthy();
    expect(screen.getByText(`${cat('graph.connections')} (0)`)).toBeTruthy();
    // The optional sections are omitted rather than rendered empty.
    const header = detailHeader()!;
    expect(within(header).queryByText(ISO.id)).toBeTruthy();
    expect(within(header).queryByText(cat('graph.fromSource'))).toBeNull();
    expect(screen.queryByText('Master requirements book. 55 sections.')).toBeNull();
  });

  it('navigates from a connection row to the node on the other end', async () => {
    useInvoke({ graph_load: () => Promise.resolve(payload()) });
    render(Graph);
    await screen.findByTestId('graph-node-PR-001');
    await fireEvent.click(screen.getByTestId('graph-node-PR-001'));

    // The outgoing row for `implements` targets REQ-GRF-001. Clicking
    // it must move the selection, not just highlight the row.
    const outgoing = connectionColumn('graph.outgoing');
    const row = within(outgoing).getByText(REQ.id).closest('button');
    await fireEvent.click(row as HTMLButtonElement);

    expect(detailTitle()).toBe(REQ.title);
    expect(screen.getByTestId('graph-node-REQ-GRF-001').getAttribute('aria-selected')).toBe('true');
  });

  it('puts the edge note in a connection row tooltip, and nothing when there is no note', async () => {
    useInvoke({ graph_load: () => Promise.resolve(payload()) });
    render(Graph);
    await screen.findByTestId('graph-node-PR-001');
    await fireEvent.click(screen.getByTestId('graph-node-PR-001'));

    // Scoped to the outgoing column. `[FACT]` `REQ-GRF-001` also appears
    // in the master-list row and as the SVG `<text>` label, so an
    // unscoped `getByText` matched three elements.
    const row = within(connectionColumn('graph.outgoing'))
      .getByText(REQ.id)
      .closest('button') as HTMLButtonElement;
    // The row that carries the note is the `implements` one — pinned so
    // a wrong-row match cannot read green.
    expect(row.textContent).toContain('implements');
    expect(row.getAttribute('title')).toBe('the PR that built it');

    // ADR-0001's edge carries no note, so the tooltip is the empty
    // string rather than the string "undefined".
    await fireEvent.click(screen.getByTestId('graph-node-REQ-GRF-001'));
    const other = within(connectionColumn('graph.outgoing'))
      .getByText(ADR.id)
      .closest('button') as HTMLButtonElement;
    expect(other.textContent).toContain('derived from');
    expect(other.getAttribute('title')).toBe('');
  });

  it('highlights the edges touching the selected node and dims the rest', async () => {
    useInvoke({ graph_load: () => Promise.resolve(payload()) });
    render(Graph);
    await screen.findByTestId('graph-node-REQ-GRF-001');

    // Nothing selected yet: every line sits at the base opacity.
    const before = Array.from(svg().querySelectorAll('line')).map((l) => l.getAttribute('opacity'));
    expect(new Set(before)).toEqual(new Set(['0.4']));

    await fireEvent.click(screen.getByTestId('graph-node-REQ-GRF-001'));
    const after = Array.from(svg().querySelectorAll('line')).map((l) => ({
      opacity: l.getAttribute('opacity'),
      width: l.getAttribute('stroke-width'),
    }));
    // Both drawn edges touch REQ-GRF-001, so both are hot; the dim
    // value must be gone from the drawn set.
    expect(after.every((l) => l.opacity === '0.95')).toBe(true);
    expect(new Set(after.map((l) => l.width))).toEqual(new Set(['1.6']));

    // The selection is drawn on the node too, not only in the master
    // list: the visualization is the pane the user is looking at.
    expect(nodeCircle(REQ.id)?.getAttribute('stroke')).toBe('white');
    expect(nodeCircle(REQ.id)?.getAttribute('stroke-width')).toBe('2');
    // …and only on the selected one.
    expect(nodeCircle(ADR.id)?.getAttribute('stroke')).toBe('rgba(0,0,0,0.4)');
    expect(nodeCircle(ADR.id)?.getAttribute('stroke-width')).toBe('1');
  });
});

/* ---------- search ---------- */

describe('route / graph — the search box', () => {
  it('filters the master list by id, title, tag or source, but not the visualization', async () => {
    // The list reads `filteredNodes`; the SVG reads `graph.nodes`.
    // A regression that filtered both would make the graph vanish
    // while the user types, so the circle count is asserted here.
    useInvoke({ graph_load: () => Promise.resolve(payload()) });
    render(Graph);
    await screen.findByTestId('graph-node-REQ-GRF-001');
    const all = svg().querySelectorAll('g[role="button"]').length;

    await fireEvent.input(screen.getByTestId('graph-search'), {
      target: { value: 'phase6' },
    });

    // `phase6` is a tag of ADR-0001 only.
    expect(listRows().map((r) => r.getAttribute('data-testid'))).toEqual([
      'graph-node-ADR-0001',
    ]);
    expect(screen.getByTestId('graph-route').textContent).toContain(
      `1 ${cat('graph.matching')} "phase6"`
    );
    expect(svg().querySelectorAll('g[role="button"]').length).toBe(all);
  });

  it('shows "*" as the query in the count line until something is typed', async () => {
    useInvoke({ graph_load: () => Promise.resolve(payload()) });
    render(Graph);
    await screen.findByTestId('graph-node-REQ-GRF-001');
    expect(screen.getByTestId('graph-route').textContent).toContain(
      `${NODES.length} ${cat('graph.matching')} "*"`
    );
  });

  it('empties the list for a query that matches nothing, without claiming the graph is empty', async () => {
    useInvoke({ graph_load: () => Promise.resolve(payload()) });
    render(Graph);
    await screen.findByTestId('graph-node-REQ-GRF-001');

    await fireEvent.input(screen.getByTestId('graph-search'), {
      target: { value: 'no-such-thing' },
    });

    expect(listRows()).toHaveLength(0);
    // The whole-graph empty state must not appear: the graph loaded
    // fine, the search simply matched nothing.
    expect(screen.queryByText(cat('graph.empty'))).toBeNull();
    expect(screen.getByTestId('graph-route').textContent).toContain(
      `0 ${cat('graph.matching')} "no-such-thing"`
    );
  });

  it('caps the master list at 500 rows and reports how many were withheld', async () => {
    // The cap is a rendering budget, not a filter: the visualization
    // still draws every node.
    const many = Array.from({ length: 501 }, (_, i) =>
      node(`N${String(i).padStart(4, '0')}`, 'document', `Node ${i}`)
    );
    useInvoke({ graph_load: () => Promise.resolve(payload({ nodes: many, edges: [] })) });
    render(Graph);
    await screen.findByTestId('graph-node-N0000');

    expect(listRows()).toHaveLength(500);
    expect(screen.getByTestId('graph-route').textContent).toContain(
      `… 1 ${cat('graph.more')}`
    );
    expect(svg().querySelectorAll('g[role="button"]')).toHaveLength(501);
  });
});

/* ---------- i18n ---------- */

describe('route / graph — every string comes from the catalogue', () => {
  it('re-renders the page in the other locale rather than caching a translation', async () => {
    useInvoke({ graph_load: () => Promise.resolve(payload()) });
    render(Graph);
    await screen.findByTestId('graph-node-REQ-GRF-001');

    // Switching away from the active locale and back must change the
    // rendered text. A non-reactive read (the `t()` helper) would
    // leave the first translation on screen and fail this.
    const active = get(locale);
    const other = active === 'en' ? 'zh-CN' : 'en';
    expect(
      screen.getByRole('heading', { level: 1 }).textContent?.trim()
    ).toBe(tFor(active, 'graph.heading'));

    const { setLocale } = await import('../../src/lib/stores/locale');
    setLocale(other);
    // `[FACT]` `setLocale` writes a store; the page re-renders on the
    // next flush, not synchronously inside the assignment, so reading
    // the heading on the next line still sees the previous paint. The
    // heading is re-queried inside the wait as well: the assertion is
    // about what is on screen, not about a captured node reference.
    await waitFor(() =>
      expect(
        screen.getByRole('heading', { level: 1 }).textContent?.trim()
      ).toBe(tFor(other, 'graph.heading'))
    );
    expect(
      screen.getByRole('heading', { level: 1 }).textContent?.trim()
    ).not.toBe(tFor(active, 'graph.heading'));

    setLocale(active);
    await waitFor(() =>
      expect(
        screen.getByRole('heading', { level: 1 }).textContent?.trim()
      ).toBe(tFor(active, 'graph.heading'))
    );
  });
});
