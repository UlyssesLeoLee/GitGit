/**
 * Gates on the AGPL-3.0 §4 licence requirement.
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
 * Section 4 requires a copy of the Licence with every copy of the Program. Shipping
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
    linux?: {
      deb?: { depends?: string[]; files?: DebFiles };
      appimage?: { files?: DebFiles };
    };
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
/** The bundle script with its prose intact — used only where prose IS the requirement. */
const bundleRaw = readFileSync(BUNDLE_SH, 'utf8');
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

/**
 * The body of one `case` branch of verify-bundle.sh.
 *
 * The assertions below are about what a branch DOES, so they are made against
 * the branch rather than against the file as a whole: a phrase printed by a
 * different branch must not be able to satisfy an assertion about this one.
 * The branch is labelled `<label>)` at two-space indent and ends at its `;;`.
 *
 * The newline after the label is part of the pattern on purpose. The argument
 * parser has branches of the same names (`  deb)      EXT="deb" ;;`), and a
 * pattern that matched those would assert against the wrong code.
 */
const caseBranch = (code: string, label: string): string => {
  const start = code.indexOf(`\n  ${label})\n`);
  expect(start, `could not find the '${label}' branch in verify-bundle.sh`).toBeGreaterThan(-1);
  const end = code.indexOf('\n    ;;', start);
  expect(end, `could not find the end of the '${label}' branch in verify-bundle.sh`).toBeGreaterThan(
    start
  );
  return code.slice(start, end);
};

/**
 * The body of one shell function, so that an assertion about a helper is made
 * against that helper and not against a phrase that happens to appear
 * somewhere else in the file.
 */
const shellFunction = (code: string, name: string): string => {
  const start = code.indexOf(`${name}() {`);
  expect(start, `could not find the shell function '${name}' in verify-bundle.sh`).toBeGreaterThan(-1);
  const end = code.indexOf('\n}', start);
  expect(end, `could not find the end of the shell function '${name}'`).toBeGreaterThan(start);
  return code.slice(start, end + 2);
};

/**
 * Collapse runs of whitespace, so a clause that the licence text hard-wraps
 * across lines becomes a contiguous string again.
 */
const flat = (text: string): string => text.replace(/\s+/g, ' ');

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
    const remediation = bundleCode.match(/die "\$\{ARTIFACT_NAME\} ships no copy of the Licence[\s\S]*?"/);
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

  it('extracts the package, because the decision is about the bytes in a file', () => {
    // A listing carries names; the licence is a text. `--contents` is still
    // used, but only to print the package layout when the check fails - the
    // decision itself is made on the extracted tree.
    const deb = caseBranch(bundleCode, 'deb');
    expect(
      deb,
      'the deb branch does not extract the package, so it cannot read the licence content'
    ).toMatch(/dpkg-deb --extract/);
  });

  it('decides on the licence CONTENT, not on the file name', () => {
    // The .deb is the one target that CAN carry the copy of the Licence, so
    // this is the load-bearing check of the three. A name-only decision here
    // would report a dependency's copyright file as the product's licence -
    // the exact false green the AppImage branch used to produce.
    //
    // This assertion exists because the AppImage and dmg branches assert
    // their own content-based decision while this branch did not: reverting
    // the deb branch to a name-only check left every other test in this file
    // green. A gate that stops guarding the one case it was built for is
    // worse than no gate, because it reads as coverage.
    const deb = caseBranch(bundleCode, 'deb');
    expect(
      deb,
      'the deb branch does not use the content-based licence helper'
    ).toContain('LICENSE_ENTRY=$(find_product_licence "$DEB_EXTRACT_DIR" || true)');
    // The candidates that were rejected are reported, because their presence
    // is exactly what made the name-only check report the wrong file.
    expect(deb).toContain('report_rejected_licences "$DEB_EXTRACT_DIR"');
  });

  it('fails closed when no copy of the Licence is present', () => {
    // The assertions below are made against the FAILURE MESSAGE rather than
    // against the file as a whole. The file mentions the licence and the AGPL
    // section in several places - the AppImage diagnostic among them - and an
    // assertion that cannot tell those apart is satisfied by whichever one
    // happens to be left, which is a gate that checks something other than
    // what it claims to check.
    const remediation = bundleCode.match(/die "\$\{ARTIFACT_NAME\} ships no copy of the Licence[\s\S]*?"/);
    expect(
      remediation,
      'could not find the no-licence failure message in verify-bundle.sh'
    ).not.toBeNull();
    // The message has to say WHY a name is not enough, because that is the
    // defect this check exists to catch: the package can carry a dependency's
    // copyright file and no copy of the Licence at all.
    expect(remediation![0]).toMatch(/A file whose NAME looks like a licence does not count/);
    // AGPL cited by section, so the requirement is checkable by a reader who
    // does not take this repository's word for it.
    expect(remediation![0]).toMatch(/AGPL-3\.0/);
    expect(remediation![0]).toMatch(/AGPL-3\.0 section 4 requires/);
    // Section 4 has no lettered subsections, so "section 4(a)" names a clause
    // that does not exist. The citation is checkable, so it is checked.
    expect(
      remediation![0],
      'the failure message cites a lettered subsection that section 4 does not have'
    ).not.toMatch(/4\(a\)/);
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
    //
    // The claim is about the pattern the script applies, so the assertion is
    // made against the function that applies it: a phrase elsewhere in the
    // file must not be able to satisfy it.
    const candidates = shellFunction(bundleCode, 'list_licence_candidates');
    expect(candidates).toMatch(/-iname 'licen\[cs\]e\*'/);
    expect(candidates).toMatch(/-iname 'copying\*'/);
    expect(candidates).toMatch(/-iname 'copyright\*'/);
    expect(candidates).toMatch(/-iname 'notice\*'/);
    // Every glob is anchored at the FIRST character of the name, so a file
    // whose name merely CONTAINS a licence word - `my-licence.txt`, `README`
    // - is not a candidate. A wildcard-prefixed glob would make it one.
    expect(
      candidates,
      'a licence candidate may be matched by a wildcard-prefixed glob, so a file whose name ' +
        'merely contains a licence word becomes a candidate'
    ).not.toMatch(/-iname '\*/);
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
    expect(msiRaw).toMatch(/AGPL-3\.0 section 4 requires a copy of the Licence/);
    // The wrong citation is forbidden anywhere in the file, not merely
    // absent from this one message: a phrase elsewhere in the file must not
    // be able to satisfy an assertion about a different one.
    expect(
      msiRaw,
      'the MSI verifier still cites a lettered subsection that section 4 does not have'
    ).not.toMatch(/4\(a\)/);
  });
});

