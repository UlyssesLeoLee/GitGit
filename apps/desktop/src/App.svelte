<script lang="ts">
  import { onMount } from 'svelte';
  import Topbar from './components/Topbar.svelte';
  import Sidebar from './components/Sidebar.svelte';
  import GraphView from './components/GraphView.svelte';
  import Detail from './components/Detail.svelte';
  import { graph, loadGraph } from './lib/store';

  onMount(() => { loadGraph(); });
</script>

<div class="app">
  <Topbar />

  {#if $graph.loading}
    <div class="splash">
      <div class="spinner"></div>
      <p>Loading engineering graph…</p>
      {#if $graph.error}
        <p class="error">{$graph.error}</p>
      {/if}
    </div>
  {:else if $graph.error}
    <div class="splash error-state">
      <h2>Failed to load graph</h2>
      <pre>{$graph.error}</pre>
      <button class="primary" onclick={() => loadGraph()}>Retry</button>
    </div>
  {:else}
    <div class="main">
      <Sidebar />
      <div class="center">
        <GraphView />
        <Detail />
      </div>
    </div>
  {/if}
</div>

<style>
  .app { display: flex; flex-direction: column; height: 100%; }
  .main { display: flex; flex: 1; overflow: hidden; }
  .center { display: flex; flex-direction: column; flex: 1; min-width: 0; }
  .splash {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    flex: 1;
    color: var(--fg-2);
  }
  .spinner {
    width: 32px;
    height: 32px;
    border: 3px solid var(--border);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 1s linear infinite;
  }
  @keyframes spin { to { transform: rotate(360deg); } }
  .error-state pre {
    background: var(--bg-1);
    padding: 8px 12px;
    border-radius: 4px;
    color: var(--red);
    max-width: 600px;
    overflow-x: auto;
  }
  .error { color: var(--red); }
</style>