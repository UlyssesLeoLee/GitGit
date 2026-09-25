import { writable, derived, get } from 'svelte/store';
import type { GraphEdge, GraphNode, GraphStats, DocMeta } from './types';
import { docsToGraph, parseDoc, scoreNode } from './parser';
import { webInvoke, platform, isTauri } from './api';

interface GraphState {
  nodes: GraphNode[];
  edges: GraphEdge[];
  docs: DocMeta[];
  selected: string | null;
  query: string;
  loading: boolean;
  error: string | null;
  source: 'tauri' | 'web' | 'cache';
  isTauri: boolean;
  os: string;
  version: string;
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
  isTauri: false,
  os: 'unknown',
  version: '0.0.0',
  buildAt: new Date().toISOString(),
};

export const graph = writable<GraphState>(initial);

export const filteredNodes = derived(graph, $g => {
  if (!$g.query.trim()) return $g.nodes;
  const q = $g.query;
  return [...$g.nodes]
    .map(n => ({ n, s: scoreNode(n, q) }))
    .filter(r => r.s > 0)
    .sort((a, b) => b.s - a.s)
    .map(r => r.n);
});

export const stats = derived(graph, $g => {
  const by_type: Record<string, number> = {};
  for (const n of $g.nodes) by_type[n.kind] = (by_type[n.kind] ?? 0) + 1;
  return { nodes: $g.nodes.length, edges: $g.edges.length, by_type } satisfies GraphStats;
});

export function selectNode(id: string | null) {
  graph.update(g => ({ ...g, selected: id }));
}

export function setQuery(q: string) {
  graph.update(g => ({ ...g, query: q }));
}

export function getSelected(): GraphNode | null {
  const g = get(graph);
  if (!g.selected) return null;
  return g.nodes.find(n => n.id === g.selected) ?? null;
}

export function neighbors(id: string | null, direction: 'in' | 'out' | 'both' = 'both') {
  const g = get(graph);
  if (!id) return { edges: [] as GraphEdge[], nodes: [] as GraphNode[] };
  const ids = new Set<string>();
  const out: GraphEdge[] = [];
  for (const e of g.edges) {
    if (direction !== 'in' && e.from === id) { ids.add(e.to); out.push(e); }
    else if (direction !== 'out' && e.to === id) { ids.add(e.from); out.push(e); }
  }
  const nodes = g.nodes.filter(n => ids.has(n.id));
  return { edges: out, nodes };
}

/**
 * Load the graph. Tries the Tauri side first (which reads the local repo's
 * `docs/requirements/`), falls back to the bundled web fixture if Tauri is
 * not present (browser preview).
 */
export async function loadGraph(): Promise<void> {
  graph.update(g => ({ ...g, loading: true, error: null }));

  try {
    const os = await platform();
    const ver = await webInvoke<{ name: string; version: string }>('app_version');
    graph.update(g => ({ ...g, isTauri: isTauri(), os: os === 'web' ? 'web-preview' : os, version: ver.version, buildAt: new Date().toISOString() }));

    if (isTauri()) {
      // Desktop path: ask Rust backend to walk docs/requirements and parse.
      const result = await webInvoke<{ nodes: GraphNode[]; edges: GraphEdge[]; docs: DocMeta[] }>('graph_load');
      graph.update(g => ({
        ...g,
        nodes: result.nodes,
        edges: result.edges,
        docs: result.docs,
        loading: false,
        source: 'tauri',
      }));
    } else {
      // Web preview path: parse the bundled docs we ship in `public/requirements/`.
      const docs = await webInvoke<DocMeta[]>('docs_list');
      const parsed = await Promise.all(
        docs.map(async d => parseDoc(d.path, await fetchText(d.path, d.preview)))
      );
      const { nodes, edges } = docsToGraph(parsed);
      graph.update(g => ({ ...g, nodes, edges, docs, loading: false, source: 'web' }));
    }
  } catch (e) {
    graph.update(g => ({ ...g, loading: false, error: (e as Error).message }));
  }
}

async function fetchText(path: string, fallback: string): Promise<string> {
  try {
    const r = await fetch(`/${path.replace(/^docs\//, 'requirements/')}`);
    if (r.ok) return await r.text();
  } catch { /* ignore */ }
  return fallback;
}