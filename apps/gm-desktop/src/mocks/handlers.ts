/**
 * Local mock layer that mimics the Rust Tauri commands when the app
 * is run under `vite dev` *without* the Tauri runtime (i.e. the
 * developer point the browser at `http://localhost:5173/` directly,
 * not via `tauri dev`).
 *
 * The real `invoke()` call throws synchronously in that case; we
 * intercept by patching `window.__TAURI_INTERNALS__` before the
 * app's first call site runs.
 *
 * Behaviors intentionally match the Rust `commands/*` shapes so the
 * Svelte side can stay API-blind about whether a Rust backend is
 * present.
 */

import type {
  AppInfo,
  RepoDetail,
  RepoSummary,
  ServerStatus,
  VersionDiffDto,
  VersionEntryDto,
} from '$lib/api/types';

interface MockStore {
  server: ServerStatus;
  repos: RepoSummary[];
  details: Map<string, RepoDetail>;
  versions: Map<string, VersionEntryDto[]>;
}

declare global {
  interface Window {
    __TAURI_INTERNALS__?: {
      invoke: (cmd: string, args?: Record<string, unknown>) => Promise<unknown>;
    };
  }
}

function initStore(): MockStore {
  const repoNames = ['demo', 'hello-world'];
  const repos: RepoSummary[] = repoNames.map((name, idx) => ({
    name,
    path: `/tmp/repos/${name}.git`,
    default_branch: idx === 0 ? 'main' : 'trunk',
    size_bytes: 1024 * (10 + idx * 7),
  }));
  const details = new Map<string, RepoDetail>();
  repos.forEach((r, idx) => {
    details.set(r.name, {
      name: r.name,
      path: r.path,
      default_branch: r.default_branch,
      refs: [
        {
          sha: `111111111111111111111111111111111111111${idx_to_hex(idx)}`,
          name: `refs/heads/${r.default_branch}`,
          kind: 'local',
        },
      ],
      commits: [
        {
          sha: 'a1b2c3d4e5f6' + idx_to_hex(idx) + '0000000000000000',
          short_sha: 'a1b2c3d',
          author: 'Ulysses',
          email: 'ulysses@example.com',
          message: 'init',
          date_iso: '2026-09-19T12:34:56+09:00',
        },
      ],
    });
  });
  return {
    server: {
      handle: 'embedded',
      bind: '',
      pid: 0,
      uptime_secs: null,
      running: false,
    },
    repos,
    details,
    versions: new Map<string, VersionEntryDto[]>([
      ['demo.api_key', [
        { version: 1, bytes_sha256: 'a'.repeat(64), byte_len: 17, created_at_unix_ms: Date.now() - 3600_000, change_note: null },
        { version: 2, bytes_sha256: 'b'.repeat(64), byte_len: 24, created_at_unix_ms: Date.now() - 60_000, change_note: null },
      ]],
    ]),
  };
}

function idx_to_hex(idx: number): string {
  return idx.toString(16).padStart(2, '0');
}

const STORE = initStore();

function clone<T>(v: T): T {
  return JSON.parse(JSON.stringify(v)) as T;
}

