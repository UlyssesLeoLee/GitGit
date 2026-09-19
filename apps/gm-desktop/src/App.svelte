<!--
  Root component. Owns the global layout: sidebar + main panel.
  All page-level navigation is delegated to `svelte-spa-router`,
  which keeps the bundle small (versus `sveltekit`).
-->
<script lang="ts">
  import { onMount } from 'svelte';
  import Router from 'svelte-spa-router';
  import { wrap } from 'svelte-spa-router/wrap';
  import { fade } from 'svelte/transition';
  import Sidebar from '$lib/components/Sidebar.svelte';
  import ServerStatusBar from '$lib/components/ServerStatusBar.svelte';
  import ToastHost from '$lib/components/ToastHost.svelte';
  import ErrorBoundary from '$lib/components/ErrorBoundary.svelte';
  import { locale, initLocale } from '$lib/stores/locale';
  import { theme, initTheme } from '$lib/stores/theme';
  import { server, refreshServerStatus } from '$lib/stores/server';
  import { repos, refreshRepos } from '$lib/stores/repos';
  import { vault, refreshVault } from '$lib/stores/vault';
  import { t } from '$lib/i18n';
  import Home from './routes/Home.svelte';
  import Repos from './routes/Repos.svelte';
  import RepoDetail from './routes/RepoDetail.svelte';
  import Vault from './routes/Vault.svelte';
  import Settings from './routes/Settings.svelte';
  import NotFound from './routes/NotFound.svelte';

  // `wrap()` adds a guard component that re-renders its slot if a
  // route renders an exception; in our case the inner ErrorBoundary
  // already owns that responsibility, but we keep the wrap so future
  // route-level guards (e.g. /settings/:tab can require feature
  // flags) compose cleanly.
  const routes = {
    '/': wrap({ component: Home }),
    '/repos': wrap({ component: Repos }),
    '/repos/:name': wrap({ component: RepoDetail }),
    '/vault': wrap({ component: Vault }),
    '/settings': wrap({ component: Settings }),
    '*': wrap({ component: NotFound }),
  } as const;

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

<svelte:head>
  <html lang={$locale} data-theme={$theme} />
</svelte:head>

<div class="flex h-screen w-screen overflow-hidden" data-testid="app-shell">
  <Sidebar />

  <main class="flex flex-1 flex-col overflow-hidden" data-testid="main-panel">
    <ServerStatusBar />

    <div class="flex-1 overflow-y-auto p-6" data-testid="page-host">
      {#if bootError}
        <ErrorBoundary>
          <div class="card max-w-prose">
            <h2 class="text-lg font-semibold text-red-600">
              {$t('errors.bootFailedTitle')}
            </h2>
            <p class="mt-2 text-sm text-slate-600 dark:text-slate-300">
              {bootError}
            </p>
            <p class="mt-2 text-xs text-slate-500">
              {$t('errors.bootFailedHint')}
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
          <div class="h-10 w-10 animate-spin rounded-full border-2 border-slate-300 border-t-accent-500" />
        </div>
      {/if}
    </div>
  </main>
</div>

<ToastHost />