describe('the AppImage carries the licence, and the verifier proves it', () => {
  /**
   * The AppImage bundler reads neither `bundle.licenseFile` nor
   * `bundle.resources`: `crates/tauri-bundler/src/bundle/linux/appimage/`
   * (both `mod.rs` and `linuxdeploy.rs`) has no reference to either.
   *
   * That used to be read as "there is no configuration that puts a licence in
   * an AppImage", and the conclusion was wrong. The bundler reads a THIRD
   * key: `bundle.linux.appimage.files`. `tauri-utils`' `AppImageConfig`
   * declares `files: HashMap<PathBuf, PathBuf>` ("The files to include in the
   * Appimage Binary"), and
   * `crates/tauri-bundler/src/bundle/linux/appimage/linuxdeploy.rs` calls
   * `fs_utils::copy_custom_files(&settings.appimage().files, &data_dir)`
   * before it copies `data_dir/usr/` into the AppDir. The map key is the
   * DESTINATION relative to the data dir, so it has to start with `usr/` to
   * end up inside the image.
   *
   * So the gap is closed by configuration rather than by repacking the image,
   * and the check is an ASSERTION: the thing that could never pass is now the
   * thing that must pass.
   */
  it('maps the licence into the AppImage through appimage.files', () => {
    const appimageFiles = conf.bundle?.linux?.appimage?.files;
    expect(
      appimageFiles,
      'bundle.linux.appimage.files is the only AppImage mechanism that reaches the image. ' +
        'Without it the AppImage carries no copy of the Licence.'
    ).toBeTruthy();
    const entries = Object.entries(appimageFiles as DebFiles);
    expect(entries.length, 'bundle.linux.appimage.files is empty').toBeGreaterThan(0);
    for (const [dest, src] of entries) {
      // Only data_dir/usr/ is copied into the AppDir, so a destination
      // outside usr/ is silently dropped.
      expect(
        dest.startsWith('usr/'),
        `"${dest}" is outside usr/, and only data_dir/usr/ is copied into the AppDir`
      ).toBe(true);
      // The source is the repository's own LICENSE, relative to
      // apps/gm-desktop/src-tauri/tauri.conf.json.
      expect(src, `"${dest}" does not point at the repository's LICENSE`).toBe('../../../LICENSE');
    }
  });

  it('the AppImage verify step extracts the image and checks its contents', () => {
    const appimage = caseBranch(bundleCode, 'appimage');
    expect(appimage).toMatch(/--appimage-extract/);
  });

  it('the AppImage check decides on the licence CONTENT, not on the file name', () => {
    const appimage = caseBranch(bundleCode, 'appimage');
    expect(
      appimage,
      'the AppImage branch does not use the content-based licence helper'
    ).toContain('APPIMAGE_LICENCE=$(find_product_licence "$APPIMAGE_ROOT" || true)');
    // The reported path is the one the helper returned - relative to the
    // extracted root - rather than a path cut out of a `find` listing.
    expect(appimage).toContain('ok "licence: ${APPIMAGE_LICENCE}"');
    // The rejected candidates are reported too: their presence is exactly
    // what made the name-only check report the wrong file.
    expect(appimage).toContain('report_rejected_licences "$APPIMAGE_ROOT"');
  });

  it('the AppImage licence check ASSERTS, because it can now be satisfied', () => {
    const appimage = caseBranch(bundleCode, 'appimage');
    const start = appimage.indexOf('APPIMAGE_LICENCE=$(find_product_licence');
    expect(start, 'the AppImage branch does not use the content-based licence helper').toBeGreaterThan(
      -1
    );
    const end = appimage.indexOf('rm -rf "$APPIMAGE_EXTRACT_DIR"', start);
    expect(end, 'could not find the end of the AppImage licence block').toBeGreaterThan(start);
    const licenceBlock = appimage.slice(start, end);
    // The die has to be the LICENCE die, not merely any die in the branch:
    // the branch also dies when the image cannot be extracted, and that die
    // says nothing about the licence. A mutation that turned the licence
    // failure into a note left this assertion green until it was narrowed -
    // the branch's other die was satisfying a check about the licence.
    expect(
      licenceBlock,
      'the AppImage licence block does not die when the licence is missing; the mechanism exists, so this must be an assertion'
    ).toMatch(/die "\$\{ARTIFACT_NAME\} ships no copy of the Licence/);
  });

  it('the AppImage failure message names the key that can fix it', () => {
    const appimage = caseBranch(bundleCode, 'appimage');
    const remediation = appimage.match(
      /die "\$\{ARTIFACT_NAME\} ships no copy of the Licence[\s\S]*?"/
    );
    expect(
      remediation,
      'could not find the no-licence failure message for the AppImage'
    ).not.toBeNull();
    // The key that reaches this bundler is appimage.files. Pointing a reader
    // at bundle.licenseFile would send them to a key this bundler never
    // reads - the mistake the repository already made once.
    expect(remediation![0]).toMatch(/bundle\.linux\.appimage\.files/);
    expect(remediation![0]).not.toMatch(/bundle\.licenseFile/);
  });

  it('reads the type-2 magic from the bytes it claims to read', () => {
    const appimage = caseBranch(bundleCode, 'appimage');
    // `od` prints two hex digits per byte, so byte N occupies characters
    // 2N+1 and 2N+2. The bytes at OFFSET 8 are characters 17-22. The
    // previous `cut -c9-14` read bytes 4-6 and printed them under a label
    // that said 8-10, which is how `020101` came to be recorded as the
    // type-2 magic.
    expect(
      appimage,
      'the AppImage branch reads the type-2 magic from the wrong bytes'
    ).toMatch(/cut -c17-22/);
    expect(appimage).not.toMatch(/cut -c9-14/);
  });
});

describe('the dmg reports what the mounted image carries', () => {
  /**
   * The dmg bundler IS handed the licence: `bundle.licenseFile` reaches it
   * and create-dmg embeds it as UDIF EULA resources. Where it leaves it is
   * not something this script can assert without guessing, so it reports
   * what the mounted image actually holds.
   */
  it('the dmg verify step reports what the mounted image carries', () => {
    const dmg = caseBranch(bundleCode, 'dmg');
    expect(dmg).toMatch(/--eula/);
    // Two distinct absences, because they mean different things: a
    // licence-shaped file that is NOT the Licence is a dependency's file
    // being mistaken for one, and no licence-shaped file at all is an image
    // that never had one.
    expect(dmg).toMatch(/NO COPY OF THE LICENCE inside the mounted image/);
    expect(dmg).toMatch(/NO LICENCE-SHAPED FILE inside the mounted image/);
  });

  it('the dmg report decides on the licence CONTENT, not on the file name', () => {
    const dmg = caseBranch(bundleCode, 'dmg');
    expect(
      dmg,
      'the dmg branch does not use the content-based licence helper'
    ).toContain('DMG_LICENCE=$(find_product_licence "$MOUNT_DIR" || true)');
    expect(dmg).toContain('ok "licence: ${DMG_LICENCE}"');
    expect(dmg).toContain('report_rejected_licences "$MOUNT_DIR"');
  });

  it('the dmg licence report is a diagnostic, never an assertion', () => {
    const dmg = caseBranch(bundleCode, 'dmg');
    const start = dmg.indexOf('DMG_LICENCE=$(find_product_licence');
    expect(start, 'the dmg branch does not use the content-based licence helper').toBeGreaterThan(-1);
    // The licence block runs to the end of the branch.
    const licenceBlock = dmg.slice(start);
    // Unlike the AppImage, the dmg keeps a diagnostic: the licence IS
    // delivered (create-dmg embeds it as a UDIF EULA resource), but where it
    // leaves it is not something this script can observe as a file, and an
    // assertion that could never pass gets switched off.
    expect(
      licenceBlock,
      'the dmg licence block calls die; it must stay a diagnostic'
    ).not.toMatch(/\bdie\b/);
  });

  it('the dmg verify step accepts the licence the image now carries', () => {
    // `bundle.licenseFile` reaches the dmg bundler, which hands it to
    // create-dmg as `--eula`, and create-dmg embeds it as UDIF EULA resources.
    // A disk image with a Software License Agreement cannot be mounted until
    // the agreement is accepted, so a non-interactive runner has to accept it
    // - measured: the attach succeeded on every run before `bundle.licenseFile`
    // was set and was cancelled 0.05 s after starting on every run after it.
    //
    // The exact shape is asserted, because each part of it is load-bearing:
    // `{ yes || true; }` keeps `yes`'s SIGPIPE death (141) out of the pipeline
    // status under `set -o pipefail`, and the `head` bounds the input so that
    // a hdiutil which never reads stdin cannot leave `yes` blocked forever.
    expect(
      bundleCode,
      'the dmg mount does not answer the Software License Agreement the image carries'
    ).toMatch(/\{ yes \|\| true; \} \| head -n \d+ \| hdiutil attach/);
  });
});

describe('the AGPL citation names a clause that exists', () => {
  /**
   * Section 4 of the AGPL-3.0 has NO lettered subsections. The obligation to
   * convey a copy of the License along with the Program is the closing clause
   * of section 4's first paragraph; section 5 (modified source versions) and
   * section 6 (non-source forms) are the sections that carry lettered
   * subsections, and section 6 applies sections 4 and 5 to object-code
   * recipients.
   *
   * The repository cited "section 4(a)" in every script, template and failure
   * message, and a test asserted that the wrong citation was present — a gate
   * that enforced a citation to a clause that does not exist.
   *
   * The citation is checkable, so it is checked against the licence text
   * itself rather than against a recollection of it.
   */
  it('the obligation this gate enforces is in section 4, unlettered', () => {
    const licence = readFileSync(join(REPO_ROOT, 'LICENSE'), 'utf8');
    const sec4 = licence.match(
      /4\. Conveying Verbatim Copies\.([\s\S]*?)\n\s*5\. Conveying Modified Source Versions\./
    );
    expect(sec4, 'could not find section 4 in the LICENSE file').not.toBeNull();
    // The licence text is hard-wrapped, so the clause is split across lines
    // ("and give all" / "recipients a copy of this License along with the
    // Program."). Collapsing the whitespace is what makes the clause a
    // contiguous string again.
    expect(flat(sec4![1])).toContain(
      'give all recipients a copy of this License along with the Program'
    );
    // No lettered subsection inside section 4, so "4(a)" cannot be cited.
    expect(
      sec4![1],
      'section 4 of the AGPL has a lettered subsection, so the citation in the scripts is wrong again'
    ).not.toMatch(/^\s{4}[a-z]\)/m);
  });

  it('section 6 is where the lettered subsections live, and it applies section 4 to object code', () => {
    // The installers ship object code, so section 6 is the operative route:
    // it conveys the object code "under the terms of sections 4 and 5".
    const licence = readFileSync(join(REPO_ROOT, 'LICENSE'), 'utf8');
    const sec6 = licence.match(
      /6\. Conveying Non-Source Forms\.([\s\S]*?)\n\s*7\. Additional Terms\./
    );
    expect(sec6, 'could not find section 6 in the LICENSE file').not.toBeNull();
    expect(flat(sec6![1])).toContain('under the terms of sections 4 and 5');
    expect(sec6![1]).toMatch(/^\s{4}[a-e]\)/m);
  });
});

