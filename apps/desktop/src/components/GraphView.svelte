<script lang="ts">
  import { graph, stats } from '../lib/store';
  import type { GraphNode, NodeKind } from '../lib/types';

  // SVG canvas — graph is force-laid-out on a simple deterministic grid that
  // groups by kind, edges drawn between cells. For the MVP desktop app this is
  // enough to make structural relations visible without a heavyweight
  // physics-engine dependency.

  const W = 880;
  const H = 540;
  const PAD = 60;

  // Place each node in a deterministic grid cell keyed by its kind.
  function positions(nodes: GraphNode[]) {
    const map = new Map<string, { x: number; y: number; r: number }>();
    if (nodes.length === 0) return map;

    // Pick grid size from node count
    const cols = Math.max(3, Math.ceil(Math.sqrt(nodes.length * 1.6)));
    const rows = Math.ceil(nodes.length / cols);
    const cellW = (W - PAD * 2) / cols;
    const cellH = (H - PAD * 2) / rows;

    nodes.forEach((n, i) => {
      const c = i % cols;
      const r = Math.floor(i / cols);
      const jitter = ((n.id.charCodeAt(0) % 13) - 6);
      map.set(n.id, {
        x: PAD + c * cellW + cellW / 2 + jitter,
        y: PAD + r * cellH + cellH / 2 + jitter,
        r: 7 + Math.min(8, (n.title.length || 8) / 10),
      });
    });
    return map;
  }

  const layout = $derived(positions($graph.nodes));

  function color(kind: NodeKind): string {
    return `var(--node-${kind}, var(--node-event))`;
  }

  function edgeColor(kind: string): string {
    switch (kind) {
      case 'implements':  return 'var(--green)';
      case 'depends_on':  return 'var(--orange)';
      case 'supersedes':  return 'var(--purple)';
      case 'gated_by':    return 'var(--red)';
      case 'blocks':      return 'var(--red)';
      default:            return 'rgba(125, 133, 144, 0.45)';
    }
  }
</script>

<div class="graph-view">
  <svg viewBox="0 0 {W} {H}" preserveAspectRatio="xMidYMid meet">
    <defs>
      <marker id="arrow-default" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="5" markerHeight="5" orient="auto-start-reverse">
        <path d="M 0 0 L 10 5 L 0 10 z" fill="rgba(125, 133, 144, 0.7)" />
      </marker>
    </defs>

    {#each $graph.edges as e (e.id)}
      {@const a = layout.get(e.from)}
      {@const b = layout.get(e.to)}
      {#if a && b}
        {@const isHot = $graph.selected && ($graph.selected === e.from || $graph.selected === e.to)}
        <line
          x1={a.x}
          y1={a.y}
          x2={b.x}
          y2={b.y}
          stroke={edgeColor(e.kind)}
          stroke-width={isHot ? 1.6 : 0.6}
          opacity={$graph.selected ? (isHot ? 0.95 : 0.18) : 0.4}
          marker-end="url(#arrow-default)"
        />
      {/if}
    {/each}

    {#each $graph.nodes as n (n.id)}
      {@const p = layout.get(n.id)}
      {#if p}
        {@const isSelected = $graph.selected === n.id}
        <g class="node" class:selected={isSelected} transform="translate({p.x}, {p.y})" tabindex="0" role="button" aria-label={n.id}>
          <circle r={p.r + 2} fill="rgba(255,255,255,0.06)" />
          <circle r={p.r} fill={color(n.kind)} stroke={isSelected ? 'white' : 'rgba(0,0,0,0.4)'} stroke-width={isSelected ? 2 : 1} />
          <text x="0" y={p.r + 12} text-anchor="middle" font-size="9" font-family="ui-monospace, monospace" fill="var(--fg-1)">
            {n.id.length > 14 ? n.id.slice(0, 13) + '…' : n.id}
          </text>
          <title>{n.id} · {n.title} · {n.kind}</title>
        </g>
      {/if}
    {/each}
  </svg>

  <div class="legend">
    {#each Object.entries($stats.by_type) as [k, v]}
      <span class="leg"><span class="dot" style="background: {color(k as NodeKind)};"></span>{k} ({v})</span>
    {/each}
  </div>
</div>

<style>
  .graph-view {
    position: relative;
    background: var(--bg-0);
    border-top: 1px solid var(--border);
    height: 320px;
    overflow: hidden;
    flex-shrink: 0;
  }
  svg { width: 100%; height: 100%; display: block; }
  .node { cursor: pointer; transition: transform 120ms; }
  .node:hover { transform: scale(1.1); transform-origin: center; }
  .node:focus-visible { outline: none; }
  .node:focus-visible circle { stroke: var(--accent); stroke-width: 3; }
  .legend {
    position: absolute;
    bottom: 8px;
    left: 12px;
    display: flex;
    gap: 12px;
    font-size: 10px;
    color: var(--fg-2);
    background: rgba(13, 17, 23, 0.8);
    padding: 4px 10px;
    border-radius: 4px;
    flex-wrap: wrap;
    max-width: calc(100% - 24px);
  }
  .leg { display: inline-flex; gap: 5px; align-items: center; }
  .leg .dot { width: 8px; height: 8px; border-radius: 50%; display: inline-block; }
</style>