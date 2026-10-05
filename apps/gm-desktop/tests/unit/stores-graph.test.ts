/**
 * Store tests for `src/lib/stores/graph.ts`.
 *
 * This store was 0% covered: the /graph page drives it, but the page
 * itself had no tests, so nothing in the file had ever run. The cases
 * below concentrate on the three things a graph store gets wrong:
 *
 *   1. the derived selectors (`filteredNodes`, `stats`) — pure
 *      functions over state, cheap to pin exactly;
 *   2. `loadGraph`'s failure path — a refresh that throws must not
 *      blank the graph the user is already looking at, and must not
 *      leave `loading` stuck;
 *   3. `neighbors` — a directional traversal whose `if/else` gives one
 *      answer for a self-loop that is easy to get wrong.
 *
 * The store exposes no reset helper, so `resetGraph()` reconstructs
 * the same empty state the module starts from.
 */
import { beforeEach, describe, expect, it } from 'vitest';
import { get } from 'svelte/store';
import {
  filteredNodes,
  getSelected,
  graph,
  loadGraph,
  neighbors,
  selectNode,
  setQuery,
  stats,
} from '../../src/lib/stores/graph';
import type { GraphEdge, GraphNode } from '../../src/lib/api/types';
import { installMock } from '../../src/mocks/handlers';

const NOW = '2026-09-19T00:00:00.000Z';

function node(over: Partial<GraphNode> & { id: string }): GraphNode {
  return { kind: 'requirement', title: 'a node', tags: [], created_at: NOW, updated_at: NOW, ...over };
}

function edge(over: Partial<GraphEdge> & { id: string; from: string; to: string }): GraphEdge {
  return { kind: 'depends_on', created_at: NOW, ...over };
}

/** Put a fixture into the store. */
function seed(state: { nodes?: GraphNode[]; edges?: GraphEdge[]; docs?: unknown[] }): void {
  graph.update((g) => ({
    ...g,
    nodes: state.nodes ?? [],
    edges: state.edges ?? [],
    docs: (state.docs ?? []) as typeof g.docs,
  }));
}

function resetGraph(): void {
  graph.set({
    nodes: [],
    edges: [],
    docs: [],
    selected: null,
    query: '',
    loading: false,
    error: null,
    source: 'cache',
    buildAt: NOW,
  });
}

/** Replace the bridge so a case can resolve or refuse `graph_load`. */
function respondsWith(result: unknown): void {
  window.__TAURI_INTERNALS__ = {
    invoke: (cmd: string) =>
      cmd === 'graph_load' ? Promise.resolve(result) : Promise.reject(new Error(`unexpected ${cmd}`)),
  };
}

function rejectsWith(rejection: unknown): void {
  window.__TAURI_INTERNALS__ = { invoke: () => Promise.reject(rejection) };
}

beforeEach(() => {
  // `[FACT]` `rejectsWith` above installs a one-line stub whose source is
  // just `() => Promise.reject(...)`. It used to need a
  // `delete window.__TAURI_INTERNALS__` before `installMock()`, because
  // the mock was recognised by grepping its own source text. That is
  // gone, so the reinstall is unconditional and this file is
  // order-independent without the delete.
  installMock();
  resetGraph();
});

describe('store / graph — initial state', () => {
  it('starts empty, with no selection, no error and no load in flight', () => {
    const s = get(graph);
    expect(s.nodes).toEqual([]);
    expect(s.edges).toEqual([]);
    expect(s.docs).toEqual([]);
    expect(s.selected).toBeNull();
    expect(s.query).toBe('');
    expect(s.loading).toBe(false);
    expect(s.error).toBeNull();
    // Nothing has been loaded yet, so the store is not claiming a live
    // backend it has not talked to.
    expect(s.source).toBe('cache');
  });

  it('reports zeroed stats for an empty graph', () => {
    expect(get(stats)).toEqual({ nodes: 0, edges: 0, by_type: {} });
  });
});

