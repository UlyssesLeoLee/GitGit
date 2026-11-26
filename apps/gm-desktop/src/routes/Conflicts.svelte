<!--
  Conflict arbitration route (ADR-0024 §3.5).

  `[FACT]` Why a route and not a modal: the same reason `/review` is a
  page and not a dialog — the V0 desktop shell has no merge action to
  anchor a dialog to, and the content is what the user came for. The
  route is the container; `ConflictDialog.svelte` is the panel a dialog
  would host.

  `[FACT]` The route table is one object in `App.svelte`, and the router
  is the local hash router in `$lib/router`, so reaching this page is a
  single entry in that table.

  `[FACT]` ADR §3.7.4: a file can carry several independent
  regions, so the region list stays visible while the dialog shows one
  of them.
-->
<script lang="ts">
  import { onMount } from 'svelte';
  import { repos, refreshRepos } from '$lib/stores/repos';
  import {
    applyDecisions,
    canApply,
    conflict,
    loadRegions,
    selectRegion,
  } from '$lib/stores/conflict';
  import ConflictDialog from '$lib/components/ConflictDialog.svelte';
  import { currentRoot } from '$lib/stores/worktreeRoot';
  import { catalog } from '$lib/i18n';

  let repo = $state('');
  let loading = $state(false);

  const selected = $derived(
    $conflict.regions.find((r) => r.id === $conflict.selectedRegionId) ?? null
  );

  async function onRepoChange(): Promise<void> {
    if (repo === '') return;
    loading = true;
    // `[FACT]` The working-tree root travels with the name: the desktop
    // keeps its repositories as bare `<name>.git` directories, and the
    // checkouts live under the root the user picked.
    await loadRegions(repo, currentRoot());
    loading = false;
  }

  async function onRefresh(): Promise<void> {
    await refreshRepos();
    if (repo !== '') await onRepoChange();
  }

  /**
   * Write the decided regions back into the work tree and stage them
   * (ADR-0024 §3.6).
   *
   * `[FACT]` The application does not drive the merge itself: it writes
   * the resolved file back and `git add`s the path, and the user commits.
   *
   * `[FACT]` The button is disabled until every arbitrable region has a
   * decision, and the store refuses as well — so a half-decided set can
   * never reach the work tree.
   */
  async function onApply(): Promise<void> {
    if (!canApply($conflict)) return;
    await applyDecisions(repo, currentRoot());
  }

  onMount(async () => {
    await refreshRepos();
    const names = $repos.map((r) => r.name);
    if (names.length > 0) {
      repo = names[0];
      await onRepoChange();
    }
  });
</script>

