/**
 * Tauri runtime detection + dynamic invoke import.
 *
 * Why: per the desktop/web dual-target constraint, a static `import { invoke }
 * from '@tauri-apps/api/core'` would break the Rollup web build (failed to
 * resolve). Instead we test for `window.__TAURI_INTERNALS__` (the marker Tauri
 * 2.x sets when running in its WebView) and dynamic-import the real module
 * inside the desktop branch — Rollup never sees it on the web branch.
 *
 * Usage:
 *   const result = await webInvoke<MyType>('hello', { name: 'world' });
 *   // desktop → goes through real Tauri invoke()
 *   // web     → returns a deterministic mock so the UI still renders
 */

declare global {
  interface Window {
    __TAURI_INTERNALS__?: unknown;
    __TAURI_OS_PLUGIN_INTERNALS__?: unknown;
  }
}

export type TauriOs = 'macos' | 'windows' | 'linux' | 'android' | 'ios' | 'freebsd' | 'dragonfly' | 'netbsd' | 'openbsd' | 'solaris' | 'unknown';

export const isTauri = (): boolean =>
  typeof window !== 'undefined' && typeof window.__TAURI_INTERNALS__ !== 'undefined';

let cachedOs: TauriOs | null = null;

/** Returns the host OS inside a Tauri runtime; 'web' when running in a plain browser. */
export async function platform(): Promise<TauriOs | 'web'> {
  if (!isTauri()) return 'web';
  if (cachedOs) return cachedOs;
  try {
    // `@tauri-apps/api/core` has no OS accessor in Tauri 2 — ask our own
    // `app_platform` command instead of pulling in `@tauri-apps/plugin-os`.
    const os = await webInvoke<string>('app_platform');
    cachedOs = (os as TauriOs) ?? 'unknown';
    return cachedOs;
  } catch {
    return 'unknown';
  }
}

/**
 * Mock implementations used by the web build (so the UI is still navigable).
 * Keys must match the real Tauri command names exactly — Tauri dispatches
 * `invoke(cmd)` by the literal `#[tauri::command]` function name (snake_case,
 * no colons), so these have to mirror `src-tauri/src/commands.rs` 1:1 or the
 * desktop build silently fails to find the command at runtime.
 */
const webMocks: Record<string, unknown> = {
  'graph_load': () => sampleGraph(),
  'graph_list_nodes': () => sampleGraph().nodes,
  'graph_list_edges': () => sampleGraph().edges,
  'graph_get_node': ({ id }: { id: string }) => sampleGraph().nodes.find(n => n.id === id) ?? null,
  'graph_add_node': (args: unknown) => ({ ok: true, echo: args }),
  'graph_update_node': (args: unknown) => ({ ok: true, echo: args }),
  'graph_delete_node': (args: unknown) => ({ ok: true, echo: args }),
  'graph_stats': () => {
    const g = sampleGraph();
    const byType: Record<string, number> = {};
    g.nodes.forEach(n => { byType[n.kind] = (byType[n.kind] ?? 0) + 1; });
    return { nodes: g.nodes.length, edges: g.edges.length, by_type: byType };
  },
  'docs_list': () => sampleDocs(),
  'docs_read': ({ path }: { path: string }) => {
    const d = sampleDocs().find(x => x.path === path);
    return d ? { path: d.path, title: d.title, content: d.preview } : null;
  },
  'app_platform': () => 'web',
  'app_version': () => ({ name: 'gitgit-desktop', version: '0.1.0-web' }),
};

/**
 * Invoke a Tauri command, with a web-build fallback.
 *
 * On desktop (Tauri runtime present) → real `invoke(cmd, args)`.
 * On web → looks up `webMocks[cmd]`, calls it, and returns the result. If no
 * mock exists, throws — surfacing the gap instead of pretending success.
 */
export async function webInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (isTauri()) {
    const mod = await import('@tauri-apps/api/core');
    return mod.invoke<T>(cmd, args);
  }
  const fn = webMocks[cmd];
  if (!fn) {
    throw new Error(`[gitgit/web] no mock for command "${cmd}" — this feature requires the Tauri desktop runtime`);
  }
  // Mimic the IPC round-trip with a microtask boundary.
  await Promise.resolve();
  return (typeof fn === 'function' ? (fn as (a: unknown) => T)(args ?? {}) : (fn as T));
}

// --- Sample data (web-build only) ---

interface SampleNode {
  id: string;
  kind: string;
  title: string;
  body?: string;
  tags: string[];
}
interface SampleEdge {
  id: string;
  kind: string;
  from: string;
  to: string;
  weight?: number;
}
interface SampleDoc { path: string; title: string; preview: string; }

