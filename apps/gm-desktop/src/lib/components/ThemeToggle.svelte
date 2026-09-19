<!--
  ThemeToggle. Three-button segmented control bound to the
  `theme` store.
-->
<script lang="ts">
  import { theme, setTheme, type ThemeMode } from '$lib/stores/theme';
  import { t } from '$lib/i18n';

  const modes: ReadonlyArray<{ id: ThemeMode; labelKey: string }> = [
    { id: 'light', labelKey: 'settings.themeLight' },
    { id: 'auto', labelKey: 'settings.themeAuto' },
    { id: 'dark', labelKey: 'settings.themeDark' },
  ];
</script>

<fieldset class="flex items-center gap-1" data-testid="theme-toggle">
  <legend class="sr-only">{$t('settings.theme')}</legend>
  {#each modes as mode (mode.id)}
    <button
      type="button"
      class="rounded-md px-2 py-1 text-xs font-medium transition-colors"
      class:bg-accent-500={$theme === mode.id}
      class:text-white={$theme === mode.id}
      class:bg-slate-200={$theme !== mode.id}
      class:dark:bg-slate-700={$theme !== mode.id}
      class:hover:bg-slate-300={$theme !== mode.id}
      class:dark:hover:bg-slate-600={$theme !== mode.id}
      onclick={() => setTheme(mode.id)}
      aria-pressed={$theme === mode.id}
      data-testid={`theme-${mode.id}`}
    >
      {$t(mode.labelKey)}
    </button>
  {/each}
</fieldset>
