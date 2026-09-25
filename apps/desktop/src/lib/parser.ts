/**
 * Parses GitGit's docs/requirements/ Markdown into engineering-graph Nodes +
 * Edges. Runs identically on desktop (Tauri FS) and web (bundled fixtures) —
 * the desktop version reads the actual repo's requirements files.
 *
 * Extraction rules (defensive; Phase 6/7/9/13 docs differ in style):
 *  - Requirement IDs: `[A-Z]{2,5}-REQ-[0-9]{3}` or domain-tagged `REQ-XXX-NNN`
 *  - ADR IDs: `ADR-NNNN`
 *  - Section anchors: `## N. <Title>` → titles
 *  - Cross-document references inside `[]()` → candidate edges
 *  - Explicit "`X` implements `Y`" / "`X` depends on `Y`" → edges
 *  - Risk register & open-question IDs → issue nodes
 */

import type { GraphEdge, GraphNode, NodeKind } from './types';

const REQ_REGEX   = /\b(?:[A-Z]{2,5}-)?REQ-(\d{3})(?:-(SEQ|MVP))?\b/g;
const ADR_REGEX   = /\bADR-(\d{4})\b/g;
const RGS_REGEX   = /\bRGS-IMPL-(\d{3})\b/g;
const SECTION_REG = /^##\s+(\d+)\.\s+(.+?)\s*$/;
const H1_REG      = /^#\s+(.+?)\s*$/;
const TBD_REG     = /\bTBD\b/g;
const PHASE_REG   = /Phase\s+(\d{1,2})/g;

export interface ParsedDoc {
  source: string;                          // file path
  title: string;
  raw: string;
  requirementIds: string[];
  adrIds: string[];
  rgsIds: string[];
  sections: { number: number; title: string; preview: string }[];
  references: string[];                    // referenced path tokens (./phaseX-*.md)
  tagCounts: { tbd: number; phases: Set<number> };
}

/**
 * Detect NodeKind from an ID.
 *  - ADR-NNNN           → adr
 *  - RGS-IMPL-NNN       → release / mvp-candidate marker → adr
 *  - XXX-REQ-NNN        → requirement (or 'issue' when XXX in {OPS, UX, CDX, CI, NFR, SEC})
 *  - REQ-NNN            → requirement
 */
export function kindOf(id: string): NodeKind {
  if (/^ADR-\d{4}$/.test(id)) return 'adr';
  if (/^RGS-IMPL-\d{3}$/.test(id)) return 'adr';
  const m = id.match(/^([A-Z]{2,5})-REQ-(\d{3})/);
  if (m) {
    const [, prefix] = m;
    if (['OPS', 'UX', 'CDX', 'CI', 'NFR', 'SEC'].includes(prefix)) return 'policy';
    if (['AGT', 'AI'].includes(prefix)) return 'agent';
    if (['CTX'].includes(prefix)) return 'document';
    return 'requirement';
  }
  if (/^REQ-\d{3}$/.test(id)) return 'requirement';
  return 'document';
}

