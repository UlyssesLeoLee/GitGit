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

/* ---------- Working-tree status and diff (T4) ----------
 *
 * Shapes mirror `commands/repos.rs` (`RepoStatus` / `StatusEntry` /
 * `RepoDiff`), which in turn serializes the plain values returned by the
 * `gitgit::repo::status` library module.
 *
 * `target` is the question being asked and is never inferred by the
 * frontend: 'staged' is index vs HEAD, 'worktree' is work tree vs index,
 * 'head' is work tree vs HEAD. They are three different diffs.
 */

/** One changed path in the status list. */
export interface StatusEntry {
  path: string;
  /** Source path of a rename or copy, otherwise null. */
  orig_path: string | null;
  /** 'M' | 'A' | 'D' | 'R' | … or null when the porcelain column is blank. */
  index_status: string | null;
  worktree_status: string | null;
  staged: boolean;
  unstaged: boolean;
  untracked: boolean;
}

export interface RepoStatus {
  branch: string | null;
  head: string | null;
  upstream: string | null;
  ahead: number;
  behind: number;
  entries: StatusEntry[];
  /** `entries` is empty. A clean tree is an answer, not an error. */
  is_clean: boolean;
}

export type DiffTarget = 'staged' | 'worktree' | 'head';

export interface RepoDiff {
  target: DiffTarget;
  /** The repo-relative path the diff was scoped to, if any. */
  path: string | null;
  text: string;
  /** True when `text` was cut at the byte cap. */
  truncated: boolean;
  /**
   * True when the scoped path is untracked and the text was synthesized,
   * because `git diff` reports nothing for a file git has never seen.
   */
  untracked: boolean;
  files: number;
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

/* ---------- Conflict arbitration (ADR-0024) ----------
 *
 * Shapes mirror `commands/ai.rs`'s `ConflictRegionDto`,
 * `ConflictStarted` and `ConflictEvent`, plus `SuggestionOutcome`.
 *
 * `[FACT]` The Rust side serializes `ConflictRegionDto` with
 * `rename_all = "camelCase"` and deserializes it the same way, so the
 * webview sends and receives camelCase field names.
 */

/**
 * One conflict region, as the webview holds it.
 *
 * `[PROPOSAL]` ADR-0024 §3.7.2: every region carries an id that is a
 * function of its own content, so a suggestion cannot be bound to a
 * region it was not asked about.
 */
export interface ConflictRegion {
  /** The region's stable id (ADR §3.7.2). */
  id: string;
  /** The repository-relative path of the conflicted file. */
  path: string;
  /** The base blob's object id, when the base was available. */
  baseOid: string;
  /** The region's ordinal in the merged output. */
  index: number;
  /** The ours side's text, verbatim. */
  ours: string;
  /** The base side's text, verbatim. */
  base: string;
  /** The theirs side's text, verbatim. */
  theirs: string;
  /** The base file's lines before the region. */
  contextBefore: string;
  /** The base file's lines after the region. */
  contextAfter: string;
  /**
   * The scope the region lives in, as a human label (ADR §3.8).
   *
   * `[PROPOSAL]` The scope layer only *locates* a region in the first
   * delivery, so this is a label for the user and not a decision.
   */
  scopeLabel: string | null;
  /**
   * Whether the region's two sides carry the same text.
   *
   * `[FACT]` ADR-0024 §3.7.4: "the region's ours and theirs text are the
   * same — the region exists but the two sides are equivalent; the default
   * suggestion is keep ours".
   *
   * `[FACT]` The judgement is the **library's**
   * (`ConflictRegion::sides_are_equivalent`, `src/ai/conflict.rs`) and it is
   * made **once**, in the backend (`commands/conflict.rs`). The webview
   * reads this flag and does not compare the two sides again.
   */
  sidesEquivalent: boolean;
}

/**
 * The conflict regions of a repository, and the paths git has to handle.
 *
 * `[FACT]` Two lists, because they answer two different questions: the
 * regions the user can arbitrate here, and the paths the core refused to
 * read — an add/delete or rename conflict has no base stage, and binary
 * content is not text (ADR §3.7.4).
 *
 * `[PROPOSAL]` The needs-git paths are **reported**, not dropped: a page
 * that said "no conflict regions" while git still had work to do would be
 * lying.
 */
export interface ConflictRegions {
  /** The regions, in the order the merged output produced them. */
  regions: ConflictRegion[];
  /** The repository-relative paths the core leaves to git. */
  needsGit: string[];
}

/**
 * What the UI learns when an arbitration starts.
 *
 * `[FACT]` Mirrors `ConflictStarted`: the fields are renamed to
 * camelCase by the Rust side's `rename_all`.
 */
export interface ConflictStartedDto {
  sessionId: string;
  regionId: string;
  provider: string;
  model: string;
  streaming: boolean;
  redactions: number;
}

/**
 * What became of the model's answer.
 *
 * `[FACT]` `SuggestionOutcome` is tagged with `state` on the Rust side
 * and its two variants are `accepted` (carrying the replacement) and
 * `rejected` (carrying the reason).
 */
export type SuggestionOutcomeDto =
  | { state: 'accepted'; suggestion: string }
  | { state: 'rejected'; reason: string };

/**
 * What the human chose for a region (ADR-0024 §3.7.3's `choice` field).
 *
 * `[FACT]` The four names are the ADR's own: `ours` / `theirs` / `ai` /
 * `manual`. They are the **decision**, not the text: the text that was
 * chosen travels with the region, and the backend recomputes the region's
 * id rather than believing a caller about it.
 *
 * `[INFERENCE]` The union lives here rather than in the store because the
 * bridge and the store both need it, and the bridge cannot import from the
 * store without a cycle.
 */
export type ConflictChoice = 'ours' | 'theirs' | 'manual' | 'ai';

/**
 * The choices the write-back command accepts.
 *
 * `[FACT]` The core keeps six (`RegionChoice`: ours, theirs, base, both,
 * ai, manual) and the command maps all six. The dialog offers four
 * (ADR-0024 §3.4.3): `base` and `both` stay in the library.
 */
export type ConflictResolveChoiceKind =
  | 'ours'
  | 'theirs'
  | 'base'
  | 'both'
  | 'ai'
  | 'manual';

/**
 * One region's decision, as it goes over the wire.
 *
 * `[FACT]` `index` is the region's ordinal in the merged output
 * (`Region::index`), which is the key the core's `compose_resolved` looks
 * the choice up by. `text` is the replacement text: required for `ai` and
 * `manual`, ignored by the other four.
 */
export interface ConflictResolveChoice {
  /** The region's ordinal in the merged output. */
  index: number;
  /** Which side won. */
  choice: ConflictResolveChoiceKind;
  /** The replacement text, for `ai` and `manual`. */
  text?: string;
}

/**
 * What the write-back command is asked to do (ADR-0024 §3.6).
 *
 * `[FACT]` One path per call: a merge spans several files, and the
 * webview groups the decisions by path and calls the command once per
 * path.
 */
export interface ConflictResolvePayload {
  /** The repository name, as the desktop keeps it. */
  repo: string;
  /** The working-tree root the repository name is resolved under. */
  root: string | null;
  /** The repository-relative path to write back. */
  path: string;
  /** One choice per region of that path. */
  choices: ConflictResolveChoice[];
}

/**
 * What the write-back command did.
 */
export interface ConflictResolveResult {
  /** The repository-relative path that was written back and staged. */
  path: string;
  /** How many conflict regions the path carried. */
  regions: number;
}

/**
 * One event on the conflict channel.
 *
 * `[FACT]` Mirrors `ConflictEvent`: a `type` tag for dispatch, camelCase
 * fields for this consumer, and a `sessionId` on every event so a late
 * event from a cancelled stream can be discarded.
 */
export type ConflictEventDto =
  | { type: 'token'; sessionId: string; regionId: string; delta: string }
  | { type: 'model'; sessionId: string; regionId: string; model: string }
  | {
      type: 'done';
      sessionId: string;
      regionId: string;
      tokens: number;
      outcome: SuggestionOutcomeDto;
    }
  | { type: 'failed'; sessionId: string; regionId: string; message: string }
  | { type: 'cancelled'; sessionId: string };

/**
 * Providers whose protocol has a streaming implementation.
 *
 * `[FACT]` Mirrors `ProviderSpec::supports_streaming`
 * (`src/ai/registry.rs:98-103`): the OpenAI-compatible protocol streams
 * and Anthropic's does not. `[FACT]` `send_stream`'s default
 * implementation refuses rather than falling back
 * (`src/ai/provider.rs:199-209`), so a provider that is not listed here
 * cannot be asked for a stream at all.
 */
export const STREAMING_PROVIDERS = ['openai', 'deepseek', 'ollama', 'vllm'] as const;

/**
 * Whether `provider` can stream.
 *
 * `[FACT]` A mirror of the registry's rule, so the dialog can decide
 * whether to offer the "ask AI" option *before* the request rather than
 * after the refusal. The Rust side refuses the same set
 * (`AiReviewUnsupported`).
 */
export function supportsStreaming(provider: string): boolean {
  return (STREAMING_PROVIDERS as readonly string[]).includes(provider);
}