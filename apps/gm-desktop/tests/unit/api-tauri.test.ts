/**
 * Wire-contract tests for the Tauri command wrappers in
 * `src/lib/api/tauri.ts`.
 *
 * These wrappers were 45.83% line-covered: everything reached through
 * an existing store had been executed, but the wrappers a store never
 * calls in a test run had not been. That leaves exactly the part of
 * this file most likely to break silently — the *argument names*.
 *
 * Serde on the Rust side matches named fields, so renaming
 * `serverBind` to `server_bind` or `targetVersion` to `target_version`
 * does not raise a type error here: it compiles, and then every call
 * quietly receives `None` for that argument at runtime. The recording
 * bridge below therefore asserts the exact `{ cmd, args }` pair, not
 * merely that *some* invoke happened.
 *
 * The round-trip cases run against the real `src/mocks/handlers.ts`
 * fixtures rather than hand-rolled payloads, so they check the
 * wrapper against the documented wire shape end to end.
 */
import { beforeEach, describe, expect, it } from 'vitest';
import * as tauri from '../../src/lib/api/tauri';
import { installMock } from '../../src/mocks/handlers';

interface Call { cmd: string; args: Record<string, unknown> | undefined }

/**
 * Replace the invoke bridge with one that records the exact command
 * and argument object the wrapper sent, and resolves `result`.
 */
function recordCalls(result: unknown = null): Call[] {
  const calls: Call[] = [];
  window.__TAURI_INTERNALS__ = {
    invoke: (cmd: string, args?: Record<string, unknown>) => {
      calls.push({ cmd, args });
      return Promise.resolve(result);
    },
  };
  return calls;
}

/** Every command name the module sends, in call order. */
function commands(calls: Call[]): string[] {
  return calls.map((c) => c.cmd);
}

beforeEach(() => {
  // `[FACT]` This used to `delete window.__TAURI_INTERNALS__` first,
  // because `installMock()` skipped reinstalling when the installed
  // `invoke`'s source text happened to contain "mock" — which a
  // hand-written stub in this file could. The mock now identifies itself
  // with a symbol, so reinstalling is unconditional and the cases are
  // order-independent on their own. `mocks-handlers.test.ts` pins both
  // the old defect and the new behaviour.
  installMock();
});

describe('api / tauri — argument names on the wire', () => {
  it('sends the bind the caller asked for', async () => {
    const calls = recordCalls();
    await tauri.startServer('0.0.0.0:9000');
    expect(calls).toEqual([{ cmd: 'start_server', args: { bind: '0.0.0.0:9000' } }]);
  });

  it('sends an explicit null bind rather than omitting it', async () => {
    // `null` is meaningful here: it asks Rust for its own default.
    // Omitting the key would decode as a missing field instead.
    const calls = recordCalls();
    await tauri.startServer(null);
    expect(calls[0]?.args).toEqual({ bind: null });
  });

  it('sends the log limit as a number', async () => {
    const calls = recordCalls([]);
    await tauri.serverLogs(25);
    expect(calls[0]?.args).toEqual({ limit: 25 });
  });

  it('sends the server bind to clone_url as `serverBind`', async () => {
    // `[FACT]` The one renamed key in this file. The Rust parameter is
    // `server_bind`, and the JS side must send the camelCase spelling
    // the Tauri bridge deserializes into it. A rename to `bind` would
    // still typecheck here and would silently drop the bind.
    const calls = recordCalls('http://x/repos/demo.git');
    await tauri.cloneUrl('demo', '127.0.0.1:38080');
    expect(calls[0]?.args).toEqual({ name: 'demo', serverBind: '127.0.0.1:38080' });
  });

  it('sends the target version to vault_restore as `targetVersion`', async () => {
    // The second camelCase key. `target_version` / `target` would
    // compile and then restore the wrong thing.
    const calls = recordCalls(3);
    await tauri.vaultRestore('demo.api_key', 2);
    expect(calls[0]?.args).toEqual({ key: 'demo.api_key', targetVersion: 2 });
  });

  it('passes the diff target through verbatim instead of defaulting it', async () => {
    // Rust refuses an unknown target rather than defaulting it, so
    // the wrapper must not quietly substitute one of the three.
    const calls = recordCalls();
    await tauri.repoDiff('demo', 'staged', 'src/dirty.ts', '/checkouts');
    expect(calls[0]?.args).toEqual({
      name: 'demo',
      target: 'staged',
      path: 'src/dirty.ts',
      root: '/checkouts',
    });
  });

  it('sends an unscoped repo_status as a null root', async () => {
    const calls = recordCalls();
    await tauri.repoStatus('demo', null);
    expect(calls[0]?.args).toEqual({ name: 'demo', root: null });
  });

  it('sends base and head to vault_diff as numbers', async () => {
    // If either were stringified, Rust's integer fields would fail to
    // deserialize. `typeof` is the assertion, not just the value.
    const calls = recordCalls({});
    await tauri.vaultDiff('demo.api_key', 1, 2);
    expect(calls[0]?.args).toEqual({ key: 'demo.api_key', base: 1, head: 2 });
    expect(typeof calls[0]?.args?.base).toBe('number');
    expect(typeof calls[0]?.args?.head).toBe('number');
  });

  it('sends the commit limit to repo_detail under `limit`', async () => {
    const calls = recordCalls();
    await tauri.repoDetail('demo', 50);
    expect(calls[0]?.args).toEqual({ name: 'demo', limit: 50 });
  });

  it('gives every no-argument command the right name', async () => {
    // The command *strings* are the other half of the contract and
    // are just as easy to typo as the keys.
    const calls = recordCalls();
    await tauri.serverStatus();
    await tauri.stopServer();
    await tauri.clearLogs();
    await tauri.listRepos();
    await tauri.openRepoInShell('demo');
    await tauri.vaultList();
    await tauri.getAdminPasswordStatus();
    await tauri.clearAdminPassword();
    await tauri.appInfo();
    await tauri.vaultDiagnostics();
    expect(commands(calls)).toEqual([
      'server_status',
      'stop_server',
      'clear_logs',
      'list_repos',
      'open_repo_in_shell',
      'vault_list',
      'get_admin_password_status',
      'clear_admin_password',
      'app_info',
      'vault_diagnostics',
    ]);
  });
});

