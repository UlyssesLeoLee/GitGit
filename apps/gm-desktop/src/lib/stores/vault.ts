/**
 * Credential vault store. Maps 1:1 onto the gitgit
 * VersionedVault surface.
 */

import { writable, get } from 'svelte/store';
import * as tauri from '$lib/api/tauri';
import type {
  VersionDiffDto,
  VersionEntryDto,
} from '$lib/api/types';

export interface VaultSecret {
  key: string;
  /** Latest value (decoded). The UI never displays long-lived values
   * in plain text except in the explicit "show value" affordance. */
  value: string;
  version: number;
}

export const vaultSecrets = writable<VaultSecret[]>([]);
export const vaultVersionsCache = writable<Record<string, VersionEntryDto[]>>({});
export const vaultBusy = writable<boolean>(false);

/**
 * Drop cached version lists, for one key or for all of them.
 *
 * `[FACT]` Added because `vaultRestore` and `vaultRotate` each write a new
 * version and then re-read through `vaultVersions`, which answers from
 * this cache whenever the key is present. The key *is* present — the UI
 * has to expand the row before it can offer a restore button — so the
 * post-restore read returned the pre-restore list and the new version
 * stayed invisible until the page was remounted. Measured, and pinned as
 * a defect by a case in `routes-vault.test.ts`.
 */
export function invalidateVaultVersions(key?: string): void {
  vaultVersionsCache.update((prev) => {
    if (key === undefined) return {};
    if (!(key in prev)) return prev;
    const { [key]: _dropped, ...rest } = prev;
    return rest;
  });
}

export async function refreshVault(): Promise<void> {
  vaultBusy.set(true);
  try {
    const keys = (await tauri.vaultList()) ?? [];
    const out = await Promise.all(keys.map(async (k) => {
      const [v, versions] = await Promise.all([
        tauri.vaultGet(k),
        // `[FACT]` A failure here must not take down the whole list — the
        // key and its value are still worth showing, they just have no
        // version number to show yet.
        tauri.vaultVersions(k).catch(() => [] as VersionEntryDto[]),
      ]);
      // Priming the cache here is deliberate: the list is about to be
      // rendered, so the version fetch is not wasted.
      if (versions.length > 0) {
        vaultVersionsCache.update((prev) => ({ ...prev, [k]: versions }));
      }
      // Was `out.length + 1`, which is the key's position in the list, not
      // its version. The vault numbers versions per key, so two keys added
      // out of order reported 1 and 2 regardless of how many versions each
      // actually had. Take the highest real version.
      const latest = versions.reduce((max, e) => Math.max(max, e.version), 0);
      return { key: k, value: v ?? '', version: latest };
    }));
    vaultSecrets.set(out);
  } catch (_err) {
    // Mock fallback; keep previous snapshot.
  } finally {
    vaultBusy.set(false);
  }
}

export async function vaultSet(key: string, value: string): Promise<number> {
  const ver = await tauri.vaultSet(key, value);
  invalidateVaultVersions(key);
  await refreshVault();
  return ver ?? 0;
}

export async function vaultVersions(key: string): Promise<VersionEntryDto[]> {
  const cached = get(vaultVersionsCache)[key];
  if (cached) return cached;
  const fresh = (await tauri.vaultVersions(key)) ?? [];
  vaultVersionsCache.update((prev) => ({ ...prev, [key]: fresh }));
  return fresh;
}

export async function vaultDiff(key: string, base: number, head: number): Promise<VersionDiffDto | null> {
  return await tauri.vaultDiff(key, base, head);
}

export async function vaultRestore(key: string, target: number): Promise<number> {
  const ver = await tauri.vaultRestore(key, target);
  invalidateVaultVersions(key);
  await refreshVault();
  return ver ?? 0;
}

export async function vaultRotate(key: string): Promise<number> {
  const ver = await tauri.vaultRotate(key);
  invalidateVaultVersions(key);
  await refreshVault();
  return ver ?? 0;
}

export async function vaultDelete(key: string): Promise<void> {
  await tauri.vaultDelete(key);
  invalidateVaultVersions(key);
  await refreshVault();
}