export function parseDoc(source: string, raw: string): ParsedDoc {
  const lines = raw.split(/\r?\n/);
  const h1 = lines.find(l => H1_REG.test(l));
  const title = h1 ? h1.replace(H1_REG, '$1') : source.split('/').pop() ?? source;

  const requirementIds = new Set<string>();
  const adrIds = new Set<string>();
  const rgsIds = new Set<string>();
  const phases = new Set<number>();
  let tbd = 0;

  for (const line of lines) {
    for (const m of line.matchAll(REQ_REGEX)) {
      // Reconstruct the canonical id (drop the SEQ/MVP suffix — they're release markers).
      const fullMatch = m[0];
      const canonical = fullMatch.replace(/-(SEQ|MVP)$/, '');
      requirementIds.add(canonical);
    }
    for (const m of line.matchAll(ADR_REGEX)) adrIds.add(`ADR-${m[1]}`);
    for (const m of line.matchAll(RGS_REGEX)) rgsIds.add(`RGS-IMPL-${m[1]}`);
    for (const m of line.matchAll(PHASE_REG)) phases.add(Number(m[1]));
    tbd += (line.match(TBD_REG) ?? []).length;
  }

  const sections: { number: number; title: string; preview: string }[] = [];
  for (let i = 0; i < lines.length; i++) {
    const m = lines[i].match(SECTION_REG);
    if (m) {
      const preview = lines.slice(i + 1, i + 6).join(' ').replace(/[#*_`>]/g, '').trim();
      sections.push({ number: Number(m[1]), title: m[2], preview: preview.slice(0, 240) });
    }
  }

  // Cross-doc references like `./phase10-architecture.md` or `(./phase13-final-baseline.md)`
  const references = Array.from(new Set(
    (raw.match(/\(\.{0,2}\/?(phase\d{1,2}[a-z0-9-]*\.md|00-[a-z0-9-]+\.md)\)/g) ?? [])
      .map(s => s.replace(/[()]/g, ''))
  ));

  return {
    source,
    title,
    raw,
    requirementIds: Array.from(requirementIds).sort(),
    adrIds: Array.from(adrIds).sort(),
    rgsIds: Array.from(rgsIds).sort(),
    sections,
    references,
    tagCounts: { tbd, phases },
  };
}

/**
 * Reduce many parsed docs to deduplicated graph Nodes + Edges.
 *  - Each requirement ID becomes a Node (kind from `kindOf(id)`).
 *  - Each ADR becomes a Node.
 *  - Each doc that *defines* a requirement emits an `references` edge to that node.
 *  - Each doc that *mentions* a requirement emits a `references` edge (lighter weight).
 *  - Each cross-doc reference `./foo.md` becomes a `references` edge from a doc-node
 *    representing the source file to a doc-node for the target file (or to a
 *    requirement mentioned in that file).
 */
export function docsToGraph(parsed: ParsedDoc[]): { nodes: GraphNode[]; edges: GraphEdge[] } {
  const now = new Date().toISOString();
  const nodes = new Map<string, GraphNode>();
  const edges: GraphEdge[] = [];
  let eid = 0;
  const edgeKey = (k: string, a: string, b: string) => `${k}|${a}|${b}`;
  const seenEdges = new Set<string>();

  const upsertNode = (n: GraphNode) => {
    const existing = nodes.get(n.id);
    if (existing) {
      existing.tags = Array.from(new Set([...existing.tags, ...n.tags]));
      existing.body = existing.body ?? n.body;
      existing.source = existing.source ?? n.source;
    } else {
      nodes.set(n.id, n);
    }
  };

  const addEdge = (kind: GraphEdge['kind'], from: string, to: string, weight = 1.0, note?: string) => {
    const k = edgeKey(kind, from, to);
    if (seenEdges.has(k) || from === to) return;
    seenEdges.add(k);
    edges.push({
      id: `e${++eid}`,
      kind,
      from,
      to,
      weight,
      note,
      created_at: now,
    });
  };

  // 1) Doc nodes + requirement / ADR nodes
  for (const d of parsed) {
    const docId = `DOC:${d.source}`;
    upsertNode({
      id: docId,
      kind: 'document',
      title: d.title,
      body: d.raw.slice(0, 240),
      tags: [`phases:${Array.from(d.tagCounts.phases).sort().join(',')}`],
      source: d.source,
      created_at: now,
      updated_at: now,
    });
    for (const r of d.requirementIds) {
      upsertNode({
        id: r,
        kind: kindOf(r),
        title: `Requirement ${r}`,
        tags: [],
        source: d.source,
        created_at: now,
        updated_at: now,
      });
      addEdge('references', docId, r, 1.0, 'defined in');
    }
    for (const a of d.adrIds) {
      upsertNode({
        id: a,
        kind: 'adr',
        title: `Architecture Decision ${a}`,
        tags: [],
        source: d.source,
        created_at: now,
        updated_at: now,
      });
      addEdge('references', docId, a, 1.0, 'defined in');
    }
  }

  // 2) Cross-doc references → edges
  for (const d of parsed) {
    const fromId = `DOC:${d.source}`;
    for (const ref of d.references) {
      const targetDoc = parsed.find(p => p.source.endsWith(ref.replace(/^\.?\//, '')));
      if (targetDoc) {
        addEdge('references', fromId, `DOC:${targetDoc.source}`, 0.7, 'cross-doc');
      }
    }
  }

  // 3) `X implements Y` / `X depends on Y` / `X supersedes Y` style edges inside raw text
  const STATEMENT = /\b([A-Z]{2,5}-REQ-\d{3}|REQ-\d{3}|ADR-\d{4})\b[^.\n]{0,40}\b(implements|depends on|depends_on|supersedes|gated by|gated_by|caused by|caused_by|blocks|references)\b[^.\n]{0,40}\b([A-Z]{2,5}-REQ-\d{3}|REQ-\d{3}|ADR-\d{4})\b/gi;
  for (const d of parsed) {
    for (const m of d.raw.matchAll(STATEMENT)) {
      const a = m[1];
      const rel = m[2].toLowerCase().replace(/\s+/g, '_');
      const b = m[3];
      if (nodes.has(a) && nodes.has(b)) {
        addEdge(rel as GraphEdge['kind'], a, b, 0.9, 'explicit statement');
      }
    }
  }

  return { nodes: Array.from(nodes.values()), edges };
}

/**
 * Score nodes by relevance to a query.
 * Matches id, title, body, tags, and the document source path.
 */
export function scoreNode(node: GraphNode, q: string): number {
  if (!q) return 0;
  const needle = q.toLowerCase();
  let s = 0;
  if (node.id.toLowerCase().includes(needle)) s += 4;
  if (node.title.toLowerCase().includes(needle)) s += 3;
  for (const t of node.tags) if (t.toLowerCase().includes(needle)) s += 2;
  if (node.body?.toLowerCase().includes(needle)) s += 1;
  if (node.source?.toLowerCase().includes(needle)) s += 1;
  return s;
}