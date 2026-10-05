/**
 * Tests for the `/settings` route component (`src/routes/Settings.svelte`).
 *
 * This file was 0% covered. Settings is where the app's own behaviour is
 * changed, so the cases below pin:
 *
 *  - the admin-password status in both of its states, and the fact that
 *    the status is *re-read* after a change rather than assumed — a save
 *    that reported success while the backend still had the old password
 *    is the failure mode that matters,
 *  - the refusal to save an empty password, which must not reach the
 *    backend at all: writing an empty secret would silently replace a
 *    real one,
 *  - that the about block is omitted when `app_info` fails rather than
 *    rendering empty values, and that the rest of the page still works —
 *    the `onMount` catch is shared with the password status, so a
 *    backend that cannot describe itself must not take the page with it,
 *  - and that the three sub-controls the page hosts (theme, locale,
 *    working-tree root) are really on it, each in exactly one place a
 *    user can reach.
 */
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { readFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { get } from 'svelte/store';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import Settings from '../../src/routes/Settings.svelte';
import { toasts } from '../../src/lib/stores/toasts';
import { locale, setLocale } from '../../src/lib/stores/locale';
import { callsTo, cat, resetRouteStores, useInvoke } from './routes-harness';

/**
 * A stand-in for the password commands, backed by one boolean.
 *
 * `[FACT]` The shipped mock keeps its vault store for the whole test
 * file, so really setting a password would leave every later case
 * reporting "set". This fake is scoped to the case that uses it.
 */
function passwordBackend(initial: boolean): {
  overrides: Record<string, (args: Record<string, unknown>) => unknown>;
  isSet: () => boolean;
} {
  let isSet = initial;
  return {
    isSet: () => isSet,
    overrides: {
      get_admin_password_status: () => ({ is_set: isSet, length: isSet ? 16 : 0 }),
      set_admin_password: () => {
        isSet = true;
        return 1;
      },
      clear_admin_password: () => {
        isSet = false;
        return null;
      },
    },
  };
}

function passwordField(): HTMLInputElement {
  return document.getElementById('new-pw') as HTMLInputElement;
}

beforeEach(() => {
  resetRouteStores();
});

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

describe('route / settings — the admin password', () => {
  it('reports the password as set, with its length', async () => {
    // The length is what tells a user whether the password they are
    // about to type replaces something.
    const backend = passwordBackend(true);
    useInvoke(backend.overrides);
    render(Settings);

    const status = await screen.findByTestId('admin-status');
    expect(status.textContent?.trim()).toBe(
      cat('settings.adminPasswordSet').replace('{n}', '16')
    );
  });

  it('reports the password as unset, and says which one is in use', async () => {
    // "Not set (currently using default)" is a different situation from
    // "set", and the page has to be able to say so.
    const backend = passwordBackend(false);
    useInvoke(backend.overrides);
    render(Settings);

    const status = await screen.findByTestId('admin-status');
    expect(status.textContent?.trim()).toBe(cat('settings.adminPasswordNotSet'));
    expect(status.textContent).not.toContain('{n}');
  });

  it('refuses an empty password without asking the backend to store one', async () => {
    // Writing an empty secret would replace a real password with one
    // nobody knows, so this is refused before the command is issued.
    const seen = useInvoke(passwordBackend(true).overrides);
    render(Settings);
    await screen.findByTestId('admin-status');

    await fireEvent.click(screen.getByRole('button', { name: cat('common.save') }));

    expect(callsTo(seen, 'set_admin_password')).toHaveLength(0);
    expect(get(toasts).map((t) => [t.kind, t.message])).toEqual([['warn', 'empty password']]);
  });

  it('saves the password, clears the field and re-reads the status', async () => {
    const backend = passwordBackend(false);
    const seen = useInvoke(backend.overrides);
    render(Settings);
    await screen.findByTestId('admin-status');
    const readsBefore = callsTo(seen, 'get_admin_password_status').length;

    await fireEvent.input(passwordField(), { target: { value: 'correct horse' } });
    await fireEvent.click(screen.getByRole('button', { name: cat('common.save') }));

    await waitFor(() => expect(callsTo(seen, 'set_admin_password')).toHaveLength(1));
    expect(callsTo(seen, 'set_admin_password')[0]?.args).toMatchObject({
      password: 'correct horse',
    });
    // The status is read again rather than assumed: the page must not
    // claim "set" before the backend agrees.
    await waitFor(() =>
      expect(callsTo(seen, 'get_admin_password_status').length).toBeGreaterThan(readsBefore)
    );
    expect(backend.isSet()).toBe(true);
    await waitFor(() =>
      expect(screen.getByTestId('admin-status').textContent).toContain(
        cat('settings.adminPasswordSet').replace('{n}', '16')
      )
    );
    // The old expectation here was `['success', 'updated']`, and it was
    // pinning the lie this change removes. The page used to toast
    // "updated" and render "Set (length 16)" while the running server
    // went on accepting the old password, because the server resolved
    // its credential once at start-up and nothing rotated it. The
    // server now reads this vault key, and the toast states the one
    // thing that is still true about *when* it takes effect.
    expect(get(toasts).map((t) => [t.kind, t.message])).toEqual([
      ['success', 'saved — the server uses it from its next start'],
    ]);
    // The secret is not left sitting in a field on screen.
    expect(passwordField().value).toBe('');
  });

  it('does not tell the user the password is already in force', async () => {
    // The specific failure mode: a success toast with no qualification
    // is indistinguishable from "the server is using this right now".
    // Pinned separately from the exact wording above so that rewording
    // the message cannot quietly re-introduce the over-claim.
    const backend = passwordBackend(false);
    useInvoke(backend.overrides);
    render(Settings);
    await screen.findByTestId('admin-status');

    await fireEvent.input(passwordField(), { target: { value: 'correct horse' } });
    await fireEvent.click(screen.getByRole('button', { name: cat('common.save') }));

    await waitFor(() => expect(get(toasts)).toHaveLength(1));
    const toast = get(toasts)[0];
    expect(toast.kind).toBe('success');
    expect(toast.message).toMatch(/next start/i);
    // "updated" on its own is exactly the claim that was not true.
    expect(toast.message).not.toBe('updated');
  });

  it('clears the password and re-reads the status', async () => {
    const backend = passwordBackend(true);
    const seen = useInvoke(backend.overrides);
    render(Settings);
    await screen.findByTestId('admin-status');

    await fireEvent.click(
      screen.getByRole('button', { name: cat('settings.adminPasswordClear') })
    );

    await waitFor(() => expect(callsTo(seen, 'clear_admin_password')).toHaveLength(1));
    expect(backend.isSet()).toBe(false);
    await waitFor(() =>
      expect(screen.getByTestId('admin-status').textContent?.trim()).toBe(
        cat('settings.adminPasswordNotSet')
      )
    );
    // Was `['info', 'cleared']`. Clearing does not restore a known
    // password — the vault key is gone, so the next start generates a
    // fresh random one. Saying only "cleared" left the user believing
    // the server had reverted to something they knew.
    expect(get(toasts).map((t) => [t.kind, t.message])).toEqual([
      ['info', 'cleared — a new password is generated at the next start'],
    ]);
  });
});

describe('route / settings — the about block', () => {
  it('shows the paths the backend reported', async () => {
    useInvoke({
      app_info: () => ({
        name: 'gm-desktop',
        version: '0.1.0',
        data_dir: '/var/data/com.gitgit.desktop',
        config_dir: '/var/data/com.gitgit.desktop',
        repos_dir: '/var/data/repos',
        vault_dir: '/var/data/vault',
        current_locale: 'zh-CN',
      }),
    });
    render(Settings);

    // Each row is label + value, so the values are read as a set rather
    // than as one blob of text.
    expect(await screen.findByText('gm-desktop')).toBeTruthy();
    expect(screen.getByText('0.1.0')).toBeTruthy();
    expect(screen.getByText('/var/data/com.gitgit.desktop')).toBeTruthy();
    expect(screen.getByText('/var/data/repos')).toBeTruthy();
    expect(screen.getByText('/var/data/vault')).toBeTruthy();
  });

  it('omits the about block when the backend cannot describe itself', async () => {
    // `onMount` wraps both reads in one try/catch, so a refused
    // `app_info` also costs the password status. The page must still be
    // usable — a settings page that renders nothing leaves the user with
    // no way to fix anything.
    useInvoke({ app_info: () => Promise.reject(new Error('no data directory')) });
    render(Settings);

    await waitFor(() => expect(screen.queryByTestId('admin-status')).toBeNull());
    expect(screen.getByRole('heading', { level: 1 }).textContent?.trim()).toBe(
      cat('settings.heading')
    );
    expect(screen.queryByText('gm-desktop')).toBeNull();
    // The password form itself is still there.
    expect(passwordField()).toBeTruthy();
  });
});

describe('route / settings — the hosted controls', () => {
  it('carries the theme and language controls', async () => {
    // Both are also in the sidebar, so a settings page without them
    // would be a page whose own heading promises controls it lacks.
    useInvoke();
    render(Settings);
    await screen.findByTestId('admin-status');

    expect(screen.getByTestId('theme-toggle')).toBeTruthy();
    expect(screen.getByTestId('locale-switcher')).toBeTruthy();
    expect(screen.getByRole('heading', { name: cat('settings.theme') })).toBeTruthy();
    expect(screen.getByRole('heading', { name: cat('settings.locale') })).toBeTruthy();
  });

  it('carries the working-tree root picker, the one place it can be unset', async () => {
    useInvoke();
    render(Settings);
    await screen.findByTestId('admin-status');

    // The picker is what makes the working-tree views usable at all, and
    // this card is its documented home.
    expect(screen.getByRole('heading', { name: cat('repos.root.label') })).toBeTruthy();
    const choose = screen.getByRole('button', { name: cat('repos.root.choose') });
    expect(choose).toBeTruthy();
  });

  it('keeps the update-on-startup preference in the box it is bound to', async () => {
    // Local state only — nothing is persisted yet — but the binding is
    // real: a checkbox that does not follow a click is worse than one
    // that is visibly inert, because it looks like it took.
    useInvoke();
    render(Settings);
    await screen.findByTestId('admin-status');

    const box = screen.getByRole('checkbox', {
      name: cat('settings.updates.checkOnStartup'),
    }) as HTMLInputElement;
    expect(box.checked).toBe(false);
    expect(screen.getByText(cat('settings.updates.placeholderNote'))).toBeTruthy();

    await fireEvent.click(box);
    expect(box.checked).toBe(true);
  });
});

/**
 * AGPL-3.0 section 5(d) requires every interactive interface to display
 * "Appropriate Legal Notices", and section 0 defines that as a prominently
 * visible feature which (1) displays a copyright notice, (2) tells the user
 * there is no warranty, (3) tells licensees they may convey the work under
 * this License, and (4) says how to view a copy of the License.
 *
 * `[FACT]` These patterns deliberately test the *substance* of each line
 * rather than comparing it to `cat('settings.license.*')`. Comparing the
 * rendered text to the catalog key it was interpolated from would pass for
 * a card whose strings are all empty, which satisfies the letter of
 * "displays a License line" and none of section 0. The per-locale tables
 * exist for the same reason: a single English pattern would make this file
 * pass while the default `zh-CN` interface said nothing at all.
 */
const SECTION_0_REQUIREMENTS: Record<
  string,
  { copyright: RegExp; warranty: RegExp; convey: RegExp }
> = {
  en: {
    copyright: /copyright/i,
    warranty: /without warranty|as is/i,
    convey: /agpl|affero|general public license/i,
  },
  'zh-CN': {
    copyright: /版权/,
    // "按「现状」提供" and "不含任何明示或默示的担保" are the two halves
    // of the English "as is, without warranty of any kind".
    warranty: /担保|现状/,
    convey: /agpl|affero|通用公共许可证/i,
  },
};

describe('route / settings — the AGPL Appropriate Legal Notices', () => {
  // `as const` so `localeId` keeps its literal type: without it the
  // array infers `string[]`, and `setLocale` — which takes `LocaleId` —
  // rejects it under `svelte-check`.
  for (const localeId of ['en', 'zh-CN'] as const) {
    it(`shows all four of section 0's requirements in ${localeId}`, async () => {
      // Both bundles are exercised: the app defaults to zh-CN, so a suite
      // that only ever rendered the English one would leave the interface
      // users actually see unverified.
      resetRouteStores();
      setLocale(localeId);
      useInvoke();
      render(Settings);
      await screen.findByTestId('license-card');

      const want = SECTION_0_REQUIREMENTS[localeId];
      expect(want).toBeDefined();

      const copyright = screen.getByTestId('license-copyright').textContent ?? '';
      const warranty = screen.getByTestId('license-warranty').textContent ?? '';
      const convey = screen.getByTestId('license-convey').textContent ?? '';
      const view = screen.getByTestId('license-view').textContent ?? '';

      // (1) a copyright notice that actually names a holder,
      expect(copyright).toMatch(want!.copyright);
      // (2) the absence of warranty, stated rather than implied,
      expect(warranty).toMatch(want!.warranty);
      // (3) the right to convey the work under this License,
      expect(convey).toMatch(want!.convey);
      // (4) how to view a copy of the License: a URL a user can actually
      // open, and the placeholder really substituted.
      expect(view).toMatch(/https:\/\/\S+/);
      expect(view).not.toContain('{url}');
    });
  }

  it('names the same copyright holder the installer metadata claims', async () => {
    // `tauri.conf.json` writes `bundle.copyright` into the MSI metadata, so
    // an About card that disagreed with it would have the application
    // claiming two different holders for the same work. Read through
    // node:fs rather than importing the JSON so the assertion is against
    // the file that actually ships, not a bundler-cached copy of it.
    //
    // `process.cwd()` and not `import.meta.url`: under vitest the module
    // URL is the Vite dev server's `http://localhost:...`, so
    // `new URL(relative, import.meta.url)` yields a non-`file:` URL and
    // `readFile` rejects it with "The URL must be of scheme file". vitest
    // runs with the Vite root as cwd, which is `apps/gm-desktop`.
    const conf = JSON.parse(
      await readFile(resolve(process.cwd(), 'src-tauri/tauri.conf.json'), 'utf8')
    ) as { bundle: { copyright: string } };
    expect(conf.bundle.copyright).toMatch(/copyright/i);
    // The holder identity itself, not just the word "copyright".
    const named = conf.bundle.copyright.replace(/^copyright\s*\(c\)\s*/i, '').trim();
    expect(named.length).toBeGreaterThan(0);

    resetRouteStores();
    setLocale('en');
    useInvoke();
    render(Settings);
    await screen.findByTestId('license-card');

    expect(screen.getByTestId('license-copyright').textContent ?? '').toContain(named);
  });

  it('still shows the notices when the backend cannot describe itself', async () => {
    // The card sits outside the `{#if info}` block on purpose. A legal
    // notice that disappears when `app_info` fails is a notice that
    // disappears exactly when something has gone wrong — and this page's
    // `onMount` shares one try/catch between `app_info` and the password
    // status, so a backend that cannot describe itself is reachable in
    // ordinary use, not only in a test double.
    useInvoke({ app_info: () => Promise.reject(new Error('no data directory')) });
    render(Settings);

    await waitFor(() => expect(screen.queryByTestId('admin-status')).toBeNull());
    expect(screen.getByTestId('license-card')).toBeTruthy();
    expect(screen.getByTestId('license-copyright').textContent).toMatch(
      SECTION_0_REQUIREMENTS[get(locale)].copyright
    );
    expect(screen.getByTestId('license-view').textContent).toMatch(/https:\/\/\S+/);
  });
});
