<!--
  Catch-all not-found route. `svelte-spa-router` lands here for
  every path that isn't a registered route.
-->
<script lang="ts">
  import { t } from '$lib/i18n';
</script>

<section class="card max-w-prose text-center" data-testid="not-found">
  <h1 class="text-2xl font-semibold">404</h1>
  <p class="mt-2 text-sm text-slate-600 dark:text-slate-300">
    {$t('common.empty')}
  </p>
  <a class="btn-primary mt-4" href="/">{$t('nav.dashboard')}</a>
</section>
