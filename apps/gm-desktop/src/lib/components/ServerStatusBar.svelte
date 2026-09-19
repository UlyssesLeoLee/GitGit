<!--
  Top status bar — embeds an always-visible indicator for the
  embedded server's runtime state. Components elsewhere can read
  the same store via `$server`.
-->
<script lang="ts">
  import { server, serverBusy, startServer, stopServer } from '$lib/stores/server';
  import { t } from '$lib/i18n';
  import { formatUptime } from '$lib/utils/format';
</script>

<header
  class="flex h-12 items-center justify-between border-b border-slate-200 bg-white px-4 dark:border-slate-700 dark:bg-slate-800"
  data-testid="server-status-bar"
>
  <div class="flex items-center gap-3 text-sm">
    <span
      class="inline-block h-2.5 w-2.5 rounded-full"
      class:bg-accent-500={$server.running}
      class:bg-slate-400={!$server.running}
      data-testid="server-indicator"
    ></span>
    <span class="font-medium">
      {#if $server.running}{$t('dashboard.running')}{:else}{$t('dashboard.stopped')}{/if}
    </span>
    {#if $server.running}
      <span class="text-slate-500 dark:text-slate-400">
        · {$t('dashboard.port')} {$server.bind} · {$t('dashboard.uptime')} {formatUptime($server.uptime_secs)}
      </span>
    {/if}
  </div>

  <div class="flex items-center gap-2">
    {#if $server.running}
      <button
        type="button"
        class="btn-secondary"
        disabled={$serverBusy}
        onclick={() => stopServer()}
        data-testid="stop-server"
      >
        {$t('dashboard.stop')}
      </button>
    {:else}
      <button
        type="button"
        class="btn-primary"
        disabled={$serverBusy}
        onclick={() => startServer()}
        data-testid="start-server"
      >
        {$t('dashboard.start')}
      </button>
    {/if}
  </div>
</header>