async function handle(cmd: string, args?: Record<string, unknown>): Promise<unknown> {
  switch (cmd) {
    case 'server_status':
      return clone(STORE.server);
    case 'start_server': {
      const bind = (args?.bind as string | null) || '127.0.0.1:38080';
      STORE.server = {
        handle: 'embedded',
        bind,
        pid: 99999, // mock pid
        uptime_secs: 0,
        running: true,
      };
      return clone(STORE.server);
    }
    case 'stop_server': {
      const prev = clone(STORE.server);
      STORE.server = {
        handle: 'embedded',
        bind: '',
        pid: STORE.server.pid,
        uptime_secs: null,
        running: false,
      };
      return prev;
    }
    case 'server_logs': return ['[mock] 2026-09-19T14:00:00Z INFO mock_server listening'];
    case 'clear_logs': return null;
    case 'list_repos':
      return clone(STORE.repos);
    case 'repo_detail': {
      const name = String(args?.name);
      return clone(STORE.details.get(name) ?? null);
    }
    case 'clone_url': {
      const name = String(args?.name);
      const bind = (args?.serverBind as string | null) || '127.0.0.1:38080';
      return `http://${bind}/repos/${name}.git`;
    }
    case 'open_repo_in_shell': return null;
    case 'vault_list': return Array.from(STORE.versions.keys());
    case 'vault_get': {
      const key = String(args?.key);
      // Return a stub value for any key with at least one version.
      const versions = STORE.versions.get(key);
      return versions && versions.length ? `value-for-${key}` : null;
    }
    case 'vault_set': {
      const key = String(args?.key);
      const value = String(args?.value);
      const existing = STORE.versions.get(key) ?? [];
      const newVersion: VersionEntryDto = {
        version: existing.length + 1,
        bytes_sha256: (existing.length + 1).toString(16).padStart(64, 'c'),
        byte_len: Buffer.byteLength(value, 'utf-8'),
        created_at_unix_ms: Date.now(),
        change_note: null,
      };
      STORE.versions.set(key, [...existing, newVersion]);
      return newVersion.version;
    }
    case 'vault_rotate': {
      const key = String(args?.key);
      const existing = STORE.versions.get(key) ?? [];
      const newVersion: VersionEntryDto = {
        version: existing.length + 1,
        bytes_sha256: (existing.length + 1).toString(16).padStart(64, 'd'),
        byte_len: 24,
        created_at_unix_ms: Date.now(),
        change_note: null,
      };
      STORE.versions.set(key, [...existing, newVersion]);
      return newVersion.version;
    }
    case 'vault_versions': {
      const key = String(args?.key);
      return clone(STORE.versions.get(key) ?? []);
    }
    case 'vault_diff': {
      const key = String(args?.key);
      const base = Number(args?.base);
      const head = Number(args?.head);
      const vs = STORE.versions.get(key) ?? [];
      const b = vs.find((v) => v.version === base);
      const h = vs.find((v) => v.version === head);
      const out: VersionDiffDto = {
        key,
        base_version: base,
        head_version: head,
        object_changed: Boolean(b && h && b.bytes_sha256 !== h.bytes_sha256),
        file_size_delta: h && b ? Number(h.byte_len) - Number(b.byte_len) : 0,
      };
      return out;
    }
    case 'vault_restore': {
      const key = String(args?.key);
      const target = Number(args?.targetVersion);
      const existing = STORE.versions.get(key) ?? [];
      const newVersion: VersionEntryDto = {
        version: existing.length + 1,
        bytes_sha256: (existing.length + 1).toString(16).padStart(64, 'e'),
        byte_len: existing.find((v) => v.version === target)?.byte_len ?? 0,
        created_at_unix_ms: Date.now(),
        change_note: `restored-to-v${target}`,
      };
      STORE.versions.set(key, [...existing, newVersion]);
      return newVersion.version;
    }
    case 'vault_delete': {
      const key = String(args?.key);
      STORE.versions.delete(key);
      return null;
    }
    case 'get_admin_password_status': {
      const hasKey = STORE.versions.has('gitgit.password');
      return {
        is_set: hasKey,
        length: hasKey ? 16 : 0,
      };
    }
    case 'set_admin_password': {
      const password = String(args?.password);
      const existing = STORE.versions.get('gitgit.password') ?? [];
      const newVersion: VersionEntryDto = {
        version: existing.length + 1,
        bytes_sha256: (existing.length + 1).toString(16).padStart(64, 'f'),
        byte_len: Buffer.byteLength(password, 'utf-8'),
        created_at_unix_ms: Date.now(),
        change_note: null,
      };
      STORE.versions.set('gitgit.password', [...existing, newVersion]);
      return newVersion.version;
    }
    case 'clear_admin_password':
      STORE.versions.delete('gitgit.password');
      return null;
    case 'app_info': {
      const info: AppInfo = {
        name: 'gm-desktop',
        version: '0.1.0',
        data_dir: '/var/data/com.gitgit.desktop',
        config_dir: '/var/data/com.gitgit.desktop',
        repos_dir: '/var/data/com.gitgit.desktop/repos',
        vault_dir: '/var/data/com.gitgit.desktop/vault',
        current_locale: 'zh-CN',
      };
      return info;
    }
    case 'vault_diagnostics':
      return {
        reachable: true,
        key_count: STORE.versions.size,
        backend: 'FileVault',
        root: '/var/data/com.gitgit.desktop/vault',
      };
    case 'set_clipboard_text': return null;
    default:
      throw new Error(`mock: command not implemented: ${cmd}`);
  }
}

/**
 * `installMock()` patches `window.__TAURI_INTERNALS__` so the rest
 * of the app sees a working backend without any Rust binary in the
 * loop. Safe to call multiple times; idempotent.
 */
export function installMock(): void {
  if (typeof window === 'undefined') return;
  if (window.__TAURI_INTERNALS__?.invoke?.toString().includes('mock')) return;
  window.__TAURI_INTERNALS__ = {
    invoke: ((cmd: string, args?: Record<string, unknown>) => handle(cmd, args)) as unknown as Window['__TAURI_INTERNALS__'] extends infer T
      ? T extends { invoke: infer F } ? F : never
      : never,
  };
}
