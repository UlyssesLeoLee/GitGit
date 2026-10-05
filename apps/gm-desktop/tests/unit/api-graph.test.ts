/**
 * Tests for the knowledge-graph command wrappers in
 * `src/lib/api/graph.ts`.
 *
 * This module was 0% covered. All seven wrappers are one-line casts
 * over `invoke`, so the behaviour worth pinning is the *command name
 * and argument key* for each one, plus the boundary the module
 * docstring claims is safe: the payload is cast to a typed shape
 * without validation, so a Rust-side rename would surface here as a
 * wrong `id` key rather than a type error.
 *
 * `docs_read` and `graph_get_node` answer `null` for a miss. That is
 * a real distinction from a failure — the graph page can show "no such
 * doc" without an error path — so it gets its own case.
 */
import { beforeEach, describe, expect, it } from 'vitest';
import {
  docsList,
  docsRead,
  graphGetNode,
  graphListEdges,
  graphListNodes,
  graphLoad,
  graphStats,
} from '../../src/lib/api/graph';
import { installMock } from '../../src/mocks/handlers';

interface Call { cmd: string; args: Record<string, unknown> | undefined }

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

beforeEach(() => {
  // `installMock()` skips reinstalling when the currently installed
  // `invoke` source happens to contain the word "mock". Clearing the
  // global first keeps these cases order-independent.
  delete window.__TAURI_INTERNALS__;
  installMock();
});

describe('api / graph — command names and arguments', () => {
  it('sends the seven documented command names', async () => {
    const calls = recordCalls();
    await graphLoad();
    await graphListNodes();
    await graphListEdges();
    await graphGetNode('REQ-GRF-001');
    await graphStats();
    await docsList();
    await docsRead('docs/requirements/00-requirements-definition.md');
    expect(calls.map((c) => c.cmd)).toEqual([
      'graph_load',
      'graph_list_nodes',
      'graph_list_edges',
      'graph_get_node',
      'graph_stats',
      'docs_list',
      'docs_read',
    ]);
  });

  it('sends a node id under the key the Rust parameter expects', async () => {
    // `id`, not `nodeId` or `node_id`. The wrapper is the only place
    // this translation exists.
    const calls = recordCalls();
    await graphGetNode('ADR-0001');
    expect(calls[0]?.args).toEqual({ id: 'ADR-0001' });
  });

  it('sends a doc path under the key the Rust parameter expects', async () => {
    const calls = recordCalls();
    await docsRead('docs/requirements/phase6-primitives.md');
    expect(calls[0]?.args).toEqual({ path: 'docs/requirements/phase6-primitives.md' });
  });

  it('sends no arguments for the four argument-less commands', async () => {
    const calls = recordCalls();
    await graphLoad();
    await graphListNodes();
    await graphListEdges();
    await graphStats();
    await docsList();
    for (const call of calls) {
      expect(call.args ?? {}).toEqual({});
    }
  });
});

describe('api / graph — responses', () => {
  it('loads the graph as nodes, edges and docs together', async () => {
    const result = await graphLoad();
    expect(result.nodes.length).toBeGreaterThan(0);
    expect(result.edges.length).toBeGreaterThan(0);
    expect(result.docs.length).toBeGreaterThan(0);
  });

  it('returns a single node and null for an unknown id', async () => {
    const node = await graphGetNode('REQ-GRF-001');
    expect(node?.id).toBe('REQ-GRF-001');
    // A miss is `null`, not a rejection: the page treats "no such node"
    // as an empty selection, not as a broken backend.
    expect(await graphGetNode('NO-SUCH-NODE')).toBeNull();
  });

  it('reads a doc and nulls an unknown path', async () => {
    const doc = await docsRead('docs/requirements/phase6-primitives.md');
    expect(doc?.title).toContain('Phase 6');
    expect(doc?.content.length).toBeGreaterThan(0);
    expect(await docsRead('docs/requirements/nope.md')).toBeNull();
  });

  it('lists nodes and edges as arrays', async () => {
    expect(Array.isArray(await graphListNodes())).toBe(true);
    expect(Array.isArray(await graphListEdges())).toBe(true);
    expect((await graphListNodes()).length).toBe(await graphLoad().then((r) => r.nodes.length));
  });

  it('reads the stats the header shows', async () => {
    const s = await graphStats();
    expect(s.nodes).toBeGreaterThan(0);
    expect(s.by_type.requirement).toBeGreaterThan(0);
  });

  it('propagates a backend refusal with its kind intact', async () => {
    window.__TAURI_INTERNALS__ = {
      invoke: () => Promise.reject({ kind: 'GraphNotBuilt', message: 'no index yet', source: '"GraphNotBuilt"' }),
    };
    // The store reads `kind` to choose a message, so the wrapper must
    // not catch. The graph store is what converts this into state.
    await expect(graphLoad()).rejects.toMatchObject({ kind: 'GraphNotBuilt' });
  });
});

describe('api / graph — the cast at the boundary', () => {
  it('returns nodes carrying every field the typed shape promises', async () => {
    // `[FACT]` `graph.ts` casts the raw payload to `GraphNode[]` with
    // no runtime check, and its docstring asserts that is safe because
    // the wire JSON conforms to `api/types.ts`. That is an unchecked
    // claim, so it is worth one test: if Rust ever renames a field,
    // this is what notices, instead of a template reading `undefined`.
    const { nodes } = await graphLoad();
    expect(nodes.length).toBeGreaterThan(0);
    for (const n of nodes) {
      expect(typeof n.id, 'id').toBe('string');
      expect(typeof n.kind, `${n.id}.kind`).toBe('string');
      expect(typeof n.title, `${n.id}.title`).toBe('string');
      expect(Array.isArray(n.tags), `${n.id}.tags`).toBe(true);
      expect(typeof n.created_at, `${n.id}.created_at`).toBe('string');
      expect(typeof n.updated_at, `${n.id}.updated_at`).toBe('string');
    }
  });

  it('returns edges whose endpoints are ids that exist in the node list', async () => {
    // A dangling edge would render as a blank row in the graph and is
    // exactly the sort of drift the unvalidated cast would hide.
    const { nodes, edges } = await graphLoad();
    const ids = new Set(nodes.map((n) => n.id));
    for (const e of edges) {
      expect(ids.has(e.from), `edge ${e.id} from ${e.from}`).toBe(true);
      expect(ids.has(e.to), `edge ${e.id} to ${e.to}`).toBe(true);
    }
  });
});
