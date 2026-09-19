/**
 * Vault per-key detail. Shows current plaintext (masked by default),
 * version timeline (oldest first), and actions: diff / restore / delete.
 */
import { useState } from 'react';
import { Link, useNavigate, useParams } from 'react-router-dom';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { vault } from '@/api';
import { CopyButton, EmptyState, ErrorState, Loading } from '@/components';
import { useTranslate } from '@/i18n/runtime';
import { useToasts } from '@/stores/toasts';

export function VaultKeyDetail() {
  const t = useTranslate();
  const { key = '' } = useParams<{ key: string }>();
  const decoded = decodeURIComponent(key);
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const toasts = useToasts();

  const [showValue, setShowValue] = useState(false);

  const detail = useQuery({
    queryKey: ['vault', 'key', decoded],
    queryFn: () => vault.getKey(decoded),
    enabled: decoded.length > 0,
  });
  const versions = useQuery({
    queryKey: ['vault', 'key', decoded, 'versions'],
    queryFn: () => vault.getVersions(decoded),
    enabled: decoded.length > 0,
  });

  const deleteMutation = useMutation({
    mutationFn: () => vault.deleteKey(decoded),
    onSuccess: () => {
      toasts.push({ kind: 'success', message: t('vault.delete.success') });
      queryClient.invalidateQueries({ queryKey: ['vault'] });
      navigate('/vault');
    },
    onError: (err: Error) => toasts.push({ kind: 'error', message: err.message }),
  });

  if (detail.isLoading || versions.isLoading) return <Loading label={t('app.states.loading')} />;
  if (detail.isError) return <ErrorState error={detail.error} onRetry={() => detail.refetch()} />;
  if (versions.isError) return <ErrorState error={versions.error} onRetry={() => versions.refetch()} />;

  const d = detail.data!;
  const v = versions.data!;

  return (
    <section aria-labelledby="vault-key-title" className="space-y-6">
      <header className="flex flex-wrap items-center gap-3">
        <h1
          id="vault-key-title"
          className="truncate font-mono text-2xl font-semibold tracking-tight"
        >
          {decoded}
        </h1>
        <div className="ml-auto flex flex-wrap gap-2">
          {v.versions.length >= 2 && (
            <Link
              to={`/vault/${encodeURIComponent(decoded)}/diff`}
              className="btn-secondary"
            >
              {t('vault.diff.title')}
            </Link>
          )}
          {v.versions.length >= 1 && (
            <Link
              to={`/vault/${encodeURIComponent(decoded)}/restore`}
              className="btn-secondary"
            >
              {t('app.actions.restore')}
            </Link>
          )}
          <button
            type="button"
            className="btn-danger"
            onClick={() => {
              if (window.confirm(t('vault.delete.confirm'))) {
                deleteMutation.mutate();
              }
            }}
            disabled={deleteMutation.isPending}
            aria-label={t('app.actions.delete')}
          >
            {t('app.actions.delete')}
          </button>
        </div>
      </header>

      <article className="card space-y-3 p-4" aria-label={t('vault.currentValue')}>
        <header className="flex items-center justify-between gap-3">
          <h2 className="text-sm font-semibold">{t('vault.currentValue')}</h2>
          <div className="flex items-center gap-2">
            <button
              type="button"
              className="btn-ghost"
              onClick={() => setShowValue((v) => !v)}
              aria-pressed={showValue}
            >
              {showValue ? t('vault.hideValue') : t('vault.showValue')}
            </button>
            {d.value != null && (
              <CopyButton
                value={d.value}
                label={t('app.actions.copy')}
                copiedLabel={t('app.actions.copied')}
              />
            )}
          </div>
        </header>
        {d.value == null ? (
          <EmptyState
            title={t('vault.notFound')}
            description=""
            icon="?"
          />
        ) : (
          <pre className="overflow-x-auto rounded bg-slate-100 p-3 font-mono text-xs dark:bg-slate-800">
            {showValue ? d.value : '•'.repeat(Math.min(d.value.length, 32))}
          </pre>
        )}
      </article>

      <article className="card overflow-x-auto p-4" aria-label={t('vault.timeline')}>
        <h2 className="mb-3 text-sm font-semibold">{t('vault.timeline')}</h2>
        {v.versions.length === 0 ? (
          <p className="text-xs text-slate-500">{t('vault.noVersions')}</p>
        ) : (
          <table className="min-w-full text-left text-xs">
            <thead>
              <tr className="border-b border-slate-200 dark:border-slate-700">
                <th className="py-2 pr-4">{t('app.common.version')}</th>
                <th className="py-2 pr-4">{t('app.common.sha256')}</th>
                <th className="py-2 pr-4">{t('app.common.bytes')}</th>
                <th className="py-2 pr-4">{t('app.common.createdAt')}</th>
                <th className="py-2 pr-4">{t('app.common.changeNote')}</th>
              </tr>
            </thead>
            <tbody>
              {v.versions.map((ver) => (
                <tr
                  key={ver.version}
                  className="border-b border-slate-100 dark:border-slate-800"
                >
                  <td className="py-2 pr-4 font-mono">v{ver.version}</td>
                  <td className="py-2 pr-4 font-mono">{ver.bytes_sha256.slice(0, 12)}…</td>
                  <td className="py-2 pr-4">{ver.byte_len}</td>
                  <td className="py-2 pr-4">{formatRelativeTime(ver.created_at_unix_ms)}</td>
                  <td className="py-2 pr-4">{ver.change_note ?? '—'}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </article>
    </section>
  );
}

/** Tiny helper kept local to avoid pulling in a date lib just for one column. */
function formatRelativeTime(unixMs: number): string {
  const ms = Date.now() - unixMs;
  if (ms < 60_000) return 'just now';
  const minutes = Math.floor(ms / 60_000);
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}h ago`;
  const days = Math.floor(hours / 24);
  return `${days}d ago`;
}