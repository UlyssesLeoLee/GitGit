<!--
  ThemeToggle. Three-button segmented control bound to the
  `theme` store.
-->
<script lang="ts">
  import { theme, setTheme, type ThemeMode } from '$lib/stores/theme';
  import { t } from '$lib/i18n';
  import { catalog } from '$lib/i18n';

  const modes: ReadonlyArray<{ id: ThemeMode; labelKey: string }> = [
    { id: 'light', labelKey: 'settings.themeLight' },
    { id: 'auto', labelKey: 'settings.themeAuto' },
    { id: 'dark', labelKey: 'settings.themeDark' },
  ];
</script>

<fieldset class="flex items-center gap-1 p-1 rounded-xl border border-white/20 bg-white/10 dark:border-white/10 dark:bg-black/20 backdrop-blur-xl shadow-inner" data-testid="theme-toggle">
  <legend class="sr-only">{$catalog['settings.theme']}</legend>
  {#each modes as mode (mode.id)}
    <button
      type="button"
      class="rounded-lg px-2.5 py-1 text-xs font-medium transition-all duration-200 border border-transparent {$theme === mode.id ? 'shadow-md border-white/20' : ''}"
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
      {$catalog[mode.labelKey]}
    </button>
  {/each}
</fieldset>
