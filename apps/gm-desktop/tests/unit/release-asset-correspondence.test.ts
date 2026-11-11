/**
 * Gates on the release's manifest-to-asset correspondence check.
 *
 * ## Why this exists
 *
 * Every manifest shipped on a release names the files it describes:
 * `deb-manifest.json` says `"file": "gitgit Desktop_0.1.0_amd64.deb"`. Nothing in
 * the release pipeline checks that such a file is actually downloadable.
 *
 * That gap is not hypothetical, because GitHub rewrites asset names — a space
 * becomes a dot — and `softprops/action-gh-release` papers over it in a
 * BEST-EFFORT path. From its `src/github.ts`:
 *
 *   const maybeRestoreAssetLabel = async (uploadedAsset) => {
 *     ...
 *     try { return await updateAssetLabel(uploadedAsset.id); }
 *     catch (error) { console.warn(...); return uploadedAsset; }
 *   };
 *
 * Every failure mode there is caught, logged at `warn`, and returns the asset
 * with no label. So the correspondence between a manifest's `file` and a real
 * download is normally present and silently absent when that call fails, and
 * nothing downstream would notice.
 *
 * Measured on release 404746754 (tag v0.1.0): all 7 installers carry a `label`
 * equal to the `file` in their manifest, while `name` is GitHub's rewritten
 * form. So a checker that only compared `name` would have reported 7 of 7
 * mismatches against a perfectly healthy release.
 */
import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { parse } from 'yaml';

const REPO_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..', '..', '..', '..');
const DOC = parse(
  readFileSync(join(REPO_ROOT, '.github', 'workflows', 'release.yml'), 'utf8')
) as {
  jobs: Record<
    string,
    {
      steps?: Array<{
        name?: string;
        id?: string;
        uses?: string;
        run?: string;
        env?: Record<string, string>;
        with?: Record<string, unknown>;
      }>;
    }
  >;
};

const job = DOC.jobs['draft-release'];
const steps = job?.steps ?? [];

const uploadStep = steps.find((s) => s.uses?.startsWith('softprops/action-gh-release'));
const checkStep = steps.find((s) => /manifests describe|is really on the draft/.test(s.name ?? ''));

/**
 * The check's body with comments stripped.
 *
 * Same reason as every other gate that reads a shell body here: these steps
 * explain themselves at length, and a check that matches its own explanation
 * both false-fires on the prose and can be SATISFIED by it. A gate that reads
 * green because of a sentence is worse than no gate.
 */
