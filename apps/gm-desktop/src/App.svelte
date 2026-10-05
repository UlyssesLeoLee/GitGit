<!--
  Root component. Owns the global layout: sidebar + main panel.
  Page-level navigation is delegated to the local hash router in
  `$lib/router` (see the note there for why `svelte-spa-router` was
  dropped: it is incompatible with Svelte 5 runes mode).
-->
<script lang="ts">
  import { onMount } from 'svelte';
  import { fade } from 'svelte/transition';
  import Router from '$lib/components/Router.svelte';
  import Sidebar from '$lib/components/Sidebar.svelte';
  import ServerStatusBar from '$lib/components/ServerStatusBar.svelte';
  import ToastHost from '$lib/components/ToastHost.svelte';
  import ErrorBoundary from '$lib/components/ErrorBoundary.svelte';
  import { initLocale } from '$lib/stores/locale';
  import { initTheme } from '$lib/stores/theme';
  import { server, refreshServerStatus } from '$lib/stores/server';
  import { repos, refreshRepos } from '$lib/stores/repos';
  import { refreshVault } from '$lib/stores/vault';
  import { t } from '$lib/i18n';
  import Home from './routes/Home.svelte';
  import Repos from './routes/Repos.svelte';
  import RepoDetail from './routes/RepoDetail.svelte';
  import Vault from './routes/Vault.svelte';
  import Settings from './routes/Settings.svelte';
  import Graph from './routes/Graph.svelte';
  import Review from './routes/Review.svelte';
  import NotFound from './routes/NotFound.svelte';
  import { catalog } from '$lib/i18n';

  // The matched route component receives its captured `:params` as a prop.
  // A `*` entry acts as the catch-all. The whole router sits inside the
  // ErrorBoundary below, so per-route guards are not needed here — if
  // one is wanted later it can wrap a single entry in this map.
  const routes = {
    '/': { component: Home },
    '/repos': { component: Repos },
    '/repos/:name': { component: RepoDetail },
    '/vault': { component: Vault },
    '/graph': { component: Graph },
    '/review': { component: Review },
    '/settings': { component: Settings },
    '*': { component: NotFound },
  };

  let booted = $state(false);
  let bootError = $state<string | null>(null);

  onMount(async () => {
    try {
      // Theme first so the first paint already uses the right palette.
      initTheme();
      await initLocale();
      // Pre-warm stores. These calls bail silently when running under
      // `vite dev` outside Tauri (mock handlers catch the missing
      // backend).
      await Promise.allSettled([
        refreshServerStatus(),
        refreshRepos(),
        refreshVault(),
      ]);
      booted = true;
    } catch (err) {
      bootError = err instanceof Error ? err.message : String(err);
      booted = true;
    }
  });
</script>

<!--
  `[FACT]` There is deliberately no `<svelte:head>` here putting `<html>`
  in the document head. That looks harmless and is not: Svelte compiles
  `<svelte:head>` content through `$.from_html`, which builds a
  `<template>` and assigns `innerHTML`, and the HTML parser's "in
  template" insertion mode **ignores a nested `<html>` start tag**. The
  element is dropped silently — measured, feeding
  `'<html lang="zh-CN" data-theme="dark"></html>'` to a template yields
  `childNodes.length === 0` and a serialised value of `""`. Svelte's head
  code then calls `.cloneNode()` on `firstChild`, which is `null`, so
  mounting this component throws

      TypeError: Cannot read properties of null (reading 'cloneNode')

  and the app never starts. WebView2 runs the same spec parser, so this
  is not a jsdom artifact.

  `data-theme` was already redundant before this was found: `stores/theme`
  subscribes on module load and writes `document.documentElement.dataset`
  itself. `lang` had no such wiring, so it is now mirrored there, in
  `stores/locale`, rather than being written from a component.
-->

<div class="flex h-screen w-screen overflow-hidden" data-testid="app-shell">
  <Sidebar />

  <main class="flex flex-1 flex-col overflow-hidden" data-testid="main-panel">
    <ServerStatusBar />

    <div class="flex-1 overflow-y-auto p-6" data-testid="page-host">
      {#if bootError}
        <ErrorBoundary>
          <div class="card max-w-prose">
            <h2 class="text-lg font-semibold text-red-600">
              {$catalog['errors.bootFailedTitle']}
            </h2>
            <p class="mt-2 text-sm text-slate-600 dark:text-slate-300">
              {bootError}
            </p>
            <p class="mt-2 text-xs text-slate-500">
              {$catalog['errors.bootFailedHint']}
            </p>
          </div>
        </ErrorBoundary>
      {:else if booted}
        <ErrorBoundary>
          <div in:fade={{ duration: 120 }}>
            <Router {routes} />
          </div>
        </ErrorBoundary>
      {:else}
        <div class="flex h-full items-center justify-center" data-testid="boot-spinner">
          <div
            class="h-10 w-10 animate-spin rounded-full border-2 border-slate-300 border-t-accent-500"
          ></div>
        </div>
      {/if}
    </div>
  </main>
</div>

<ToastHost />