<section class="space-y-4" aria-labelledby="conflicts-h">
  <header>
    <h1 id="conflicts-h" class="text-2xl font-semibold" data-testid="conflicts-heading">
      {$catalog['conflict.heading']}
    </h1>
    <p class="mt-1 text-sm text-slate-500" data-testid="conflicts-subhead">
      {$catalog['conflict.subhead']}
    </p>
  </header>

  <div class="card flex flex-wrap items-end gap-3">
    <div>
      <label class="block text-sm font-medium" for="conflict-repo">
        {$catalog['conflict.repoLabel']}
      </label>
      <select
        id="conflict-repo"
        class="mt-1 rounded-md border border-slate-300 px-2 py-1 text-sm dark:border-slate-600 dark:bg-slate-900"
        bind:value={repo}
        onchange={onRepoChange}
        data-testid="conflict-repo"
      >
        {#each $repos as r (r.name)}
          <option value={r.name}>{r.name}</option>
        {/each}
      </select>
    </div>
    <button type="button" class="btn-secondary" onclick={onRefresh} disabled={loading}>
      {$catalog['common.refresh']}
    </button>
  </div>

  {#if $conflict.status === 'error' && $conflict.errorMessage !== null && $conflict.regions.length === 0}
    <div class="card border-l-4 border-l-red-500 text-sm" data-testid="conflicts-error">
      <p class="font-medium">{$catalog['conflict.loadFailed']}</p>
      <p class="mt-1 break-words font-mono text-xs text-slate-500">
        {$conflict.errorMessage}
      </p>
    </div>
  {:else if $conflict.regions.length === 0}
    <div class="card text-sm text-slate-500" data-testid="conflicts-empty">
      {$catalog['conflict.noRegions']}
    </div>
  {:else}
    <!-- `[FACT]` ADR §3.7.4: the region list stays visible — a file
         can carry several independent regions, and the user has to know
         which one the dialog is about. `[FACT]` Each button says whether
         its region has been decided: a path is only written back when
         every one of its regions has a decision (ADR §3.6). -->
    <ul class="flex flex-wrap gap-2" data-testid="conflict-regions">
      {#each $conflict.regions as region (region.id)}
        {@const decided = $conflict.decisions[region.id] !== undefined}
        <li>
          <button
            type="button"
            class="btn-secondary text-xs"
            class:ring-2={region.id === $conflict.selectedRegionId}
            onclick={() => selectRegion(region.id)}
            data-testid={`conflict-region-${region.index}`}
            data-decided={decided}
          >
            {region.path} · {$catalog['conflict.regionIndex'].replace('{n}', String(region.index + 1))}
            <span class="ml-1 text-[10px] uppercase" data-testid={`conflict-region-state-${region.index}`}>
              {decided ? $catalog['conflict.decided'] : $catalog['conflict.undecided']}
            </span>
          </button>
        </li>
      {/each}
    </ul>

    <!-- The write-back (ADR-0024 §3.6). -->
    <div class="card flex flex-wrap items-center gap-3" data-testid="conflict-apply-panel">
      <button
        type="button"
        class="btn-primary"
        data-testid="conflict-apply"
        disabled={!canApply($conflict) || $conflict.applying}
        onclick={onApply}
      >
        {$catalog['conflict.apply']}
      </button>
      {#if !canApply($conflict)}
        <p class="text-sm text-slate-500" data-testid="conflict-apply-hint">
          {$catalog['conflict.applyHint']}
        </p>
      {/if}
      <!-- `[FACT]` A failure is shown to the user, not only logged: the
           write-back is the step that changes the work tree, so the user
           has to know whether it happened. -->
      {#if $conflict.applyError !== null}
        <p class="text-sm text-red-600" data-testid="conflicts-apply-error">{$conflict.applyError}</p>
      {/if}
      {#if $conflict.applyOutcome !== null && $conflict.applyOutcome.written.length > 0}
        <p class="text-sm text-green-700" data-testid="conflicts-apply-written">
          {$catalog['conflict.applyWritten'].replace('{paths}', $conflict.applyOutcome.written.join(', '))}
        </p>
      {/if}
      {#if $conflict.applyOutcome !== null && $conflict.applyOutcome.skipped.length > 0}
        <p class="text-sm text-amber-700" data-testid="conflicts-apply-skipped">
          {$catalog['conflict.applySkipped'].replace('{paths}', $conflict.applyOutcome.skipped.join(', '))}
        </p>
      {/if}
    </div>

    {#if selected}
      {#key selected.id}
        <ConflictDialog region={selected} />
      {/key}
    {/if}
  {/if}

  <!-- `[FACT]` ADR §3.7.4: a path the core refused to read is
       reported, not dropped. An add/delete or rename conflict has no base
       stage, and binary content is not text — git has to handle those, and
       the user has to be told. It is shown next to the regions, not only
       when there are none, because a repository can have both. -->
  {#if $conflict.needsGit.length > 0}
    <div class="card border-l-4 border-l-amber-500 text-sm" data-testid="conflicts-needs-git">
      <p class="font-medium">{$catalog['conflict.needsGitHeading']}</p>
      <ul class="mt-1 list-disc pl-5 font-mono text-xs text-slate-600 dark:text-slate-300">
        {#each $conflict.needsGit as path (path)}
          <li>{path}</li>
        {/each}
      </ul>
    </div>
  {/if}
</section>
