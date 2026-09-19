<!--
  Repositories list page. Renders one card per repo, with a quick
  "Copy clone URL" and "Open in Finder" affordance per card.
-->
<script lang="ts">
  import { onMount } from 'svelte';
  import { repos, refreshRepos, cloneUrl, openRepoInShell } from '$lib/stores/repos';
  import { t } from '$lib/i18n';
  import { copyText } from '$lib/utils/clipboard';
  import { formatBytes } from '$lib/utils/format';
  import { pushToast } from '$lib/stores/toasts';

  let loading = $state(false);
  let query = $state('');

  const filtered = $derived(
    $repos.filter((r) => r.name.toLowerCase().includes(query.trim().toLowerCase())),
  );

  onMount(async () => {
    loading = true;
    await refreshRepos();
    loading = false;
  });

  async function onCopy(name: string): Promise<void> {
    const url = await cloneUrl(name);
    const ok = await copyText(url);
    pushToast(ok ? 'success' : 'error', $t('common.copiedToClipboard'));
  }

  async function onOpen(name: string): Promise<void> {
    try {
      await openRepoInShell(name);
    } catch (e) {
      pushToast('error', String(e));
    }
  }
</script>

<section class="space-y-4" aria-labelledby="repos-h">
  <header class="flex flex-wrap items-end justify-between gap-3">
    <h1 id="repos-h" class="text-2xl font-semibold">{$t('repos.heading')}</h1>
    <div class="flex items-center gap-2">
      <input
        class="input w-56"
        type="search"
        placeholder={$t('common.search')}
        bind:value={query}
        data-testid="repos-search"
      />
      <button class="btn-secondary" type="button" onclick={refreshRepos} disabled={loading}>
        {$t('common.refresh')}
      </button>
    </div>
  </header>

  <p class="text-xs text-slate-500" data-testid="repos-count">
    {$t('repos.countOne').replace('{n}', String(filtered.length))}
  </p>

  {#if filtered.length === 0}
    <div class="card text-center text-sm text-slate-500" data-testid="repos-empty">
      {$t('common.empty')}
    </div>
  {:else}
    <ul class="grid gap-3 sm:grid-cols-2 xl:grid-cols-3" data-testid="repos-grid">
      {#each filtered as repo (repo.name)}
        <li class="card space-y-2" data-testid={`repo-${repo.name}`}>
          <div class="flex items-baseline justify-between gap-2">
            <a class="text-lg font-semibold hover:underline" href={`/repos/${repo.name}`}>
              {repo.name}
            </a>
            <span class="pill">{repo.default_branch}</span>
          </div>
          <p class="text-xs text-slate-500 break-all" title={repo.path}>{repo.path}</p>
          <p class="text-xs text-slate-500">{formatBytes(repo.size_bytes)}</p>

          <div class="flex flex-wrap gap-2 pt-1">
            <a class="btn-secondary" href={`/repos/${repo.name}`}>{$t('repos.detail')}</a>
            <button class="btn-secondary" type="button" onclick={() => onCopy(repo.name)}>
              {$t('repos.copyCloneUrl')}
            </button>
            <button class="btn-secondary" type="button" onclick={() => onOpen(repo.name)}>
              {$t('repos.openInFinder')}
            </button>
          </div>
        </li>
      {/each}
    </ul>
  {/if}
</section>
