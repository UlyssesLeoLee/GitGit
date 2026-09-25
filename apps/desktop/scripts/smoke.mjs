// Smoke test — runs the parser module against the live GitGit requirements docs
// and reports counts, sanity checks. No DOM, no Svelte.
import { build } from 'esbuild';
import fs from 'node:fs';
import path from 'node:path';
import url from 'node:url';

const __dirname = path.dirname(url.fileURLToPath(import.meta.url));
const projectRoot = path.resolve(__dirname, '..');

const result = await build({
  entryPoints: [path.resolve(projectRoot, 'src/lib/parser.ts')],
  bundle: true,
  format: 'esm',
  platform: 'node',
  write: false,
  target: 'node20',
});

const moduleUrl = 'data:text/javascript;base64,' + Buffer.from(result.outputFiles[0].text).toString('base64');
const { parseDoc, docsToGraph, scoreNode, kindOf } = await import(moduleUrl);

// Try real repo first; fall back to bundled copy in public/requirements.
let docsDir = path.resolve(projectRoot, '..', '..', 'docs', 'requirements');
if (!fs.existsSync(docsDir)) {
  docsDir = path.resolve(projectRoot, 'public', 'requirements');
}
const files = fs.readdirSync(docsDir).filter(f => f.endsWith('.md')).sort();
console.log('Reading', files.length, 'files from', docsDir);

const parsed = files.map(f => {
  const raw = fs.readFileSync(path.join(docsDir, f), 'utf8');
  const src = 'docs/requirements/' + f;
  return parseDoc(src, raw);
});

const totalReqs = parsed.reduce((s, d) => s + d.requirementIds.length, 0);
const totalAdrs = parsed.reduce((s, d) => s + d.adrIds.length, 0);
const totalRgs  = parsed.reduce((s, d) => s + d.rgsIds.length, 0);
const totalTbd  = parsed.reduce((s, d) => s + d.tagCounts.tbd, 0);

console.log('--- Parse summary ---');
console.log('requirements extracted:', totalReqs);
console.log('adrs extracted:        ', totalAdrs);
console.log('rgs-impl extracted:    ', totalRgs);
console.log('TBD mentions:          ', totalTbd);

const g = docsToGraph(parsed);
console.log('--- Graph ---');
console.log('nodes:', g.nodes.length);
console.log('edges:', g.edges.length);
const byKind = {};
for (const n of g.nodes) byKind[n.kind] = (byKind[n.kind] ?? 0) + 1;
console.log('by kind:', byKind);

const byEdge = {};
for (const e of g.edges) byEdge[e.kind] = (byEdge[e.kind] ?? 0) + 1;
console.log('by edge kind:', byEdge);

// Use any well-known node from the parsed docs
const sampleId = parsed[0].requirementIds[0];
const sample = g.nodes.find(n => n.id === sampleId);
console.log('--- first extracted requirement ---');
console.log(sample ? { id: sample.id, kind: sample.kind, title: sample.title, source: sample.source } : 'NOT FOUND');

console.log('search score for "audit" on first req:', sample ? scoreNode(sample, 'audit') : 'n/a');
console.log('search score for "first id text" on first req:', sample ? scoreNode(sample, sample.id.split('-').pop()) : 'n/a');

console.log('kindOf tests:', {
  'REQ-GRF-001': kindOf('REQ-GRF-001'),
  'OPS-REQ-001': kindOf('OPS-REQ-001'),
  'AGT-REQ-008': kindOf('AGT-REQ-008'),
  'SEC-REQ-001': kindOf('SEC-REQ-001'),
  'REQ-001': kindOf('REQ-001'),
  'ADR-0001': kindOf('ADR-0001'),
  'RGS-IMPL-001': kindOf('RGS-IMPL-001'),
});

const edgesOfSample = g.edges.filter(e => e.from === sampleId || e.to === sampleId);
console.log(sampleId, 'edges:', edgesOfSample.length, edgesOfSample.slice(0, 5).map(e => `${e.from}-[${e.kind}]->${e.to}`));

const docRefs = g.edges.filter(e => e.kind === 'references' && e.from.startsWith('DOC:'));
console.log('doc-to-doc refs:', docRefs.length);

// Spot-check a few more nodes (REQs have REQ-NNN, ADRs would be ADR-NNNN if any existed in this repo)
const samples = [parsed[0].requirementIds[0], parsed[1]?.requirementIds[0], `DOC:docs/requirements/${files[0]}`].filter(Boolean);
console.log('--- Spot checks ---');
for (const id of samples) {
  const n = g.nodes.find(x => x.id === id);
  console.log(id, n ? `kind=${n.kind} title="${n.title.slice(0, 60)}" source=${n.source}` : 'NOT FOUND');
}