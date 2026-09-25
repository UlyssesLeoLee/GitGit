<script lang="ts">
  import { graph, selectNode, getSelected, neighbors } from '../lib/store';
  import type { GraphNode, GraphEdge } from '../lib/types';

  const selected = $derived.by(() => {
    const id = $graph.selected;
    if (!id) return null;
    return $graph.nodes.find(n => n.id === id) ?? null;
  });

  const nbh = $derived(neighbors($graph.selected, 'both'));
  const incoming = $derived(neighbors($graph.selected, 'in'));
  const outgoing = $derived(neighbors($graph.selected, 'out'));

  function edgeColor(kind: GraphEdge['kind']): string {
    switch (kind) {
      case 'implements':   return 'var(--green)';
      case 'depends_on':   return 'var(--orange)';
      case 'supersedes':   return 'var(--purple)';
      case 'gated_by':     return 'var(--red)';
      case 'references':   return 'var(--fg-2)';
      case 'blocks':       return 'var(--red)';
      case 'reviewed_by':  return 'var(--accent)';
      case 'caused_by':    return 'var(--orange)';
      default:             return 'var(--fg-2)';
    }
  }
</script>

<section class="detail">
  {#if !selected}
    <div class="empty">
      <h2>Select a node</h2>
      <p>Pick something in the sidebar to inspect its definition, source, and graph neighbors.</p>
      <p class="hint">Try <code>REQ-GRF-001</code> (the engine of the whole platform), <code>ADR-0001</code>, or search for <em>audit</em>.</p>
    </div>
  {:else}
    <header class="dh">
      <div class="dh-row">
        <span class="id">{selected.id}</span>
        <span class="kind kind-{selected.kind}">{selected.kind}</span>
      </div>
      <h1>{selected.title}</h1>
      {#if selected.source}
        <div class="source">from <code>{selected.source}</code></div>
      {/if}
      {#if selected.tags.length}
        <div class="tags">
          {#each selected.tags as t}<span class="tag">{t}</span>{/each}
        </div>
      {/if}
    </header>

    {#if selected.body}
      <div class="body">
        <p>{selected.body}</p>
      </div>
    {/if}

    <div class="conns">
      <h3>Connections <span class="count">({nbh.edges.length})</span></h3>
      {#if nbh.edges.length === 0}
        <p class="hint">No graph edges. This node is isolated (orphan or seed).</p>
      {:else}
        <div class="conn-grid">
          <div>
            <h4>Outgoing <span class="count">({outgoing.edges.length})</span></h4>
            {#each outgoing.edges.slice(0, 50) as e (e.id)}
              {@const target = $graph.nodes.find(n => n.id === e.to)}
              {#if target}
                <button class="edge" onclick={() => selectNode(e.to)} title={e.note ?? ''} style="--c: {edgeColor(e.kind)}">
                  <span class="rel">{e.kind.replace('_', ' ')}</span>
                  <span class="arrow">→</span>
                  <span class="to">{target.id}</span>
                </button>
              {/if}
            {/each}
          </div>
          <div>
            <h4>Incoming <span class="count">({incoming.edges.length})</span></h4>
            {#each incoming.edges.slice(0, 50) as e (e.id)}
              {@const source = $graph.nodes.find(n => n.id === e.from)}
              {#if source}
                <button class="edge" onclick={() => selectNode(e.from)} title={e.note ?? ''} style="--c: {edgeColor(e.kind)}">
                  <span class="from">{source.id}</span>
                  <span class="arrow">→</span>
                  <span class="rel">{e.kind.replace('_', ' ')}</span>
                </button>
              {/if}
            {/each}
          </div>
        </div>
      {/if}
    </div>
  {/if}
</section>

<style>
  .detail {
    flex: 1;
    overflow-y: auto;
    padding: 24px;
    min-width: 0;
  }
  .empty { color: var(--fg-2); padding: 48px; text-align: center; }
  .empty h2 { color: var(--fg-1); }
  .empty .hint { font-size: 12px; }
  .dh-row { display: flex; gap: 8px; align-items: center; margin-bottom: 6px; }
  .id { font-family: var(--font-mono); color: var(--fg-1); font-size: 12px; }
  .kind {
    font-size: 10px;
    text-transform: uppercase;
    padding: 2px 8px;
    border-radius: 10px;
    background: var(--bg-3);
    letter-spacing: 0.4px;
  }
  .kind-requirement { background: var(--node-requirement); color: white; }
  .kind-adr         { background: var(--node-adr); color: white; }
  .kind-policy      { background: var(--node-policy); color: white; }
  .kind-agent       { background: var(--node-agent); color: black; }
  .kind-document    { background: var(--node-event); color: white; }
  h1 { margin: 0; font-size: 20px; }
  .source { color: var(--fg-2); font-size: 12px; margin-top: 6px; }
  .tags { margin-top: 8px; display: flex; gap: 4px; flex-wrap: wrap; }
  .tag {
    background: var(--bg-2);
    border: 1px solid var(--border);
    padding: 1px 6px;
    font-size: 11px;
    border-radius: 3px;
    color: var(--fg-1);
  }
  .body {
    margin-top: 16px;
    padding: 12px;
    background: var(--bg-1);
    border: 1px solid var(--border);
    border-radius: 6px;
    font-size: 13px;
    line-height: 1.55;
  }
  .conns { margin-top: 24px; }
  .conns h3, .conn-grid h4 {
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.4px;
    color: var(--fg-2);
    margin: 0 0 8px;
  }
  .count { color: var(--fg-1); font-weight: normal; }
  .conn-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 16px;
  }
  .edge {
    display: flex;
    gap: 6px;
    align-items: center;
    width: 100%;
    text-align: left;
    background: var(--bg-1);
    border: 1px solid var(--border);
    border-left: 3px solid var(--c, var(--accent));
    padding: 4px 8px;
    margin-bottom: 4px;
    border-radius: 4px;
    cursor: pointer;
    font-size: 11px;
    overflow: hidden;
  }
  .edge:hover { background: var(--bg-2); border-color: var(--c, var(--accent)); }
  .edge .rel { color: var(--c, var(--accent)); font-weight: 500; text-transform: lowercase; }
  .edge .arrow { color: var(--fg-2); }
  .edge .to, .edge .from { font-family: var(--font-mono); color: var(--fg-1); }
</style>