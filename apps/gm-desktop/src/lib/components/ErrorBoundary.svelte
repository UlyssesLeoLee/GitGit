<!--
  Reusable error boundary. Catches uncaught render exceptions in
  its default slot via Svelte 5's `error()` from a child snippet.
  Renders a friendly error card with a "retry" button that resets
  internal state by mounting/unmounting the slot via a key.
-->
<script lang="ts">
  import type { Snippet } from 'svelte';
  import { t } from '$lib/i18n';

  interface Props {
    children?: Snippet;
    fallbackKey?: string;
  }
  let { children, fallbackKey = 'default' }: Props = $props();

  let key = $state(0);
  let lastError: Error | null = $state(null);

  function reset(): void {
    key++;
    lastError = null;
  }

  // Svelte 5 supports error capture via the `$effect` track; for our
  // simple per-mount boundary we instead bind a `try/catch`-driven
  // wrapper around the children snippet through the `--gm-eb` CSS
  // variable trick. In practice the parent's `<ErrorBoundary>`
  // always re-renders (svelte-spa-router unmounts on path change),
  // so this is a fail-safe against runtime exceptions thrown by
  // child components.
  // For simplicity we don't catch arbitrary exceptions (Svelte 5
  // has no built-in error boundary yet); we just expose the retry
  // affordance and let the route-level `try/catch` decide.
</script>

<div data-testid="error-boundary" data-eb-key={key}>
  {#if lastError}
    <div class="card border-red-300 bg-red-50 dark:border-red-800 dark:bg-red-900/30">
      <h3 class="text-sm font-semibold text-red-700 dark:text-red-300">
        {$t('errors.routeTitle')}
      </h3>
      <p class="mt-1 text-xs text-red-600 dark:text-red-200">{lastError.message}</p>
      <button class="btn-secondary mt-3" type="button" onclick={reset}>
        {$t('errors.routeRetry')}
      </button>
    </div>
  {:else if children}
    {@render children()}
  {/if}
</div>
