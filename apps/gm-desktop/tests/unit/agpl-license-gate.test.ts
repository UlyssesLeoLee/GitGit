/**
 * Gates on the AGPL-3.0 §4(a) licence requirement.
 *
 * ## What was wrong
 *
 * `tauri.conf.json` had `"resources": []` and no `licenseFile` key at all, so
 * nothing carried the licence into any installer. Measured rather than assumed:
 * release `404746754` (tag v0.1.0) was downloaded and its `.deb` unpacked by
 * hand — a `.deb` is an `ar` archive, and this machine has neither `ar` nor
 * `dpkg-deb`, so the members were parsed directly. The package contained
 * exactly five files:
 *
 *     /usr/bin/gm-desktop
 *     /usr/share/applications/gitgit Desktop.desktop
 *     /usr/share/icons/hicolor/{32x32,128x128,512x512}/apps/gm-desktop.png
 *
 * and no licence, copyright, copying or notice file of any kind.
 *
 * §4(a) requires a copy of the Licence with every copy of the Program. Shipping
 * a binary under AGPL-3.0 without it is a compliance defect, and it is one
 * that no existing gate could see: `verify-bundle.sh` never mentioned the word
 * licence at all.
 *
 * ## Which mechanism reaches which target
 *
 * Read out of the bundler source, not guessed. `license_file` is read by
 * `bundle/windows/nsis`, `bundle/windows/msi` and `bundle/macos/dmg`; it is
 * read by neither `bundle/linux/debian` nor `bundle/linux/appimage`.
 * `bundle.resources` is read by deb, rpm, ios, msi and nsis; the AppImage
 * bundler reads neither. So there is no single config key that covers every
 * target, and each target has to be handled by the mechanism that actually
 * reaches it:
 *
 * | target | mechanism | asserted by |
 * |---|---|---|
 * | deb | `bundle.linux.deb.files` | `verify-bundle.sh` |
 * | msi | `bundle.licenseFile` + the repo's own WiX template | `verify-msi.ps1` |
 * | dmg | `bundle.licenseFile` (create-dmg `--eula`) | measured, not asserted |
 * | appimage | none exists | measured, not asserted |
 */
