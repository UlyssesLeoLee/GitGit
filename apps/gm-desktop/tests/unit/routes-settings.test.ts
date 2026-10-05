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
import { get } from 'svelte/store';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import Settings from '../../src/routes/Settings.svelte';
import { toasts } from '../../src/lib/stores/toasts';
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
    expect(get(toasts).map((t) => [t.kind, t.message])).toEqual([['success', 'updated']]);
    // The secret is not left sitting in a field on screen.
    expect(passwordField().value).toBe('');
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
    expect(get(toasts).map((t) => [t.kind, t.message])).toEqual([['info', 'cleared']]);
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
