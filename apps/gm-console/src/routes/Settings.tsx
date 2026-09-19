/**
 * Settings page. Lets the operator view backend status (health) and
 * rotate the Basic-auth password used by gitgit-server (stored in the
 * vault under the conventional `gitgit.password` key).
 */
import { useState } from 'react';
import { useMutation, useQuery } from '@tanstack/react-query';
import { repos, vault } from '@/api';
import { ErrorState, Loading } from '@/components';
import { useTranslate } from '@/i18n/runtime';
import { useToasts } from '@/stores/toasts';

export function Settings() {
  const t = useTranslate();
  const toasts = useToasts();
  const [password, setPassword] = useState('');

  // Health-check via /api/repos (cheap, exercises the API base URL).
  const health = useQuery({
    queryKey: ['health'],
    queryFn: () => repos.health().catch(() => null),
    refetchInterval: 30_000,
  });

  const save = useMutation({
    mutationFn: () =>
      vault.setVersion('gitgit.password', {
        value: password,
        change_note: `rotated via gm-console at ${new Date().toISOString()}`,
      }),
    onSuccess: () => {
      toasts.push({ kind: 'success', message: t('settings.saveSuccess') });
      setPassword('');
    },
    onError: (err: Error) => toasts.push({ kind: 'error', message: err.message }),
  });

  return (
    <section aria-labelledby="settings-title" className="space-y-6">
      <header>
        <h1 id="settings-title" className="text-2xl font-semibold tracking-tight">
          {t('settings.title')}
        </h1>
        <p className="text-sm text-slate-500 dark:text-slate-400">
          {t('settings.description')}
        </p>
      </header>

      <article className="card space-y-3 p-4" aria-label={t('settings.backendStatus')}>
        <h2 className="text-sm font-semibold">{t('settings.backendStatus')}</h2>
        {health.isLoading && <Loading label={t('app.states.loading')} />}
        {health.isError && (
          <ErrorState error={health.error} onRetry={() => health.refetch()} />
        )}
        {health.data && (
          <dl className="grid grid-cols-2 gap-2 text-xs">
            <dt className="font-semibold">{t('settings.backendStatus')}</dt>
            <dd>
              {health.data.vault_online
                ? t('settings.backendOnline')
                : t('settings.backendOffline')}
            </dd>
            <dt className="font-semibold">{t('settings.backendVersion')}</dt>
            <dd className="font-mono">{health.data.version}</dd>
          </dl>
        )}
      </article>

      <article className="card space-y-3 p-4">
        <h2 className="text-sm font-semibold">{t('settings.gitgitPassword')}</h2>
        <p className="text-xs text-slate-500 dark:text-slate-400">
          {t('settings.gitgitPasswordHint')}
        </p>
        <form
          className="flex flex-col gap-3 sm:flex-row sm:items-end"
          onSubmit={(e) => {
            e.preventDefault();
            if (password.length === 0) return;
            save.mutate();
          }}
        >
          <label className="flex flex-1 flex-col gap-1 text-xs">
            <span className="sr-only">{t('settings.gitgitPassword')}</span>
            <input
              type="password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              autoComplete="new-password"
              minLength={1}
              required
              className="input"
            />
          </label>
          <button
            type="submit"
            className="btn-primary"
            disabled={save.isPending || password.length === 0}
          >
            {save.isPending ? t('app.states.loading') : t('app.actions.save')}
          </button>
        </form>
      </article>
    </section>
  );
}