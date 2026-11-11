/**
 * Release-metadata gate: the shipped version must be stated once.
 *
 * ## Why this exists
 *
 * [FACT] The product's version is written out in three files that no tool
 * keeps in sync:
 *
 *   - `apps/gm-desktop/package.json`      (`version`)
 *   - `apps/gm-desktop/src-tauri/tauri.conf.json` (`version`, becomes the
 *     installer filename and the Windows product version)
 *   - `Cargo.toml`                        (`package.version`, baked into the
 *     `gitgit` binary)
 *
 * All three currently read `0.1.0`. That agreement is a coincidence, not
 * an invariant: nothing fails if one is edited.
 *
 * [FACT] `gm-desktop-bundle.yml` states in its own header that this
 * workflow is "a build-and-prove gate, not a publishing pipeline" and
 * deliberately excludes release. So there is no release path today that
 * would catch a mismatch, and no tag has ever been cut for a version —
 * the repository has exactly one tag, `archive/2026-08-26-pre-mvp-simplify`,
 * which is not a release.
 *
 * A user who installs 0.1.0 and then reports a bug against 0.1.1 is
 * unanswerable when the three files disagree, and Tauri's bundler will
 * happily produce a 0.1.0-named artifact containing 0.1.1 code.
 *
 * ## Why a test and not a script
 *
 * The desktop test suite runs in `gm-desktop.yml`'s `build` job, on every
 * pull request that touches `apps/gm-desktop/**`, so a case here is checked
 * on the same trigger a version bump lands on. It fails the PR that
 * introduces the drift rather than the release that ships it.
 *
 * `release.yml` is a fifth workflow, added after this file was written, and
 * it is deliberately not part of that claim: it is tag-triggered, not
 * PR-triggered. It does check the tag against `tauri.conf.json` before it
 * builds anything, which is a second line of defence — but it cannot be the
 * one that stops the drift entering the tree, which is what a case in this
 * file is for.
 */
import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';

const here = dirname(fileURLToPath(import.meta.url));
const appRoot = resolve(here, '../..');
const repoRoot = resolve(appRoot, '../..');

function readJson(path: string): Record<string, unknown> {
  return JSON.parse(readFileSync(path, 'utf8')) as Record<string, unknown>;
}

/**
 * Version as each of the three files states it.
 *
 * [FACT] Read through a JSON parser for the two JSON files rather than by
 * regex, so a match against a *comment* or an unrelated field cannot
 * masquerade as a version read. A substring search over
 * `tauri.conf.json` would also hit `bundle.macOS.minimumSystemVersion`
 * or any future `version`-ish key, which is the wrong property to assert.
 */
function declaredVersions(): Record<string, string> {
  const pkg = readJson(resolve(appRoot, 'package.json'));
  const tauri = readJson(resolve(appRoot, 'src-tauri/tauri.conf.json'));

  // `tauri.conf.json` allows a `version` to be null when the version is
  // injected at build time. That is a legitimate configuration, so it is
  // asserted explicitly rather than silently producing "null".
  const tauriVersion = tauri['version'];

  // `Cargo.toml` has no parser in the toolchain available to a unit test,
  // so the `package.version` line is read directly. It is anchored to the
  // start of a line under `[package]` because `rust-version` and the many
  // `version = ...` entries in `[dependencies]` also match a loose
  // pattern.
  const cargoToml = readFileSync(resolve(repoRoot, 'Cargo.toml'), 'utf8');
  const packageSection = cargoToml.split(/^\[package\]\s*$/m)[1] ?? '';
  const cargoMatch = packageSection.match(/^version\s*=\s*"([^"]+)"/m);
  if (cargoMatch === null) {
    throw new Error('could not read package.version from the root Cargo.toml');
  }

  return {
    'apps/gm-desktop/package.json': String(pkg['version']),
    'apps/gm-desktop/src-tauri/tauri.conf.json': String(tauriVersion),
    'Cargo.toml': cargoMatch[1],
  };
}

describe('release metadata', () => {
  it('declares the same version in every file that carries one', () => {
    const versions = declaredVersions();
    const distinct = [...new Set(Object.values(versions))];

    const report = Object.entries(versions)
      .map(([file, v]) => `  ${file}: ${v}`)
      .join('\n');

    expect(
      distinct.length,
      `the shipped version is stated differently in different files:\n${report}`
    ).toBe(1);
  });

  it('does not let tauri.conf.json defer its version to build time', () => {
    // [FACT] `"version": null` is legal for Tauri and is how some teams
    // inject the version from CI. It is not acceptable *here*: nothing in
    // this repository injects it, and a null version makes the bundler fall
    // back to something unspecified, which is precisely the ambiguity this
    // file exists to prevent. If a real injection step is added, this case
    // is the one to revisit.
    const tauri = readJson(resolve(appRoot, 'src-tauri/tauri.conf.json'));
    expect(
      tauri['version'],
      'tauri.conf.json version must be a literal, or the installer filename is unspecified'
    ).toBeTypeOf('string');
  });

  it('carries a version the installer filenames can use', () => {
    // [FACT] WiX and Debian both constrain the version string format. A
    // pre-release suffix like `0.1.0-rc.1` is accepted by cargo but is not
    // a valid MSI ProductVersion, and Tauri's `deb` target additionally
    // requires the version to sort after any released version. Catching
    // this in a unit test costs nothing; catching it in a Windows release
    // build costs a 15-minute job.
    const versions = Object.values(declaredVersions());
    for (const v of versions) {
      expect(
        v,
        'a version must be dotted numeric (major.minor.patch) with no suffix'
      ).toMatch(/^\d+\.\d+\.\d+$/);
    }
  });

  it('declares a license the repository actually ships', () => {
    // [FACT] The top-level LICENSE is the AGPL-3.0 text and both manifests
    // say so. A mismatch here is not cosmetic: the license field is what
    // `cargo package` and the crate index record, and it is the field a
    // downstream consumer reads before deciding whether they may use the
    // code at all.
    const cargoToml = readFileSync(resolve(repoRoot, 'Cargo.toml'), 'utf8');
    const packageSection = cargoToml.split(/^\[package\]\s*$/m)[1] ?? '';
    const license = packageSection.match(/^license\s*=\s*"([^"]+)"/m);
    expect(license, 'root Cargo.toml must declare a license').not.toBeNull();

    const tauriToml = readFileSync(resolve(appRoot, 'src-tauri/Cargo.toml'), 'utf8');
    const tauriPackage = tauriToml.split(/^\[package\]\s*$/m)[1] ?? '';
    const tauriLicense = tauriPackage.match(/^license\s*=\s*"([^"]+)"/m);
    expect(tauriLicense, 'src-tauri/Cargo.toml must declare a license').not.toBeNull();

    expect(tauriLicense?.[1], 'the two manifests disagree about the license').toBe(
      license?.[1]
    );

    // The license identifier must correspond to the file that is actually
    // in the repository root.
    const licenseFile = readFileSync(resolve(repoRoot, 'LICENSE'), 'utf8');
    const id = license?.[1] ?? '';
    if (id.toUpperCase().includes('AGPL')) {
      expect(
        licenseFile,
        'the manifest claims AGPL but LICENSE is not the AGPL text'
      ).toMatch(/AFFERO GENERAL PUBLIC LICENSE/i);
    }
  });
});