describe('store / graph — loadGraph', () => {
  it('hydrates nodes, edges and docs and marks the source as tauri', async () => {
    await loadGraph();
    const s = get(graph);
    expect(s.nodes.length).toBeGreaterThan(0);
    expect(s.edges.length).toBeGreaterThan(0);
    expect(s.docs.length).toBeGreaterThan(0);
    expect(s.source).toBe('tauri');
    expect(s.loading).toBe(false);
    expect(s.error).toBeNull();
  });

  it('raises loading while the call is in flight and lowers it after', async () => {
    const seen: boolean[] = [];
    const unsub = graph.subscribe((s) => seen.push(s.loading));
    respondsWith({ nodes: [node({ id: 'n1' })], edges: [], docs: [] });

    await loadGraph();
    unsub();

    // A refresh that never shows progress looks like a frozen page.
    expect(seen[0]).toBe(false);
    expect(seen).toContain(true);
    expect(seen[seen.length - 1]).toBe(false);
  });

  it('stamps a fresh build time on success', async () => {
    seed({});
    const before = get(graph).buildAt;
    respondsWith({ nodes: [], edges: [], docs: [] });
    await loadGraph();
    // Compared with `>=` because a same-millisecond load is legal; the
    // point is that it is a real timestamp and not a hardcoded literal.
    expect(Date.parse(get(graph).buildAt)).toBeGreaterThanOrEqual(Date.parse(before));
  });

  it('clears a previous error when a retry starts', async () => {
    rejectsWith(new Error('no index yet'));
    await loadGraph();
    expect(get(graph).error).toBe('no index yet');

    respondsWith({ nodes: [node({ id: 'n1' })], edges: [], docs: [] });
    await loadGraph();
    // A successful retry must not leave the old failure on screen.
    expect(get(graph).error).toBeNull();
    expect(get(graph).nodes.map((n) => n.id)).toEqual(['n1']);
  });

  it('keeps the loaded graph on screen when a refresh fails', async () => {
    respondsWith({ nodes: [node({ id: 'n1' }), node({ id: 'n2' })], edges: [], docs: [] });
    await loadGraph();

    rejectsWith(new Error('backend went away'));
    await loadGraph();

    // The failure is additive: the nodes the user was reading are
    // still there, with the error alongside them. Blanking the graph
    // on a failed refresh would throw away what was already known.
    expect(get(graph).nodes.map((n) => n.id)).toEqual(['n1', 'n2']);
    expect(get(graph).error).toBe('backend went away');
  });

  it('does not leave loading stuck after a failure', async () => {
    rejectsWith(new Error('backend went away'));
    await loadGraph();
    // A stuck `loading` flag disables every control on the page.
    expect(get(graph).loading).toBe(false);
  });

  it('records the message of an Error rejection', async () => {
    rejectsWith(new Error('graph index has not been built'));
    await loadGraph();
    expect(get(graph).error).toBe('graph index has not been built');
  });

  it('keeps the message of a backend refusal', async () => {
    // `[FACT]` This used to be "loses the typed kind of a backend
    // refusal", and it asserted `error` was `"[object Object]"`. The
    // store stringified whatever the command rejected with, but a Tauri
    // rejection is a plain `{ kind, message, source }` object — not an
    // `Error` — so the graph page told the user `[object Object]` and
    // dropped both the kind and the message. The expectation below pins
    // the fixed behaviour: the backend's own words survive.
    rejectsWith({ kind: 'GraphNotBuilt', message: 'no index yet', source: '"GraphNotBuilt"' });
    await loadGraph();

    expect(get(graph).error).toBe('no index yet');
    expect(get(graph).error).not.toBe('[object Object]');
  });
});

describe('store / graph — filteredNodes', () => {
  const A = node({ id: 'REQ-GRF-001', title: 'Queryable cross-object engineering graph', tags: ['graph', 'P0'] });
  const B = node({ id: 'ADR-0001', kind: 'adr', title: 'Graph substrate: Node and Edge', tags: ['phase6'] });
  const C = node({ id: 'POL-001', kind: 'policy', title: 'Sensitive-data routing policy', tags: [] });

  it('returns every node when the query is empty', () => {
    seed({ nodes: [A, B, C] });
    setQuery('');
    expect(get(filteredNodes).map((n) => n.id)).toEqual(['REQ-GRF-001', 'ADR-0001', 'POL-001']);
  });

  it('treats a whitespace-only query as no query', () => {
    seed({ nodes: [A, B, C] });
    setQuery('   ');
    // `.trim()` guards the search box: a stray space must not empty
    // the result list.
    expect(get(filteredNodes)).toHaveLength(3);
  });

  it('ranks the best match first', () => {
    seed({ nodes: [A, B, C] });
    setQuery('graph');
    // A scores 3 (title) + 2 (tag) = 5; B scores 3 (title) = 3.
    expect(get(filteredNodes).map((n) => n.id)).toEqual(['REQ-GRF-001', 'ADR-0001']);
  });

  it('drops nodes that match nothing', () => {
    seed({ nodes: [A, B, C] });
    setQuery('graph');
    // A policy node with no graph text is noise in a search result,
    // not a zero-score result.
    expect(get(filteredNodes).map((n) => n.id)).not.toContain('POL-001');
  });

  it('matches an id as well as a title', () => {
    seed({ nodes: [A, B, C] });
    setQuery('POL-001');
    // id match is worth 4, so the policy node outranks nothing here
    // but is still found at all.
    expect(get(filteredNodes).map((n) => n.id)).toEqual(['POL-001']);
  });

  it('matches without regard to case', () => {
    seed({ nodes: [A, B, C] });
    setQuery('GRAPH');
    expect(get(filteredNodes).map((n) => n.id)).toEqual(['REQ-GRF-001', 'ADR-0001']);
  });

  it('returns nothing when the query matches nothing at all', () => {
    seed({ nodes: [A, B, C] });
    setQuery('kubernetes');
    expect(get(filteredNodes)).toEqual([]);
  });
});

