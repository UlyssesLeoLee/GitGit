/**
 * Unit tests for the /graph route's frontend parser port.
 *
 * This test mirrors the legacy `apps/desktop/scripts/smoke.mjs`
 * smoke script (PR-D cleanup target) but runs in vitest so we can
 * gate the parser behavior in CI. The Rust side has its own
 * `cargo test --lib graph::tests` suite in PR-B; this test is the
 * frontend fallback / TypeScript parity check.
 *
 * We use a couple of inline fixtures rather than walking the real
 * `docs/requirements/` directory — that way the test is hermetic
 * and doesn't depend on the host machine's repo layout.
 */

import { describe, expect, it } from 'vitest';
import { parseDoc, docsToGraph, scoreNode, kindOf } from '../../src/lib/graph/parser';

describe('graph / parser / kindOf', () => {
  it('detects ADR ids as adr kind', () => {
    expect(kindOf('ADR-0001')).toBe('adr');
    expect(kindOf('ADR-1234')).toBe('adr');
  });

  it('detects RGS-IMPL as adr (release marker)', () => {
    expect(kindOf('RGS-IMPL-001')).toBe('adr');
    expect(kindOf('RGS-IMPL-042')).toBe('adr');
  });

  it('maps policy-prefixed REQs to policy kind', () => {
    expect(kindOf('OPS-REQ-001')).toBe('policy');
    expect(kindOf('NFR-REQ-003')).toBe('policy');
    expect(kindOf('SEC-REQ-008')).toBe('policy');
  });

  it('maps agent-prefixed REQs to agent kind', () => {
    expect(kindOf('AGT-REQ-001')).toBe('agent');
    expect(kindOf('AI-REQ-005')).toBe('agent');
  });

  it('falls back to requirement for bare REQ-NNN', () => {
    expect(kindOf('REQ-007')).toBe('requirement');
  });

  it('falls back to document for unknown shapes', () => {
    expect(kindOf('POL-001')).toBe('document');
  });
});

describe('graph / parser / parseDoc', () => {
  const sample = [
    '# Sample Doc',
    '',
    '## 1. Overview',
    '',
    'This is the overview. REQ-007 must be implemented before REQ-008.',
    'See also ADR-0001 and ADR-0002 for the architecture decisions.',
    '',
    '## 2. Implementation',
    '',
    'AGT-REQ-001 enforces policy gating on PR-001. TBD.',
  ].join('\n');

  it('extracts the H1 title', () => {
    const parsed = parseDoc('docs/requirements/sample.md', sample);
    expect(parsed.title).toBe('Sample Doc');
  });

  it('extracts requirement IDs and dedupes across lines', () => {
    const parsed = parseDoc('docs/requirements/sample.md', sample);
    expect(parsed.requirementIds).toContain('REQ-007');
    expect(parsed.requirementIds).toContain('REQ-008');
    // dedupe: REQ-007 referenced twice should appear once.
    expect(parsed.requirementIds.filter((r) => r === 'REQ-007')).toHaveLength(1);
  });

  it('extracts ADR IDs', () => {
    const parsed = parseDoc('docs/requirements/sample.md', sample);
    expect(parsed.adrIds).toEqual(['ADR-0001', 'ADR-0002']);
  });

  it('captures section previews', () => {
    const parsed = parseDoc('docs/requirements/sample.md', sample);
    expect(parsed.sections).toHaveLength(2);
    expect(parsed.sections[0]?.title).toBe('Overview');
    expect(parsed.sections[1]?.title).toBe('Implementation');
  });

  it('counts TBD mentions', () => {
    const parsed = parseDoc('docs/requirements/sample.md', sample);
    expect(parsed.tagCounts.tbd).toBe(1);
  });
});

