<!--
  Working-tree status and diff view for one repository (V0 task T4).

  Three things it is careful about, each of which was a way to be
  quietly wrong:

  1. "Clean" is a real answer. A repository with no changed paths renders
     a clean-tree message, not an empty box and not an error.
  2. Staged / not staged / untracked are three different questions, so
     they are three groups, and the diff target is a choice the user
     makes rather than one the view infers.
  3. Untracked files are listed and readable. `git diff` reports nothing
     for a file git has never seen, so the backend synthesizes a
     new-file diff; the view labels that so the user is not misled into
     thinking git produced it.

  All user-visible strings come from the catalogue.
-->
<script lang="ts">
  import { onMount } from 'svelte';
  import {
    worktree,
    diff,
    loadStatus,
    loadDiff,
    resetWorktree,
    stagedEntries,
    unstagedEntries,
    untrackedEntries,
  } from '$lib/stores/worktree';
  import { catalog } from '$lib/i18n';
  import { pushToast } from '$lib/stores/toasts';
  import { copyText } from '$lib/utils/clipboard';
  import type { DiffTarget, StatusEntry } from '$lib/api/types';

  interface Props {
    /** Repository name, as listed by `list_repos`. */
    name: string;
    /** Directory the name resolves under. Defaults to the app's repos dir. */
    root?: string | null;
  }
  let { name, root = null }: Props = $props();

  let target = $state<DiffTarget>('worktree');
  let selected = $state<string | null>(null);
  let copied = $state(false);

  const TARGETS: DiffTarget[] = ['staged', 'worktree', 'head'];

  onMount(() => {
    resetWorktree();
    void loadStatus(name, root);
    // The initial diff is fetched too, not left for the user to ask for.
    // Rendering "no changes for this comparison" before any request has
    // run would be a claim the page has not earned.
    void loadDiff(name, target, null, root);
  });

  async function refresh(): Promise<void> {
    await loadStatus(name, root);
    await loadDiff(name, target, selected, root);
  }

  async function chooseTarget(next: DiffTarget): Promise<void> {
    target = next;
    // A path scoped to one comparison is kept: the user is narrowing the
    // same question, not asking a new one.
    await loadDiff(name, target, selected, root);
  }

  async function openEntry(entry: StatusEntry): Promise<void> {
    selected = entry.path;
    // An untracked file is scoped to its own synthesized diff, so the
    // target does not change the answer; pick the one that shows edits
    // for a tracked file.
    if (!entry.untracked && entry.staged && !entry.unstaged) {
      target = 'staged';
    } else if (entry.untracked) {
      target = 'worktree';
    }
    await loadDiff(name, target, selected, root);
  }

  async function viewAll(): Promise<void> {
    selected = null;
    await loadDiff(name, target, null, root);
  }

  async function copyDiff(): Promise<void> {
    if (!$diff.text) return;
    const ok = await copyText($diff.text);
    copied = ok;
    // Same wording as the other copy buttons in the app: the kind
    // carries the outcome, the message does not.
    pushToast(ok ? 'success' : 'error', $catalog['common.copiedToClipboard']);
    if (ok) setTimeout(() => (copied = false), 1500);
  }

  /** Localized headline for a typed `AppError` kind, with a real remedy. */
  function errorHeadline(kind: string | null): string {
    if (kind === 'NotAWorkTree') return $catalog['repos.error.notWorkTree'];
    if (kind === 'InvalidRepoName') return $catalog['repos.error.invalidName'];
    if (kind === 'InvalidDiffTarget') return $catalog['repos.error.invalidTarget'];
    return $catalog['repos.error.generic'];
  }

  function targetLabel(t: DiffTarget): string {
    if (t === 'staged') return $catalog['repos.status.staged'];
    if (t === 'head') return $catalog['repos.diff.viewAll'];
    return $catalog['repos.status.unstaged'];
  }
</script>

