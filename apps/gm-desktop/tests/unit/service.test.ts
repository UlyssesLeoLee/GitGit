import { describe, expect, it, vi, beforeAll } from 'vitest';
import { get } from 'svelte/store';
import { vaultSecrets, refreshVault, vaultSet, vaultVersions } from '../../src/lib/stores/vault';
import { repoDetail, refreshRepos } from '../../src/lib/stores/repos';
import { server, startServer, stopServer } from '../../src/lib/stores/server';
import { installMock } from '../../src/mocks/handlers';

beforeAll(() => {
  installMock();
});

describe('stores / server', () => {
  it('startServer transitions to running', async () => {
    await startServer('127.0.0.1:38080');
    const s = get(server);
    expect(s.running).toBe(true);
    expect(s.bind).toBe('127.0.0.1:38080');
  });

  it('stopServer transitions back to stopped', async () => {
    await stopServer();
    const s = get(server);
    expect(s.running).toBe(false);
    expect(s.bind).toBe('');
  });
});

describe('stores / repos', () => {
  it('refreshRepos populates the list', async () => {
    await refreshRepos();
    // refreshRepos uses the names from the mock store. We can't read
    // `repos` directly without exporting it; instead we go through
    // `repoDetail` to confirm the data layer works end-to-end.
    const detail = await repoDetail('demo');
    expect(detail).not.toBeNull();
    expect(detail?.name).toBe('demo');
  });
});

describe('stores / vault', () => {
  it('refreshVault / vaultSet / vaultVersions flow', async () => {
    await refreshVault();
    const before = get(vaultSecrets);
    expect(before.length).toBeGreaterThan(0);

    const ver = await vaultSet('test.key', 'hello world');
    expect(typeof ver).toBe('number');
    expect(ver).toBeGreaterThan(0);

    const list = await vaultVersions('test.key');
    expect(list.length).toBeGreaterThan(0);
  });
});
