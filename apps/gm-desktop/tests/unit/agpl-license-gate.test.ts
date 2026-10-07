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

const conf = JSON.parse(readFileSync(CONF, 'utf8')) as {
  bundle?: { licenseFile?: string; copyright?: string; targets?: string[] };
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
    // exist and the bundler quietly ships nothing. Both verification scripts
    // now name the key in their error, so the two failure modes stay distinct.
    expect(bundleCode).toMatch(/bundle\.licenseFile/);
    expect(msiCode).toMatch(/bundle\.licenseFile/);
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

describe('the MSI verifier would notice it too', () => {
  it('checks the extracted payload, not the package metadata', () => {
    expect(msiCode).toMatch(/Get-ChildItem -LiteralPath \$target -File -Recurse/);
    expect(msiCode).toMatch(/AGPL-3\.0/);
  });

  it('fails closed and lists what it did find', () => {
    // A failure that names the extracted file list turns "red" into a
    // diagnosable red, which is the difference between a gate that gets
    // investigated and one that gets disabled.
    expect(msiCode).toMatch(/Extracted:/);
    expect(msiCode).toMatch(/throw \(/);
  });

  it('returns the licence path so the success line is evidence, not decoration', () => {
    expect(msiCode).toMatch(/LicenseRelPath/);
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