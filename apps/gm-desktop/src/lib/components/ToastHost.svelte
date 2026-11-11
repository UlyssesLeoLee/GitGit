<!--
  Toast host. Renders each toast in the global `toasts` store as a
  dismissible card. Position is fixed-bottom-right to match the
  brief's "桌面通知" UX hint.
-->
<script lang="ts">
  import { toasts, dismissToast } from '$lib/stores/toasts';
  import { fly } from 'svelte/transition';

  function kindClass(kind: string): string {
    switch (kind) {
      case 'success': return 'bg-emerald-500/95 text-white';
      case 'warn': return 'bg-amber-500/95 text-amber-950';
      case 'error': return 'bg-red-500/95 text-white';
      default: return 'bg-slate-800/95 text-white';
    }
  }
</script>

<div
  class="pointer-events-none fixed bottom-4 right-4 z-50 flex w-80 flex-col gap-2"
  role="region"
  aria-label="toast region"
  data-testid="toast-host"
>
  {#each $toasts as toast (toast.id)}
    <div
      class="pointer-events-auto rounded-md px-3 py-2 text-sm shadow-lg {kindClass(toast.kind)}"
      role="status"
      data-testid={`toast-${toast.kind}`}
      in:fly={{ y: 12, duration: 160 }}
      out:fly={{ y: 12, duration: 120 }}
    >
      <div class="flex items-start justify-between gap-2">
        <span>{toast.message}</span>
        <button
          type="button"
          class="rounded p-1 text-xs opacity-70 hover:bg-black/10 hover:opacity-100"
          onclick={() => dismissToast(toast.id)}
          aria-label="Dismiss"
        >
          ✕
        </button>
      </div>
    </div>
  {/each}
</div>
