/**
 * Tests for the `/vault` route component (`src/routes/Vault.svelte`).
 *
 * This file was 0% covered. The vault is the only page that writes
 * secrets, so the cases below are weighted towards the paths that could
 * quietly lose or leak one:
 *
 *  - a write with no key or no value is refused *before* the backend is
 *    asked, and the refusal is a warning rather than a silent no-op,
 *  - a successful write reports the version it recorded and clears both
 *    fields, so the next write cannot inherit the previous value,
 *  - the value field is a password input. A page that showed stored
 *    secrets in plain text would leak every one of them on a shared
 *    screen,
 *  - expanding a key, diffing a version pair, restoring, rotating and
 *    deleting each reach a *different* command, and the delete asks
 *    first — a "no" must leave the credential in place,
 *  - and a restore or a rotate reports the version the backend
 *    returned, not the one that was asked for.
 *
 * `[FACT]` The shipped mock keeps its key store for the lifetime of the
 * test file, so a case that really adds a credential changes what every
 * later case sees in the table. Each case therefore adds its own
 * uniquely named key and scopes its assertions to that row; only the
 * empty-state case overrides `vault_list`.
 */
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { get } from 'svelte/store';
import { cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import Vault from '../../src/routes/Vault.svelte';
import { vaultSecrets } from '../../src/lib/stores/vault';
import { clearToasts, toasts } from '../../src/lib/stores/toasts';
import { callsTo, cat, resetRouteStores, t, useInvoke } from './routes-harness';

/** The mock's seed credential: two versions, 17 and 24 bytes. */
const SEED = 'demo.api_key';

function row(key: string): HTMLElement {
  const cell = screen.getByText(key, { selector: 'td' });
  const tr = cell.closest('tr');
  if (!tr) throw new Error(`no table row for ${key}`);
  return tr as HTMLElement;
}

/** A control of one credential's row, by its rendered label. */
function rowButton(key: string, name: string): HTMLElement {
  return within(row(key)).getByRole('button', { name });
}

/** The per-version row for one version of an expanded key. */
function versionRow(key: string, version: number): HTMLElement {
  const cell = within(row(key)).getByText(`v${version}`);
  const div = cell.closest('div');
  if (!div) throw new Error(`no row for v${version} of ${key}`);
  return div as HTMLElement;
}

/** Write a credential through the page's own form, and wait for it. */
async function addSecret(key: string, value: string): Promise<void> {
  const before = get(toasts).length;
  await fireEvent.input(screen.getByLabelText(cat('vault.keyLabel')), {
    target: { value: key },
  });
  await fireEvent.input(screen.getByLabelText(cat('vault.valueLabel')), {
    target: { value },
  });
  await fireEvent.click(screen.getByRole('button', { name: cat('common.save') }));
  // The write is async; returning before it lands would let a later
  // assertion race the table refresh and the toast.
  await waitFor(() => expect(get(toasts).length).toBeGreaterThan(before));
}

/**
 * Drop the toasts a setup step produced.
 *
 * `[FACT]` `pushToast` keeps a toast for four seconds, so a credential
 * created in a test's setup is still in the store when the action under
 * test pushes its own. Without this, "what did this action tell the
 * user" cannot be read off the ledger.
 */
function forgetSetupToasts(): void {
  clearToasts();
}

beforeEach(() => {
  resetRouteStores();
  // A vault page that asked the user to confirm nothing is a hazard in
  // its own right; each case states the answer it means.
  vi.spyOn(window, 'confirm').mockReturnValue(true);
});

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

describe('route / vault — the list', () => {
  it('says so plainly when nothing is stored', async () => {
    useInvoke({ vault_list: () => [] });
    render(Vault);

    expect(await screen.findByText(cat('vault.empty'))).toBeTruthy();
    expect(get(vaultSecrets)).toEqual([]);
    // The add form is still there: an empty vault is where the user
    // starts, not a dead end.
    expect(screen.getByLabelText(cat('vault.keyLabel'))).toBeTruthy();
    expect(screen.queryByRole('table')).toBeNull();
  });

  it('lists each key with its three actions', async () => {
    useInvoke();
    render(Vault);
    await screen.findByText(SEED, { selector: 'td' });

    const actions = within(row(SEED));
    expect(actions.getByRole('button', { name: t('vault.versions') })).toBeTruthy();
    expect(actions.getByRole('button', { name: cat('vault.rotate') })).toBeTruthy();
    expect(actions.getByRole('button', { name: cat('common.delete') })).toBeTruthy();
    // The collapsed cell names a version.
    // `[FACT]` It is the key's *position* in the list, not the vault's
    // latest version for it: `refreshVault` fills `version` from the
    // loop index, so the seed credential reads "v1" although its newest
    // version is v2. A real defect in `src/lib/stores/vault.ts`,
    // reported rather than fixed here; the expectation pins what the page
    // really renders.
    expect(actions.getByText('v1')).toBeTruthy();
  });

  it('keeps the value field masked', async () => {
    // Every stored secret is written through this one input. A `text`
    // type would put the next credential on screen in plain text.
    useInvoke();
    render(Vault);
    await screen.findByText(SEED, { selector: 'td' });

    const field = document.getElementById('vault-val');
    expect(field?.getAttribute('type')).toBe('password');
    // The key is not a secret, so it stays readable.
    expect(document.getElementById('vault-key')?.getAttribute('type')).toBe('text');
  });
});

describe('route / vault — writing a credential', () => {
  it('refuses an empty key or value without asking the backend', async () => {
    const seen = useInvoke();
    render(Vault);
    await screen.findByText(SEED, { selector: 'td' });

    await fireEvent.click(screen.getByRole('button', { name: cat('common.save') }));

    expect(callsTo(seen, 'vault_set')).toHaveLength(0);
    expect(get(toasts).map((t2) => [t2.kind, t2.message])).toEqual([
      ['warn', 'key + value required'],
    ]);
  });

  it('refuses a whitespace-only value, which `trim` would empty anyway', async () => {
    const seen = useInvoke();
    render(Vault);
    await screen.findByText(SEED, { selector: 'td' });

    await fireEvent.input(screen.getByLabelText(cat('vault.keyLabel')), {
      target: { value: 'k.blank' },
    });
    await fireEvent.input(screen.getByLabelText(cat('vault.valueLabel')), {
      target: { value: '   ' },
    });
    await fireEvent.click(screen.getByRole('button', { name: cat('common.save') }));

    expect(callsTo(seen, 'vault_set')).toHaveLength(0);
    expect(get(toasts).map((t2) => t2.kind)).toEqual(['warn']);
  });

  it('writes the credential, reports the version and clears both fields', async () => {
    const seen = useInvoke();
    render(Vault);
    await screen.findByText(SEED, { selector: 'td' });

    await addSecret('k.add', 'the-secret');

    // The value is trimmed on the way out: a trailing newline typed into
    // a password field is a very common accident and would silently
    // become part of the secret.
    await waitFor(() => expect(callsTo(seen, 'vault_set')).toHaveLength(1));
    expect(callsTo(seen, 'vault_set')[0]?.args).toMatchObject({
      key: 'k.add',
      value: 'the-secret',
    });
    // The new key is in the table, which means the list was re-read.
    await waitFor(() => expect(screen.getByText('k.add', { selector: 'td' })).toBeTruthy());
    // The version the backend recorded, not a guess.
    expect(get(toasts).map((t2) => t2.message)).toEqual([
      t('vault.secretCreated').replace('{n}', '1'),
    ]);
    // Both fields empty, so the next write cannot inherit this value.
    expect((document.getElementById('vault-key') as HTMLInputElement).value).toBe('');
    expect((document.getElementById('vault-val') as HTMLInputElement).value).toBe('');
  });
});

describe('route / vault — versions', () => {
  it('expands a key into its version list and collapses it again', async () => {
    useInvoke();
    render(Vault);
    await screen.findByText(SEED, { selector: 'td' });

    await fireEvent.click(rowButton(SEED, t('vault.versions')));

    // `onToggle` opens the key synchronously and fills the list when the
    // fetch answers, so the expanded rows arrive a tick later.
    await waitFor(() => expect(within(row(SEED)).getByText('v2')).toBeTruthy());
    // Two versions, each with a 12-character short digest and a size.
    expect(within(row(SEED)).getByText('v1')).toBeTruthy();
    expect(within(row(SEED)).getByText('aaaaaaaaaaaa')).toBeTruthy();
    expect(within(row(SEED)).getByText('bbbbbbbbbbbb')).toBeTruthy();
    expect(within(row(SEED)).getByText('17 B')).toBeTruthy();
    expect(within(row(SEED)).getByText('24 B')).toBeTruthy();
    // The toggle becomes a close affordance while the list is open.
    expect(rowButton(SEED, t('common.close'))).toBeTruthy();

    await fireEvent.click(rowButton(SEED, t('common.close')));
    await waitFor(() => expect(within(row(SEED)).queryByText('aaaaaaaaaaaa')).toBeNull());
    expect(within(row(SEED)).getByText('v1')).toBeTruthy();
  });

  it('diffs the default base/head pair, which is the two most recent versions', async () => {
    // The pair is chosen for the user: base = second-newest, head =
    // newest. Getting it wrong shows a diff of two unrelated versions.
    const seen = useInvoke();
    render(Vault);
    await screen.findByText(SEED, { selector: 'td' });
    await fireEvent.click(rowButton(SEED, t('vault.versions')));
    await waitFor(() => expect(within(row(SEED)).getByText('v2')).toBeTruthy());

    await fireEvent.click(
      within(versionRow(SEED, 2)).getByRole('button', { name: '↔' })
    );

    expect(callsTo(seen, 'vault_diff')[0]?.args).toMatchObject({
      key: SEED,
      base: 1,
      head: 2,
    });
    const panel = await screen.findByTestId(`diff-${SEED}`);
    // Byte delta, from the two sizes: 24 - 17.
    expect(panel.textContent).toContain(
      t('vault.diffTitle').replace('{base}', '1').replace('{head}', '2')
    );
    expect(panel.textContent).toContain(`${t('vault.sizeDelta')} 7`);
  });

  it('restores the version it was asked for and reports the new one', async () => {
    const seen = useInvoke();
    render(Vault);
    await screen.findByText(SEED, { selector: 'td' });
    // A key of its own, so the seed credential's history is untouched.
    await addSecret('k.restore', 'v');
    await screen.findByText('k.restore', { selector: 'td' });
    forgetSetupToasts();

    await fireEvent.click(rowButton('k.restore', t('vault.versions')));
    await waitFor(() => expect(within(row('k.restore')).getByText('v1')).toBeTruthy());
    await fireEvent.click(
      within(versionRow('k.restore', 1)).getByRole('button', { name: '↺' })
    );

    expect(callsTo(seen, 'vault_restore')[0]?.args).toMatchObject({
      key: 'k.restore',
      targetVersion: 1,
    });
    // The version the backend actually recorded, which is not the one
    // that was restored.
    await waitFor(() =>
      expect(get(toasts).map((t2) => t2.message)).toEqual([
        'restored k.restore → v2',
      ])
    );
    // `[FACT]` The refreshed list still shows one version. `onRestore`
    // calls `vaultVersions(key)`, which answers from
    // `vaultVersionsCache` when the key is already in it — and expanding
    // the row just put it there. The restore is therefore invisible in
    // the list until the page is remounted. A real defect in
    // `src/lib/stores/vault.ts`, reported rather than fixed here.
    expect(within(row('k.restore')).queryByText('v2')).toBeNull();
  });

  it('rotates a key and reports the version the rotation produced', async () => {
    const seen = useInvoke();
    render(Vault);
    await screen.findByText(SEED, { selector: 'td' });
    await addSecret('k.rotate', 'old-value');
    await screen.findByText('k.rotate', { selector: 'td' });
    forgetSetupToasts();

    await fireEvent.click(rowButton('k.rotate', cat('vault.rotate')));

    expect(callsTo(seen, 'vault_rotate')[0]?.args).toMatchObject({ key: 'k.rotate' });
    await waitFor(() =>
      expect(get(toasts).map((t2) => t2.message)).toEqual(['rotated k.rotate → v2'])
    );
  });
});

describe('route / vault — deleting a credential', () => {
  it('asks first, and keeps the credential when the answer is no', async () => {
    // The confirm text names the key, so a user with several rows can
    // tell which one they are about to lose.
    vi.mocked(window.confirm).mockReturnValue(false);
    const seen = useInvoke();
    render(Vault);
    await screen.findByText(SEED, { selector: 'td' });
    await addSecret('k.keep', 'v');
    await screen.findByText('k.keep', { selector: 'td' });
    forgetSetupToasts();

    await fireEvent.click(rowButton('k.keep', cat('common.delete')));

    expect(window.confirm).toHaveBeenCalledWith(
      t('vault.confirmDelete').replace('{key}', 'k.keep')
    );
    expect(callsTo(seen, 'vault_delete')).toHaveLength(0);
    expect(get(toasts)).toEqual([]);
    expect(screen.getByText('k.keep', { selector: 'td' })).toBeTruthy();
  });

  it('deletes the credential once confirmed, and drops it from the table', async () => {
    const seen = useInvoke();
    render(Vault);
    await screen.findByText(SEED, { selector: 'td' });
    await addSecret('k.drop', 'v');
    await screen.findByText('k.drop', { selector: 'td' });
    forgetSetupToasts();

    await fireEvent.click(rowButton('k.drop', cat('common.delete')));

    expect(callsTo(seen, 'vault_delete')[0]?.args).toMatchObject({ key: 'k.drop' });
    await waitFor(() =>
      expect(get(toasts).map((t2) => [t2.kind, t2.message])).toEqual([['info', 'deleted k.drop']])
    );
    // The list is re-read, so the row is really gone — not merely hidden.
    await waitFor(() => expect(screen.queryByText('k.drop', { selector: 'td' })).toBeNull());
    // …and the other credentials are untouched.
    expect(screen.getByText(SEED, { selector: 'td' })).toBeTruthy();
  });
});
