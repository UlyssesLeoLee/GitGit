<!--
  Locale switcher. Two-button toggle bound to the `locale` store.
-->
<script lang="ts">
  import { locale, setLocale } from '$lib/stores/locale';

  const choices: ReadonlyArray<{ id: 'zh-CN' | 'en'; label: string }> = [
    { id: 'zh-CN', label: '中文' },
    { id: 'en', label: 'English' },
  ];
</script>

<fieldset class="flex items-center gap-1" data-testid="locale-switcher">
  <legend class="sr-only">Language</legend>
  {#each choices as c (c.id)}
    <button
      type="button"
      class="rounded-md px-2 py-1 text-xs font-medium transition-colors"
      class:bg-accent-500={$locale === c.id}
      class:text-white={$locale === c.id}
      class:bg-slate-200={$locale !== c.id}
      class:dark:bg-slate-700={$locale !== c.id}
      onclick={() => setLocale(c.id)}
      aria-pressed={$locale === c.id}
    >
      {c.label}
    </button>
  {/each}
</fieldset>
