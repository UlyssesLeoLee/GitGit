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

export async function refreshVault(): Promise<void> {
  vaultBusy.set(true);
  try {
    const keys = (await tauri.vaultList()) ?? [];
    const out: VaultSecret[] = [];
    for (const k of keys) {
      const v = await tauri.vaultGet(k);
      out.push({
        key: k,
        value: v ?? '',
        version: out.length + 1,
      });
    }
    vaultSecrets.set(out);
  } catch (_err) {
    // Mock fallback; keep previous snapshot.
  } finally {
    vaultBusy.set(false);
  }
}

export async function vaultSet(key: string, value: string): Promise<number> {
  const ver = await tauri.vaultSet(key, value);
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
  await refreshVault();
  return ver ?? 0;
}

export async function vaultRotate(key: string): Promise<number> {
  const ver = await tauri.vaultRotate(key);
  await refreshVault();
  return ver ?? 0;
}

export async function vaultDelete(key: string): Promise<void> {
  await tauri.vaultDelete(key);
  await refreshVault();
}
