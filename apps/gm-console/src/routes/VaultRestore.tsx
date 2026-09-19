/**
 * Vault version restore page. Picks a target version, calls the restore
 * endpoint, and shows the resulting new version (the restore appends a
 * new entry rather than mutating the timeline in place).
 */
import { useMemo } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { vault } from '@/api';
import { EmptyState, ErrorState, Loading } from '@/components';
import { useTranslate } from '@/i18n/runtime';
import { useToasts } from '@/stores/toasts';

export function VaultRestore() {
  const t = useTranslate();
  const { key = '' } = useParams<{ key: string }>();
  const decoded = decodeURIComponent(key);
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const toasts = useToasts();

  const versions = useQuery({
    queryKey: ['vault', 'key', decoded, 'versions'],
    queryFn: () => vault.getVersions(decoded),
    enabled: decoded.length > 0,
  });

  const list = versions.data?.versions ?? [];
  const defaultTarget = useMemo(() => {
    if (list.length === 0) return 1;
    return list[0].version;
  }, [list]);
  const target = useMemo(() => {
    if (typeof window === 'undefined') return defaultTarget;
    const raw = new URLSearchParams(window.location.search).get('target');
    const parsed = raw ? parseInt(raw, 10) : NaN;
    return Number.isFinite(parsed) && parsed >= 1 ? parsed : defaultTarget;
  }, [list, defaultTarget]);

  const restore = useMutation({
    mutationFn: () => vault.restoreVersion(decoded, { target_version: target }),
    onSuccess: (resp) => {
      toasts.push({ kind: 'success', message: t('vault.restore.success')(resp.new_version) });
      queryClient.invalidateQueries({ queryKey: ['vault', 'key', decoded] });
      queryClient.invalidateQueries({ queryKey: ['vault'] });
      navigate(`/vault/${encodeURIComponent(decoded)}`);
    },
    onError: (err: Error) => toasts.push({ kind: 'error', message: `${t('vault.restore.error')}: ${err.message}` }),
  });

  if (versions.isLoading) return <Loading label={t('app.states.loading')} />;
  if (versions.isError) return <ErrorState error={versions.error} onRetry={() => versions.refetch()} />;

  if (list.length === 0) {
    return (
      <EmptyState
        title={t('vault.title')}
        description={t('vault.noVersions')}
        icon="?"
      />
    );
  }

  return (
    <section aria-labelledby="vault-restore-title" className="space-y-6">
      <header>
        <h1 id="vault-restore-title" className="font-mono text-2xl font-semibold">
          {decoded} — {t('vault.restore.title')}
        </h1>
        <button
          type="button"
          className="btn-ghost mt-2"
          onClick={() => navigate(`/vault/${encodeURIComponent(decoded)}`)}
        >
          ← {t('app.actions.back')}
        </button>
      </header>

      <form
        className="card space-y-4 p-4"
        onSubmit={(e) => {
          e.preventDefault();
          restore.mutate();
        }}
      >
        <label className="flex flex-col gap-1 text-xs">
          <span className="font-semibold">{t('vault.restore.pickVersion')}</span>
          <select
            value={target}
            onChange={(e) =>
              navigate(
                `/vault/${encodeURIComponent(decoded)}/restore?target=${e.target.value}`,
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

        <p className="text-xs text-slate-500 dark:text-slate-400">
          {t('vault.restore.confirmHint')}
        </p>

        <button
          type="submit"
          className="btn-primary"
          disabled={restore.isPending || target < 1}
        >
          {restore.isPending ? t('app.states.loading') : t('app.actions.restore')}
        </button>
      </form>
    </section>
  );
}