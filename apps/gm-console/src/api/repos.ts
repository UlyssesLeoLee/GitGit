import { getClient } from './client';
import type { HealthResponse, RepoDetail, RepoLogEntry, RepoRef, RepoSummary } from './types';

/** Health probe — returns null on failure so the Settings page can
 *  render a generic "offline" badge without a thrown query. */
export async function health(): Promise<HealthResponse> {
  const resp = await getClient().get<HealthResponse>('/health');
  return resp.data;
}

/** List every repo currently registered with the server. */
export async function listRepos(): Promise<RepoSummary[]> {
  const resp = await getClient().get<RepoSummary[]>('/repos');
  return resp.data;
}

/** Detail view for a single repo (refs + recent log). */
export async function getRepo(name: string): Promise<RepoDetail> {
  const resp = await getClient().get<RepoDetail>(`/repos/${encodeURIComponent(name)}`);
  return resp.data;
}

/** All refs for a repo. */
export async function getRepoRefs(name: string): Promise<RepoRef[]> {
  const resp = await getClient().get<RepoRef[]>(`/repos/${encodeURIComponent(name)}/refs`);
  return resp.data;
}

/** Recent commit log (newest first). */
export async function getRepoLog(name: string): Promise<RepoLogEntry[]> {
  const resp = await getClient().get<RepoLogEntry[]>(`/repos/${encodeURIComponent(name)}/log`);
  return resp.data;
}