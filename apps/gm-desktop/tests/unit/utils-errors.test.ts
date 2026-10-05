/**
 * Tests for `src/lib/utils/errors.ts` — the single place a Tauri
 * rejection becomes text a person can read.
 *
 * `[FACT]` Every case here is a decision that was wrong somewhere in the
 * codebase before this module existed. The app had three partial answers
 * and four call sites with none, and two of those partial answers
 * disagreed about what to do with a rejection that carries no `kind`.
 *
 * Assertions go against `tFor(get(locale), key)` rather than a pinned
 * language, following the house rule that jsdom reports
 * `navigator.language === 'en-US'`, so the active locale is `en`.
 */
import { describe, expect, it } from 'vitest';
import { get } from 'svelte/store';
import { friendlyError, normalizeError } from '../../src/lib/utils/errors';
import { locale, tFor } from '../../src/lib/i18n';

/** A rejection shaped like the Rust `AppError` serializer emits. */
function appError(kind: string, message: string) {
  return { kind, message, source: `"${kind}"` };
}

const tr = (key: string): string => tFor(get(locale), key);

describe('utils / errors — normalizeError', () => {
  it('keeps both the kind and the message of a Tauri refusal', () => {
    expect(normalizeError(appError('GraphNotBuilt', 'no index yet'))).toEqual({
      kind: 'GraphNotBuilt',
      message: 'no index yet',
    });
  });

  it('reports a plain Error as an Internal failure, keeping its message', () => {
    expect(normalizeError(new Error('backend went away'))).toEqual({
      kind: 'Internal',
      message: 'backend went away',
    });
  });

  it('keeps a string rejection verbatim', () => {
    expect(normalizeError('boom')).toEqual({ kind: 'Internal', message: 'boom' });
  });

  it('substitutes Internal for a kind that is not a non-empty string', () => {
    // A payload with `kind: ''` is a malformed `AppError`; inventing a
    // kind from it would put a key in the catalogue that does not exist.
    expect(normalizeError({ kind: '', message: 'm' }).kind).toBe('Internal');
    expect(normalizeError({ kind: 7, message: 'm' }).kind).toBe('Internal');
  });

  it('tolerates a payload whose message is missing or the wrong type', () => {
    // Measured shape mismatch: a caller can reject with `{ kind }` alone.
    expect(normalizeError({ kind: 'Git' })).toEqual({ kind: 'Git', message: '' });
    expect(normalizeError({ kind: 'Git', message: 42 })).toEqual({ kind: 'Git', message: '' });
  });

  it('does not treat null or undefined as an app error', () => {
    // `[FACT]` `'kind' in null` throws, so the guard has to check the
    // value is a non-null object first.
    expect(normalizeError(null).message).toBe('null');
    expect(normalizeError(undefined).message).toBe('undefined');
  });
});

describe('utils / errors — friendlyError', () => {
  it('prefers the catalogue wording for a known kind', () => {
    // The kind is what makes the message actionable — "Bare repository —
    // no working tree" tells the user what to do next in their language.
    expect(friendlyError(appError('NotAWorkTree', 'raw backend text'))).toBe(
      tr('errors.kind.NotAWorkTree')
    );
  });

  it('falls back to the backend message when the kind has no translation', () => {
    // An unrecognised kind must not render the raw i18n key, and must not
    // be replaced by a generic label either: the backend's own words are
    // more useful than "Internal error".
    expect(friendlyError(appError('SomethingBrandNew', 'the backend said this'))).toBe(
      'the backend said this'
    );
  });

  it('falls back to the backend message when there is no kind at all', () => {
    // `[FACT]` This is the case the first version of this helper got
    // wrong. It substituted the `Internal` fallback kind and then looked
    // *that* up — and `errors.kind.Internal` has a translation, so a
    // payload carrying a perfectly good message but no `kind` rendered
    // the generic label and threw the message away. The mock layer and
    // any non-Tauri caller produce exactly this shape.
    const message = friendlyError({ message: 'plain refusal' });
    expect(message).toBe('plain refusal');
    expect(message).not.toBe(tr('errors.kind.Internal'));
  });

  it('shows the Internal label only when there is nothing else to show', () => {
    // A payload with neither a usable kind nor a message leaves nothing
    // to say, so the label is better than an empty string.
    expect(friendlyError({ kind: '' })).toBe(tr('errors.kind.Internal'));
  });

  it('renders an Error and a bare string without consulting the catalogue', () => {
    expect(friendlyError(new Error('plain failure'))).toBe('plain failure');
    expect(friendlyError('bare string')).toBe('bare string');
  });

  it('never renders an i18n key to the user', () => {
    // The miss-detection relies on `t` echoing the key back, so a
    // regression there would put `errors.kind.…` on screen. Assert the
    // shape of that invariant directly.
    for (const e of [appError('Nope', 'm'), { message: 'm' }, new Error('m'), 'm', null]) {
      expect(friendlyError(e)).not.toMatch(/errors\.kind\./);
    }
  });
});