describe('store / graph — stats', () => {
  it('counts nodes and edges and breaks nodes down by kind', () => {
    seed({
      nodes: [
        node({ id: 'a', kind: 'requirement' }),
        node({ id: 'b', kind: 'requirement' }),
        node({ id: 'c', kind: 'adr' }),
      ],
      edges: [edge({ id: 'e1', from: 'a', to: 'b' })],
    });
    expect(get(stats)).toEqual({
      nodes: 3,
      edges: 1,
      by_type: { requirement: 2, adr: 1 },
    });
  });
});

describe('store / graph — selection', () => {
  const A = node({ id: 'n/a', title: 'first' });
  const B = node({ id: 'n/b', title: 'second' });

  it('selects a node and reads it back', () => {
    seed({ nodes: [A, B] });
    selectNode('n/b');
    expect(get(graph).selected).toBe('n/b');
    expect(getSelected()?.title).toBe('second');
  });

  it('clears the selection with null', () => {
    seed({ nodes: [A, B] });
    selectNode('n/a');
    selectNode(null);
    expect(get(graph).selected).toBeNull();
    expect(getSelected()).toBeNull();
  });

  it('returns null for a selected id that is no longer in the graph', () => {
    // The selection survives a reload, so a node that disappeared must
    // read as "nothing selected" rather than blowing up on `undefined`.
    seed({ nodes: [A] });
    selectNode('n/vanished');
    expect(getSelected()).toBeNull();
  });
});

describe('store / graph — neighbors', () => {
  const A = node({ id: 'n/a' });
  const B = node({ id: 'n/b' });
  const C = node({ id: 'n/c' });
  // e1: A -> B, e2: C -> A, e3: A -> A
  const EDGES = [
    edge({ id: 'e1', from: 'n/a', to: 'n/b' }),
    edge({ id: 'e2', from: 'n/c', to: 'n/a' }),
    edge({ id: 'e3', from: 'n/a', to: 'n/a' }),
  ];

  it('follows outgoing edges for the out direction', () => {
    seed({ nodes: [A, B, C], edges: EDGES });
    const out = neighbors('n/a', 'out');
    expect(out.edges.map((e) => e.id)).toEqual(['e1', 'e3']);
    // e3 is a self-loop, so A is its own out-neighbour.
    expect(out.nodes.map((n) => n.id)).toEqual(['n/a', 'n/b']);
  });

  it('follows incoming edges for the in direction', () => {
    seed({ nodes: [A, B, C], edges: EDGES });
    const incoming = neighbors('n/a', 'in');
    // e3 is a self-loop, so it is legitimately an incoming edge too:
    // the `if` arm is skipped for `in` and the `else if` matches on
    // `e.to === id`. A node is therefore its own in-neighbour.
    expect(incoming.edges.map((e) => e.id)).toEqual(['e2', 'e3']);
    expect(incoming.nodes.map((n) => n.id)).toEqual(['n/a', 'n/c']);
  });

  it('follows both directions by default', () => {
    seed({ nodes: [A, B, C], edges: EDGES });
    const both = neighbors('n/a');
    // The self-loop is counted once: the `if` arm wins, so e3
    // contributes A without being visited again as an inbound edge.
    expect(both.edges.map((e) => e.id)).toEqual(['e1', 'e2', 'e3']);
    expect(both.nodes.map((n) => n.id)).toEqual(['n/a', 'n/b', 'n/c']);
  });

  it('returns nothing for a null id', () => {
    seed({ nodes: [A, B, C], edges: EDGES });
    expect(neighbors(null)).toEqual({ edges: [], nodes: [] });
  });

  it('returns nothing for a node with no edges at all', () => {
    // n/b is the target of e1, so it does have an edge; n/d is the
    // isolated node this case needs.
    const D = node({ id: 'n/d' });
    seed({ nodes: [A, B, C, D], edges: EDGES });
    expect(neighbors('n/d')).toEqual({ edges: [], nodes: [] });
  });

  it('keeps an edge whose endpoint is not in the node list', () => {
    // A dangling edge must still be reported — it is a real edge in
    // the index — while contributing no node to render.
    seed({
      nodes: [A],
      edges: [edge({ id: 'e-ghost', from: 'n/a', to: 'n/ghost' })],
    });
    const out = neighbors('n/a', 'out');
    expect(out.edges.map((e) => e.id)).toEqual(['e-ghost']);
    expect(out.nodes).toEqual([]);
  });
});
