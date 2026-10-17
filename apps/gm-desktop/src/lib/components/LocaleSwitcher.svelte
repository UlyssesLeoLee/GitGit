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

<fieldset class="flex items-center gap-1 p-1 rounded-xl border border-white/20 bg-white/10 dark:border-white/10 dark:bg-black/20 backdrop-blur-xl shadow-inner" data-testid="locale-switcher">
  <legend class="sr-only">Language</legend>
  {#each choices as c (c.id)}
    <button
      type="button"
      class="flex-1 rounded-lg px-2.5 py-1 text-xs font-medium transition-all duration-200 border border-transparent {$locale === c.id ? 'shadow-md border-white/20' : ''}"
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
