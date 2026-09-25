<script lang="ts">
  import { graph, filteredNodes, selectNode, stats } from '../lib/store';
  import type { GraphNode, NodeKind } from '../lib/types';

  function color(kind: NodeKind): string {
    return `var(--node-${kind}, var(--node-event))`;
  }

  function rowForKind(kind: NodeKind): number {
    const order: NodeKind[] = ['requirement', 'adr', 'policy', 'document', 'agent', 'pr', 'commit', 'issue', 'incident', 'release', 'human', 'event'];
    return order.indexOf(kind);
  }

  function badge(kind: NodeKind): string {
    return kind.toUpperCase();
  }

  function select(n: GraphNode) {
    selectNode(n.id);
  }
</script>

<aside class="sidebar">
  <div class="filter">
    <h3>Nodes <span class="count">{$stats.nodes}</span></h3>
    <p class="hint">{$filteredNodes.length} matching "{$graph.query || '*'}"</p>
  </div>

  {#if $filteredNodes.length === 0}
    <div class="empty">
      <p>No nodes match.</p>
      <p class="hint">Try clearing the search box.</p>
    </div>
  {:else}
    <ul role="listbox">
      {#each $filteredNodes.slice(0, 500) as n (n.id)}
        <li role="option" aria-selected={$graph.selected === n.id} class:selected={$graph.selected === n.id} onclick={() => select(n)} onkeydown={(e) => e.key === 'Enter' && select(n)} tabindex="0" style="--kind-color: {color(n.kind)}; --row: {rowForKind(n.kind)}">
          <span class="dot" style="background: {color(n.kind)};"></span>
          <span class="id">{n.id}</span>
          <span class="kind">{badge(n.kind)}</span>
          <span class="title">{n.title}</span>
        </li>
      {/each}
      {#if $filteredNodes.length > 500}
        <li class="more">… {$filteredNodes.length - 500} more (refine search)</li>
      {/if}
    </ul>
  {/if}
</aside>

<style>
  .sidebar {
    width: 360px;
    border-right: 1px solid var(--border);
    background: var(--bg-1);
    display: flex;
    flex-direction: column;
    flex-shrink: 0;
    overflow: hidden;
  }
  .filter {
    padding: 12px 14px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }
  .filter h3 {
    margin: 0;
    font-size: 12px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--fg-2);
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .filter h3 .count { color: var(--fg-1); }
  .filter .hint { margin: 4px 0 0; font-size: 11px; color: var(--fg-2); }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    overflow-y: auto;
    flex: 1;
  }
  li {
    display: grid;
    grid-template-columns: 8px 96px 64px 1fr;
    gap: 8px;
    align-items: center;
    padding: 4px 12px;
    border-left: 2px solid transparent;
    cursor: pointer;
    font-size: 12px;
    line-height: 1.4;
    user-select: none;
  }
  li:hover { background: var(--bg-2); }
  li.selected { background: var(--accent-soft); border-left-color: var(--accent); }
  li:focus-visible { outline: 2px solid var(--accent); outline-offset: -2px; }
  li .dot { width: 8px; height: 8px; border-radius: 50%; }
  li .id { font-family: var(--font-mono); font-size: 11px; color: var(--fg-1); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  li .kind { font-size: 9px; color: var(--fg-2); text-transform: uppercase; letter-spacing: 0.4px; text-align: center; padding: 1px 4px; border-radius: 3px; background: var(--bg-3); overflow: hidden; text-overflow: ellipsis; }
  li.selected .kind { background: var(--accent); color: white; }
  li .title { color: var(--fg-0); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  li.more { text-align: center; padding: 12px; color: var(--fg-2); font-style: italic; grid-template-columns: 1fr; }
  .empty { padding: 24px; text-align: center; color: var(--fg-2); }
</style>