import { useState, type FormEvent, type ReactNode } from 'react';
import { listRepos } from '@/api/repos';
import { useTranslate } from '@/i18n/runtime';
import { useCredentialsStore } from '@/stores/credentials';

interface LoginGateProps {
  children: ReactNode;
}

/**
 * Gate the whole console behind the admin credential.
 *
 * `[FACT]` `/api/*` has required HTTP Basic auth since 2026-10-05
 * (`src/server/api.rs` → `build_api_router`). This component exists
 * because the console has a real, complete REST client
 * (`api/client.ts` + `api/repos.ts` + `api/vault.ts`), so without a way
 * to present a credential every screen would sit at a 401.
 *
 * `[FACT]` The probe request is `GET /api/repos`, not `/api/health`.
 * `/api/health` is deliberately outside the auth layer — it is a
 * liveness probe — so it answers 200 to any caller and would report a
 * wrong password as a successful sign-in. Probing with an endpoint that
 * is actually behind the layer is the only way the form can tell the
 * user their password is wrong before handing them an empty console.
 */
export function LoginGate({ children }: LoginGateProps) {
  const t = useTranslate();
  const credential = useCredentialsStore((s) => s.credential);
  const signIn = useCredentialsStore((s) => s.signIn);
  const signOut = useCredentialsStore((s) => s.signOut);

  const [user, setUser] = useState('');
  const [password, setPassword] = useState('');
  const [checking, setChecking] = useState(false);
  const [failed, setFailed] = useState(false);

  if (credential) {
    return (
      <>
        <SignOutBar onSignOut={signOut} label={t('login.signOut')} />
        {children}
      </>
    );
  }

  async function onSubmit(event: FormEvent<HTMLFormElement>): Promise<void> {
    event.preventDefault();
    setChecking(true);
    setFailed(false);
    // Store first: the axios request interceptor reads the credential
    // from here on every request, so the probe carries it.
    signIn({ user, password });
    try {
      await listRepos();
      // Success — the gate re-renders the app on the next paint because
      // `credential` is now set. The password is not kept in component
      // state beyond the store, and neither is written anywhere durable.
      setPassword('');
    } catch {
      // The response interceptor already called `signOut` on a 401, so
      // the store is already clean; the explicit call is belt-and-braces
      // for the other failures. Every failure mode ends the same way —
      // stay on the form and say so — so they are deliberately not
      // branched apart: distinguishing "wrong password" from "server
      // down" here would need a second message the gate cannot yet
      // justify, because a network error is indistinguishable from a
      // wrong password until the server is known to be up.
      signOut();
      setFailed(true);
    } finally {
      setChecking(false);
    }
  }

  return (
    <main className="flex min-h-screen items-center justify-center p-6">
      <form
        onSubmit={onSubmit}
        className="card flex w-full max-w-sm flex-col gap-4 p-6"
        aria-labelledby="login-title"
      >
        <h1 id="login-title" className="text-lg font-semibold">
          {t('login.title')}
        </h1>
        <p className="text-sm text-slate-500 dark:text-slate-400">{t('login.description')}</p>

        <label className="flex flex-col gap-1 text-sm" htmlFor="login-user">
          <span>{t('login.user')}</span>
          <input
            id="login-user"
            name="username"
            className="input"
            autoComplete="username"
            value={user}
            onChange={(e) => setUser(e.target.value)}
            required
          />
        </label>

        <label className="flex flex-col gap-1 text-sm" htmlFor="login-password">
          <span>{t('login.password')}</span>
          <input
            id="login-password"
            name="password"
            type="password"
            className="input"
            autoComplete="current-password"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            required
          />
        </label>

        {failed ? (
          <p role="alert" className="text-sm text-rose-600 dark:text-rose-400">
            {t('login.failed')}
          </p>
        ) : null}

        <button type="submit" className="btn-primary" disabled={checking}>
          {checking ? t('login.checking') : t('login.submit')}
        </button>
      </form>
    </main>
  );
}

/**
 * A minimal bar offering the way back out. Signing out is not cosmetic:
 * without it, an operator who signed in on a shared machine has no way
 * to clear the in-memory credential short of reloading the page.
 */
function SignOutBar({ onSignOut, label }: { onSignOut: () => void; label: string }) {
  return (
    <div className="flex justify-end border-b border-slate-200 p-2 dark:border-slate-800">
      <button type="button" className="btn-secondary" onClick={onSignOut}>
        {label}
      </button>
    </div>
  );
}
