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
  /** Session id the mock currently considers streaming, if any. */
  session: string | null;
  /** Monotonic id source for mock review sessions. */
  sessionSeq: number;
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
    session: null,
    sessionSeq: 0,
  };
}

function idx_to_hex(idx: number): string {
  return idx.toString(16).padStart(2, '0');
}

const STORE = initStore();

function clone<T>(v: T): T {
  return JSON.parse(JSON.stringify(v)) as T;
}

/* ---------- Knowledge-graph mock fixtures (PR-C) ----------
 *
 * Tiny in-memory substitute for the Rust `graph_load` /
 * `graph_list_*` / `graph_stats` / `docs_list` / `docs_read`
 * surface. The real Tauri build never hits these branches — only
 * `vite dev` without Tauri does.
 */

interface MockNode {
  id: string;
  kind: string;
  title: string;
  body?: string;
  tags: string[];
  source?: string;
  created_at: string;
  updated_at: string;
}
interface MockEdge {
  id: string;
  kind: string;
  from: string;
  to: string;
  weight?: number;
  note?: string;
  created_at: string;
}
interface MockDoc { path: string; title: string; preview: string; }

function graphFixtureNodes(): MockNode[] {
  const now = '2026-09-19T00:00:00.000Z';
  return [
    { id: 'REQ-GRF-001',  kind: 'requirement', title: 'Queryable cross-object engineering graph', tags: ['graph', 'P0', 'MVP'], source: 'docs/requirements/00-requirements-definition.md', body: 'Master requirements book. 55 sections.', created_at: now, updated_at: now },
    { id: 'REQ-AGT-001',  kind: 'requirement', title: 'First-class Agent node with policy gates', tags: ['agent', 'P0', 'MVP'], source: 'docs/requirements/00-requirements-definition.md', body: 'Agents participate in the graph as first-class nodes.', created_at: now, updated_at: now },
    { id: 'REQ-OPS-001',  kind: 'requirement', title: 'Single self-hostable binary for MVP',      tags: ['ops', 'P0', 'MVP'],   source: 'docs/requirements/00-requirements-definition.md', created_at: now, updated_at: now },
    { id: 'ADR-0001',     kind: 'adr',         title: 'Graph substrate: Node/Edge/Event/Policy/View', tags: ['phase6'], source: 'docs/requirements/phase6-primitives.md', created_at: now, updated_at: now },
    { id: 'PR-001',       kind: 'pr',          title: 'feat: ingest requirement IDs into graph',  tags: [], created_at: now, updated_at: now },
    { id: 'POL-001',      kind: 'policy',      title: 'Sensitive-data routing policy',             tags: [], created_at: now, updated_at: now },
    { id: 'AGENT-claude', kind: 'agent',       title: 'Claude Code agent',                         tags: ['executor'], created_at: now, updated_at: now },
  ];
}

function graphFixtureEdges(): MockEdge[] {
  const now = '2026-09-19T00:00:00.000Z';
  return [
    { id: 'e1', kind: 'implements',   from: 'PR-001',       to: 'REQ-GRF-001', weight: 1.0, created_at: now },
    { id: 'e2', kind: 'derived_from', from: 'REQ-GRF-001',  to: 'ADR-0001',    weight: 1.0, created_at: now },
    { id: 'e3', kind: 'supersedes',   from: 'ADR-0001',     to: 'REQ-AGT-001', weight: 1.0, created_at: now },
    { id: 'e4', kind: 'gated_by',     from: 'AGENT-claude', to: 'POL-001',     weight: 1.0, created_at: now },
    { id: 'e5', kind: 'depends_on',   from: 'REQ-OPS-001',  to: 'REQ-GRF-001', weight: 1.0, created_at: now },
  ];
}

