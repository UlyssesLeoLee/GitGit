<!--
  Working-tree root picker.

  One control, used in two places: the Settings page (the persistent
  home) and the working-tree error panel, because that panel is where a
  user actually discovers they need it. Both mount the same component
  so there is one behaviour and one place to read it.

  It shows the current value, or says plainly that none is set and what
  happens then. A bare path with no label would be worse than no
  control at all.

  Cancelling the dialog is not an error and does not clear anything:
  the previous value survives a cancelled pick untouched.
-->
<script lang="ts">
  import { worktreeRoot, pickWorktreeRoot, clearWorktreeRoot } from '$lib/stores/worktreeRoot';
  import { pushToast } from '$lib/stores/toasts';
  import { catalog } from '$lib/i18n';

  interface Props {
    /** Compact form for the error panel: no heading, no hint. */
    compact?: boolean;
  }
  let { compact = false }: Props = $props();

  let busy = $state(false);

  async function choose(): Promise<void> {
    busy = true;
    try {
      const outcome = await pickWorktreeRoot($catalog['repos.root.choose']);
      if (outcome === 'set') {
        pushToast('success', $catalog['repos.root.chose']);
      } else if (outcome === 'invalid') {
        pushToast('warn', $catalog['repos.root.invalid']);
      } else if (outcome === 'error') {
        pushToast('error', $catalog['repos.root.pickerFailed']);
      }
      // `cancelled` says nothing: the user changed their mind, which is
      // not a failure and must not touch the stored value.
    } finally {
      busy = false;
    }
  }

  function clear(): void {
    clearWorktreeRoot();
    pushToast('info', $catalog['repos.root.cleared']);
  }
</script>

<div class="space-y-2" data-testid="wt-root-picker">
  {#if !compact}
    <p class="text-sm font-medium" data-testid="wt-root-label">{$catalog['repos.root.label']}</p>
    <p class="text-xs text-slate-500" data-testid="wt-root-hint">{$catalog['repos.root.hint']}</p>
  {/if}

  {#if $worktreeRoot}
    <p class="break-all font-mono text-xs" data-testid="wt-root-value">{$worktreeRoot}</p>
  {:else}
    <p class="text-xs text-slate-500" data-testid="wt-root-unset">{$catalog['repos.root.unset']}</p>
  {/if}

  <div class="flex flex-wrap gap-2">
    <button
      class="btn-secondary text-xs"
      type="button"
      disabled={busy}
      onclick={choose}
      data-testid="wt-root-choose"
    >
      {$catalog['repos.root.choose']}
    </button>
    {#if $worktreeRoot}
      <button
        class="btn-secondary text-xs"
        type="button"
        onclick={clear}
        data-testid="wt-root-clear"
      >
        {$catalog['repos.root.clear']}
      </button>
    {/if}
  </div>
</div>
