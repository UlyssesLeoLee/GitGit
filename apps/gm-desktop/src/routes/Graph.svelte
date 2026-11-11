<!--
  /graph route — knowledge-graph viewer (per ADR-0023 §3.2).

  3-pane layout: a master list (left), an SVG visualization (top right),
  and a detail / connections panel (bottom right). Everything is wired
  through `src/lib/stores/graph.ts`; this component owns no graph state
  of its own beyond the local search input.

  On mount we call `loadGraph()` once. The store hydrates from the Rust
  backend (or the mock layer under `vite dev`). The user can search by
  ID / title / tag / source via the top bar; clicking a node — in any
  of the three panes — selects it and the detail pane renders incoming
  and outgoing edges.
-->
<script lang="ts">
  import { onMount } from 'svelte';
  import {
    graph,
    filteredNodes,
    stats,
    loadGraph,
    selectNode,
    setQuery,
    selectFrom,
    neighbors,
  } from '$lib/stores/graph';
  import { catalog } from '$lib/i18n';
  import type { GraphEdge, GraphNode, NodeKind } from '$lib/api/types';

  let query = $state('');

  onMount(async () => {
    await loadGraph();
  });

  function onQueryInput(e: Event): void {
    query = (e.target as HTMLInputElement).value;
    setQuery(query);
  }

  function pick(id: string): void {
    selectNode(id);
  }

  function nodeColor(kind: NodeKind): string {
    // Tailwind-aligned palette: each kind gets a distinct hue; the
    // SVG <circle> uses the `fill` attribute directly.
    const map: Record<NodeKind, string> = {
      requirement: '#3b82f6', // blue-500
      issue:       '#f59e0b', // amber-500
      pr:          '#8b5cf6', // violet-500
      commit:      '#64748b', // slate-500
      adr:         '#ec4899', // pink-500
      agent:       '#10b981', // emerald-500
      policy:      '#ef4444', // red-500
      event:       '#06b6d4', // cyan-500
      human:       '#a855f7', // purple-500
      release:     '#22c55e', // green-500
      incident:    '#dc2626', // red-600
      document:    '#0ea5e9', // sky-500
    };
    return map[kind] ?? '#94a3b8';
  }

  function edgeColor(kind: GraphEdge['kind']): string {
    switch (kind) {
      case 'implements':  return '#22c55e';
      case 'depends_on':  return '#f59e0b';
      case 'supersedes':  return '#a855f7';
      case 'gated_by':    return '#ef4444';
      case 'blocks':      return '#dc2626';
      case 'reviewed_by': return '#0ea5e9';
      case 'caused_by':   return '#f97316';
      case 'derived_from':return '#8b5cf6';
      case 'created_by':  return '#64748b';
      case 'references':  return 'rgba(100,116,139,0.45)';
      default:            return 'rgba(100,116,139,0.45)';
    }
  }

  // `[FACT]` `selectFrom($graph)`, not `getSelected()`. `getSelected()`
  // reads the store via `get()`, which tracks nothing, so wrapping it in
  // `$derived` produced a derived with zero dependencies that Svelte
  // evaluated once at mount and never invalidated — the detail pane was
  // permanently stuck on its empty state. Passing `$graph` in is what
  // makes this derived track the store. See `selectFrom` in the store.
  const selected = $derived(selectFrom($graph));
  const nbh = $derived(neighbors($graph.selected, 'both'));
  const incoming = $derived(neighbors($graph.selected, 'in'));
  const outgoing = $derived(neighbors($graph.selected, 'out'));

  // Deterministic SVG layout: place nodes on a jittered grid so the
  // MVP visualization has no force-engine dependency. Same algorithm
  // as the legacy `apps/desktop` implementation, ported verbatim.
  const W = 880;
  const H = 540;
  const PAD = 60;

  function positions(nodes: GraphNode[]): Map<string, { x: number; y: number; r: number }> {
    const map = new Map<string, { x: number; y: number; r: number }>();
    if (nodes.length === 0) return map;
    const cols = Math.max(3, Math.ceil(Math.sqrt(nodes.length * 1.6)));
    const rows = Math.ceil(nodes.length / cols);
    const cellW = (W - PAD * 2) / cols;
    const cellH = (H - PAD * 2) / rows;
    nodes.forEach((n, i) => {
      const c = i % cols;
      const r = Math.floor(i / cols);
      const jitter = (n.id.charCodeAt(0) % 13) - 6;
      map.set(n.id, {
        x: PAD + c * cellW + cellW / 2 + jitter,
        y: PAD + r * cellH + cellH / 2 + jitter,
        r: 7 + Math.min(8, (n.title.length || 8) / 10),
      });
    });
    return map;
  }

  const layout = $derived(positions($graph.nodes));
