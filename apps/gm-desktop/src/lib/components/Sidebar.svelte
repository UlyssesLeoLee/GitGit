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
    { href: '/review', labelKey: 'nav.review', icon: '🔍' },
    { href: '/settings', labelKey: 'nav.settings', icon: '⚙️' },
  ];

  const activePath = derived(location, ($loc) => $loc);

  function isActive(item: NavItem, current: string): boolean {
    if (item.href === '/') return current === '/';
    return current === item.href || current.startsWith(item.href + '/');
  }
</script>

<aside
  class="flex h-full w-60 shrink-0 flex-col border-r border-white/20 bg-white/10 dark:border-white/10 dark:bg-slate-950/25 backdrop-blur-3xl shadow-[4px_0_30px_0_rgba(0,0,0,0.05)]"
  data-testid="sidebar"
>
  <div class="flex items-center gap-2.5 px-5 py-5 text-base font-semibold border-b border-white/10">
    <span class="text-xl filter drop-shadow-[0_0_12px_rgba(34,197,94,0.6)]">🌿</span>
    <span class="dynamic-invert tracking-tight font-bold text-lg">{$catalog['common.appName']}</span>
  </div>

  <nav class="flex-1 space-y-1.5 px-3 py-3" aria-label="primary">
    {#each items as item (item.href)}
      {@const active = isActive(item, $activePath)}
      <a
        href={href(item.href)}
        class="group flex items-center rounded-xl px-3.5 py-2.5 text-sm font-medium transition-all duration-200 border border-transparent hover:border-white/20 hover:bg-white/15 dark:hover:bg-white/5 {active ? 'shadow-[0_4px_16px_rgba(34,197,94,0.15)]' : ''}"
        class:bg-accent-50={active}
        class:text-accent-700={active}
        class:dark:bg-slate-700={active}
        class:dark:text-white={active}
        aria-current={active ? 'page' : undefined}
        data-testid={`nav-${item.labelKey}`}
      >
        <span class="mr-2.5 text-base transition-transform duration-200 group-hover:scale-110">{item.icon}</span>
        <span class="font-medium tracking-wide">{$catalog[item.labelKey]}</span>
      </a>
    {/each}
  </nav>

  <div class="border-t border-white/20 p-3.5 space-y-2 dark:border-white/10">
    <ThemeToggle />
    <LocaleSwitcher />
  </div>
</aside>