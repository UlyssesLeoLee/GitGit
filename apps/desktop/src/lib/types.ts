/**
 * Engineering knowledge graph — primitive data types.
 *
 * These mirror the five Phase-6 primitives (Node, Edge, Event, Policy, View)
 * with Agent as a Node subtype. MVP desktop app handles Node + Edge + Event
 * storage; Policy + View land on the platform itself.
 */

export type NodeKind =
  | 'requirement'
  | 'issue'
  | 'pr'
  | 'commit'
  | 'adr'
  | 'agent'
  | 'policy'
  | 'event'
  | 'human'
  | 'release'
  | 'incident'
  | 'document';

export interface GraphNode {
  id: string;                  // stable, human-readable (e.g. REQ-GRF-001)
  kind: NodeKind;
  title: string;
  body?: string;               // markdown excerpt
  tags: string[];              // P0 / MVP / phase6 / etc.
  source?: string;             // file path, ADR #, PR #, commit sha
  created_at: string;          // ISO
  updated_at: string;          // ISO
  properties?: Record<string, unknown>;
}

export type EdgeKind =
  | 'implements'
  | 'depends_on'
  | 'supersedes'
  | 'gated_by'
  | 'reviewed_by'
  | 'derived_from'
  | 'caused_by'
  | 'created_by'
  | 'references'
  | 'blocks';

export interface GraphEdge {
  id: string;
  kind: EdgeKind;
  from: string;                // node id
  to: string;                  // node id
  weight?: number;             // optional, 0..1
  note?: string;
  created_at: string;
}

export interface GraphStats {
  nodes: number;
  edges: number;
  by_type: Record<string, number>;
}

export interface DocMeta {
  path: string;
  title: string;
  preview: string;
}

export interface AppVersion { name: string; version: string; }