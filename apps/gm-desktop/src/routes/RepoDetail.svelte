<!--
  Repository detail page. Fetches refs + commits on mount and
  renders a compact branch graph (refs grouped by branch family).
-->
<script lang="ts">
  import { onMount } from 'svelte';
  import { repoDetail } from '$lib/stores/repos';
  import { t } from '$lib/i18n';
  import { copyText } from '$lib/utils/clipboard';
  import { pushToast } from '$lib/stores/toasts';
  import { shortSha } from '$lib/utils/format';
  import type { RepoDetail, RefEntry } from '$lib/api/types';

  interface Props { params?: { name?: string } }
  let { params }: Props = $props();

  let detail = $state<RepoDetail | null>(null);
  let loading = $state(true);
  let notFound = $state(false);

  onMount(async () => {
    const name = String(params?.name ?? '');
    if (!name) {
      notFound = true;
      loading = false;
      return;
    }
    const d = await repoDetail(name);
    if (!d) {
      notFound = true;
    } else {
      detail = d;
    }
    loading = false;
  });

  async function copyCloneUrl(): Promise<void> {
    if (!detail) return;
    const url = `http://127.0.0.1:38080/repos/${detail.name}.git`;
    const ok = await copyText(url);
    pushToast(ok ? 'success' : 'error', $t('common.copiedToClipboard'));
  }

  function filterLocal(refs: RefEntry[]): RefEntry[] {
    return refs.filter((r) => r.kind === 'local');
  }
  function filterRemote(refs: RefEntry[]): RefEntry[] {
    return refs.filter((r) => r.kind === 'remote');
  }
  function filterTags(refs: RefEntry[]): RefEntry[] {
    return refs.filter((r) => r.kind === 'tag');
  }
</script>

<section class="space-y-4" aria-labelledby="detail-h">
  <header class="flex flex-wrap items-end justify-between gap-3">
    <div>
      <a class="text-xs text-slate-500 hover:underline" href="/repos">← {$t('repos.back')}</a>
      <h1 id="detail-h" class="text-2xl font-semibold">
        {#if detail}{detail.name}{:else}{$t('common.loading')}{/if}
      </h1>
    </div>
    <button class="btn-secondary" type="button" onclick={copyCloneUrl}>
      {$t('repos.copyCloneUrl')}
    </button>
  </header>

  {#if loading}
    <div class="card text-center text-sm text-slate-500">{$t('common.loading')}</div>
  {:else if notFound}
    <div class="card border-red-300 bg-red-50 dark:border-red-800 dark:bg-red-900/30">
      <p class="text-sm text-red-700 dark:text-red-200">
        {$t('repos.notFound').replace('{name}', params?.name ?? '')}
      </p>
    </div>
  {:else if detail}
    <div class="grid gap-6 lg:grid-cols-2">
      <div class="card">
        <h2 class="mb-2 text-sm font-semibold">{$t('repos.refsHeading')}</h2>
        {#if detail.refs.length === 0}
          <p class="text-xs text-slate-500">{$t('repos.refs.none')}</p>
        {:else}
          <h3 class="label mt-2">local</h3>
          <ul class="space-y-1 text-xs font-mono">
            {#each filterLocal(detail.refs) as r (r.sha + r.name)}
              <li><span class="text-accent-600">{shortSha(r.sha, 10)}</span> {r.name}</li>
            {/each}
            {#if filterLocal(detail.refs).length === 0}
              <li class="text-slate-500">—</li>
            {/if}
          </ul>

          <h3 class="label mt-3">remote</h3>
          <ul class="space-y-1 text-xs font-mono">
            {#each filterRemote(detail.refs) as r (r.sha + r.name)}
              <li><span class="text-slate-500">{shortSha(r.sha, 10)}</span> {r.name}</li>
            {/each}
            {#if filterRemote(detail.refs).length === 0}
              <li class="text-slate-500">—</li>
            {/if}
          </ul>

          <h3 class="label mt-3">tag</h3>
          <ul class="space-y-1 text-xs font-mono">
            {#each filterTags(detail.refs) as r (r.sha + r.name)}
              <li><span class="text-amber-600">{shortSha(r.sha, 10)}</span> {r.name}</li>
            {/each}
            {#if filterTags(detail.refs).length === 0}
              <li class="text-slate-500">—</li>
            {/if}
          </ul>
        {/if}

        <details class="mt-4 text-xs text-slate-500">
          <summary class="cursor-pointer">{$t('repos.branchGraph')}</summary>
          <pre class="mt-2 overflow-auto rounded-md bg-slate-50 p-2 dark:bg-slate-900">
{[
  ...detail.refs.filter((r) => r.kind === 'local').map((r) => `  ${shortSha(r.sha, 7)} ${r.name}`),
  ...detail.refs.filter((r) => r.kind === 'remote').map((r) => `~ ${shortSha(r.sha, 7)} ${r.name}`),
  ...detail.refs.filter((r) => r.kind === 'tag').map((r) => `* ${shortSha(r.sha, 7)} ${r.name}`),
].join('\n')}
          </pre>
        </details>
      </div>

      <div class="card">
        <h2 class="mb-2 text-sm font-semibold">{$t('repos.commitsHeading')}</h2>
        {#if detail.commits.length === 0}
          <p class="text-xs text-slate-500">{$t('repos.commits.none')}</p>
        {:else}
          <ol class="space-y-3 text-sm">
            {#each detail.commits as c (c.sha)}
              <li class="border-l-2 border-accent-500 pl-3">
                <div class="font-mono text-xs text-accent-700">{shortSha(c.sha, 12)}</div>
                <div class="text-sm">{c.message}</div>
                <div class="text-xs text-slate-500">{c.author} · {c.date_iso}</div>
              </li>
            {/each}
          </ol>
        {/if}
      </div>
    </div>
  {/if}
</section>