describe('api / tauri — refusals reach the caller', () => {
  it('propagates a typed backend refusal without wrapping it', async () => {
    // The module doc says the wrappers deliberately do not catch, so
    // that stores can read `kind` and pick a localized message. A
    // `try/catch` added here would collapse every refusal into one
    // generic error and break every `errorKind` assertion downstream.
    await expect(tauri.repoDiff('demo', 'sideways' as never, null, null)).rejects.toMatchObject({
      kind: 'InvalidDiffTarget',
    });
  });

  it('propagates a bare Error unchanged too', async () => {
    const boom = new Error('bridge is gone');
    window.__TAURI_INTERNALS__ = { invoke: () => Promise.reject(boom) };
    await expect(tauri.listRepos()).rejects.toBe(boom);
  });
});

describe('api / tauri — round trips against the mock backend', () => {
  it('reads the server as stopped before anything starts it', async () => {
    const s = await tauri.serverStatus();
    expect(s.running).toBe(false);
    expect(s.handle).toBe('embedded');
  });

  it('starts a server and reports the bind it came up on', async () => {
    const s = await tauri.startServer('127.0.0.1:38081');
    expect(s.running).toBe(true);
    expect(s.bind).toBe('127.0.0.1:38081');
  });

  it('returns the prior snapshot from stop_server', async () => {
    // Documented Rust choice: the stop command answers with what it
    // killed, so callers can log it. The wrapper must not swap it for
    // the post-stop status.
    await tauri.startServer('127.0.0.1:38082');
    const prev = await tauri.stopServer();
    expect(prev.running).toBe(true);
    expect((await tauri.serverStatus()).running).toBe(false);
  });

  it('returns the log lines and a null from clear_logs', async () => {
    const logs = await tauri.serverLogs(10);
    expect(logs.length).toBeGreaterThan(0);
    expect(logs[0]).toContain('mock_server');
    expect(await tauri.clearLogs()).toBeNull();
  });

  it('lists the fixture repositories', async () => {
    const repos = await tauri.listRepos();
    expect(repos.map((r) => r.name)).toEqual(expect.arrayContaining(['demo', 'hello-world']));
  });

  it('reads a repository detail and nulls an unknown name', async () => {
    const detail = await tauri.repoDetail('demo', 20);
    expect(detail.name).toBe('demo');
    expect(detail.commits.length).toBeGreaterThan(0);
    // A missing repository is `null`, not a throw: the page has to be
    // able to tell "gone" from "broken".
    expect(await tauri.repoDetail('nope', 20)).toBeNull();
  });

  it('builds a clone url from the bind', async () => {
    expect(await tauri.cloneUrl('demo', '127.0.0.1:38080')).toBe(
      'http://127.0.0.1:38080/repos/demo.git'
    );
  });

  it('resolves open_repo_in_shell to null', async () => {
    // The mock has nothing to open. The point of the case is that the
    // wrapper resolves rather than rejecting, and does not turn the
    // backend's `null` into `undefined`.
    expect(await tauri.openRepoInShell('demo')).toBeNull();
  });

  it('reads a working-tree status', async () => {
    const st = await tauri.repoStatus('demo', null);
    expect(st.is_clean).toBe(false);
    expect(st.entries.map((e) => e.path)).toEqual(
      expect.arrayContaining(['staged.txt', 'src/dirty.ts', 'notes.txt'])
    );
  });

  it('reads a diff for each accepted target', async () => {
    for (const target of ['staged', 'worktree', 'head'] as const) {
      const d = await tauri.repoDiff('demo', target, null, null);
      expect(d.target).toBe(target);
      expect(d.text.length).toBeGreaterThan(0);
    }
  });

  it('reads the vault, including a key that is not there', async () => {
    expect(await tauri.vaultGet('demo.api_key')).toBe('value-for-demo.api_key');
    expect(await tauri.vaultGet('never.written')).toBeNull();
    const keys = await tauri.vaultList();
    expect(keys).toContain('demo.api_key');
  });

  it('versions a key across set and rotate', async () => {
    // A private key, so the version numbers below cannot be perturbed
    // by another case writing to the shared mock store.
    expect(await tauri.vaultSet('t.rotate', 'v1')).toBe(1);
    expect(await tauri.vaultSet('t.rotate', 'v2')).toBe(2);
    expect(await tauri.vaultRotate('t.rotate')).toBe(3);
    const versions = await tauri.vaultVersions('t.rotate');
    expect(versions.map((v) => v.version)).toEqual([1, 2, 3]);
  });

  it('reports whether two vault versions differ', async () => {
    const d = await tauri.vaultDiff('demo.api_key', 1, 2);
    expect(d.key).toBe('demo.api_key');
    expect(d.object_changed).toBe(true);
    expect(d.file_size_delta).toBe(7);
  });

  it('restores an old version as a new one', async () => {
    // Restore is additive: the new entry keeps the old byte length and
    // records where it came from, so the history stays auditable.
    await tauri.vaultSet('t.restore', 'aaaaaaaaaaaa');
    const restored = await tauri.vaultRestore('t.restore', 1);
    const versions = await tauri.vaultVersions('t.restore');
    expect(restored).toBe(2);
    expect(versions).toHaveLength(2);
    expect(versions[1]?.change_note).toBe('restored-to-v1');
    expect(versions[1]?.byte_len).toBe(versions[0]?.byte_len);
  });

  it('deletes a key and drops it from the listing', async () => {
    await tauri.vaultSet('t.delete', 'v1');
    expect(await tauri.vaultDelete('t.delete')).toBeNull();
    expect(await tauri.vaultList()).not.toContain('t.delete');
    expect(await tauri.vaultGet('t.delete')).toBeNull();
  });

  it('sets, reports and clears the admin password', async () => {
    // One case for the whole cycle: the status answers depend on the
    // shared mock store, so splitting them would make each depend on
    // the order the file happens to run in.
    expect(await tauri.getAdminPasswordStatus()).toMatchObject({ is_set: false, length: 0 });
    expect(await tauri.setAdminPassword('hunter2')).toBe(1);
    expect(await tauri.getAdminPasswordStatus()).toMatchObject({ is_set: true });
    expect(await tauri.clearAdminPassword()).toBeNull();
    expect(await tauri.getAdminPasswordStatus()).toMatchObject({ is_set: false });
  });

  it('reads the app info the settings page shows', async () => {
    const info = await tauri.appInfo();
    expect(info.name).toBe('gm-desktop');
    expect(info.repos_dir).toContain('repos');
  });

  it('reads the vault diagnostics', async () => {
    const d = await tauri.vaultDiagnostics();
    expect(d.reachable).toBe(true);
    expect(d.backend).toBe('FileVault');
    expect(d.key_count).toBeGreaterThan(0);
  });
});
