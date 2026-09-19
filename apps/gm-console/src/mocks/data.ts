/**
 * In-memory seed data for the MSW handlers. Used by the dev-only mock
 * server so the SPA can be demoed without a live gitgit-server. The
 * shape mirrors `src/server/api.rs` DTOs 1:1 so failures caught here
 * would surface in production too.
 */
import type {
  HealthResponse,
  RepoDetail,
  RepoLogEntry,
  RepoRef,
  RepoSummary,
  VaultKey,
  VaultKeyDetail,
  VaultVersionDiff,
  VaultVersionSummary,
  VaultVersionsResponse,
} from '@/api/types';

export const mockHealth: HealthResponse = {
  status: 'ok',
  version: '0.1.0-mock',
  backend: 'mock',
  vault_online: true,
};

const repoAlphaRefs: RepoRef[] = [
  { name: 'refs/heads/main', sha: 'a'.repeat(40) },
  { name: 'refs/heads/feature/x', sha: 'b'.repeat(40) },
];
const repoAlphaLog: RepoLogEntry[] = [
  { sha: 'c'.repeat(40), short_sha: 'c'.repeat(7), subject: 'Initial commit' },
  { sha: 'd'.repeat(40), short_sha: 'd'.repeat(7), subject: 'Add README' },
];

export const mockRepos: RepoSummary[] = [
  {
    name: 'alpha.git',
    path: '/var/git/alpha.git',
    head_ref: 'refs/heads/main',
    head_sha: repoAlphaRefs[0].sha,
    ref_count: repoAlphaRefs.length,
  },
  {
    name: 'beta.git',
    path: '/var/git/beta.git',
    head_ref: null,
    head_sha: null,
    ref_count: 0,
  },
];

export const mockRepoDetails: Record<string, RepoDetail> = {
  'alpha.git': {
    ...mockRepos[0],
    refs: repoAlphaRefs,
    log: repoAlphaLog,
  },
  'beta.git': {
    ...mockRepos[1],
    refs: [],
    log: [],
  },
};

export const mockRepoRefs: Record<string, RepoRef[]> = {
  'alpha.git': repoAlphaRefs,
  'beta.git': [],
};

export const mockRepoLog: Record<string, RepoLogEntry[]> = {
  'alpha.git': repoAlphaLog,
  'beta.git': [],
};

export const mockVaultVersions = (key: string): VaultVersionSummary[] => [
  {
    version: 1,
    bytes_sha256: '1'.repeat(64),
    byte_len: 8,
    created_at_unix_ms: Date.now() - 86_400_000,
    change_note: 'initial',
  },
  {
    version: 2,
    bytes_sha256: '2'.repeat(64),
    byte_len: 10,
    created_at_unix_ms: Date.now() - 3_600_000,
    change_note: null,
  },
];

export const mockVaultKeys: VaultKey[] = [
  {
    key: 'openai',
    version_count: 2,
    current_version: 2,
    byte_len: 10,
  },
  {
    key: 'anthropic',
    version_count: 1,
    current_version: 1,
    byte_len: 7,
  },
];

export const mockVaultKeyDetail = (key: string): VaultKeyDetail => {
  const v = mockVaultVersions(key);
  return {
    key,
    value: `secret-${key}-v2`,
    current_version: v.length > 0 ? v[v.length - 1].version : null,
    byte_len: v.length > 0 ? v[v.length - 1].byte_len : null,
    bytes_sha256: v.length > 0 ? v[v.length - 1].bytes_sha256 : null,
    created_at_unix_ms: v.length > 0 ? v[v.length - 1].created_at_unix_ms : null,
    change_note: v.length > 0 ? v[v.length - 1].change_note : null,
  };
};

export const mockVaultVersionsResponse = (key: string): VaultVersionsResponse => ({
  key,
  versions: mockVaultVersions(key),
});

export const mockVaultDiff = (key: string, base: number, head: number): VaultVersionDiff => {
  const all = mockVaultVersions(key);
  const baseV = all.find((v) => v.version === base) ?? all[0];
  const headV = all.find((v) => v.version === head) ?? all[all.length - 1];
  return {
    base: baseV,
    head: headV,
    object_changed: baseV.bytes_sha256 !== headV.bytes_sha256,
    file_size_delta: headV.byte_len - baseV.byte_len,
  };
};

/** Mutable in-memory store for the duration of the mock session. */
export const mockStore: {
  repos: RepoSummary[];
  repoDetails: Record<string, RepoDetail>;
  repoRefs: Record<string, RepoRef[]>;
  repoLog: Record<string, RepoLogEntry[]>;
  vaultKeys: VaultKey[];
  vaultDetails: Record<string, VaultKeyDetail>;
  vaultVersions: Record<string, VaultVersionSummary[]>;
} = {
  repos: mockRepos,
  repoDetails: mockRepoDetails,
  repoRefs: mockRepoRefs,
  repoLog: mockRepoLog,
  vaultKeys: mockVaultKeys,
  vaultDetails: {
    openai: mockVaultKeyDetail('openai'),
    anthropic: mockVaultKeyDetail('anthropic'),
  },
  vaultVersions: {
    openai: mockVaultVersions('openai'),
    anthropic: mockVaultVersions('anthropic'),
  },
};