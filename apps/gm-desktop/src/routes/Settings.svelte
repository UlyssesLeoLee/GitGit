<!--
  Settings page. Composes the four setting families required by the
  brief: theme, locale, admin password, about/data dir.
-->
<script lang="ts">
  import { onMount } from 'svelte';
  import * as tauri from '$lib/api/tauri';
  import { t } from '$lib/i18n';
  import { pushToast } from '$lib/stores/toasts';
  import { theme, type ThemeMode } from '$lib/stores/theme';
  import { locale, type LocaleId } from '$lib/stores/locale';
  import { setTheme } from '$lib/stores/theme';
  import { setLocale } from '$lib/stores/locale';
  import ThemeToggle from '$lib/components/ThemeToggle.svelte';
  import LocaleSwitcher from '$lib/components/LocaleSwitcher.svelte';
  import WorktreeRootPicker from '$lib/components/WorktreeRootPicker.svelte';
  import type { AdminPasswordStatus, AppInfo } from '$lib/api/types';
  import { catalog } from '$lib/i18n';

  let info = $state<AppInfo | null>(null);
  let pwStatus = $state<AdminPasswordStatus | null>(null);
  let newPassword = $state('');
  let updating = $state(false);
  let updatesOnStartup = $state(false);

  onMount(async () => {
    try {
      info = await tauri.appInfo();
      pwStatus = await tauri.getAdminPasswordStatus();
    } catch (_err) {
      // mock fallback covered by installMock
    }
  });

  async function refreshAdmin(): Promise<void> {
    pwStatus = await tauri.getAdminPasswordStatus();
  }

  async function onSavePassword(): Promise<void> {
    if (!newPassword) {
      pushToast('warn', 'empty password');
      return;
    }
    updating = true;
    try {
      await tauri.setAdminPassword(newPassword);
      newPassword = '';
      await refreshAdmin();
      // `[FACT]` The server resolves the admin credential once, when it
      // starts, and caches it. This toast used to say just "updated",
      // which is what made the page actively misleading: a user saved a
      // password, saw a success toast and a "Set (length N)" line, and
      // the running server went on accepting the old one. The message
      // now states the one true thing about when it takes effect.
      pushToast('success', 'saved — the server uses it from its next start');
    } finally {
      updating = false;
    }
  }

  async function onClearPassword(): Promise<void> {
    await tauri.clearAdminPassword();
    await refreshAdmin();
    // Clearing removes the stored password, so the next start has
    // nothing configured and generates a fresh random one.
    pushToast('info', 'cleared — a new password is generated at the next start');
  }
</script>

<section class="space-y-6" aria-labelledby="settings-h">
  <h1 id="settings-h" class="text-2xl font-semibold">{$catalog['settings.heading']}</h1>

  <div class="grid gap-6 lg:grid-cols-2">
    <div class="card">
      <h2 class="mb-2 text-sm font-semibold">{$catalog['settings.theme']}</h2>
      <ThemeToggle />
    </div>

    <div class="card">
      <h2 class="mb-2 text-sm font-semibold">{$catalog['settings.locale']}</h2>
      <LocaleSwitcher />
    </div>

    <div class="card">
      <h2 class="mb-2 text-sm font-semibold">{$catalog['settings.adminPassword']}</h2>
      {#if pwStatus}
        <p class="mb-2 text-xs text-slate-500" data-testid="admin-status">
          {#if pwStatus.is_set}
            {$catalog['settings.adminPasswordSet'].replace('{n}', String(pwStatus.length))}
          {:else}
            {$catalog['settings.adminPasswordNotSet']}
          {/if}
        </p>
      {/if}
      <label class="label" for="new-pw">{$catalog['settings.adminPasswordUpdate']}</label>
      <input
        id="new-pw"
        class="input"
        type="password"
        bind:value={newPassword}
        autocomplete="off"
      />
      <div class="mt-3 flex gap-2">
        <button class="btn-primary" type="button" disabled={updating} onclick={onSavePassword}>
          {$catalog['common.save']}
        </button>
        <button class="btn-secondary" type="button" onclick={onClearPassword}>
          {$catalog['settings.adminPasswordClear']}
        </button>
      </div>
    </div>

    <div class="card">
      <!-- The persistent home for the working-tree root. The same
           control also appears in the working-tree error panel, which
           is where a user meets the problem in the first place. -->
      <h2 class="mb-2 text-sm font-semibold">{$catalog['repos.root.label']}</h2>
      <WorktreeRootPicker />
    </div>

    <div class="card">
      <h2 class="mb-2 text-sm font-semibold">{$catalog['settings.updates.title']}</h2>
      <label class="flex items-center gap-2 text-sm">
        <input type="checkbox" bind:checked={updatesOnStartup} />
        {$catalog['settings.updates.checkOnStartup']}
      </label>
      <p class="mt-1 text-xs text-slate-500">{$catalog['settings.updates.placeholderNote']}</p>
    </div>
  </div>

  <div class="card">
    <h2 class="mb-2 text-sm font-semibold">{$catalog['settings.about']}</h2>
    {#if info}
      <dl class="grid grid-cols-2 gap-2 text-xs">
        <dt class="label">{$catalog['common.appName']}</dt>
        <dd class="font-mono">{info.name}</dd>
        <dt class="label">{$catalog['common.version']}</dt>
        <dd class="font-mono">{info.version}</dd>
        <dt class="label">{$catalog['settings.dataDir']}</dt>
        <dd class="break-all font-mono">{info.data_dir}</dd>
        <dt class="label">repos</dt>
        <dd class="break-all font-mono">{info.repos_dir}</dd>
        <dt class="label">vault</dt>
        <dd class="break-all font-mono">{info.vault_dir}</dd>
      </dl>
    {/if}
  </div>
</section>
