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
