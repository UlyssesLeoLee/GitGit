/**
 * Cross-boundary gate: every `AppError` kind must be localizable.
 *
 * ## Why this file exists separately from the per-page i18n parity cases
 *
 * [FACT] `review.test.ts` and the other page suites each assert parity
 * for a hand-written list of keys. That list is a *subset*, so it can
 * only ever prove the keys somebody remembered to write down. Adding a
 * variant to `AppError` in `apps/gm-desktop/src-tauri/src/error.rs`
 * breaks nothing: the new kind has no catalogue entry, and the parity
 * cases stay green because the key is simply not on their list.
 *
 * [FACT] That is not hypothetical. `ServerManagerPoisoned` had no entry
 * in either catalogue. The page renders
 * `$catalog[`errors.kind.${kind}`] ?? $catalog['errors.routeTitle']`,
 * so the user saw a generic route-failure heading with no explanation
 * of a condition that has a perfectly good sentence to give it.
 *
 * ## How the two sides are read
 *
 * [FACT] The Rust side is parsed from the `AppError::X .. => "Ident"`
 * arms of `kind()`, because that function's return value is exactly what
 * crosses the bridge. Reading the enum's variant *names* instead would
 * be wrong in the general case, though for this enum the two happen to
 * coincide — the gate asserts against `kind()` precisely because that is
 * the contract, not because it is convenient.
 *
 * [FACT] The catalogue side is imported, not read as text. A previous
 * version of this project's own tooling matched keys with a substring
 * regex and reported zero matches, which reads identically to "nothing
 * is missing". Importing the objects means a parse failure throws
 * instead of silently passing, and the comparison is set algebra over
 * real keys.
 */
import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';
import en from '../../src/lib/i18n/en';
import zhCN from '../../src/lib/i18n/zh-CN';

const here = dirname(fileURLToPath(import.meta.url));

/** `AppError::kind()` in the Tauri crate — the identifiers the Svelte side receives. */
const RUST_ERROR_KIND_FILE = resolve(
  here,
  '../../src-tauri/src/error.rs'
);

/**
 * Match one `kind()` arm.
 *
 * [FACT] Both arms that carry data use a tuple — `AppError::Bind(_)`
 * and `AppError::InvalidRepoName(_)` — so the pattern accepts an
 * optional parenthesised payload. A missing payload group is *not*
 * accepted silently: `expect` below turns "the shape changed" into a
 * readable failure instead of a silently shortened list.
 *
 * Deliberately not tolerant of reformatting. If someone reformats the
 * match, this gate should fail loudly and be updated, rather than quietly
 * stop seeing kinds.
 */
function rustErrorKinds(): string[] {
  const source = readFileSync(RUST_ERROR_KIND_FILE, 'utf8');

  // Sanity: the file must actually contain a `kind()` fn. Without this,
  // a moved or renamed function would make every match return zero and
  // this gate would pass while checking nothing.
  expect(
    source,
    `${RUST_ERROR_KIND_FILE} no longer defines AppError::kind()`
  ).toMatch(/pub fn kind\(&self\)/);

  const body = source.slice(source.indexOf('pub fn kind(&self)'));
  // Stop at the end of the match, not the end of the impl block: the
  // test module below also has `=>` arms with string literals.
  const matchEnd = body.indexOf('\n    }');
  expect(matchEnd, 'could not locate the end of the kind() match').toBeGreaterThan(0);

  const arms = body.slice(0, matchEnd).matchAll(
    /AppError::\w+(?:\([^)]*\))?\s*=>\s*"(\w+)"/g
  );

  const kinds = [...arms].map((m) => m[1]);

  // Reverse sanity: a count of zero must never pass. This is the failure
  // mode the project has hit before — a pattern that matches nothing is
  // indistinguishable from "nothing is missing".
  expect(
    kinds.length,
    'no error kinds were parsed; the regex is wrong, not the catalogue'
  ).toBeGreaterThan(0);

  return kinds;
}

function catalogueErrorKeys(catalogue: Readonly<Record<string, string>>): Set<string> {
  return new Set(
    Object.keys(catalogue)
      .filter((k) => k.startsWith('errors.kind.'))
      .map((k) => k.slice('errors.kind.'.length))
  );
}

describe('error-kind catalogue parity (Rust ↔ i18n)', () => {
  it('parses a plausible number of error kinds out of the Rust source', () => {
    // [FACT] A lower bound rather than an exact count, so adding a
    // variant does not break this case — that is what the next case is
    // for. This one exists only to catch a regex that silently matches
    // nothing.
    const kinds = rustErrorKinds();
    expect(kinds.length).toBeGreaterThanOrEqual(10);
    // No duplicates: two arms returning the same identifier would make a
    // later Set comparison lossy.
    expect(new Set(kinds).size).toBe(kinds.length);
  });

  it('gives every Rust error kind a string in both catalogues', () => {
    const kinds = rustErrorKinds();
    const enKeys = catalogueErrorKeys(en);
    const zhKeys = catalogueErrorKeys(zhCN);

    const missingEn = kinds.filter((k) => !enKeys.has(k));
    const missingZh = kinds.filter((k) => !zhKeys.has(k));

    expect(
      missingEn,
      'no en string; the page falls back to errors.routeTitle and says nothing useful'
    ).toEqual([]);
    expect(
      missingZh,
      'no zh-CN string; a Chinese user sees the key or the generic heading'
    ).toEqual([]);
  });

  it('has no catalogue entry for an error kind the backend cannot produce', () => {
    // [FACT] The other direction. A stale entry is not user-visible on
    // its own, but it is how a *renamed* kind survives: the old string
    // stays, the gate above passes, and the rename is invisible until a
    // user hits the new variant.
    const kinds = new Set(rustErrorKinds());
    const orphansEn = [...catalogueErrorKeys(en)].filter((k) => !kinds.has(k));
    const orphansZh = [...catalogueErrorKeys(zhCN)].filter((k) => !kinds.has(k));

    expect(orphansEn, 'stale en entry').toEqual([]);
    expect(orphansZh, 'stale zh-CN entry').toEqual([]);
  });

  it('keeps the placeholder syntax identical across locales', () => {
    // A `{n}` left untranslated on one side renders as a literal brace in
    // that language. Existing per-page cases cover their own keys; this
    // covers the whole error catalogue at once.
    const placeholders = (s: string): string[] => (s.match(/\{[a-z]+\}/g) ?? []).sort();

    for (const kind of catalogueErrorKeys(en)) {
      const enValue = en[`errors.kind.${kind}`];
      const zhValue = zhCN[`errors.kind.${kind}`];
      expect(enValue, kind).toBeDefined();
      expect(zhValue, kind).toBeDefined();
      if (enValue === undefined || zhValue === undefined) continue;
      expect(placeholders(zhValue), `placeholders differ for ${kind}`).toEqual(
        placeholders(enValue)
      );
    }
  });
});
