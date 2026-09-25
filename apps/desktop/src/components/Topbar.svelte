<script lang="ts">
  import { graph, stats, setQuery, selectNode, filteredNodes } from '../lib/store';

  let query = $state($graph.query);

  function oninput(e: Event) {
    query = (e.target as HTMLInputElement).value;
    setQuery(query);
  }

  function pick(id: string) {
    selectNode(id);
  }
</script>

<header class="topbar">
  <div class="brand">
    <img src="/logo.svg" alt="GitGit" width="28" height="28" />
    <div>
      <div class="title">GitGit Desktop</div>
      <div class="subtitle">
        Local-first viewer · engineering knowledge graph
        <span class="badge {$graph.source}">{$graph.source}</span>
        <span class="badge os">{$graph.os}</span>
        <span class="badge ver">v{$graph.version}</span>
      </div>
    </div>
  </div>

  <div class="search">
    <input
      type="search"
      placeholder="Filter by ID, title, tag… (e.g. GRF, audit, phase6)"
      value={query}
      oninput={oninput}
    />
  </div>

  <div class="stats">
    <div class="stat"><b>{$stats.nodes}</b><span>nodes</span></div>
    <div class="stat"><b>{$stats.edges}</b><span>edges</span></div>
  </div>
</header>

<style>
  .topbar {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 8px 16px;
    background: var(--bg-1);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }
  .brand { display: flex; align-items: center; gap: 10px; }
  .brand .title { font-weight: 600; }
  .brand .subtitle { font-size: 11px; color: var(--fg-2); display: flex; gap: 6px; align-items: center; }
  .search { flex: 1; max-width: 480px; }
  .search input { width: 100%; }
  .stats { display: flex; gap: 12px; }
  .stat { display: flex; flex-direction: column; align-items: center; font-size: 12px; }
  .stat b { font-size: 16px; }
  .stat span { color: var(--fg-2); font-size: 10px; text-transform: uppercase; letter-spacing: 0.5px; }
  .badge {
    font-size: 9px;
    padding: 1px 6px;
    border-radius: 8px;
    background: var(--bg-3);
    color: var(--fg-1);
    text-transform: uppercase;
    letter-spacing: 0.4px;
  }
  .badge.tauri { background: rgba(47, 129, 247, 0.25); color: #6cb0ff; }
  .badge.web   { background: rgba(163, 113, 247, 0.25); color: #c8a8ff; }
</style>