/**
 * Repository store. Holds the current list and a per-repo detail
 * cache. The detail endpoint is heavy (refs + commits) so we
 * lazy-fetch on `repoDetail()` and keep the result in a map.
 */

import { writable, get } from 'svelte/store';
import * as tauri from '$lib/api/tauri';
import type { RepoDetail, RepoSummary } from '$lib/api/types';

export const repos = writable<RepoSummary[]>([]);
export const repoDetails = writable<Record<string, RepoDetail>>({});
export const reposLoading = writable<boolean>(false);

export async function refreshRepos(): Promise<void> {
  reposLoading.set(true);
  try {
    const next = await tauri.listRepos();
    repos.set(next ?? []);
  } catch (_err) {
    // Mock fallback: leave previous state untouched. The UI banner
    // already explains the dev-mode behaviour.
  } finally {
    reposLoading.set(false);
  }
}

export async function repoDetail(name: string): Promise<RepoDetail | null> {
  const cache = get(repoDetails);
  if (cache[name]) return cache[name];
  try {
    const detail = await tauri.repoDetail(name, 30);
    if (detail) {
      repoDetails.update((prev) => ({ ...prev, [name]: detail }));
    }
    return detail;
  } catch (_err) {
    return null;
  }
}

export async function cloneUrl(name: string, bind?: string): Promise<string> {
  return await tauri.cloneUrl(name, bind ?? null);
}

export async function openRepoInShell(name: string): Promise<void> {
  await tauri.openRepoInShell(name);
}
