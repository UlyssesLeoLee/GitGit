import { beforeEach, describe, expect, it, vi } from 'vitest';
import { health, listRepos, getRepo, getRepoRefs, getRepoLog } from '@/api/repos';
import {
  deleteKey,
  diffVersions,
  getKey,
  getVersions,
  listKeys,
  restoreVersion,
  setVersion,
} from '@/api/vault';

// `vi.mock` is hoisted above the imports, so the spies have to be created
// through `vi.hoisted` to be referenceable from inside the factory.
const http = vi.hoisted(() => ({
  get: vi.fn(),
  post: vi.fn(),
  del: vi.fn(),
}));

vi.mock('@/api/client', () => ({
  getClient: () => ({ get: http.get, post: http.post, delete: http.del }),
}));

beforeEach(() => {
  http.get.mockReset();
  http.post.mockReset();
  http.del.mockReset();
  http.get.mockResolvedValue({ data: { ok: true } });
  http.post.mockResolvedValue({ data: { written: true } });
  http.del.mockResolvedValue({ data: undefined });
});

describe('repos endpoints', () => {
  it('health() unwraps the response body', async () => {
    http.get.mockResolvedValue({ data: { status: 'ok', version: '0.1.0' } });
    await expect(health()).resolves.toEqual({ status: 'ok', version: '0.1.0' });
    expect(http.get).toHaveBeenCalledWith('/health');
  });

  it('listRepos() unwraps the response body', async () => {
    http.get.mockResolvedValue({ data: [{ name: 'demo' }] });
    await expect(listRepos()).resolves.toEqual([{ name: 'demo' }]);
    expect(http.get).toHaveBeenCalledWith('/repos');
  });

  it('getRepo() percent-encodes the repo name', async () => {
    await getRepo('my repo/with slash');
    expect(http.get).toHaveBeenCalledWith('/repos/my%20repo%2Fwith%20slash');
  });

  it('getRepoRefs() targets the refs sub-resource', async () => {
    http.get.mockResolvedValue({ data: [{ name: 'refs/heads/main' }] });
    await expect(getRepoRefs('demo')).resolves.toEqual([{ name: 'refs/heads/main' }]);
    expect(http.get).toHaveBeenCalledWith('/repos/demo/refs');
  });

  it('getRepoLog() targets the log sub-resource', async () => {
    http.get.mockResolvedValue({ data: [{ sha: 'abc' }] });
    await expect(getRepoLog('demo')).resolves.toEqual([{ sha: 'abc' }]);
    expect(http.get).toHaveBeenCalledWith('/repos/demo/log');
  });

  it('propagates a rejected request to the caller', async () => {
    http.get.mockRejectedValue(new Error('offline'));
    await expect(listRepos()).rejects.toThrow('offline');
  });
});

describe('vault endpoints', () => {
  it('listKeys() targets the keys collection', async () => {
    http.get.mockResolvedValue({ data: [{ key: 'openai' }] });
    await expect(listKeys()).resolves.toEqual([{ key: 'openai' }]);
    expect(http.get).toHaveBeenCalledWith('/vault/keys');
  });

  it('getKey() percent-encodes the key name', async () => {
    await getKey('openai/api-token');
    expect(http.get).toHaveBeenCalledWith('/vault/keys/openai%2Fapi-token');
  });

  it('getVersions() targets the versions sub-resource', async () => {
    http.get.mockResolvedValue({ data: { versions: [] } });
    await expect(getVersions('openai')).resolves.toEqual({ versions: [] });
    expect(http.get).toHaveBeenCalledWith('/vault/keys/openai/versions');
  });

  it('setVersion() POSTs the body to the versions sub-resource', async () => {
    await setVersion('openai', { value: 'sk-test' });
    expect(http.post).toHaveBeenCalledWith('/vault/keys/openai/versions', { value: 'sk-test' });
  });

  it('diffVersions() passes base and head as query params', async () => {
    http.get.mockResolvedValue({ data: { added: 1, removed: 0 } });
    await expect(diffVersions('openai', 1, 3)).resolves.toEqual({ added: 1, removed: 0 });
    expect(http.get).toHaveBeenCalledWith('/vault/keys/openai/diff', {
      params: { base: 1, head: 3 },
    });
  });

  it('restoreVersion() POSTs the target version', async () => {
    await restoreVersion('openai', { target_version: 2 });
    expect(http.post).toHaveBeenCalledWith('/vault/keys/openai/restore', { target_version: 2 });
  });

  it('deleteKey() issues a DELETE and resolves without a payload', async () => {
    await expect(deleteKey('openai')).resolves.toBeUndefined();
    expect(http.del).toHaveBeenCalledWith('/vault/keys/openai');
  });

  it('propagates a rejected write to the caller', async () => {
    http.post.mockRejectedValue(new Error('vault down'));
    await expect(setVersion('openai', { value: 'x' })).rejects.toThrow('vault down');
  });
});
