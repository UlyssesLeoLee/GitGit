<!--
  Dashboard / server-control page. Composes:
  - Start / Stop button (state-bound to the same store as the top bar)
  - Bind configuration (defaults to 127.0.0.1:38080)
  - Recent logs (with a clear button)
  - Diagnostics
-->
<script lang="ts">
  import { onMount } from 'svelte';
  import { server, serverBusy, startServer, stopServer, refreshServerStatus } from '$lib/stores/server';
  import * as tauri from '$lib/api/tauri';
  import { t, locale } from '$lib/i18n';
  import { formatUptime } from '$lib/utils/format';
  import { pushToast } from '$lib/stores/toasts';
  import { friendlyError } from '$lib/utils/errors';
  import { catalog } from '$lib/i18n';

  let bind = $state('127.0.0.1:38080');
  let logs = $state<string[]>([]);
  let logLimit = $state<number>(200);
  let refreshing = $state<boolean>(false);

  onMount(async () => {
    await refreshLogs();
    // Poll the status every 2s when the page is mounted.
    refreshServerStatus();
  });

  // `[FACT]` Was a single `refreshing` boolean, with `refreshLogs` setting
  // it true on entry and false in a `finally`. Two overlapping calls both
  // set it, and the *earlier* one to finish cleared the flag while the
  // later one was still in flight — so the page showed "not refreshing"
  // while a fetch was outstanding, and a caller that checked the flag to
  // decide whether to start its own refresh would start a duplicate.
  // Counter instead: the flag is released only when the last outstanding
  // call finishes, whichever order they complete in.
  let refreshesInFlight = 0;

  async function refreshLogs(): Promise<void> {
    refreshesInFlight += 1;
    refreshing = true;
    try {
      logs = (await tauri.serverLogs(logLimit)) ?? [];
    } finally {
      refreshesInFlight -= 1;
      if (refreshesInFlight === 0) refreshing = false;
    }
  }

  async function onStart(): Promise<void> {
    try {
      await startServer(bind);
      pushToast('success', t('dashboard.running'));
      await refreshLogs();
    } catch (e) {
      pushToast('error', friendlyError(e));
    }
  }

  async function onStop(): Promise<void> {
    try {
      await stopServer();
      pushToast('info', t('dashboard.stopped'));
      await refreshLogs();
    } catch (e) {
      pushToast('error', friendlyError(e));
    }
  }

  async function onClearLogs(): Promise<void> {
    await tauri.clearLogs();
    await refreshLogs();
  }
</script>

<section class="space-y-6" aria-labelledby="home-h">
  <header class="flex items-center justify-between">
    <h1 id="home-h" class="text-2xl font-semibold">{$catalog['dashboard.heading']}</h1>
  </header>

  <div class="grid gap-6 lg:grid-cols-3">
    <div class="card lg:col-span-1">
      <h2 class="mb-3 text-sm font-semibold">{$catalog['dashboard.bind']}</h2>
      <label class="label" for="bind-input">host:port</label>
      <input
        id="bind-input"
        class="input"
        type="text"
        bind:value={bind}
        disabled={$server.running}
        data-testid="bind-input"
      />
      <p class="mt-1 text-xs text-slate-500">{$catalog['dashboard.defaultBindHint']}</p>

      <div class="mt-4 flex gap-2">
        {#if $server.running}
          <button class="btn-secondary" type="button" disabled={$serverBusy} onclick={onStop}>
            {$catalog['dashboard.stop']}
          </button>
        {:else}
          <button class="btn-primary" type="button" disabled={$serverBusy} onclick={onStart} data-testid="home-start">
            {$serverBusy ? t('dashboard.starting') : t('dashboard.start')}
          </button>
        {/if}
      </div>

      <dl class="mt-4 grid grid-cols-2 gap-2 text-xs text-slate-600 dark:text-slate-300">
        <div>
          <dt class="label">{$catalog['dashboard.pid']}</dt>
          <dd class="font-mono">{$server.pid || '—'}</dd>
        </div>
        <div>
          <dt class="label">{$catalog['dashboard.uptime']}</dt>
          <dd class="font-mono">{formatUptime($server.uptime_secs)}</dd>
        </div>
        <div class="col-span-2">
          <dt class="label">{$catalog['dashboard.port']}</dt>
          <dd class="font-mono break-all">{$server.bind || '—'}</dd>
        </div>
      </dl>
    </div>

    <div class="card lg:col-span-2">
      <div class="mb-2 flex items-center justify-between">
        <h2 class="text-sm font-semibold">{$catalog['dashboard.recentLogs']}</h2>
        <div class="flex gap-2">
          <button class="btn-secondary" type="button" onclick={refreshLogs} disabled={refreshing}>
            {$catalog['common.refresh']}
          </button>
          <button class="btn-secondary" type="button" onclick={onClearLogs}>
            {$catalog['common.delete']}
          </button>
        </div>
      </div>
      <pre class="max-h-80 overflow-auto rounded-md bg-slate-950/90 p-3 text-xs leading-relaxed text-slate-100" data-testid="logs-pane">{#each logs as line, i (i)}{line}
{/each}</pre>
      <p class="mt-1 text-xs text-slate-500">locale: {$locale}</p>
    </div>
  </div>
</section>