function graphFixtureDocs(): MockDoc[] {
  return [
    { path: 'docs/requirements/00-requirements-definition.md', title: '00 — Requirements Definition (Baseline v1.0)', preview: 'Master requirements book. 55 sections.' },
    { path: 'docs/requirements/phase6-primitives.md',         title: 'Phase 6 — Product Primitives',                preview: 'MVP substrate: Node, Edge, Event, Policy, View.' },
  ];
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
    /* --- Working-tree status and diff (T4) ---
     *
     * The payload below is the real wire shape from
     * `commands/repos.rs`. The diff text is a real `git diff --cached`
     * body (a rename, because that is what a bare `git diff` would not
     * show) so the dev workflow exercises the same rendering as
     * production rather than a placeholder.
     */
    case 'repo_status': {
      const name = String(args?.name);
      if (name === 'hello-world') {
        // Stands in for a bare repository: real refs and commits, no
        // working tree. The page must show the remedy, not a crash.
        throw {
          kind: 'NotAWorkTree',
          message: `${name} is a bare repository: it has no working tree`,
          source: '"NotAWorkTree"',
        };
      }
      return {
        branch: 'main',
        head: 'a1b2c3d',
        upstream: 'origin/main',
        ahead: 1,
        behind: 0,
        is_clean: false,
        entries: [
          {
            path: 'staged.txt',
            orig_path: null,
            index_status: 'A',
            worktree_status: null,
            staged: true,
            unstaged: false,
            untracked: false,
          },
          {
            path: 'src/dirty.ts',
            orig_path: null,
            index_status: null,
            worktree_status: 'M',
            staged: false,
            unstaged: true,
            untracked: false,
          },
          {
            path: 'notes.txt',
            orig_path: null,
            index_status: '?',
            worktree_status: '?',
            staged: false,
            unstaged: false,
            untracked: true,
          },
        ],
      };
    }
    case 'repo_diff': {
      const target = String(args?.target);
      if (target !== 'staged' && target !== 'worktree' && target !== 'head') {
        throw {
          kind: 'InvalidDiffTarget',
          message: `unknown diff target: ${target} (expected staged, worktree or head)`,
          source: '"InvalidDiffTarget"',
        };
      }
      const path = args?.path == null ? null : String(args.path);
      if (path === 'notes.txt') {
        return {
          target,
          path,
          untracked: true,
          truncated: false,
          files: 1,
          text: [
            'diff --git a/notes.txt b/notes.txt',
            'new file mode 100644',
            'index 0000000..e69de29',
            '--- /dev/null',
            '+++ b/notes.txt',
            '@@ -0,0 +1,2 @@',
            '+first note',
            '+second note',
            '',
          ].join('\n'),
        };
      }
      const body =
        target === 'staged'
          ? [
              'diff --git a/staged.txt b/staged.txt',
              'new file mode 100644',
              'index 0000000..3b18e51',
              '--- /dev/null',
              '+++ b/staged.txt',
              '@@ -0,0 +1 @@',
              '+staged line',
            ]
          : [
              'diff --git a/src/dirty.ts b/src/dirty.ts',
              'index 1234567..89abcde 100644',
              '--- a/src/dirty.ts',
              '+++ b/src/dirty.ts',
              '@@ -1 +1,2 @@',
              ' const before = 1;',
              '+const after = 2;',
            ];
      return {
        target,
        path,
        untracked: false,
        truncated: false,
        files: 1,
        text: [...body, ''].join('\n'),
      };
    }
    case 'vault_list': return Array.from(STORE.versions.keys());
    case 'vault_get': {
      const key = String(args?.key);
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

    /* --- AI review (T9) ---
     *
     * Command shapes and refusals only. Token *delivery* is not mocked:
     * it rides the Tauri event bridge (`ai-review://*`), and
     * `@tauri-apps/api`'s `listen` needs the real runtime to register
     * its callback. Under `vite dev` without Tauri the subscription
     * therefore fails and the review page shows its error state, which
     * is the same outcome every other command has in this mode. The
     * refusal for `anthropic` is kept honest on purpose — a mock that
     * streamed where production cannot would teach the wrong lesson.
     */
    case 'ai_review_start': {
      const diff = String(args?.diff ?? '');
      const provider = String(args?.provider ?? 'openai');
      if (diff.trim() === '') {
        throw {
          kind: 'AiReviewInvalid',
          message: 'invalid review request: the diff is empty',
          source: '"AiReviewInvalid"',
        };
      }
      if (provider === 'anthropic') {
        throw {
          kind: 'AiReviewUnsupported',
          message: 'anthropic cannot stream: its protocol has no streaming implementation',
          source: '"AiReviewUnsupported"',
        };
      }
      const sessionId = `mock-session-${++STORE.sessionSeq}`;
      STORE.session = sessionId;
      return {
        session_id: sessionId,
        provider,
        model: String(args?.model ?? 'mock-model'),
        streaming: true,
        redactions: 0,
      };
    }
    case 'ai_review_cancel': {
      if (!STORE.session) {
        throw {
          kind: 'AiReviewNotRunning',
          message: 'no review is currently streaming',
          source: '"AiReviewNotRunning"',
        };
      }
      const sessionId = STORE.session;
      STORE.session = null;
      return sessionId;
    }

    /* --- Knowledge-graph commands (PR-C) --- */
    case 'graph_load':
      return {
        nodes: graphFixtureNodes(),
        edges: graphFixtureEdges(),
        docs: graphFixtureDocs(),
      };
    case 'graph_list_nodes':
      return graphFixtureNodes();
    case 'graph_list_edges':
      return graphFixtureEdges();
    case 'graph_get_node': {
      const id = String(args?.id);
      return graphFixtureNodes().find((n) => n.id === id) ?? null;
    }
    case 'graph_stats': {
      const nodes = graphFixtureNodes();
      const edges = graphFixtureEdges();
      const by_type: Record<string, number> = {};
      for (const n of nodes) by_type[n.kind] = (by_type[n.kind] ?? 0) + 1;
      return { nodes: nodes.length, edges: edges.length, by_type };
    }
    case 'docs_list':
      return graphFixtureDocs();
    case 'docs_read': {
      const path = String(args?.path);
      const d = graphFixtureDocs().find((x) => x.path === path);
      return d ? { path: d.path, title: d.title, content: d.preview } : null;
    }

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