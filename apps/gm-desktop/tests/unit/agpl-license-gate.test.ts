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
 */
import { describe, expect, it } from 'vitest';
import { existsSync, readFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const REPO_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..', '..', '..', '..');
const CONF = join(REPO_ROOT, 'apps', 'gm-desktop', 'src-tauri', 'tauri.conf.json');
const BUNDLE_SH = join(REPO_ROOT, 'apps', 'gm-desktop', 'scripts', 'verify-bundle.sh');
const MSI_PS1 = join(REPO_ROOT, 'apps', 'gm-desktop', 'scripts', 'verify-msi.ps1');

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

describe('the bundle is configured to carry the licence', () => {
  it('declares bundle.licenseFile', () => {
    // `resources` is the wrong key here: it is a generic file list whose
    // placement semantics differ per bundler, and Tauri v2.11 is known to
    // require per-platform spellings of it. `licenseFile` is the key the
    // reference documents as "the path to the license file to be included in
    // the appropriate bundles", which is exactly this requirement.
    expect(
      conf.bundle?.licenseFile,
      'tauri.conf.json declares no bundle.licenseFile, so no installer carries the licence'
    ).toBeTruthy();
  });

  it('the path it points at resolves to the repository LICENSE', () => {
    const declared = conf.bundle?.licenseFile;
    expect(declared, 'no licenseFile to resolve').toBeTruthy();
    // Resolved relative to the config's own directory, which is what the
    // bundler does. An absolute path here would also "resolve" on this machine
    // and fail on a runner, so the check is deliberately relative.
    const abs = resolve(dirname(CONF), declared!);
    expect(existsSync(abs), `licenseFile '${declared}' does not resolve to a file`).toBe(true);
    expect(
      readFileSync(abs, 'utf8'),
      'the file licenseFile points at is not the AGPL text'
    ).toMatch(/GNU AFFERO GENERAL PUBLIC LICENSE/);
  });

  it('a missing LICENSE is a build-time failure, not a silent omission', () => {
    // The failure mode this guards: `licenseFile` points at a path that does not
    // exist and the bundler quietly ships nothing.
    expect(bundleCode).toMatch(/bundle\.licenseFile/);
  });

  it('declares the two mechanisms the deb bundler actually reads', () => {
    // `[FACT]` `crates/tauri-bundler/src/bundle/linux/debian.rs` builds its data
    // directory from `copy_resource_files` (-> `settings.copy_resources`),
    // binaries, icons + desktop entry, an optional `deb.changelog`, and
    // `fs_utils::copy_custom_files(&settings.deb().files, &data_dir)`.
    // `licenseFile` is not among them, and run 37642251095 proved it: with
    // `licenseFile` set, the built .deb still held exactly the same five files.
    expect(
      conf.bundle?.resources,
      'bundle.resources is empty; the deb bundler reads nothing that would carry the licence'
    ).toBeTruthy();

    const debFiles = conf.bundle?.linux?.deb?.files;
    expect(
      debFiles,
      'bundle.linux.deb.files is what reaches /usr/share/doc/<pkg>/copyright. Without it ' +
        'the licence would sit in usr/lib/<product>/, which is not where a licence belongs.'
    ).toBeTruthy();
    expect(Object.values(debFiles!)).toEqual(
      expect.arrayContaining([expect.stringMatching(/usr\/share\/doc\/.+\/(copyright|LICEN[CS]E)$/)])
    );
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

describe('the MSI verifier records the boundary rather than asserting an impossible check', () => {
  /**
   * There WAS an assertion here. It worked — it fired on run `37642251415` and
   * reported the real omission. It was removed because it could never pass: a
   * custom WiX template gets no licence material from Tauri, so keeping it would
   * make this job permanently red, and a gate that cannot be satisfied gets
   * switched off, at which point it protects nothing and blocks every merge.
   *
   * The boundary is asserted instead: the script must SAY that it is not
   * checking, and say why. A silent gap and a stated gap are different things,
   * and only one of them is honest.
   *
   * These three assertions read the RAW file, not the comment-stripped one, and
   * that is deliberate rather than a lapse. Everywhere else in this repository
   * the rule is "never let a comment satisfy a gate", because there the gate is
   * asserting what the CODE does. Here the requirement IS the prose — the thing
   * being demanded is that the boundary is documented — so matching prose is
   * matching the requirement. The rule protects against a gate claiming more
   * than it checked; it does not forbid a gate from checking documentation.
   */
  it('says out loud that it does not check the MSI payload for a licence', () => {
    expect(msiRaw).toMatch(/NOT CHECKED HERE, DELIBERATELY/);
    expect(
      msiRaw,
      'the note must cite the measurement that prompted it, not just assert a conclusion'
    ).toMatch(/37642251415/);
  });

  it('records why configuration cannot fix it', () => {
    // A future reader who "fixes" this by adding bundle.licenseFile again will
    // be repeating a change that was measured not to work. The reason has to be
    // in the file, not only in a PR.
    expect(msiRaw).toMatch(/wix\/main\.wxs/);
    expect(msiRaw).toMatch(/Handlebars/);
    expect(msiRaw).toMatch(/4\(a\)/);
  });

  it('does not carry a licence assertion that could never pass', () => {
    expect(
      msiCode,
      'a permanently-red check protects nothing and blocks every merge. If this is ' +
        'satisfied, the mechanism exists and the assertion should come back.'
    ).not.toMatch(/\$licencePattern/);
    expect(msiCode).not.toMatch(/LicenseRelPath/);
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
 * [INFERENCE] The remaining risk is on macOS and AppImage, which this machine
 * cannot build. `bundle.licenseFile` is a single config key applied to every
 * target, so all four receive the same instruction, but only `.deb` and `.msi`
 * have an assertion that has been reasoned about and exercised here. A CI run
 * that stays green on dmg and AppImage would be the first real evidence for
 * those two; until then this is inference, not measurement.
 */