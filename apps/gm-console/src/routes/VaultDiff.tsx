/**
 * Vault version diff page. Lets the operator pick two versions from the
 * timeline and reports whether the object changed and the byte delta.
 */
import { useMemo } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { useQuery } from '@tanstack/react-query';
import { vault } from '@/api';
import { EmptyState, ErrorState, Loading } from '@/components';
import { useTranslate } from '@/i18n/runtime';

export function VaultDiff() {
  const t = useTranslate();
  const { key = '' } = useParams<{ key: string }>();
  const decoded = decodeURIComponent(key);
  const navigate = useNavigate();

  const versions = useQuery({
    queryKey: ['vault', 'key', decoded, 'versions'],
    queryFn: () => vault.getVersions(decoded),
    enabled: decoded.length > 0,
  });

  const base = useQueryParam('base');
  const head = useQueryParam('head');

  const parsedBase = useMemo(() => parseInt(base ?? '', 10), [base]);
  const parsedHead = useMemo(() => parseInt(head ?? '', 10), [head]);

  const canRun =
    Number.isFinite(parsedBase) &&
    Number.isFinite(parsedHead) &&
    parsedBase >= 1 &&
    parsedHead >= 1 &&
    parsedBase < parsedHead;

  const diff = useQuery({
    queryKey: ['vault', 'key', decoded, 'diff', parsedBase, parsedHead],
    queryFn: () => vault.diffVersions(decoded, parsedBase, parsedHead),
    enabled: canRun,
  });

  if (versions.isLoading) return <Loading label={t('app.states.loading')} />;
  if (versions.isError) return <ErrorState error={versions.error} onRetry={() => versions.refetch()} />;

  const list = versions.data?.versions ?? [];

  return (
    <section aria-labelledby="vault-diff-title" className="space-y-6">
      <header>
        <h1 id="vault-diff-title" className="font-mono text-2xl font-semibold">
          {decoded} — {t('vault.diff.title')}
        </h1>
        <button
          type="button"
          className="btn-ghost mt-2"
          onClick={() => navigate(`/vault/${encodeURIComponent(decoded)}`)}
        >
          ← {t('app.actions.back')}
        </button>
      </header>

      {list.length < 2 ? (
        <EmptyState
          title={t('vault.title')}
          description={t('vault.noVersions')}
          icon="?"
        />
      ) : (
        <>
          <form
            className="card flex flex-wrap items-end gap-3 p-4"
            onSubmit={(e) => {
              e.preventDefault();
              navigate(
                `/vault/${encodeURIComponent(decoded)}/diff?base=${parsedBase}&head=${parsedHead}`,
              );
            }}
          >
            <label className="flex flex-col gap-1 text-xs">
              <span className="font-semibold">{t('vault.diff.pickBase')}</span>
              <select
                value={base ?? ''}
                onChange={(e) =>
                  navigate(
                    `/vault/${encodeURIComponent(decoded)}/diff?base=${e.target.value}&head=${head ?? ''}`,
                  )
                }
                className="select"
              >
                {list.map((v) => (
                  <option key={v.version} value={v.version}>
                    v{v.version}
                  </option>
                ))}
              </select>
            </label>
            <label className="flex flex-col gap-1 text-xs">
              <span className="font-semibold">{t('vault.diff.pickHead')}</span>
              <select
                value={head ?? ''}
                onChange={(e) =>
                  navigate(
                    `/vault/${encodeURIComponent(decoded)}/diff?base=${base ?? ''}&head=${e.target.value}`,
                  )
                }
                className="select"
              >
                {list.map((v) => (
                  <option key={v.version} value={v.version}>
                    v{v.version}
                  </option>
                ))}
              </select>
            </label>
            <button type="submit" className="btn-primary" disabled={!canRun}>
              {t('vault.diff.run')}
            </button>
          </form>

          {!canRun && (
            <p className="text-xs text-amber-600 dark:text-amber-400">
              {t('vault.diff.baseGreater')}
            </p>
          )}

          {diff.isLoading && <Loading label={t('app.states.loading')} />}
          {diff.isError && (
            <ErrorState error={diff.error} onRetry={() => diff.refetch()} />
          )}
          {diff.data && (
            <article className="card space-y-2 p-4">
              <p className="text-sm">
                {t('vault.diff.objectChanged')(diff.data.object_changed)}
              </p>
              <p className="text-sm">
                {t('vault.diff.fileSizeDelta')(diff.data.file_size_delta)}
              </p>
              <dl className="grid grid-cols-2 gap-2 text-xs">
                <div>
                  <dt className="font-semibold">{t('app.common.base')}</dt>
                  <dd className="font-mono">v{diff.data.base.version}</dd>
                </div>
                <div>
                  <dt className="font-semibold">{t('app.common.headVer')}</dt>
                  <dd className="font-mono">v{diff.data.head.version}</dd>
                </div>
              </dl>
            </article>
          )}
        </>
      )}
    </section>
  );
}

/** Tiny local URLSearchParams helper to avoid pulling in a router hook just for one param. */
function useQueryParam(name: string): string | null {
  if (typeof window === 'undefined') return null;
  return new URLSearchParams(window.location.search).get(name);
}