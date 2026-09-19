import { getClient } from './client';
import type {
  RestoreBody,
  RestoreResponse,
  SetVersionBody,
  SetVersionResponse,
  VaultKey,
  VaultKeyDetail,
  VaultVersionDiff,
  VaultVersionsResponse,
} from './types';

/** List every user-visible vault key. Internal `_attachments` / `_versions`
 *  sidecars are filtered server-side; the client doesn't need to know.
 */
export async function listKeys(): Promise<VaultKey[]> {
  const resp = await getClient().get<VaultKey[]>('/vault/keys');
  return resp.data;
}

/** Detail view for a single key, including the current plaintext value. */
export async function getKey(key: string): Promise<VaultKeyDetail> {
  const resp = await getClient().get<VaultKeyDetail>(`/vault/keys/${encodeURIComponent(key)}`);
  return resp.data;
}

/** Full version timeline (oldest first). */
export async function getVersions(key: string): Promise<VaultVersionsResponse> {
  const resp = await getClient().get<VaultVersionsResponse>(
    `/vault/keys/${encodeURIComponent(key)}/versions`,
  );
  return resp.data;
}

/** Create a new version for the key (also overwrites the current value). */
export async function setVersion(key: string, body: SetVersionBody): Promise<SetVersionResponse> {
  const resp = await getClient().post<SetVersionResponse>(
    `/vault/keys/${encodeURIComponent(key)}/versions`,
    body,
  );
  return resp.data;
}

/** Compute the diff between two versions. */
export async function diffVersions(
  key: string,
  base: number,
  head: number,
): Promise<VaultVersionDiff> {
  const resp = await getClient().get<VaultVersionDiff>(
    `/vault/keys/${encodeURIComponent(key)}/diff`,
    { params: { base, head } },
  );
  return resp.data;
}

/** Restore the key to `target_version`, creating a new version with the same bytes. */
export async function restoreVersion(key: string, body: RestoreBody): Promise<RestoreResponse> {
  const resp = await getClient().post<RestoreResponse>(
    `/vault/keys/${encodeURIComponent(key)}/restore`,
    body,
  );
  return resp.data;
}

/** Delete a key entirely. Resolves on 204 with no payload. */
export async function deleteKey(key: string): Promise<void> {
  await getClient().delete(`/vault/keys/${encodeURIComponent(key)}`);
}