function sampleGraph(): { nodes: SampleNode[]; edges: SampleEdge[] } {
  const nodes: SampleNode[] = [
    { id: 'REQ-GIT-001', kind: 'requirement', title: 'Local clone preserves full history', tags: ['git', 'P0', 'MVP'] },
    { id: 'REQ-GRF-001', kind: 'requirement', title: 'Queryable cross-object engineering graph', tags: ['graph', 'P0', 'MVP'] },
    { id: 'REQ-AGT-001', kind: 'requirement', title: 'First-class Agent node with policy gates', tags: ['agent', 'P0', 'MVP'] },
    { id: 'REQ-AGT-008', kind: 'requirement', title: 'MCP server exposed to external agents', tags: ['agent', 'mcp'] },
    { id: 'REQ-AI-001',  kind: 'requirement', title: 'AI Gateway routes by sensitivity', tags: ['ai'] },
    { id: 'REQ-CTX-001', kind: 'requirement', title: 'Context engine assembles per task', tags: ['context'] },
    { id: 'REQ-SEC-001', kind: 'requirement', title: 'Audit trail is structural to the graph', tags: ['security', 'P0'] },
    { id: 'REQ-OPS-001', kind: 'requirement', title: 'Single self-hostable binary for MVP', tags: ['ops', 'P0', 'MVP'] },
    { id: 'ADR-0001',    kind: 'adr', title: 'Graph substrate: Node/Edge/Event/Policy/View', tags: ['phase6'] },
    { id: 'PR-001',      kind: 'pr', title: 'feat: ingest requirement IDs into graph', tags: [] },
    { id: 'POL-001',     kind: 'policy', title: 'Sensitive-data routing policy', tags: [] },
    { id: 'AGENT-claude', kind: 'agent', title: 'Claude Code agent', tags: ['executor'] },
  ];
  const edges: SampleEdge[] = [
    { id: 'e1', kind: 'implements', from: 'PR-001', to: 'REQ-GRF-001' },
    { id: 'e2', kind: 'derived_from', from: 'REQ-GRF-001', to: 'ADR-0001' },
    { id: 'e3', kind: 'supersedes', from: 'ADR-0001', to: 'REQ-AGT-001' },
    { id: 'e4', kind: 'gated_by', from: 'AGENT-claude', to: 'POL-001' },
    { id: 'e5', kind: 'depends_on', from: 'REQ-AI-001', to: 'REQ-GRF-001' },
    { id: 'e6', kind: 'depends_on', from: 'REQ-CTX-001', to: 'REQ-GRF-001' },
    { id: 'e7', kind: 'depends_on', from: 'REQ-OPS-001', to: 'REQ-GIT-001' },
  ];
  return { nodes, edges };
}

function sampleDocs(): SampleDoc[] {
  return [
    { path: 'docs/requirements/00-requirements-definition.md', title: '00 — Requirements Definition (Baseline v1.0)', preview: 'Master requirements book. 55 sections. Substrate = Node/Edge/Event/Policy/View + Agent subtype.' },
    { path: 'docs/requirements/phase6-primitives.md', title: 'Phase 6 — Product Primitives', preview: 'MVP substrate needs only five primitives: Node, Edge, Event, Policy, View.' },
    { path: 'docs/requirements/phase9-mvp-reduction.md', title: 'Phase 9 — MVP Reduction', preview: 'Minimum complete closed loop: Repository→Issue→AI Context→Agent Branch→Code Change→CI→AI Review→Human Approval→Merge→Graph Update.' },
    { path: 'docs/requirements/phase10-architecture.md', title: 'Phase 10 — Architecture', preview: 'Principal Architect review; component weight, graph storage decisions, Local→Cloud path.' },
    { path: 'docs/requirements/phase11-red-team.md', title: 'Phase 11 — Red Team Review', preview: '17 findings from a 17-angle self-attack.' },
    { path: 'docs/requirements/phase12-ux-review.md', title: 'Phase 12 — UX Red Team Review', preview: '9 UX findings; ambient AI budget 80/15/5.' },
    { path: 'docs/requirements/phase13-final-baseline.md', title: 'Phase 13 — Final Baseline v1.0', preview: 'Disposition log; Three Moats; governing-question answer.' },
    { path: 'docs/requirements/phase14-ipa-compliance-review.md', title: 'Phase 14 — IPA Compliance Review', preview: 'Japan IPA gap analysis; NFR-REQ-001..003, SEC-REQ-008..010 added.' },
    { path: 'docs/requirements/phase15-final-audit.md', title: 'Phase 15 — Final Audit', preview: 'ID integrity / cross-ref / link integrity / count agreement — 3 found and fixed.' },
  ];
}