describe('graph / parser / docsToGraph', () => {
  const docA = [
    '# A',
    '## 1. Body',
    'REQ-007 implements ADR-0001. REQ-008 depends on REQ-007.',
    '',
  ].join('\n');

  const docB = [
    '# B',
    '## 1. Body',
    'See (./phase6-primitives.md) for the substrate rationale.',
    'AGT-REQ-001 depends on REQ-007.',
    '',
  ].join('\n');

  it('produces one node per requirement + one per ADR + one per doc', () => {
    const parsed = [
      parseDoc('docs/requirements/a.md', docA),
      parseDoc('docs/requirements/b.md', docB),
    ];
    const g = docsToGraph(parsed);
    // 3 docs (A, B + phase6 referenced) → 2 known (A,B); + REQ-007, REQ-008, AGT-REQ-001, ADR-0001
    const ids = new Set(g.nodes.map((n) => n.id));
    expect(ids.has('REQ-007')).toBe(true);
    expect(ids.has('REQ-008')).toBe(true);
    expect(ids.has('ADR-0001')).toBe(true);
    expect(ids.has('AGT-REQ-001')).toBe(true);
    expect(ids.has('DOC:docs/requirements/a.md')).toBe(true);
    expect(ids.has('DOC:docs/requirements/b.md')).toBe(true);
  });

  it('emits an implements edge for "REQ-007 implements ADR-0001"', () => {
    const parsed = [parseDoc('docs/requirements/a.md', docA)];
    const g = docsToGraph(parsed);
    expect(g.edges.some((e) => e.from === 'REQ-007' && e.to === 'ADR-0001' && e.kind === 'implements')).toBe(true);
  });

  it('emits a depends_on edge for explicit statement', () => {
    const parsed = [parseDoc('docs/requirements/a.md', docA)];
    const g = docsToGraph(parsed);
    expect(g.edges.some((e) => e.from === 'REQ-008' && e.to === 'REQ-007' && e.kind === 'depends_on')).toBe(true);
  });

  it('deduplicates edges so the same (kind, from, to) appears once', () => {
    const parsed = [
      parseDoc('docs/requirements/a.md', docA),
      parseDoc('docs/requirements/a.md', docA), // duplicate
    ];
    const g = docsToGraph(parsed);
    const implEdges = g.edges.filter((e) => e.kind === 'implements' && e.from === 'REQ-007' && e.to === 'ADR-0001');
    expect(implEdges).toHaveLength(1);
  });

  it('emits a DOC→DOC references edge for cross-doc link', () => {
    // docB references `./phase6-primitives.md`; the target doc lives at
    // `docs/requirements/phase6-primitives.md`, so the parser should
    // match and emit a cross-doc edge from docB to that file.
    const parsed = [
      parseDoc('docs/requirements/b.md', docB),
      parseDoc('docs/requirements/phase6-primitives.md', '# phase6 substrate\n'),
    ];
    const g = docsToGraph(parsed);
    expect(
      g.edges.some(
        (e) =>
          e.from === 'DOC:docs/requirements/b.md' &&
          e.to === 'DOC:docs/requirements/phase6-primitives.md' &&
          e.kind === 'references',
      ),
    ).toBe(true);
  });
});

describe('graph / parser / scoreNode', () => {
  const node = {
    id: 'REQ-GRF-001',
    kind: 'requirement' as const,
    title: 'Queryable cross-object engineering graph',
    tags: ['graph', 'P0'],
    body: 'The graph substrate is the canonical project state.',
    source: 'docs/requirements/00-requirements-definition.md',
    created_at: '2026-09-19T00:00:00.000Z',
    updated_at: '2026-09-19T00:00:00.000Z',
  };

  it('returns 0 for empty query', () => {
    expect(scoreNode(node, '')).toBe(0);
  });

  it('matches id with highest weight', () => {
    expect(scoreNode(node, 'GRF')).toBeGreaterThanOrEqual(4);
  });

  it('matches title with second-highest weight', () => {
    expect(scoreNode(node, 'engineering')).toBeGreaterThanOrEqual(3);
  });

  it('case-insensitive', () => {
    expect(scoreNode(node, 'grf')).toBe(scoreNode(node, 'GRF'));
  });
});