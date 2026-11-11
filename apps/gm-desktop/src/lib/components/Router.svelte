<script lang="ts">
  /**
   * Renders the component matching the current `location`.
   * The matched component receives its captured `:params` as a prop,
   * which is the contract `routes/RepoDetail.svelte` already relies on.
   */
  import { location, matchRoute, type RouteMap } from '$lib/router';

  let { routes }: { routes: RouteMap } = $props();

  const match = $derived(matchRoute(routes, $location));
</script>

{#if match}
  {@const RouteComponent = match.component}
  <RouteComponent params={match.params} />
{/if}