const body = (checkStep?.run ?? '')
  .split('\n')
  .map((line) => line.replace(/(^|\s)#.*$/, ''))
  .join('\n');

describe('draft-release — the upload step publishes an id the check can use', () => {
  it('found both steps', () => {
    expect(uploadStep, 'no softprops/action-gh-release step in draft-release').toBeDefined();
    expect(
      checkStep,
      'no step verifies that the manifests describe assets actually on the release'
    ).toBeDefined();
  });

  it('the upload step has an id, because its output is the only way to read the asset list back', () => {
    expect(
      uploadStep?.id,
      'the upload step publishes `id`, `url`, `upload_url` and `assets`. Without an id the ' +
        'check has nothing to query and cannot exist.'
    ).toBeTruthy();
  });

  it('the check runs AFTER the upload, not before it', () => {
    const uploadAt = steps.indexOf(uploadStep!);
    const checkAt = steps.indexOf(checkStep!);
    expect(
      checkAt,
      'a correspondence check placed before the upload would verify an empty release'
    ).toBeGreaterThan(uploadAt);
  });

  it('reads the release id from the upload step output, not from a literal', () => {
    const env = checkStep?.env ?? {};
    expect(String(env.RELEASE_ID ?? '')).toMatch(/steps\.[A-Za-z0-9_-]+\.outputs\.id/);
    expect(
      String(env.RELEASE_ID ?? ''),
      'a hardcoded release id would verify a different release than the one just built'
    ).not.toMatch(/outputs\.url|outputs\.upload_url|[0-9]{6,}/);
  });
});

describe('draft-release — the check matches name OR label, because only one is stable', () => {
  it('accepts an asset whose name was rewritten but whose label was restored', () => {
    // The measured shape: name rewritten, label equal to the manifest's file.
    expect(body).toMatch(/\$1\s*==\s*want\s*\|\|\s*\$2\s*==\s*want/);
  });

  it('fails closed when a manifest entry matches nothing', () => {
    expect(body).toMatch(/no uploaded asset carries that name or label/);
    expect(body, 'the unmatched branch must increment a counter that is checked').toMatch(
      /missing=\$\(\(missing\s*\+\s*1\)\)/
    );
    expect(body).toMatch(/exit 1/);
  });

  it('refuses to run when the upload step published no id', () => {
    expect(
      body,
      'an empty RELEASE_ID would make `gh api` query /releases//assets and report a failure ' +
        'that reads like a network problem rather than a wiring mistake'
    ).toMatch(/if \[ -z "\$RELEASE_ID" \]/);
  });
});

describe('draft-release — both manifest shapes are read', () => {
  /**
   * The per-target manifests (`deb`, `dmg`, `appimage`) carry `artifacts`; the
   * MSI manifest carries `variants`. They are not the same key, and there is no
   * single array that holds both.
   *
   * Reading one of them yields 3 of 7 entries checked and a green run, because
   * "not checked" and "checked and correct" are indistinguishable from the
   * outside. Mutation B below is exactly that failure, and it is caught only
   * because of the count assertion in the next describe block.
   */
  it('reads `artifacts` and `variants` in one expression', () => {
    expect(body).toMatch(/\.artifacts/);
    expect(body).toMatch(/\.variants/);
  });

  it('globs every staged manifest, not a named one', () => {
    // Naming manifests means a new platform's manifest is silently unchecked.
    expect(body).toMatch(/-name '\*-manifest\.json'/);
    expect(body).not.toMatch(/-name 'msi-manifest\.json'|-name 'deb-manifest\.json'/);
  });
});

describe('draft-release — the counts are compared, so a half-check cannot pass', () => {
  /**
   * `checked` is every manifest entry; `installers` is every .msi/.dmg/.deb/.AppImage
   * on the release. They must be the same set.
   *
   * Without this, a checker that reads only `artifacts` verifies 3 files and
   * exits 0 while 4 MSI variants shipped undescribed. That is the entire class
   * of defect this step is for, so the count is asserted rather than assumed.
   */
  it('fails when nothing was checked at all', () => {
    expect(body).toMatch(/no manifest entry was found to check/);
    expect(body).toMatch(/if \[ "\$checked" -eq 0 \]/);
  });

  it('fails when the manifests and the release disagree on how many files there are', () => {
    expect(body).toMatch(/the two sets must be the same/);
    expect(body).toMatch(/if \[ "\$checked" -ne "\$installers" \]/);
  });

  it('fails when the release carries no installer at all', () => {
    expect(body).toMatch(/if \[ "\$installers" -eq 0 \]/);
  });
});

/*
 * Mutation evidence, run against the ACTUAL `run:` body extracted from
 * release.yml rather than a hand-copy, with the API response replayed from the
 * real release 404746754.
 *
 * Baseline, healthy release:  `ok all 7 manifest entries resolve`, exit 0.
 *
 * A. Labels stripped, which is what `maybeRestoreAssetLabel` leaves behind when
 *    its API call fails: `::error::7 of 7 manifest entries do not correspond to
 *    any uploaded asset`, exit 1.
 * B. The expression narrowed to `.artifacts` only — the shape trap described
 *    above: `::error::manifests describe 3 files but the release carries 7
 *    installers`, exit 1. Caught by the count assertion, not by the per-file
 *    match, which is the whole reason the count is there.
 * C. One installer absent from the release: `::error::5 of 7 manifest entries do
 *    not correspond`, exit 1.
 *
 * The first run of this harness reported A and C as GREEN. The cause was in the
 * harness, not the gate: it copied each mutated input to `ghbin/gh.assets` while
 * the `gh` stand-in read `gh.assets`, so no mutation ever changed what the step
 * saw. A mutation harness that does not vary the input is worse than none — it
 * manufactures evidence that the thing under test was fed a failure.
 */