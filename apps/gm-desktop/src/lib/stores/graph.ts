/**
 * Graph state store for the /graph route.
 *
 * Per ADR-0023 §3.2 the gm-desktop /graph page reads nodes/edges/docs
 * from the Rust backend (`apps/gm-desktop/src-tauri/src/commands/graph.rs`).
 * On the real Tauri runtime we call `graphLoad()` and trust the Rust
 * side; under `vite dev` (no Tauri) the mock layer in
 * `src/mocks/handlers.ts` patches `invoke` so the same store call
 * succeeds against in-memory fixtures.
 *
 * Reactive shape (Svelte writable + derived):
 *   - `graph`             — master state (nodes/edges/docs/selected/query/loading/error/source)
 *   - `filteredNodes`     — derived: `graph.nodes` ranked by `scoreNode` against `query`
 *   - `stats`             — derived: count + per-kind breakdown
 *
 * Action helpers:
 *   - `loadGraph()`       — kicks off `graphLoad()`, hydrates state
 *   - `selectNode(id)`    — sets selection (or clears with null)
 *   - `setQuery(q)`       — updates the search box state
 *   - `getSelected()`     — convenience accessor for templates
 *   - `neighbors(id, dir)`— directional traversal: 'in' | 'out' | 'both'
 */

import { writable, derived, get } from 'svelte/store';
import type { GraphEdge, GraphNode, GraphStats } from '$lib/api/types';
import { graphLoad } from '$lib/api/graph';
import { scoreNode } from '$lib/graph/parser';

export type GraphSource = 'tauri' | 'web' | 'cache';

export interface GraphState {
  nodes: GraphNode[];
  edges: GraphEdge[];
  docs: { path: string; title: string; preview: string }[];
  selected: string | null;
  query: string;
  loading: boolean;
  error: string | null;
  source: GraphSource;
  buildAt: string;
}

const initial: GraphState = {
  nodes: [],
  edges: [],
  docs: [],
  selected: null,
  query: '',
  loading: false,
  error: null,
  source: 'cache',
  buildAt: new Date().toISOString(),
};

export const graph = writable<GraphState>(initial);

export const filteredNodes = derived(graph, ($g) => {
  if (!$g.query.trim()) return $g.nodes;
  const q = $g.query;
  return [...$g.nodes]
    .map((n) => ({ n, s: scoreNode(n, q) }))
    .filter((r) => r.s > 0)
    .sort((a, b) => b.s - a.s)
    .map((r) => r.n);
});

export const stats = derived(graph, ($g) => {
  const by_type: Record<string, number> = {};
  for (const n of $g.nodes) by_type[n.kind] = (by_type[n.kind] ?? 0) + 1;
  return { nodes: $g.nodes.length, edges: $g.edges.length, by_type } satisfies GraphStats;
});

export function selectNode(id: string | null): void {
  graph.update((g) => ({ ...g, selected: id }));
}

export function setQuery(q: string): void {
  graph.update((g) => ({ ...g, query: q }));
}

/**
 * Resolve the currently selected node from a given graph state.
 *
 * `[FACT]` This exists as a separate export from `getSelected()` because
 * of a measured bug, not for tidiness. `getSelected()` reads the store
 * with `get(graph)`, which is a one-time read and establishes **no**
 * reactive dependency. `Graph.svelte` was calling it as
 * `$derived(getSelected())`; that derived had zero tracked dependencies,
 * so Svelte evaluated it once at mount — when `selected` is `null` — and
 * never invalidated it again. The detail pane was therefore stuck in its
 * empty state for the lifetime of the component: clicking a node updated
 * the store and the list highlighting, but never rendered the node's
 * details.
 *
 * Taking the state as a parameter is what makes the caller's `$derived`
 * track `$graph`. `getSelected()` delegates here so imperative callers
 * keep working and there is only one copy of the lookup.
 */
export function selectFrom(g: GraphState): GraphNode | null {
  if (!g.selected) return null;
  return g.nodes.find((n) => n.id === g.selected) ?? null;
}

export function getSelected(): GraphNode | null {
  return selectFrom(get(graph));
}

export interface Neighbors {
  edges: GraphEdge[];
  nodes: GraphNode[];
}

export function neighbors(id: string | null, direction: 'in' | 'out' | 'both' = 'both'): Neighbors {
  const g = get(graph);
  if (!id) return { edges: [], nodes: [] };
  const ids = new Set<string>();
  const out: GraphEdge[] = [];
  for (const e of g.edges) {
    if (direction !== 'in' && e.from === id) {
      ids.add(e.to);
      out.push(e);
    } else if (direction !== 'out' && e.to === id) {
      ids.add(e.from);
      out.push(e);
    }
  }
  const nodes = g.nodes.filter((n) => ids.has(n.id));
  return { edges: out, nodes };
}

/**
 * Load the graph. Calls Rust `graph_load` (or the mock equivalent
 * under `vite dev`) and hydrates the writable store. Errors are
 * captured into `state.error` instead of thrown so the UI can render
 * a retry path.
 */
export async function loadGraph(): Promise<void> {
  graph.update((g) => ({ ...g, loading: true, error: null }));
  try {
    const result = await graphLoad();
    graph.update((g) => ({
      ...g,
      nodes: result.nodes,
      edges: result.edges,
      docs: result.docs,
      loading: false,
      source: 'tauri',
      buildAt: new Date().toISOString(),
    }));
  } catch (e) {
    graph.update((g) => ({
      ...g,
      loading: false,
      error: e instanceof Error ? e.message : String(e),
    }));
  }
}