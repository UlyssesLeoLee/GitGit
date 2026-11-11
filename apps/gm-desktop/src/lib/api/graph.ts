/**
 * Tauri command wrappers for the knowledge-graph surface added in
 * PR-B (Rust engine in `apps/gm-desktop/src-tauri/src/commands/graph.rs`).
 *
 * The wire contract is `serde_json::Value` from the Rust side; we
 * cast at the boundary into the typed shapes in `./types` so the
 * rest of the app can stay type-safe.
 *
 * 7 commands are exposed:
 *   - graph_load     → full {nodes, edges, docs}
 *   - graph_list_nodes / graph_list_edges → narrow slices
 *   - graph_get_node → single node lookup
 *   - graph_stats    → counts by kind
 *   - docs_list      → discovered requirements docs
 *   - docs_read      → full markdown content for one doc
 *
 * When the Svelte app runs in a plain browser tab (during `vite
 * dev` without the Tauri runtime), `tauriInvoke()` throws
 * synchronously — callers in `stores/graph.ts` catch at a higher
 * level and the `mocks/handlers.ts` layer patches `invoke` so the
 * dev workflow stays smooth.
 */

import { invoke as tauriInvoke } from '@tauri-apps/api/core';
import type {
  DocMeta,
  DocReadResult,
  GraphEdge,
  GraphLoadResult,
  GraphNode,
  GraphStats,
} from './types';

export async function graphLoad(): Promise<GraphLoadResult> {
  // Rust returns serde_json::Value for nodes/edges; the cast is
  // safe because the on-the-wire JSON conforms to the schema in
  // `lib/api/types.ts` (verified by hand against `commands/graph.rs`).
  return (await tauriInvoke('graph_load')) as GraphLoadResult;
}

export async function graphListNodes(): Promise<GraphNode[]> {
  return (await tauriInvoke('graph_list_nodes')) as GraphNode[];
}

export async function graphListEdges(): Promise<GraphEdge[]> {
  return (await tauriInvoke('graph_list_edges')) as GraphEdge[];
}

export async function graphGetNode(id: string): Promise<GraphNode | null> {
  return (await tauriInvoke('graph_get_node', { id })) as GraphNode | null;
}

export async function graphStats(): Promise<GraphStats> {
  return (await tauriInvoke('graph_stats')) as GraphStats;
}

export async function docsList(): Promise<DocMeta[]> {
  return (await tauriInvoke('docs_list')) as DocMeta[];
}

export async function docsRead(path: string): Promise<DocReadResult> {
  return (await tauriInvoke('docs_read', { path })) as DocReadResult;
}