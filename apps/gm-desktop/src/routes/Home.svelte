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
  import type { AppErrorPayload } from '$lib/api/types';

  let bind = $state('127.0.0.1:38080');
  let logs = $state<string[]>([]);
  let logLimit = $state<number>(200);
  let refreshing = $state<boolean>(false);

  onMount(async () => {
    await refreshLogs();
    // Poll the status every 2s when the page is mounted.
    refreshServerStatus();
  });

  async function refreshLogs(): Promise<void> {
    refreshing = true;
    try {
      logs = (await tauri.serverLogs(logLimit)) ?? [];
    } finally {
      refreshing = false;
    }
  }

  async function onStart(): Promise<void> {
    try {
      await startServer(bind);
      pushToast('success', $t('dashboard.running'));
      await refreshLogs();
    } catch (e) {
      pushToast('error', friendlyError(e));
    }
  }

  async function onStop(): Promise<void> {
    try {
      await stopServer();
      pushToast('info', $t('dashboard.stopped'));
      await refreshLogs();
    } catch (e) {
      pushToast('error', friendlyError(e));
    }
  }

  async function onClearLogs(): Promise<void> {
    await tauri.clearLogs();
    await refreshLogs();
  }

  function friendlyError(err: unknown): string {
    if (typeof err === 'object' && err !== null && 'kind' in err) {
      const e = err as AppErrorPayload;
      const tmplKey = `errors.kind.${e.kind}`;
      // The i18n lookup returns the key when missing; fall back to
      // the raw `message` so we still surface something useful.
      const tmpl = $t(tmplKey);
      return tmpl === tmplKey ? e.message : tmpl;
    }
    return String(err);
  }
</script>

<section class="space-y-6" aria-labelledby="home-h">
  <header class="flex items-center justify-between">
    <h1 id="home-h" class="text-2xl font-semibold">{$t('dashboard.heading')}</h1>
  </header>

  <div class="grid gap-6 lg:grid-cols-3">
    <div class="card lg:col-span-1">
      <h2 class="mb-3 text-sm font-semibold">{$t('dashboard.bind')}</h2>
      <label class="label" for="bind-input">host:port</label>
      <input
        id="bind-input"
        class="input"
        type="text"
        bind:value={bind}
        disabled={$server.running}
        data-testid="bind-input"
      />
      <p class="mt-1 text-xs text-slate-500">{$t('dashboard.defaultBindHint')}</p>

      <div class="mt-4 flex gap-2">
        {#if $server.running}
          <button class="btn-secondary" type="button" disabled={$serverBusy} onclick={onStop}>
            {$t('dashboard.stop')}
          </button>
        {:else}
          <button class="btn-primary" type="button" disabled={$serverBusy} onclick={onStart} data-testid="home-start">
            {$serverBusy ? $t('dashboard.starting') : $t('dashboard.start')}
          </button>
        {/if}
      </div>

      <dl class="mt-4 grid grid-cols-2 gap-2 text-xs text-slate-600 dark:text-slate-300">
        <div>
          <dt class="label">{$t('dashboard.pid')}</dt>
          <dd class="font-mono">{$server.pid || '—'}</dd>
        </div>
        <div>
          <dt class="label">{$t('dashboard.uptime')}</dt>
          <dd class="font-mono">{formatUptime($server.uptime_secs)}</dd>
        </div>
        <div class="col-span-2">
          <dt class="label">{$t('dashboard.port')}</dt>
          <dd class="font-mono break-all">{$server.bind || '—'}</dd>
        </div>
      </dl>
    </div>

    <div class="card lg:col-span-2">
      <div class="mb-2 flex items-center justify-between">
        <h2 class="text-sm font-semibold">{$t('dashboard.recentLogs')}</h2>
        <div class="flex gap-2">
          <button class="btn-secondary" type="button" onclick={refreshLogs} disabled={refreshing}>
            {$t('common.refresh')}
          </button>
          <button class="btn-secondary" type="button" onclick={onClearLogs}>
            {$t('common.delete')}
          </button>
        </div>
      </div>
      <pre class="max-h-80 overflow-auto rounded-md bg-slate-950/90 p-3 text-xs leading-relaxed text-slate-100" data-testid="logs-pane">{#each logs as line, i (i)}{line}
{/each}</pre>
      <p class="mt-1 text-xs text-slate-500">locale: {$locale}</p>
    </div>
  </div>
</section>
