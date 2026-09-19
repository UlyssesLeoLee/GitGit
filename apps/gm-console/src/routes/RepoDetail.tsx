import { Link, useParams } from 'react-router-dom';
import { useQuery } from '@tanstack/react-query';
import { repos } from '@/api';
import { CopyButton, EmptyState, ErrorState, Loading } from '@/components';
import { formatUnixMs, shortenSha } from '@/lib/format';
import { useTranslate } from '@/i18n/runtime';

function cloneUrl(name: string): string {
  if (typeof window === 'undefined') return `http://localhost/repos/${name}.git`;
  const { protocol, host } = window.location;
  return `${protocol}//${host}/repos/${encodeURIComponent(name)}.git`;
}

export function RepoDetail() {
  const { name = '' } = useParams<{ name: string }>();
  const t = useTranslate();
  const query = useQuery({
    queryKey: ['repo', name],
    queryFn: () => repos.getRepo(name),
    enabled: name.length > 0,
  });

  if (query.isLoading) return <Loading label={t('app.states.loading')} />;
  if (query.isError) return <ErrorState error={query.error} onRetry={() => query.refetch()} />;
  const repo = query.data;
  if (!repo) return <EmptyState title={t('repo.notFound')} icon="?" />;

  const url = cloneUrl(repo.name);

  return (
    <section aria-labelledby="repo-title" className="space-y-6">
      <header className="flex flex-col gap-2">
        <Link to="/" className="text-xs text-brand-600 hover:underline">
          ← {t('app.actions.back')}
        </Link>
        <h1 id="repo-title" className="text-2xl font-semibold tracking-tight">
          {repo.name}
        </h1>
        <p className="font-mono text-xs text-slate-500 dark:text-slate-400">{repo.path}</p>
      </header>

      {/* Clone URL card */}
      <div className="card p-4">
        <h2 className="text-sm font-semibold uppercase tracking-wide text-slate-500 dark:text-slate-400">
          {t('repo.cloneUrl')}
        </h2>
        <p className="mt-1 text-xs text-slate-500 dark:text-slate-400">{t('repo.cloneHint')}</p>
        <div className="mt-3 flex flex-col gap-3 sm:flex-row sm:items-center">
          <code
            className="flex-1 truncate rounded border border-slate-200 bg-slate-50 px-3 py-2 text-sm dark:border-slate-700 dark:bg-slate-900"
            aria-label={t('repo.cloneUrl')}
          >
            {url}
          </code>
          <CopyButton text={url} toastMessage={t('app.actions.copied')} />
        </div>
      </div>

      <div className="grid grid-cols-1 gap-4 lg:grid-cols-2">
        {/* Refs */}
        <div className="card p-4">
          <h2 className="mb-3 text-sm font-semibold uppercase tracking-wide text-slate-500 dark:text-slate-400">
            {t('repo.refs')} ({repo.refs.length})
          </h2>
          {repo.refs.length === 0 ? (
            <p className="text-sm text-slate-500 dark:text-slate-400">{t('repo.noRefs')}</p>
          ) : (
            <table className="w-full text-sm">
              <thead>
                <tr className="text-left text-xs uppercase tracking-wide text-slate-500 dark:text-slate-400">
                  <th className="py-1 pr-3">{t('repo.refName')}</th>
                  <th className="py-1">{t('repo.refSha')}</th>
                </tr>
              </thead>
              <tbody>
                {repo.refs.map((ref) => (
                  <tr key={ref.name} className="border-t border-slate-100 dark:border-slate-800">
                    <td className="py-1 pr-3 font-mono">{ref.name}</td>
                    <td className="py-1 font-mono text-xs">{shortenSha(ref.sha, 12)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </div>

        {/* Log */}
        <div className="card p-4">
          <h2 className="mb-3 text-sm font-semibold uppercase tracking-wide text-slate-500 dark:text-slate-400">
            {t('repo.log')}
          </h2>
          {repo.log.length === 0 ? (
            <p className="text-sm text-slate-500 dark:text-slate-400">{t('repo.noLog')}</p>
          ) : (
            <ol className="space-y-2" role="list">
              {repo.log.map((entry) => (
                <li key={entry.sha} className="flex gap-3 text-sm">
                  <code className="shrink-0 font-mono text-xs text-slate-500 dark:text-slate-400">
                    {entry.short_sha}
                  </code>
                  <span className="min-w-0 truncate">{entry.subject}</span>
                </li>
              ))}
            </ol>
          )}
        </div>
      </div>

      <footer className="text-xs text-slate-400">
        {t('app.common.lastUpdated')}: {formatUnixMs(Date.now())}
      </footer>
    </section>
  );
}