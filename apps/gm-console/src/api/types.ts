// API DTOs — 1:1 mirror of `src/server/api.rs` (V0.1 surface).
// Field names are snake_case to match the wire format; we keep them
// snake_case on the client to avoid lossy renames when types cross the
// network boundary. Local-only types (envelopes, derived views) live in
// other files.

export interface HealthResponse {
  status: 'ok' | string;
  version: string;
  backend: string;
  vault_online: boolean;
}

export interface RepoSummary {
  name: string;
  path: string;
  head_ref: string | null;
  head_sha: string | null;
  ref_count: number;
}

export interface RepoRef {
  name: string;
  sha: string;
}

export interface RepoLogEntry {
  sha: string;
  short_sha: string;
  subject: string;
}

export interface RepoDetail extends RepoSummary {
  refs: RepoRef[];
  log: RepoLogEntry[];
}

export interface VaultKey {
  key: string;
  version_count: number;
  current_version: number | null;
  byte_len: number | null;
}

export interface VaultVersionSummary {
  version: number;
  bytes_sha256: string;
  byte_len: number;
  created_at_unix_ms: number;
  change_note: string | null;
}

export interface VaultKeyDetail {
  key: string;
  value: string | null;
  current_version: number | null;
  byte_len: number | null;
  bytes_sha256: string | null;
  created_at_unix_ms: number | null;
  change_note: string | null;
}

export interface VaultVersionsResponse {
  key: string;
  versions: VaultVersionSummary[];
}

export interface VaultVersionDiff {
  base: VaultVersionSummary;
  head: VaultVersionSummary;
  object_changed: boolean;
  file_size_delta: number;
}

export interface SetVersionBody {
  value: string;
  change_note?: string | null;
}

export interface SetVersionResponse {
  key: string;
  version: number;
}

export interface RestoreBody {
  target_version: number;
}

export interface RestoreResponse {
  key: string;
  target_version: number;
  new_version: number;
}

export interface ApiErrorBody {
  error: string;
  code?: string;
}