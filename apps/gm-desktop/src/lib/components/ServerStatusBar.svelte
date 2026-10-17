<!--
  Top status bar — embeds an always-visible indicator for the
  embedded server's runtime state. Components elsewhere can read
  the same store via `$server`.
-->
<script lang="ts">
  import { server, serverBusy, startServer, stopServer } from '$lib/stores/server';
  import { t } from '$lib/i18n';
  import { formatUptime } from '$lib/utils/format';
  import { catalog } from '$lib/i18n';
</script>

<header
  class="flex h-14 items-center justify-between border-b border-white/20 bg-white/10 dark:border-white/10 dark:bg-slate-950/20 px-6 backdrop-blur-3xl shadow-[0_4px_24px_0_rgba(0,0,0,0.03)]"
  data-testid="server-status-bar"
>
  <div class="flex items-center gap-3.5 text-sm">
    <span
      class="inline-block h-2.5 w-2.5 rounded-full transition-all duration-300 {$server.running ? 'shadow-[0_0_12px_rgba(34,197,94,0.8)]' : ''}"
      class:bg-accent-500={$server.running}
      class:bg-slate-400={!$server.running}
      class:animate-pulse={$server.running}
      data-testid="server-indicator"
    ></span>
    <span class="dynamic-invert font-semibold tracking-wide">
      {#if $server.running}{$catalog['dashboard.running']}{:else}{$catalog['dashboard.stopped']}{/if}
    </span>
    {#if $server.running}
      <span class="text-xs font-mono opacity-70">
        · {$catalog['dashboard.port']} {$server.bind} · {$catalog['dashboard.uptime']} {formatUptime($server.uptime_secs)}
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
        {$catalog['dashboard.stop']}
      </button>
    {:else}
      <button
        type="button"
        class="btn-primary"
        disabled={$serverBusy}
        onclick={() => startServer()}
        data-testid="start-server"
      >
        {$catalog['dashboard.start']}
      </button>
    {/if}
  </div>
</header>