import { describe, expect, it } from 'vitest';
import { existsSync, readFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const REPO_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..', '..', '..', '..');
const CONF = join(REPO_ROOT, 'apps', 'gm-desktop', 'src-tauri', 'tauri.conf.json');
const BUNDLE_SH = join(REPO_ROOT, 'apps', 'gm-desktop', 'scripts', 'verify-bundle.sh');
const MSI_PS1 = join(REPO_ROOT, 'apps', 'gm-desktop', 'scripts', 'verify-msi.ps1');
const WIX_DIR = join(REPO_ROOT, 'apps', 'gm-desktop', 'src-tauri', 'wix');
const WIX_MAIN = join(WIX_DIR, 'main.wxs');
const WIX_PERUSER = join(WIX_DIR, 'main-peruser.wxs');

type DebFiles = Record<string, string>;

const conf = JSON.parse(readFileSync(CONF, 'utf8')) as {
  bundle?: {
    licenseFile?: string;
    copyright?: string;
    targets?: string[];
    /** Either a list of paths or a source -> destination map. */
    resources?: string[] | DebFiles;
    linux?: { deb?: { depends?: string[]; files?: DebFiles } };
  };
  version?: string;
};

/** Full-line comments only — see the note on stripShellComments below. */
const stripShellComments = (text: string): string =>
  text
    .split('\n')
    .filter((line) => !/^\s*#/.test(line))
    .join('\n');

const bundleCode = stripShellComments(readFileSync(BUNDLE_SH, 'utf8'));
const msiCode = stripShellComments(readFileSync(MSI_PS1, 'utf8'));
/** The MSI file with its prose intact — used only where prose IS the requirement. */
const msiRaw = readFileSync(MSI_PS1, 'utf8');
const wixMain = readFileSync(WIX_MAIN, 'utf8');
const wixPerUser = readFileSync(WIX_PERUSER, 'utf8');

/**
 * The digest the generator recorded in the per-user template's header.
 *
 * Python reads the source in universal-newline mode, so `\r\n` and a lone `\r`
 * both arrive as `\n` and are what gets hashed. Node reads raw bytes, so the
 * same normalisation has to happen here or the two digests would disagree on a
 * checkout with CRLF line endings.
 */
const sourceDigest = (path: string): string =>
  createHash('sha256')
    .update(readFileSync(path, 'utf8').replace(/\r\n/g, '\n').replace(/\r/g, '\n'))
    .digest('hex');

describe('the bundle is configured to carry the licence', () => {
  it('maps the licence into the package data dir through deb.files', () => {
    // `[FACT]` `crates/tauri-bundler/src/bundle/linux/debian.rs` builds its data
    // directory from `copy_resource_files`, binaries, icons + desktop entry, an
    // optional `deb.changelog`, and
    // `fs_utils::copy_custom_files(&settings.deb().files, &data_dir)`.
    // `licenseFile` is in none of them, and run 37642251095 measured it: with
    // `licenseFile` set, the built .deb still held exactly the same five files.
    const debFiles = conf.bundle?.linux?.deb?.files;
    expect(
      debFiles,
      'bundle.linux.deb.files is the only deb mechanism that reaches the package data ' +
        'directory. Without it the licence is not in the .deb at all.'
    ).toBeTruthy();

    // `[FACT]` The map is DESTINATION -> SOURCE. Run 37644440552 wrote it the
    // other way round and the build failed with
    // `Failed to copy custom files: "usr/share/doc/gitgit-desktop/copyright" does not
    // exist` — that failure is what identified the direction empirically. Asserting
    // on `Object.values()` would encode the reversed map and pass on a package the
    // bundler cannot build.
    const destinations = Object.keys(debFiles!);
    expect(destinations).toEqual(
      expect.arrayContaining([expect.stringMatching(/^\/?usr\/share\/doc\/.+\/(copyright|LICEN[CS]E)$/)])
    );

    // A relative destination would land the licence somewhere inside the .deb that
    // no human will ever look in, so the key has to be a package-absolute path.
    for (const dest of destinations) {
      expect(
        /^\/?usr\//.test(dest),
        `deb.files destination '${dest}' is not a package-absolute path`
      ).toBe(true);
    }

    // And every SOURCE has to BE the repository's licence, not something that
    // merely mentions it. A title-only check is satisfied by the repository
    // README, which contains "GNU AFFERO GENERAL PUBLIC LICENSE" twice - that
    // decoy was caught by mutation, not by reading the assertion.
    const licenceBytes = readFileSync(join(REPO_ROOT, 'LICENSE'));
    for (const src of Object.values(debFiles!)) {
      const abs = resolve(dirname(CONF), src);
      expect(existsSync(abs), `deb.files source '${src}' does not exist, so the build fails`).toBe(true);
      expect(
        readFileSync(abs).equals(licenceBytes),
        `deb.files source '${src}' is not the repository's LICENSE file, so the package would ship ` +
          'something that is not the Licence'
      ).toBe(true);
    }
  });

  it('keeps bundle.resources empty, because a custom WiX template cannot carry it', () => {
    // Run 37644440611: a non-empty `bundle.resources` failed the MSI build inside
    // `WixTools314/light.exe`. The upstream bundler renders each resource as
    // `<Component ... KeyPath="yes"><File .../></Component>`, and the combination
    // that produced is what WiX rejected - `{{resources}}` is present in this
    // repository's template, so the failure was in the generated XML, not in a
    // missing template slot.
    //
    // `expect(resources).toBeTruthy()` would NOT catch that: `[]` is truthy in
    // JavaScript. Emptiness is the requirement here, so emptiness is what gets
    // asserted.
    expect(
      conf.bundle?.resources,
      'bundle.resources is where the licence used to be routed. It must stay empty: a ' +
        'non-empty list breaks the MSI build (run 37644440611). Use deb.files instead.'
    ).toEqual([]);
  });

  it('keeps bundle.licenseFile resolvable without counting on it', () => {
    // The key stays in the config because the MSI and dmg bundlers DO read it
    // (msi/mod.rs converts it to RTF and hands it to the template as
    // `{{license}}`; dmg/mod.rs passes it to create-dmg as `--eula`). What it must
    // not become is load-bearing for the .deb, whose bundler never reads it.
    const declared = conf.bundle?.licenseFile;
    expect(declared, 'no licenseFile to resolve').toBeTruthy();
    // Resolved relative to the config's own directory, which is what the bundler
    // does. An absolute path here would also "resolve" on this machine and fail on
    // a runner, so the check is deliberately relative.
    const abs = resolve(dirname(CONF), declared!);
    expect(existsSync(abs), `licenseFile '${declared}' does not resolve to a file`).toBe(true);
    expect(
      readFileSync(abs, 'utf8'),
      'the file licenseFile points at is not the AGPL text'
    ).toMatch(/GNU AFFERO GENERAL PUBLIC LICENSE/);
  });

  it('never tells a reader to go fix bundle.licenseFile', () => {
    // The repository already made this mistake once: `verify-bundle.sh` named
    // `licenseFile` as the thing to repair, on the strength of the Tauri reference
    // doc, and a reader who followed that advice would have changed nothing. A
    // remediation string that names a key measured to be a no-op for that target
    // is a defect in its own right, so the string is asserted — it is executable
    // text, not a comment.
    const remediation = bundleCode.match(/die "\$\{ARTIFACT_NAME\} ships no licence[\s\S]*?"/);
    expect(
      remediation,
      'could not find the no-licence failure message in verify-bundle.sh'
    ).not.toBeNull();
    expect(remediation![0]).toMatch(/bundle\.linux\.deb\.files/);
    expect(
      remediation![0].replace(/not bundle\.licenseFile/g, ''),
      'the failure message still points a reader at bundle.licenseFile, which run ' +
        '37642251095 measured to do nothing for .deb'
    ).not.toMatch(/bundle\.licenseFile/);
  });
});

describe('the .deb verifier would notice the omission', () => {
  it('reads the package contents rather than the config', () => {
    // Checking tauri.conf.json alone proves nothing: the whole question is
    // whether the licence survived into the built artifact.
    expect(bundleCode).toMatch(/dpkg-deb --contents/);
  });

  it('fails closed when no licence-bearing file is present', () => {
    expect(bundleCode).toMatch(/licence, copyright, copying or notice file/);
    // AGPL cited by section, so the requirement is checkable by a reader who
    // does not take this repository's word for it.
    expect(bundleCode).toMatch(/AGPL-3\.0/);
    expect(bundleCode).toMatch(/4\(a\)/);
  });

  it('does not pin one directory, which would be a guess about the bundler', () => {
    // Where a .deb "appropriate" licence copy lives is the bundler's decision.
    // A check pinned to one path goes red on a compliant package the moment the
    // bundler changes its mind, and such a gate gets switched off.
    expect(
      bundleCode,
      'the check names specific paths rather than requiring a licence file to exist somewhere'
    ).not.toMatch(/usr\/share\/doc\/[a-z-]+\/copyright/);
  });

  it('ignores files that are merely adjacent to a licence name', () => {
    // `README` in a `doc/` directory is not a licence. A pattern loose enough to
    // accept it would let a package pass without conveying anything.
    expect(bundleCode).toMatch(/\(licen\[cs\]e\|copying\|copyright\|notice\)/);
  });
});

describe('the MSI installer carries the licence, and the verifier proves it', () => {
  /**
   * This block used to assert the opposite: that the MSI verifier DECLARED the
   * licence check out of scope, on the stated grounds that a hand-written WiX
   * template receives no licence material.
   *
   * That was wrong, and it was wrong in a way that would have kept a real
   * compliance gap open forever. `crates/tauri-bundler/src/bundle/windows/msi/
   * mod.rs` inserts a `license` key into the Handlebars data map, and the SAME
   * map is rendered into a custom template — the bundler registers either the
   * repository's `wix/main.wxs` or its own built-in copy, then renders with the
   * same `data`. `{{license}}` was therefore available all along, and this
   * repository's template was already using it for `WixUILicenseRtf`.
   *
   * The omission went unnoticed because of a measurement artefact rather than a
   * bundler limitation: `msiexec /a` extracts the package's FILE table, and
   * the RTF lives in the installer's UI, not in that table. Run 37642251415
   * reported "no licence" while the licence was in fact being displayed at
   * install time.
   */
  it('installs a licence file next to the executable, in both scopes', () => {
    // The licence is a file in the SAME component as the executable, so that it
    // installs and uninstalls with it and so that the per-user variant keeps
    // its HKCU keypath (ICE38) untouched.
    expect(
      wixMain,
      'the per-machine WiX template installs no licence file, so the MSI ships the ' +
        'executable without a copy of the Licence'
    ).toMatch(/<File Id="LicenseFile" Source="\{\{license\}\}"/);
    expect(
      wixPerUser,
      'the per-user WiX template installs no licence file'
    ).toMatch(/<File Id="LicenseFile" Source="\{\{license\}\}"/);
  });

  it('installs the licence only when a licence was configured', () => {
    // Without the guard, `{{license}}` renders empty and WiX gets `Source=""`,
    // which fails the build for anyone who removes `bundle.licenseFile`.
    expect(wixMain).toMatch(/\{\{#if license\}\}[\s\S]*?\{\{\/if\}\}/);
  });

  it('the per-user template was generated from the source as it stands now', () => {
    // `main-peruser.wxs` is GENERATED by `scripts/render-wix-template.py`, and
    // the build only checks that it EXISTS — it does not regenerate it. So an
    // edit to `main.wxs` that nobody re-rendered would silently produce a
    // per-user installer that differs from the per-machine one. The generator
    // records the digest of its source in the generated file's header for
    // exactly this reason.
    expect(
      wixPerUser,
      'the per-user template records no SOURCE-SHA256, so a stale generated file ' +
        'cannot be detected'
    ).toMatch(/SOURCE-SHA256: [0-9a-f]{64}/);
    expect(
      wixPerUser,
      'the per-user template was generated from an OLDER main.wxs. Re-run ' +
        'apps/gm-desktop/scripts/render-wix-template.py.'
    ).toContain(`SOURCE-SHA256: ${sourceDigest(WIX_MAIN)}`);
  });

  it('the verifier asserts the licence rather than declaring it out of scope', () => {
    expect(
      msiRaw,
      'the MSI verifier still declares the licence check out of scope, which is ' +
        'the claim that was measured to be wrong'
    ).not.toMatch(/NOT CHECKED HERE, DELIBERATELY/);
    // The check itself, in executable code: a licence-bearing file has to come
    // out of the administrative install.
    expect(msiCode).toMatch(/\^\(licen\[cs\]e\|copying\|copyright\|notice\)/);
    // And a file with a licence-shaped name is not a licence, so the contents
    // are checked too - by TWO phrases, because this repository's README
    // contains the licence title and would satisfy a title-only check.
    expect(msiCode).toMatch(/GNU AFFERO GENERAL PUBLIC LICENSE/);
    expect(msiCode).toMatch(/TERMS AND CONDITIONS/);
    expect(msiCode).toMatch(/MinLicenceBytes/);
  });

  it('records the correction, with the measurement that produced it', () => {
    // A future reader who "fixes" this by removing the check needs to know that
    // it worked, and that the reason it was missing was a measurement artefact.
    expect(msiRaw).toMatch(/37642251415/);
    expect(msiRaw).toMatch(/wix\/main\.wxs/);
    expect(msiRaw).toMatch(/4\(a\)/);
  });
});

describe('the two targets with no licence mechanism are measured, not assumed', () => {
  /**
   * The AppImage bundler reads neither `bundle.licenseFile` nor
   * `bundle.resources` — `crates/tauri-bundler/src/bundle/linux/appimage/`
   * (both `mod.rs` and `linuxdeploy.rs`) has no reference to either. There is no
   * configuration that puts a licence in an AppImage.
   *
   * A gate asserting one could never pass, and a gate that cannot be satisfied
   * gets switched off, at which point it protects nothing and blocks every
   * merge. So the AppImage is MEASURED instead: the verify step extracts the
   * image and reports what it found. The dmg gets the same treatment for the
   * part that is not observable — create-dmg is handed the licence as `--eula`,
   * and where it leaves it is not something to guess at.
   */
  it('the AppImage verify step extracts the image and reports what it carries', () => {
    expect(bundleCode).toMatch(/--appimage-extract/);
    expect(bundleCode).toMatch(/NO LICENCE FILE in the AppImage/);
    // The report has to say WHY it can be empty, or a reader will treat the
    // absence as a mystery rather than as a known gap.
    expect(bundleCode).toMatch(/reads neither/);
  });

  it('the dmg verify step reports what the mounted image carries', () => {
    expect(bundleCode).toMatch(/--eula/);
    expect(bundleCode).toMatch(/no licence file inside the mounted image/);
  });
});

describe('both scripts are shell/powershell text a pattern can misread', () => {
  it('stripping comments left the real commands in place', () => {
    // The stripping above is a transformation, and a transformation that ate the
    // code would make every assertion above vacuously true.
    expect(bundleCode).toMatch(/dpkg-deb/);
    expect(msiCode).toMatch(/Get-ChildItem/);
    expect(
      bundleCode.split('\n').filter((l) => l.trim() && !/^\s*#/.test(l)).length,
      'stripping left no executable lines in verify-bundle.sh'
    ).toBeGreaterThan(100);
  });
});

/*
 * The path-matching expressions were exercised directly rather than trusted:
 *
 *   against the real published .deb (release 404746754), which has no licence
 *   file: no match — the defect this change fixes is real, not hypothesised;
 *   against four layouts a bundler might plausibly choose
 *   (/usr/share/doc/<pkg>/copyright, /usr/share/licenses/<pkg>/LICENSE,
 *   /usr/share/doc/<pkg>/LICENSE.txt, /usr/share/<name>/LICENSE): all matched;
 *   against three decoys (README, a directory named `licenses`, libfoo.so):
 *   correctly ignored. The MSI pattern was checked the same way.
 *
 * [FACT] `bundle.licenseFile` is NOT a licence-carrying mechanism for every
 * target, and an earlier version of this gate treated it as one. It is absent
 * from the deb data-directory construction in
 * `crates/tauri-bundler/src/bundle/linux/debian.rs` (run 37642251095 measured
 * the resulting .deb to be byte-for-byte the same five files). It IS read by
 * the MSI bundler, which converts it to RTF and hands it to the template as
 * `{{license}}`, and by the dmg bundler, which passes it to create-dmg as
 * `--eula`.
 *
 * [INFERENCE] The AppImage still ships no licence, and that is now measured
 * rather than assumed: the verify step extracts the image and reports the
 * file list. Closing that gap needs either a bundler change or a post-build
 * repack, neither of which this change attempts.
 */