describe('the licence check decides on content, not on the file name', () => {
  /**
   * A file whose NAME looks like a licence is not a licence.
   *
   * [FACT] Measured on the AppImage this repository builds: it contains
   * `usr/share/doc/libwayland-egl1/copyright`, which is the copyright file of
   * a BUNDLED LIBRARY. The name-only check printed
   *
   *     ok  licence: usr/share/doc/libwayland-egl1/copyright
   *
   * and the artefact carried no copy of the Licence at all. The name is a way
   * to find candidates; the content decides which candidate is the Licence.
   */
  it('finds candidates by an anchored name pattern', () => {
    const candidates = shellFunction(bundleCode, 'list_licence_candidates');
    // `find` matches the pattern against the file's own name, and `-iname`
    // makes it case-insensitive on every platform.
    expect(candidates).toMatch(/find "\$1" -type f/);
    expect(candidates).toMatch(/-iname 'licen\[cs\]e\*'/);
    expect(candidates).toMatch(/-iname 'copying\*'/);
    expect(candidates).toMatch(/-iname 'copyright\*'/);
    expect(candidates).toMatch(/-iname 'notice\*'/);
    // No wildcard-prefixed glob: a file whose name merely CONTAINS a licence
    // word is not a candidate.
    expect(
      candidates,
      'a licence candidate may be matched by a wildcard-prefixed glob, so a file whose name ' +
        'merely contains a licence word becomes a candidate'
    ).not.toMatch(/-iname '\*/);
  });

  it('accepts a candidate only when it carries the AGPL text', () => {
    const carries = shellFunction(bundleCode, 'carries_agpl');
    // `grep -F`, so the phrases are taken literally - a regular expression
    // would read the parentheses in them.
    expect(carries).toMatch(/grep -q -F "\$AGPL_TITLE"/);
    expect(carries).toMatch(/grep -q -F "\$AGPL_SECTION"/);
  });

  it('defines the two phrases it greps for', () => {
    expect(bundleCode).toMatch(/^AGPL_TITLE='GNU AFFERO GENERAL PUBLIC LICENSE'$/m);
    expect(bundleCode).toMatch(/^AGPL_SECTION='TERMS AND CONDITIONS'$/m);
  });

  it('reports only a candidate that carries the Licence, relative to the root', () => {
    const finder = shellFunction(bundleCode, 'find_product_licence');
    expect(
      finder,
      'find_product_licence does not test the candidate content'
    ).toContain('carries_agpl "$candidate"');
    expect(
      finder,
      'find_product_licence does not report a path relative to the root it was given'
    ).toContain('"${candidate#"$root"/}"');
  });

  it('needs BOTH phrases, because the repository README is a decoy', () => {
    // [FACT] This repository's own README mentions the licence by name, so a
    // title-only check would accept it as the Licence. The section heading is
    // what a decoy does not carry.
    const readme = readFileSync(join(REPO_ROOT, 'README.md'), 'utf8');
    expect(
      readme,
      'the README no longer mentions the licence title, so the two-phrase rule is no longer ' +
        'exercised by a real decoy'
    ).toContain('GNU AFFERO GENERAL PUBLIC LICENSE');
    expect(
      readme,
      'the README carries the AGPL section heading too, so the two-phrase rule cannot tell the ' +
        'decoy from the Licence'
    ).not.toContain('TERMS AND CONDITIONS');

    // And both phrases really are in the Licence, or the check could never
    // pass on a correct artefact.
    const licence = readFileSync(join(REPO_ROOT, 'LICENSE'), 'utf8');
    expect(licence).toContain('GNU AFFERO GENERAL PUBLIC LICENSE');
    expect(licence).toContain('TERMS AND CONDITIONS');
  });

  it('records the measurement that produced the content-based decision', () => {
    // The script has to say which file was mistaken for the Licence, or a
    // reader has no way to know the check was ever wrong.
    expect(bundleRaw).toMatch(/usr\/share\/doc\/libwayland-egl1\/copyright/);
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
 * [FACT] A file whose NAME looks like a licence is not a licence. The
 * AppImage this repository builds contains
 * `usr/share/doc/libwayland-egl1/copyright`, which is the copyright file of a
 * BUNDLED LIBRARY, and the name-only check reported it as the product's
 * licence. All three targets now decide on CONTENT: a candidate counts only
 * when it carries BOTH the AGPL title and the AGPL section heading, because
 * this repository's own README carries the title and would satisfy a
 * title-only check.
 *
 * [INFERENCE] The AppImage still ships no licence, and that is now measured
 * rather than assumed: the verify step extracts the image and reports the
 * licence-shaped files it found and whether each of them carries the Licence.
 * Closing that gap needs either a bundler change or a post-build repack,
 * neither of which this change attempts.
 */
