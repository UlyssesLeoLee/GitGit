/**
 * Vault listing page. Shows every user-visible key with version count and
 * byte length. Click a row to open the per-key detail view.
 */
import { Link } from 'react-router-dom';
import { useQuery } from '@tanstack/react-query';
import { vault } from '@/api';
import { EmptyState, ErrorState, Loading } from '@/components';
import { formatBytes } from '@/lib/format';
import { useTranslate } from '@/i18n/runtime';

export function Vault() {
  const t = useTranslate();
  const query = useQuery({ queryKey: ['vault', 'keys'], queryFn: vault.listKeys });

  if (query.isLoading) return <Loading label={t('app.states.loading')} />;
  if (query.isError) return <ErrorState error={query.error} onRetry={() => query.refetch()} />;

  const items = query.data ?? [];
  if (items.length === 0) {
    return (
      <EmptyState
        title={t('vault.title')}
        description={t('vault.description')}
        icon="🔐"
      />
    );
  }

  return (
    <section aria-labelledby="vault-title" className="space-y-6">
      <header>
        <h1 id="vault-title" className="text-2xl font-semibold tracking-tight">
          {t('vault.title')}
        </h1>
        <p className="text-sm text-slate-500 dark:text-slate-400">
          {t('vault.description')}
        </p>
        <p className="text-xs text-slate-400">{t('vault.keyCount')(items.length)}</p>
      </header>
      <ul role="list" aria-label={t('vault.title')} className="space-y-2">
        {items.map((k) => (
          <li key={k.key}>
            <Link
              to={`/vault/${encodeURIComponent(k.key)}`}
              className="card flex items-center justify-between gap-4 p-4 focus:outline-none focus-visible:ring-2 focus-visible:ring-brand-500"
            >
              <div className="min-w-0 flex-1">
                <p className="truncate font-mono text-sm font-semibold">{k.key}</p>
                <p className="mt-1 text-xs text-slate-500 dark:text-slate-400">
                  {k.version_count} {t('app.common.versions')}
                  {k.byte_len != null && (
                    <> · {formatBytes(k.byte_len)}</>
                  )}
                  {k.current_version != null && (
                    <> · v{k.current_version}</>
                  )}
                </p>
              </div>
              <span className="badge-neutral">→</span>
            </Link>
          </li>
        ))}
      </ul>
    </section>
  );
}