<section class="space-y-4" aria-labelledby="wt-h" data-testid="repo-worktree">
  <h2 id="wt-h" class="text-sm font-semibold">{$catalog['repos.statusHeading']}</h2>

  {#if $worktree.status === 'loading' || $worktree.status === 'idle'}
    <p class="card text-sm text-slate-500" data-testid="wt-loading">
      {$catalog['repos.statusLoading']}
    </p>
  {:else if $worktree.status === 'error'}
    <div
      class="card border-amber-300 bg-amber-50 dark:border-amber-800 dark:bg-amber-900/30"
      data-testid="wt-error"
    >
      <p class="text-sm font-medium text-amber-800 dark:text-amber-200">
        {$catalog['repos.error.heading']}
      </p>
      <p class="mt-1 text-sm text-amber-700 dark:text-amber-100" data-testid="wt-error-detail">
        {errorHeadline($worktree.errorKind)}
      </p>
      {#if $worktree.errorMessage}
        <!-- Raw git detail, shown small: it can name a host path the user
             did not intend to surface, so it is never the headline. -->
        <p class="mt-1 break-all font-mono text-[11px] text-amber-700/80 dark:text-amber-200/70">
          {$worktree.errorMessage}
        </p>
      {/if}
      <button class="btn-secondary mt-2" type="button" onclick={refresh} data-testid="wt-retry">
        {$catalog['repos.error.retry']}
      </button>
    </div>
  {:else if $worktree.status === 'clean'}
    <p class="card text-sm text-emerald-700 dark:text-emerald-300" data-testid="wt-clean">
      {$catalog['repos.statusClean']}
    </p>
  {:else}
    <div class="card space-y-3" data-testid="wt-dirty">
      <div class="flex flex-wrap items-center gap-x-3 gap-y-1 text-xs text-slate-500">
        {#if $worktree.branch}
          <span data-testid="wt-branch"
            >{$catalog['repos.status.branch'].replace('{branch}', $worktree.branch)}</span
          >
        {:else}
          <span>{$catalog['repos.status.detached']}</span>
        {/if}
        {#if $worktree.upstream}
          <span data-testid="wt-upstream">
            {$catalog['repos.status.aheadBehind']
              .replace('{ahead}', String($worktree.ahead))
              .replace('{behind}', String($worktree.behind))
              .replace('{upstream}', $worktree.upstream)}
          </span>
        {/if}
        <span data-testid="wt-count">
          {$catalog['repos.statusDirty'].replace('{n}', String($worktree.entries.length))}
        </span>
      </div>

      <div class="grid gap-3 md:grid-cols-3">
        {#each [{ id: 'staged', label: $catalog['repos.status.staged'], rows: stagedEntries($worktree.entries) }, { id: 'unstaged', label: $catalog['repos.status.unstaged'], rows: unstagedEntries($worktree.entries) }, { id: 'untracked', label: $catalog['repos.status.untracked'], rows: untrackedEntries($worktree.entries) }] as group (group.id)}
          <div data-testid="wt-group-{group.id}">
            <h3 class="label">{group.label} ({group.rows.length})</h3>
            {#if group.rows.length === 0}
              <p class="text-xs text-slate-500">{$catalog['repos.status.none']}</p>
            {:else}
              <ul class="space-y-1 text-xs font-mono">
                {#each group.rows as e (e.path)}
                  <li>
                    <button
                      type="button"
                      class="text-left hover:underline"
                      data-testid="wt-entry"
                      onclick={() => openEntry(e)}
                    >
                      <span class="text-slate-400">
                        {e.index_status ?? e.worktree_status ?? '?'}
                      </span>
                      <span>{e.path}</span>
                    </button>
                    {#if e.orig_path}
                      <span class="block pl-4 text-[11px] text-slate-500">
                        {$catalog['repos.status.renamedFrom'].replace('{path}', e.orig_path)}
                      </span>
                    {/if}
                  </li>
                {/each}
              </ul>
            {/if}
          </div>
        {/each}
      </div>
    </div>
  {/if}

  <div class="card" data-testid="wt-diff">
    <div class="mb-2 flex flex-wrap items-center justify-between gap-2">
      <div class="flex flex-wrap items-center gap-1">
        <h3 class="mr-2 text-sm font-semibold">{$catalog['repos.diffHeading']}</h3>
        {#each TARGETS as t (t)}
          <button
            type="button"
            class="btn-secondary text-xs"
            data-testid="wt-target-{t}"
            aria-pressed={target === t}
            class:ring-2={target === t}
            onclick={() => chooseTarget(t)}
          >
            {targetLabel(t)}
          </button>
        {/each}
      </div>
      <div class="flex items-center gap-2">
        {#if selected}
          <button class="btn-secondary text-xs" type="button" onclick={viewAll} data-testid="wt-view-all">
            {$catalog['repos.diff.viewAll']}
          </button>
        {/if}
        {#if $diff.text}
          <button class="btn-secondary text-xs" type="button" onclick={copyDiff} data-testid="wt-copy">
            {copied ? $catalog['common.copiedToClipboard'] : $catalog['common.copy']}
          </button>
        {/if}
      </div>
    </div>

    {#if $diff.status === 'loading'}
      <p class="text-xs text-slate-500" data-testid="wt-diff-loading">
        {$catalog['common.loading']}
      </p>
    {:else if $diff.status === 'error'}
      <div class="text-sm text-red-700 dark:text-red-300" data-testid="wt-diff-error">
        <p>{errorHeadline($diff.errorKind)}</p>
        {#if $diff.errorMessage}
          <p class="mt-1 break-all font-mono text-[11px] opacity-80">{$diff.errorMessage}</p>
        {/if}
      </div>
    {:else if $diff.status === 'empty' || $diff.status === 'idle'}
      <p class="text-xs text-slate-500" data-testid="wt-diff-empty">
        {#if selected}{$catalog['repos.diff.pickFile']}{:else}{$catalog['repos.diff.empty']}{/if}
      </p>
    {:else}
      <div class="mb-1 flex flex-wrap gap-2 text-[11px] text-slate-500">
        <span data-testid="wt-diff-files">
          {$catalog['repos.diff.fileCount'].replace('{n}', String($diff.files))}
        </span>
        {#if $diff.truncated}
          <span class="text-amber-600" data-testid="wt-diff-truncated">
            {$catalog['repos.diff.truncated']}
          </span>
        {/if}
        {#if $diff.untracked}
          <span class="text-amber-600" data-testid="wt-diff-untracked">
            {$catalog['repos.diff.untracked']}
          </span>
        {/if}
      </div>
      <!-- Diff text is untrusted input from a subprocess. Svelte escapes
           the interpolation and there is no {@html} here, so a `+` line
           containing markup renders as characters. -->
      <pre class="overflow-auto rounded-md bg-slate-50 p-2 text-xs dark:bg-slate-900" data-testid="wt-diff-text">{$diff.text}</pre>
    {/if}
  </div>
</section>
