<!--
  Reusable error boundary.

  `[FACT]` This component used to claim to catch render exceptions and
  catch nothing. `lastError` was only ever assigned `null`, so the
  `{#if lastError}` card and its retry button were unreachable — the
  component rendered its children and did nothing else. Two comments in
  the file even said so, contradicting the header: "we don't catch
  arbitrary exceptions ... we just expose the retry affordance". The
  affordance was the dead code. A test that rendered a throwing child
  proved the throw escaped `render()` entirely.

  Svelte has had a real primitive for this since 5.3.0, and the pinned
  toolchain is 5.57.1, so this is now built on `<svelte:boundary>` and
  the header's claim is true. The `{#key}` remount is kept because
  retrying a boundary whose child threw during render does not recover
  unless the subtree is torn down and rebuilt.

  Note the deliberate asymmetry: the boundary catches *render* errors and
  the errors its own `onerror` reports. It does not catch errors thrown
  from an event handler or an async callback, because the render pass
  has already finished by then and there is nothing to unwind.
-->
<script lang="ts">
  import type { Snippet } from 'svelte';
  import { catalog } from '$lib/i18n';

  interface Props {
    children?: Snippet;
    /** Surfaced to tests; the boundary itself renders no wrapper of its own. */
    fallbackKey?: string;
  }
  let { children, fallbackKey = 'default' }: Props = $props();

  let key = $state(0);
  let lastError = $state<string | null>(null);

  function toMessage(e: unknown): string {
    if (e instanceof Error) return e.message;
    return typeof e === 'string' ? e : String(e);
  }
</script>

<!--
  `[FACT]` The wrapper is deliberate, not a leftover. `<svelte:boundary>`
  emits nothing at all when its children are healthy, which makes a
  healthy boundary indistinguishable from one that was never mounted —
  to a test, and to any e2e selector. The boundary is a route-level
  wrapper in `App.svelte`, so "which route is mounted" is a question the
  page needs to be able to answer. One element is a fair price for that.
-->
<div data-testid="error-boundary" data-eb-key={key} data-state={lastError ? 'failed' : 'ok'}>
  <svelte:boundary
    onerror={(e) => {
      lastError = toMessage(e);
    }}
  >
    {#key key}
      {@render children?.()}
    {/key}

    <!--
      `[FACT]` The `failed` snippet takes two positional parameters, not a
      destructured object. The compiler's own type is
      `Snippet<[error: unknown, reset: () => void]>`; passing
      `{ error, reset }` is a type error, not a style preference.
    -->
    {#snippet failed(error, reset)}
      <div
        class="card border-red-300 bg-red-50 dark:border-red-800 dark:bg-red-900/30"
        data-testid="error-boundary-failed"
        data-fallback-key={fallbackKey}
      >
        <h3 class="text-sm font-semibold text-red-700 dark:text-red-300">
          {$catalog['errors.routeTitle']}
        </h3>
        <p class="mt-1 text-xs text-red-600 dark:text-red-200">{toMessage(error)}</p>
        <button
          class="btn-secondary mt-3"
          type="button"
          onclick={() => {
            lastError = null;
            key += 1;
            reset();
          }}
        >
          {$catalog['errors.routeRetry']}
        </button>
      </div>
    {/snippet}
  </svelte:boundary>
</div>
