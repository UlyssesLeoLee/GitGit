import { Link } from 'react-router-dom';
import { useQuery } from '@tanstack/react-query';
import { repos } from '@/api';
import { EmptyState, ErrorState, Loading } from '@/components';
import { formatBytes, shortenSha } from '@/lib/format';
import { useTranslate } from '@/i18n/runtime';
import type { ReactNode } from 'react';

function Card({ children, to }: { children: ReactNode; to: string }) {
  return (
    <Link
      to={to}
      className="card group block p-4 transition hover:border-brand-400 hover:shadow-md focus:outline-none focus-visible:ring-2 focus-visible:ring-brand-500 dark:hover:border-brand-600"
    >
      {children}
    </Link>
  );
}

export function Home() {
  const t = useTranslate();
  const query = useQuery({ queryKey: ['repos'], queryFn: repos.listRepos });

  if (query.isLoading) return <Loading label={t('app.states.loading')} />;
  if (query.isError) return <ErrorState error={query.error} onRetry={() => query.refetch()} />;
  const items = query.data ?? [];
  if (items.length === 0) {
    return (
      <EmptyState
        title={t('home.title')}
        description={t('home.description')}
        icon="📦"
      />
    );
  }

  return (
    <section aria-labelledby="home-title" className="space-y-6">
      <header className="flex flex-col gap-1">
        <h1 id="home-title" className="text-2xl font-semibold tracking-tight">
          {t('home.title')}
        </h1>
        <p className="text-sm text-slate-500 dark:text-slate-400">{t('home.description')}</p>
        <p className="text-xs text-slate-400">{t('home.repoCount')(items.length)}</p>
      </header>
      <ul
        className="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-3"
        role="list"
        aria-label={t('home.title')}
      >
        {items.map((r) => (
          <li key={r.name}>
            <Card to={`/repos/${encodeURIComponent(r.name)}`}>
              <div className="flex items-start justify-between gap-3">
                <h2 className="truncate text-base font-semibold text-slate-900 group-hover:text-brand-700 dark:text-slate-100 dark:group-hover:text-brand-300">
                  {r.name}
                </h2>
                <span className="badge-neutral">{r.ref_count} refs</span>
              </div>
              <dl className="mt-3 space-y-1 text-xs text-slate-500 dark:text-slate-400">
                <div className="flex justify-between gap-2">
                  <dt>{t('app.common.head')}</dt>
                  <dd className="truncate font-mono">{r.head_ref ?? '—'}</dd>
                </div>
                <div className="flex justify-between gap-2">
                  <dt>{t('app.common.headSha')}</dt>
                  <dd className="truncate font-mono">{shortenSha(r.head_sha)}</dd>
                </div>
                <div className="flex justify-between gap-2">
                  <dt>{t('app.common.path')}</dt>
                  <dd className="truncate font-mono">{r.path}</dd>
                </div>
                <div className="flex justify-between gap-2">
                  <dt>{t('app.common.size')}</dt>
                  <dd>{formatBytes(r.head_sha ? r.head_sha.length * 2 : 0)}</dd>
                </div>
              </dl>
            </Card>
          </li>
        ))}
      </ul>
    </section>
  );
}