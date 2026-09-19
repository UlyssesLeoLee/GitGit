<!--
  Credential vault page. Surfaces the version-management surface
  (list / versions / diff / restore / rotate / delete) over the
  same FileVault that the gitgit CLI uses.
-->
<script lang="ts">
  import { onMount } from 'svelte';
  import {
    vaultSecrets,
    refreshVault,
    vaultVersions,
    vaultDiff,
    vaultRestore,
    vaultRotate,
    vaultDelete,
    vaultSet,
  } from '$lib/stores/vault';
  import { t } from '$lib/i18n';
  import { pushToast } from '$lib/stores/toasts';
  import { shortSha } from '$lib/utils/format';
  import type { VersionDiffDto, VersionEntryDto } from '$lib/api/types';

  let newKey = $state('');
  let newValue = $state('');
  let saving = $state(false);
  let openKey = $state<string | null>(null);
  let versionLists = $state<Record<string, VersionEntryDto[]>>({});
  let diffs = $state<Record<string, VersionDiffDto | null>>({});
  let diffKey = $state<string | null>(null);
  let baseV = $state<number>(1);
  let headV = $state<number>(2);

  onMount(async () => {
    await refreshVault();
  });

  async function onAdd(): Promise<void> {
    if (!newKey.trim() || !newValue.trim()) {
      pushToast('warn', 'key + value required');
      return;
    }
    saving = true;
    try {
      const ver = await vaultSet(newKey.trim(), newValue.trim());
      pushToast('success', $t('vault.secretCreated').replace('{n}', String(ver)));
      newKey = '';
      newValue = '';
    } finally {
      saving = false;
    }
  }

  async function onToggle(key: string): Promise<void> {
    if (openKey === key) {
      openKey = null;
      diffKey = null;
      return;
    }
    openKey = key;
    diffKey = null;
    const vs = await vaultVersions(key);
    versionLists = { ...versionLists, [key]: vs };
    if (vs.length >= 2) {
      baseV = vs[vs.length - 2].version;
      headV = vs[vs.length - 1].version;
    }
  }

  async function onDiff(key: string, b: number, h: number): Promise<void> {
    diffKey = key;
    const d = await vaultDiff(key, b, h);
    diffs = { ...diffs, [key]: d };
  }

  async function onRestore(key: string, target: number): Promise<void> {
    const ver = await vaultRestore(key, target);
    pushToast('success', `restored ${key} → v${ver}`);
    const vs = await vaultVersions(key);
    versionLists = { ...versionLists, [key]: vs };
  }

  async function onRotate(key: string): Promise<void> {
    const ver = await vaultRotate(key);
    pushToast('success', `rotated ${key} → v${ver}`);
    const vs = await vaultVersions(key);
    versionLists = { ...versionLists, [key]: vs };
  }

  async function onDelete(key: string): Promise<void> {
    if (!confirm($t('vault.confirmDelete').replace('{key}', key))) return;
    await vaultDelete(key);
    pushToast('info', `deleted ${key}`);
  }
</script>

<section class="space-y-6" aria-labelledby="vault-h">
  <header>
    <h1 id="vault-h" class="text-2xl font-semibold">{$t('vault.heading')}</h1>
    <p class="mt-1 text-xs text-slate-500">{$t('vault.subhead')}</p>
  </header>

  <div class="card">
    <h2 class="mb-2 text-sm font-semibold">{$t('vault.add')}</h2>
    <div class="grid gap-2 sm:grid-cols-2">
      <div>
        <label class="label" for="vault-key">{$t('vault.keyLabel')}</label>
        <input id="vault-key" class="input" type="text" bind:value={newKey} />
      </div>
      <div>
        <label class="label" for="vault-val">{$t('vault.valueLabel')}</label>
        <input id="vault-val" class="input" type="password" bind:value={newValue} />
      </div>
    </div>
    <button class="btn-primary mt-3" type="button" disabled={saving} onclick={onAdd}>
      {$t('common.save')}
    </button>
  </div>

  {#if $vaultSecrets.length === 0}
    <div class="card text-center text-sm text-slate-500">{$t('vault.empty')}</div>
  {:else}
    <div class="card overflow-hidden p-0">
      <table class="w-full table-auto text-sm">
        <thead class="bg-slate-100 dark:bg-slate-700">
          <tr>
            <th class="px-3 py-2 text-left text-xs font-medium uppercase">{$t('vault.keyLabel')}</th>
            <th class="px-3 py-2 text-left text-xs font-medium uppercase">{$t('vault.versions')}</th>
            <th class="px-3 py-2 text-right text-xs font-medium uppercase">{$t('vault.actions')}</th>
          </tr>
        </thead>
        <tbody>
          {#each $vaultSecrets as s (s.key)}
            <tr class="border-t border-slate-200 dark:border-slate-700">
              <td class="px-3 py-2 align-top font-mono">{s.key}</td>
              <td class="px-3 py-2 align-top">
                {#if openKey === s.key && versionLists[s.key]}
                  <div class="space-y-1 text-xs">
                    {#each versionLists[s.key] as v (v.version)}
                      <div class="flex items-center gap-2">
                        <span class="font-mono">v{v.version}</span>
                        <span class="text-slate-500">{shortSha(v.bytes_sha256, 12)}</span>
                        <span class="text-slate-500">{v.byte_len} B</span>
                        <button
                          class="btn-secondary px-1 py-0.5 text-xs"
                          type="button"
                          onclick={() => onDiff(s.key, baseV, v.version)}>↔</button>
                        <button
                          class="btn-secondary px-1 py-0.5 text-xs"
                          type="button"
                          onclick={() => onRestore(s.key, v.version)}>↺</button>
                      </div>
                    {/each}
                    {#if diffKey === s.key && diffs[s.key]}
                      {@const d = diffs[s.key]}
                      {#if d}
                        <div class="mt-2 rounded bg-slate-100 p-2 text-xs dark:bg-slate-900" data-testid={`diff-${s.key}`}>
                          {$t('vault.diffTitle').replace('{base}', String(d.base_version)).replace('{head}', String(d.head_version))}
                          · {$t('vault.sizeDelta')} {d.file_size_delta}
                        </div>
                      {/if}
                    {/if}
                  </div>
                {:else}
                  <span class="text-xs text-slate-500">v{s.version}</span>
                {/if}
              </td>
              <td class="px-3 py-2 text-right align-top">
                <div class="flex flex-wrap justify-end gap-1">
                  <button class="btn-secondary" type="button" onclick={() => onToggle(s.key)}>
                    {openKey === s.key ? $t('common.close') : $t('vault.versions')}
                  </button>
                  <button class="btn-secondary" type="button" onclick={() => onRotate(s.key)}>
                    {$t('vault.rotate')}
                  </button>
                  <button class="btn-danger" type="button" onclick={() => onDelete(s.key)}>
                    {$t('common.delete')}
                  </button>
                </div>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}
</section>
