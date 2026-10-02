<!--
  Sidebar nav. Routing is hash-based: the router listens to
  `hashchange`, so anchors must carry the `#` prefix (see `href()` in
  `$lib/router`). The active link is computed from `location` plus a
  per-link prefix check, so `/repos/foo` keeps "Repositories" lit.
-->
<script lang="ts">
  import { location, href } from '$lib/router';
  import { t } from '$lib/i18n';
  import { derived } from 'svelte/store';
  import ThemeToggle from './ThemeToggle.svelte';
  import LocaleSwitcher from './LocaleSwitcher.svelte';
  import { catalog } from '$lib/i18n';

  type NavItem = { href: string; labelKey: string; icon: string };

  const items: NavItem[] = [
    { href: '/', labelKey: 'nav.dashboard', icon: '🏠' },
    { href: '/repos', labelKey: 'nav.repos', icon: '📦' },
    { href: '/vault', labelKey: 'nav.vault', icon: '🔐' },
    { href: '/graph', labelKey: 'nav.graph', icon: '🕸️' },
    { href: '/settings', labelKey: 'nav.settings', icon: '⚙️' },
  ];

  const activePath = derived(location, ($loc) => $loc);

  function isActive(item: NavItem, current: string): boolean {
    if (item.href === '/') return current === '/';
    return current === item.href || current.startsWith(item.href + '/');
  }
</script>

<aside
  class="flex h-full w-56 shrink-0 flex-col border-r border-slate-200 bg-white dark:border-slate-700 dark:bg-slate-800"
  data-testid="sidebar"
>
  <div class="flex items-center gap-2 px-4 py-4 text-base font-semibold">
    <span class="text-xl">🌿</span>
    <span>{$catalog['common.appName']}</span>
  </div>

  <nav class="flex-1 space-y-1 px-2" aria-label="primary">
    {#each items as item (item.href)}
      {@const active = isActive(item, $activePath)}
      <a
        href={href(item.href)}
        class="block rounded-md px-3 py-2 text-sm font-medium transition-colors hover:bg-slate-100 dark:hover:bg-slate-700"
        class:bg-accent-50={active}
        class:text-accent-700={active}
        class:dark:bg-slate-700={active}
        class:dark:text-white={active}
        aria-current={active ? 'page' : undefined}
        data-testid={`nav-${item.labelKey}`}
      >
        <span class="mr-2">{item.icon}</span>
        <span>{$catalog[item.labelKey]}</span>
      </a>
    {/each}
  </nav>

  <div class="border-t border-slate-200 p-3 space-y-2 dark:border-slate-700">
    <ThemeToggle />
    <LocaleSwitcher />
  </div>
</aside>