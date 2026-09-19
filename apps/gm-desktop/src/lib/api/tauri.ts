/**
 * Tauri command wrappers. Every function delegates to
 * `window.__TAURI_INTERNALS__.invoke` (or its v2 equivalent) using
 * `@tauri-apps/api/core::invoke`.
 *
 * When the Svelte app runs in a plain browser tab (during `vite
 * dev` without the Tauri runtime), `invoke()` throws synchronously.
 * We deliberately don't catch here — the stores catch at a higher
 * level — but the `mock/handlers.ts` module can patch the global
 * `invoke` so the dev workflow stays smooth.
 */

import { invoke as tauriInvoke } from '@tauri-apps/api/core';
import type {
  AdminPasswordStatus,
  AppInfo,
  CommitEntry,
  RefEntry,
  RepoDetail,
  RepoSummary,
  ServerStatus,
  VaultDiagnostic,
  VersionDiffDto,
  VersionEntryDto,
} from './types';

export async function serverStatus(): Promise<ServerStatus> {
  return await tauriInvoke('server_status');
}

export async function startServer(bind: string | null): Promise<ServerStatus> {
  return await tauriInvoke('start_server', { bind });
}

export async function stopServer(): Promise<ServerStatus> {
  return await tauriInvoke('stop_server');
}

export async function serverLogs(limit: number | null): Promise<string[]> {
  return await tauriInvoke('server_logs', { limit });
}

export async function clearLogs(): Promise<void> {
  return await tauriInvoke('clear_logs');
}

export async function listRepos(): Promise<RepoSummary[]> {
  return await tauriInvoke('list_repos');
}

export async function repoDetail(name: string, limit: number | null): Promise<RepoDetail> {
  return await tauriInvoke('repo_detail', { name, limit });
}

export async function cloneUrl(name: string, bind: string | null): Promise<string> {
  return await tauriInvoke('clone_url', { name, serverBind: bind });
}

export async function openRepoInShell(name: string): Promise<void> {
  return await tauriInvoke('open_repo_in_shell', { name });
}

export async function vaultList(): Promise<string[]> {
  return await tauriInvoke('vault_list');
}

export async function vaultGet(key: string): Promise<string | null> {
  return await tauriInvoke('vault_get', { key });
}

export async function vaultSet(key: string, value: string): Promise<number> {
  return await tauriInvoke('vault_set', { key, value });
}

export async function vaultRotate(key: string): Promise<number> {
  return await tauriInvoke('vault_rotate', { key });
}

export async function vaultVersions(key: string): Promise<VersionEntryDto[]> {
  return await tauriInvoke('vault_versions', { key });
}

export async function vaultDiff(key: string, base: number, head: number): Promise<VersionDiffDto> {
  return await tauriInvoke('vault_diff', { key, base, head });
}

export async function vaultRestore(key: string, targetVersion: number): Promise<number> {
  return await tauriInvoke('vault_restore', { key, targetVersion });
}

export async function vaultDelete(key: string): Promise<void> {
  return await tauriInvoke('vault_delete', { key });
}

export async function getAdminPasswordStatus(): Promise<AdminPasswordStatus> {
  return await tauriInvoke('get_admin_password_status');
}

export async function setAdminPassword(password: string): Promise<number> {
  return await tauriInvoke('set_admin_password', { password });
}

export async function clearAdminPassword(): Promise<void> {
  return await tauriInvoke('clear_admin_password');
}

export async function appInfo(): Promise<AppInfo> {
  return await tauriInvoke('app_info');
}

export async function vaultDiagnostics(): Promise<VaultDiagnostic> {
  return await tauriInvoke('vault_diagnostics');
}

// Silence "unused" warnings for types only referenced by
// generated bindings today; keep them as part of the public surface
// so consumer code can be typed without re-importing.
export type { CommitEntry, RefEntry };
