/**
 * TypeScript mirrors of the Rust DTOs declared in
 * `apps/gm-desktop/src-tauri/src/commands/`. Keep these in sync
 * with the `Serialize` impls on the Rust side.
 *
 * The `AppError` shape mirrors `error.rs::AppError`'s `Serialize`
 * impl — see the `kind` enumeration there.
 */

export interface ServerStatus {
  handle: string;
  bind: string;
  pid: number;
  uptime_secs: number | null;
  running: boolean;
}

export interface RepoSummary {
  name: string;
  path: string;
  default_branch: string;
  size_bytes: number;
}

export interface RefEntry {
  sha: string;
  name: string;
  kind: 'local' | 'remote' | 'tag' | 'other' | string;
}

export interface CommitEntry {
  sha: string;
  short_sha: string;
  author: string;
  email: string;
  message: string;
  date_iso: string;
}

export interface RepoDetail {
  name: string;
  path: string;
  default_branch: string;
  refs: RefEntry[];
  commits: CommitEntry[];
}

export interface VersionEntryDto {
  version: number;
  bytes_sha256: string;
  byte_len: number;
  created_at_unix_ms: number;
  change_note: string | null;
}

export interface VersionDiffDto {
  key: string;
  base_version: number;
  head_version: number;
  object_changed: boolean;
  file_size_delta: number;
}

export interface AdminPasswordStatus {
  is_set: boolean;
  length: number;
}

export interface AppInfo {
  name: string;
  version: string;
  data_dir: string;
  config_dir: string;
  repos_dir: string;
  vault_dir: string;
  current_locale: string;
}

export interface VaultDiagnostic {
  reachable: boolean;
  key_count: number;
  backend: string;
  root: string;
}

export interface AppErrorPayload {
  kind: string;
  message: string;
  source: string;
}

/* ---------- Knowledge graph (PR-B Rust backend) ----------
 *
 * Shapes mirror `commands/graph.rs`. The Rust side serializes
 * graph nodes/edges as `serde_json::Value` so the JSON contract
 * on the wire is `Record<string, unknown>`; we cast into our
 * typed `GraphNode` / `GraphEdge` at the consumption boundary in
 * `src/lib/graph/types.ts` (frontend fallback parser) and in the
 * store (`src/lib/stores/graph.ts`).
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

export interface GraphNode {
  id: string;
  kind: NodeKind;
  title: string;
  body?: string;
  tags: string[];
  source?: string;
  created_at: string;
  updated_at: string;
  properties?: Record<string, unknown>;
}

export interface GraphEdge {
  id: string;
  kind: EdgeKind;
  from: string;
  to: string;
  weight?: number;
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

export interface DocReadResult {
  path: string;
  title: string;
  content: string;
}

export interface GraphLoadResult {
  nodes: GraphNode[];
  edges: GraphEdge[];
  docs: DocMeta[];
}

export interface AppVersion { name: string; version: string; }

/* ---------- AI review (T9) ----------
 *
 * Shapes mirror `commands/ai.rs`. The Rust side tags every event with
 * `#[serde(tag = "type")]` and renames fields to camelCase, so the
 * discriminated union below is the literal wire shape rather than a
 * hand-maintained parallel copy.
 */

export interface ReviewStartedDto {
  session_id: string;
  provider: string;
  model: string;
  streaming: boolean;
  redactions: number;
}

export type ReviewEventDto =
  | { type: 'token'; sessionId: string; delta: string }
  | { type: 'model'; sessionId: string; model: string }
  | { type: 'done'; sessionId: string; tokens: number }
  | { type: 'failed'; sessionId: string; message: string }
  | { type: 'cancelled'; sessionId: string };

/** Provider registry keys. Mirrors `src/ai/registry.rs::builtin_specs`. */
export const AI_PROVIDERS = [
  'openai',
  'deepseek',
  'ollama',
  'vllm',
  'anthropic',
] as const;

export type AiProviderKey = (typeof AI_PROVIDERS)[number];