</script>

<section class="flex h-full w-full flex-col" data-testid="graph-route">
  <header class="flex items-center gap-3 border-b border-slate-200 px-4 py-3 dark:border-slate-700">
    <div class="flex-1">
      <h1 class="text-lg font-semibold">{$catalog['graph.heading']}</h1>
      <p class="text-xs text-slate-500 dark:text-slate-400">
        {$catalog['graph.subhead']}
        <span class="ml-2 inline-flex items-center gap-1 rounded bg-slate-100 px-2 py-0.5 text-[10px] uppercase tracking-wider text-slate-600 dark:bg-slate-700 dark:text-slate-200">
          {$graph.source}
        </span>
      </p>
    </div>
    <div class="w-72">
      <input
        type="search"
        class="w-full rounded-md border border-slate-300 bg-white px-3 py-1.5 text-sm focus:border-accent-500 focus:outline-none dark:border-slate-600 dark:bg-slate-800"
        placeholder={$catalog['graph.searchPlaceholder']}
        value={query}
        oninput={onQueryInput}
        data-testid="graph-search"
      />
    </div>
    <div class="flex gap-3 text-xs">
      <span><b>{$stats.nodes}</b> {$catalog['graph.nodes']}</span>
      <span><b>{$stats.edges}</b> {$catalog['graph.edges']}</span>
    </div>
  </header>

  {#if $graph.loading}
    <div class="flex flex-1 items-center justify-center text-sm text-slate-500">
      {$catalog['common.loading']}
    </div>
  {:else if $graph.error}
    <div class="m-4 rounded border border-red-300 bg-red-50 p-3 text-sm text-red-700 dark:border-red-700 dark:bg-red-950 dark:text-red-200">
      {$catalog['graph.errorPrefix']}: {$graph.error}
      <button class="ml-3 underline" onclick={() => loadGraph()}>
        {$catalog['common.refresh']}
      </button>
    </div>
  {:else if $graph.nodes.length === 0}
    <div class="flex flex-1 items-center justify-center text-sm text-slate-500">
      {$catalog['graph.empty']}
    </div>
  {:else}
    <div class="flex flex-1 overflow-hidden">
      <!-- Left: master list -->
      <aside class="flex w-80 shrink-0 flex-col border-r border-slate-200 bg-white dark:border-slate-700 dark:bg-slate-800">
        <div class="border-b border-slate-200 px-4 py-2 text-xs uppercase tracking-wider text-slate-500 dark:border-slate-700">
          {$catalog['graph.nodesTitle']} <span class="text-slate-700 dark:text-slate-200">{$stats.nodes}</span>
          <p class="mt-1 normal-case tracking-normal text-[11px] text-slate-500">
            {$filteredNodes.length} {$catalog['graph.matching']} "{query || '*'}"
          </p>
        </div>
        <ul class="flex-1 overflow-y-auto" role="listbox">
          {#each $filteredNodes.slice(0, 500) as n (n.id)}
            <li
              role="option"
              aria-selected={$graph.selected === n.id}
              class:selected={$graph.selected === n.id}
              class="flex cursor-pointer items-center gap-2 border-l-2 border-transparent px-3 py-1.5 text-xs hover:bg-slate-100 dark:hover:bg-slate-700"
              class:bg-accent-50={$graph.selected === n.id}
              class:!border-accent-500={$graph.selected === n.id}
              onclick={() => pick(n.id)}
              onkeydown={(e) => e.key === 'Enter' && pick(n.id)}
              tabindex="0"
              data-testid={`graph-node-${n.id}`}
            >
              <span class="inline-block h-2 w-2 rounded-full" style="background: {nodeColor(n.kind)}"></span>
              <span class="font-mono text-[11px] text-slate-700 dark:text-slate-200">{n.id}</span>
              <span class="rounded bg-slate-200 px-1.5 py-0.5 text-[9px] uppercase tracking-wide text-slate-600 dark:bg-slate-700 dark:text-slate-300">
                {n.kind}
              </span>
              <span class="truncate text-slate-600 dark:text-slate-300">{n.title}</span>
            </li>
          {/each}
          {#if $filteredNodes.length > 500}
            <li class="px-3 py-3 text-center text-[11px] italic text-slate-500">
              … {$filteredNodes.length - 500} {$catalog['graph.more']}
            </li>
          {/if}
        </ul>
      </aside>

      <!-- Right column: visualization + detail -->
      <div class="flex flex-1 flex-col overflow-hidden">
        <!-- SVG visualization -->
        <div class="relative border-b border-slate-200 dark:border-slate-700" style="height: 320px;">
          <svg viewBox="0 0 {W} {H}" preserveAspectRatio="xMidYMid meet" class="h-full w-full">
            <defs>
              <marker id="arrow-default" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="5" markerHeight="5" orient="auto-start-reverse">
                <path d="M 0 0 L 10 5 L 0 10 z" fill="rgba(100,116,139,0.7)" />
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
                <g
                  class="cursor-pointer transition-transform"
                  class:scale-110={isSelected}
                  transform="translate({p.x}, {p.y})"
                  tabindex="0"
                  role="button"
                  aria-label={n.id}
                  onclick={() => pick(n.id)}
                  onkeydown={(e) => e.key === 'Enter' && pick(n.id)}
                >
                  <circle r={p.r + 2} fill="rgba(255,255,255,0.06)" />
                  <circle
                    r={p.r}
                    fill={nodeColor(n.kind)}
                    stroke={isSelected ? 'white' : 'rgba(0,0,0,0.4)'}
                    stroke-width={isSelected ? 2 : 1}
                  />
                  <text x="0" y={p.r + 12} text-anchor="middle" font-size="9" font-family="ui-monospace, monospace" fill="currentColor">
                    {n.id.length > 14 ? n.id.slice(0, 13) + '…' : n.id}
                  </text>
                  <title>{n.id} · {n.title} · {n.kind}</title>
                </g>
              {/if}
            {/each}
          </svg>
          <div class="absolute bottom-2 left-3 flex flex-wrap gap-3 rounded bg-slate-900/80 px-3 py-1 text-[10px] text-slate-200">
            {#each Object.entries($stats.by_type) as [k, v]}
              <span class="inline-flex items-center gap-1.5">
                <span class="inline-block h-2 w-2 rounded-full" style="background: {nodeColor(k as NodeKind)}"></span>
                {k} ({v})
              </span>
            {/each}
          </div>
        </div>

        <!-- Detail panel -->
        <div class="flex-1 overflow-y-auto p-6">
          {#if !selected}
            <div class="py-12 text-center text-sm text-slate-500">
              <h2 class="text-base font-medium text-slate-700 dark:text-slate-200">
                {$catalog['graph.detailEmptyTitle']}
              </h2>
              <p class="mt-2">{$catalog['graph.detailEmptyHint']}</p>
            </div>
          {:else}
            <header class="mb-4">
              <div class="flex items-center gap-2">
                <span class="font-mono text-xs text-slate-600 dark:text-slate-300">{selected.id}</span>
                <span class="rounded-full bg-slate-200 px-2 py-0.5 text-[10px] uppercase tracking-wider text-slate-700 dark:bg-slate-700 dark:text-slate-200">
                  {selected.kind}
                </span>
              </div>
              <h1 class="mt-1 text-xl font-semibold">{selected.title}</h1>
              {#if selected.source}
                <div class="mt-1 text-xs text-slate-500">
                  {$catalog['graph.fromSource']} <code class="rounded bg-slate-100 px-1 dark:bg-slate-800">{selected.source}</code>
                </div>
              {/if}
              {#if selected.tags.length}
                <div class="mt-2 flex flex-wrap gap-1">
                  {#each selected.tags as tag}
                    <span class="rounded border border-slate-200 bg-slate-50 px-1.5 py-0.5 text-[11px] text-slate-600 dark:border-slate-700 dark:bg-slate-800 dark:text-slate-300">
                      {tag}
                    </span>
                  {/each}
                </div>
              {/if}
            </header>

            {#if selected.body}
              <div class="mb-4 rounded border border-slate-200 bg-slate-50 p-3 text-sm leading-relaxed dark:border-slate-700 dark:bg-slate-800">
                <p>{selected.body}</p>
              </div>
            {/if}

            <div>
              <h3 class="text-xs font-medium uppercase tracking-wider text-slate-500">
                {$catalog['graph.connections']} ({nbh.edges.length})
              </h3>
              {#if nbh.edges.length === 0}
                <p class="mt-2 text-xs italic text-slate-500">
                  {$catalog['graph.noConnections']}
                </p>
              {:else}
                <div class="mt-3 grid grid-cols-1 gap-4 md:grid-cols-2">
                  <div>
                    <h4 class="mb-2 text-[11px] font-medium uppercase tracking-wider text-slate-500">
                      {$catalog['graph.outgoing']} ({outgoing.edges.length})
                    </h4>
                    {#each outgoing.edges.slice(0, 50) as e (e.id)}
                      {@const target = $graph.nodes.find((n) => n.id === e.to)}
                      {#if target}
                        <button
                          class="mb-1 flex w-full items-center gap-2 rounded border border-slate-200 border-l-[3px] bg-white px-2 py-1 text-left text-[11px] hover:bg-slate-50 dark:border-slate-700 dark:bg-slate-800 dark:hover:bg-slate-700"
                          style="border-left-color: {edgeColor(e.kind)}"
                          onclick={() => pick(e.to)}
                          title={e.note ?? ''}
                        >
                          <span class="font-medium text-slate-600 dark:text-slate-300">{e.kind.replace('_', ' ')}</span>
                          <span class="text-slate-400">→</span>
                          <span class="font-mono text-slate-700 dark:text-slate-200">{target.id}</span>
                        </button>
                      {/if}
                    {/each}
                  </div>
                  <div>
                    <h4 class="mb-2 text-[11px] font-medium uppercase tracking-wider text-slate-500">
                      {$catalog['graph.incoming']} ({incoming.edges.length})
                    </h4>
                    {#each incoming.edges.slice(0, 50) as e (e.id)}
                      {@const source = $graph.nodes.find((n) => n.id === e.from)}
                      {#if source}
                        <button
                          class="mb-1 flex w-full items-center gap-2 rounded border border-slate-200 border-l-[3px] bg-white px-2 py-1 text-left text-[11px] hover:bg-slate-50 dark:border-slate-700 dark:bg-slate-800 dark:hover:bg-slate-700"
                          style="border-left-color: {edgeColor(e.kind)}"
                          onclick={() => pick(e.from)}
                          title={e.note ?? ''}
                        >
                          <span class="font-mono text-slate-700 dark:text-slate-200">{source.id}</span>
                          <span class="text-slate-400">→</span>
                          <span class="font-medium text-slate-600 dark:text-slate-300">{e.kind.replace('_', ' ')}</span>
                        </button>
                      {/if}
                    {/each}
                  </div>
                </div>
              {/if}
            </div>
          {/if}
        </div>
      </div>
    </div>
  {/if}
</section>