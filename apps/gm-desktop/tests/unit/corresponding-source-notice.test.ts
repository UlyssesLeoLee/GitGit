/**
 * Gates on the AGPL-3.0 §6(d) "clear directions next to the object code".
 *
 * ## What this is about
 *
 * Section 4 of the AGPL is about the PACKAGE: "give all recipients a copy of
 * this License along with the Program". `agpl-license-gate.test.ts` and
 * `verify-bundle.sh` enforce that, and they enforce it on CONTENT.
 *
 * Section 6(d) is about the DISTRIBUTION, and it asks for something else: of
 * object code conveyed from a designated place, a distributor must "maintain
 * clear directions next to the object code saying where to find the
 * Corresponding Source" (LICENSE:263-275).
 *
 * So there are two artefacts here, and they are not the same artefact:
 *
 *   * `CORRESPONDING-SOURCE.md` - a committed notice, mapped into the `.deb`
 *     and the `.AppImage` next to the object code, naming the licence and the
 *     place the source is offered at;
 *   * the release body's "Corresponding Source" section - the page the object
 *     code was offered from, naming the exact source tag of THIS build.
 *
 * ## What is asserted, and how
 *
 * Three kinds, and the middle one is the one that matters:
 *
 *   1. **The notice says what it has to say.** Its content, not its name.
 *
 *   2. **The verifier, EXECUTED.** The helpers in `verify-bundle.sh` are
 *      extracted from the real file and run against fixture trees, because a
 *      check that is only read can be satisfied by a phrase that never
 *      executes. A fixture that carries the notice has to be found; a fixture
 *      that carries a file with the right NAME and the wrong content has to
 *      be rejected.
 *
 *   3. **The release body carries the section.** Parsed out of the workflow
 *      with a YAML parser, and the URLs it names are checked against the
 *      configuration, so the page cannot point somewhere the configuration
 *      does not.
 *
 * ## What is deliberately NOT asserted
 *
 * [FACT] Whether this arrangement satisfies section 6(d) is a legal
 * judgement, and this file does not make one. It asserts that the facts are
 * recorded and that the mechanisms work. The judgement is left to a
 * responsible human, and `docs/plan/v0-tasks.md` carries it as an open
 * question.
 */
import { describe, expect, it } from 'vitest';
import { spawnSync } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { parse } from 'yaml';

const REPO_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..', '..', '..', '..');

/** The committed notice. It is at the root of the repository, next to LICENSE. */
const NOTICE_REL = 'CORRESPONDING-SOURCE.md';
const NOTICE = join(REPO_ROOT, NOTICE_REL);

const CONF = join(REPO_ROOT, 'apps', 'gm-desktop', 'src-tauri', 'tauri.conf.json');
const BUNDLE_SH = join(REPO_ROOT, 'apps', 'gm-desktop', 'scripts', 'verify-bundle.sh');
const WIX_MAIN = join(REPO_ROOT, 'apps', 'gm-desktop', 'src-tauri', 'wix', 'main.wxs');
const RELEASE = join(REPO_ROOT, '.github', 'workflows', 'release.yml');
const TASKS = join(REPO_ROOT, 'docs', 'plan', 'v0-tasks.md');

type FilesMap = Record<string, string>;

const conf = JSON.parse(readFileSync(CONF, 'utf8')) as {
  bundle?: {
    homepage?: string;
    licenseFile?: string;
    resources?: string[] | FilesMap;
    linux?: {
      deb?: { depends?: string[]; files?: FilesMap };
      appimage?: { files?: FilesMap };
    };
  };
};

/** The homepage the project publishes itself under. */
const HOMEPAGE = conf.bundle?.homepage ?? '';
/** The designated place: where the object code is offered AND the source with it. */
const PLACE = `${HOMEPAGE}/releases`;

/**
 * A destination that names the licence.
 *
 * The same shape the licence gate uses, so the two files agree on which
 * entry of a files map is the licence.
 */
const LICENCE_DEST = /\/[^/]*(copyright|LICEN[CS]E)$/;

/** The expression the release body must use for the tag. Never `github.ref`. */
const TAG_OUTPUT = '${{ needs.verify-tag.outputs.tag }}';

// ---------------------------------------------------------------------------
// Reading the shell script
// ---------------------------------------------------------------------------

/** Full-line comments only - see the note on stripShellComments below. */
const stripShellComments = (text: string): string =>
  text
    .split('\n')
    .filter((line) => !/^\s*#/.test(line))
    .join('\n');

const bundleCode = stripShellComments(readFileSync(BUNDLE_SH, 'utf8'));
/** The bundle script with its prose intact - used only where prose IS the requirement. */
const bundleRaw = readFileSync(BUNDLE_SH, 'utf8');
/** The vendored WiX template, which is what installs the licence into the .msi. */
const wixMain = readFileSync(WIX_MAIN, 'utf8');

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
 * The body of one `case` branch of verify-bundle.sh.
 *
 * The assertions below are about what a branch DOES, so they are made against
 * the branch rather than against the file as a whole: a phrase printed by a
 * different branch must not be able to satisfy an assertion about this one.
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

// ---------------------------------------------------------------------------
// Running the shell
// ---------------------------------------------------------------------------

/**
 * The bash this machine runs.
 *
 * Same lookup as `release-tag-resolution.test.ts`: on Windows the `bash` on
 * PATH is usually WSL, which cannot see this repository.
 */
function resolveBash(): string {
  const override = process.env.GIT_BASH;
  if (override && existsSync(override)) return override;
  if (process.platform === 'win32') {
    for (const candidate of [
      'C:\\Program Files\\Git\\bin\\bash.exe',
      'C:\\Program Files\\Git\\usr\\bin\\bash.exe',
    ]) {
      if (existsSync(candidate)) return candidate;
    }
  }
  return 'bash';
}

const BASH = resolveBash();

/** A Windows path bash can read: forward slashes, because a backslash in a
 *  double-quoted bash string is an escape character. */
const toBashPath = (p: string): string => p.replace(/\\/g, '/');

/**
 * The notice helpers, lifted out of the real `verify-bundle.sh`.
 *
 * They are extracted rather than re-implemented, so what is executed here is
 * the code the bundle verifier runs. The two constants come with them,
 * because a helper that greps for a phrase the script never defines would
 * find nothing and pass.
 */
const noticeHelpers = (): string => {
  const marker = bundleCode.match(/^CS_NOTICE_MARKER='[^']*'$/m);
  const licence = bundleCode.match(/^CS_LICENCE_MARKER='[^']*'$/m);
  expect(marker, 'verify-bundle.sh defines no CS_NOTICE_MARKER').not.toBeNull();
  expect(licence, 'verify-bundle.sh defines no CS_LICENCE_MARKER').not.toBeNull();
  return [
    marker![0],
    licence![0],
    shellFunction(bundleCode, 'carries_corresponding_source'),
    shellFunction(bundleCode, 'find_corresponding_source_notice'),
  ].join('\n');
};

/** Run the extracted helpers over a directory and report what they found. */
const runFinder = (root: string): { status: number | null; output: string } => {
  const script = [
    'set -euo pipefail',
    noticeHelpers(),
    `find_corresponding_source_notice "${toBashPath(root)}" "${PLACE}"`,
  ].join('\n');
  const res = spawnSync(BASH, ['-c', script], { encoding: 'utf8' });
  return { status: res.status, output: `${res.stdout ?? ''}${res.stderr ?? ''}` };
};

/** A throwaway directory tree, with the given files in it. */
const fixture = (files: Record<string, string>): string => {
  const dir = mkdtempSync(join(tmpdir(), 'cs-notice-'));
  for (const [rel, content] of Object.entries(files)) {
    const abs = join(dir, rel);
    mkdirSync(dirname(abs), { recursive: true });
    writeFileSync(abs, content);
  }
  return dir;
};

// ---------------------------------------------------------------------------
// Reading the workflow
// ---------------------------------------------------------------------------

const DOC = parse(readFileSync(RELEASE, 'utf8')) as {
  jobs: Record<
    string,
    {
      steps?: Array<{
        name?: string;
        id?: string;
        uses?: string;
        run?: string;
        with?: Record<string, unknown>;
      }>;
    }
  >;
};

const releaseStep = (DOC.jobs['draft-release']?.steps ?? []).find((s) =>
  typeof s.uses === 'string' && s.uses.startsWith('softprops/action-gh-release')
);
const body = String((releaseStep?.with as Record<string, unknown>)?.['body'] ?? '');

// ---------------------------------------------------------------------------
// The configuration
// ---------------------------------------------------------------------------

/** The entry of a files map whose SOURCE is the committed notice. */
const noticeEntry = (map: FilesMap | undefined): [string, string] => {
  const hit = Object.entries(map ?? {}).find(
    ([, src]) => resolve(dirname(CONF), src) === NOTICE
  );
  expect(hit, 'the committed notice is not mapped into this bundle at all').toBeDefined();
  return hit!;
};

// ---------------------------------------------------------------------------
// The notice file
// ---------------------------------------------------------------------------

describe('the notice is a committed file that says what it has to say', () => {
  it('exists, at the root of the repository, next to the licence', () => {
    expect(existsSync(NOTICE), `${NOTICE_REL} is missing`).toBe(true);
    expect(readFileSync(NOTICE, 'utf8').length, `${NOTICE_REL} is empty`).toBeGreaterThan(0);
  });

  it('names the licence', () => {
    expect(
      readFileSync(NOTICE, 'utf8'),
      'the notice must name the licence the program is under'
    ).toContain('AGPL-3.0');
  });

  it('names where the licence text is, and the paths it names are the ones shipped', () => {
    const notice = readFileSync(NOTICE, 'utf8');
    const debLicence = Object.entries(conf.bundle?.linux?.deb?.files ?? {}).find(([dest]) =>
      LICENCE_DEST.test(dest)
    );
    const appimageLicence = Object.entries(conf.bundle?.linux?.appimage?.files ?? {}).find(
      ([dest]) => LICENCE_DEST.test(dest)
    );
    expect(debLicence, 'the deb has no licence destination').toBeDefined();
    expect(appimageLicence, 'the AppImage has no licence destination').toBeDefined();

    // The notice says the licence text is installed alongside the notice, so
    // it names the exact destinations. If the configuration moves them, the
    // notice is saying something that is no longer true, and that is a red.
    expect(
      notice,
      'the notice does not say where the licence text is installed in the .deb'
    ).toContain(debLicence![0]);
    expect(
      notice,
      'the notice does not say where the licence text is installed in the .AppImage'
    ).toContain(appimageLicence![0]);
  });

  it('names the place the source is offered at', () => {
    const notice = readFileSync(NOTICE, 'utf8');
    expect(notice, 'the notice must name the repository the source lives in').toContain(HOMEPAGE);
    expect(
      notice,
      'the notice must name the designated place - the same place the object code was offered from'
    ).toContain(PLACE);
  });

  it('is distinguishable from the licence and from the README', () => {
    // The marker is what tells the notice apart from everything else in the
    // package. The AGPL text uses the phrase "Corresponding Source" 24
    // times, so the bare phrase cannot be the marker; if the marker were in
    // the licence too, the check could not tell the notice from the licence.
    const marker = bundleCode.match(/^CS_NOTICE_MARKER='([^']*)'$/m);
    expect(marker, 'verify-bundle.sh defines no CS_NOTICE_MARKER').not.toBeNull();
    const phrase = marker![1];

    expect(
      readFileSync(join(REPO_ROOT, 'LICENSE'), 'utf8'),
      'the licence carries the notice marker, so a package carrying only the licence would ' +
        'pass the notice check'
    ).not.toContain(phrase);
    expect(
      readFileSync(join(REPO_ROOT, 'README.md'), 'utf8'),
      'the README carries the notice marker, so the marker does not identify the notice'
    ).not.toContain(phrase);
    expect(readFileSync(NOTICE, 'utf8'), 'the notice does not carry its own marker').toContain(phrase);
  });
});

// ---------------------------------------------------------------------------
// The two Linux installers carry it
// ---------------------------------------------------------------------------

describe('the notice travels with the object code in both Linux installers', () => {
  it('the .deb maps it into the package data dir', () => {
    const [dest, src] = noticeEntry(conf.bundle?.linux?.deb?.files);
    expect(
      dest.startsWith('usr/'),
      `"${dest}" is outside usr/, so it does not land inside the package data dir`
    ).toBe(true);
    expect(src, 'the notice is not mapped from the committed file').toBe('../../../CORRESPONDING-SOURCE.md');
  });

  it('the AppImage maps it into the image', () => {
    const [dest, src] = noticeEntry(conf.bundle?.linux?.appimage?.files);
    expect(
      dest.startsWith('usr/'),
      `"${dest}" is outside usr/, and only data_dir/usr/ is copied into the AppDir`
    ).toBe(true);
    expect(src, 'the notice is not mapped from the committed file').toBe('../../../CORRESPONDING-SOURCE.md');
  });

  it('it lands in the same directory as the licence, in both', () => {
    // "Next to the object code" is a location, and the location that is
    // reachable is the one the licence is already in: the package's own
    // documentation directory.
    for (const [label, map] of [
      ['deb', conf.bundle?.linux?.deb?.files],
      ['appimage', conf.bundle?.linux?.appimage?.files],
    ] as Array<[string, FilesMap | undefined]>) {
      const [noticeDest] = noticeEntry(map);
      const licenceDest = Object.entries(map ?? {}).find(([dest]) => LICENCE_DEST.test(dest));
      expect(licenceDest, `the ${label} has no licence destination`).toBeDefined();
      expect(
        dirname(noticeDest),
        `the ${label} notice and its licence are in different directories`
      ).toBe(dirname(licenceDest![0]));
    }
  });
});

// ---------------------------------------------------------------------------
// The verifier decides on CONTENT
// ---------------------------------------------------------------------------

describe('the bundle verifier decides on the notice by content, not by name', () => {
  it('the helper greps for the phrases, it does not match a file name', () => {
    const carries = shellFunction(bundleCode, 'carries_corresponding_source');
    expect(carries, 'the helper does not grep for the marker').toMatch(/grep -q -F "\$CS_NOTICE_MARKER"/);
    expect(carries, 'the helper does not grep for the licence').toMatch(
      /grep -q -F "\$CS_LICENCE_MARKER"/
    );
    // The place is the third phrase: a file that says what the notice is
    // about but does not say where the source is offered is not directions.
    expect(carries, 'the helper does not check the place the source is offered at').toMatch(
      /grep -q -F "\$2"/
    );
    expect(
      carries,
      'the helper matches a file NAME, so a file that merely looks like a notice would count'
    ).not.toMatch(/-iname|-name/);
  });

  it('every file in the package is a candidate', () => {
    const finder = shellFunction(bundleCode, 'find_corresponding_source_notice');
    expect(finder, 'the finder does not walk the package').toMatch(/find "\$root" -type f/);
    expect(
      finder,
      'the finder narrows the candidates by name, so a notice under a name the check does not ' +
        'know about would be missed'
    ).not.toMatch(/-iname|-name/);
  });

  it('the deb branch asserts the notice is there', () => {
    const deb = caseBranch(bundleCode, 'deb');
    expect(deb, 'the deb branch does not look for the notice').toContain(
      'DEB_NOTICE=$(find_corresponding_source_notice "$DEB_EXTRACT_DIR" "$CS_SOURCE_PLACE" || true)'
    );
    expect(
      deb,
      'the deb branch finds the notice but does not fail when it is missing, so a package with ' +
        'no directions next to the object code would pass'
    ).toMatch(/die "\$\{ARTIFACT_NAME\} ships no Corresponding Source notice/);
  });

  it('the AppImage branch asserts the notice is there', () => {
    const appimage = caseBranch(bundleCode, 'appimage');
    expect(appimage, 'the AppImage branch does not look for the notice').toContain(
      'APPIMAGE_NOTICE=$(find_corresponding_source_notice "$APPIMAGE_ROOT" "$CS_SOURCE_PLACE" || true)'
    );
    expect(
      appimage,
      'the AppImage branch finds the notice but does not fail when it is missing'
    ).toMatch(/die "\$\{ARTIFACT_NAME\} ships no Corresponding Source notice/);
  });

  it('the dmg branch only reports the notice, because it has no file slot', () => {
    const dmg = caseBranch(bundleCode, 'dmg');
    expect(dmg, 'the dmg branch does not report the notice at all').toContain(
      'DMG_NOTICE=$(find_corresponding_source_notice "$MOUNT_DIR" "$CS_SOURCE_PLACE" || true)'
    );
    // [FACT] The licence reaches a .dmg as a UDIF EULA resource, not as a
    // file, so there is nothing in the image to carry a notice and nothing in
    // the configuration that could put one there. An assertion that could
    // never pass is a gate that gets switched off, so this one reports.
    const start = dmg.indexOf('DMG_NOTICE=$(find_corresponding_source_notice');
    expect(start, 'the dmg branch does not look for the notice').toBeGreaterThan(-1);
    expect(
      dmg.slice(start),
      'the dmg notice block calls die; it must stay a diagnostic'
    ).not.toMatch(/\bdie\b/);
  });

  it('the notice check runs on the bytes that were really packaged', () => {
    // The configuration is not the artifact. Both branches decide on the
    // EXTRACTED tree, which is the only place the packaged bytes exist.
    expect(caseBranch(bundleCode, 'deb')).toMatch(/dpkg-deb --extract/);
    expect(caseBranch(bundleCode, 'appimage')).toMatch(/--appimage-extract/);
  });
});

// ---------------------------------------------------------------------------
// The verifier, EXECUTED
// ---------------------------------------------------------------------------

describe('the notice finder, executed against fixtures', () => {
  it('finds the notice in a package that carries it', () => {
    const dir = fixture({
      'usr/share/doc/gitgit-desktop/copyright': readFileSync(join(REPO_ROOT, 'LICENSE'), 'utf8'),
      'usr/share/doc/gitgit-desktop/CORRESPONDING-SOURCE.md': readFileSync(NOTICE, 'utf8'),
    });
    try {
      const res = runFinder(dir);
      expect(res.status, res.output).toBe(0);
      expect(
        res.output.trim(),
        'the finder did not report the notice it was given'
      ).toBe('usr/share/doc/gitgit-desktop/CORRESPONDING-SOURCE.md');
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });

  it('does not accept a file that merely has the notice NAME', () => {
    // The failure this guards is the one the licence check already made
    // once: a file whose name looks like the thing is not the thing.
    const dir = fixture({
      'usr/share/doc/gitgit-desktop/CORRESPONDING-SOURCE.md':
        '# Corresponding Source\n\nSee the project website.\n',
    });
    try {
      const res = runFinder(dir);
      expect(res.status, 'a file with the right name and no directions was accepted').not.toBe(0);
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });

  it('does not accept a file that names the notice but not the licence or the place', () => {
    const dir = fixture({
      'usr/share/doc/gitgit-desktop/CORRESPONDING-SOURCE.md':
        'Corresponding Source for this build\n',
    });
    try {
      const res = runFinder(dir);
      expect(res.status, 'a heading with no directions was accepted as the notice').not.toBe(0);
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });

  it('does not accept a notice that points at a different place', () => {
    const dir = fixture({
      'usr/share/doc/gitgit-desktop/CORRESPONDING-SOURCE.md':
        'Corresponding Source for this build\nAGPL-3.0\nhttps://example.invalid/releases\n',
    });
    try {
      const res = runFinder(dir);
      expect(res.status, 'a notice naming another place was accepted').not.toBe(0);
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });

  it('finds nothing in a package with no notice at all', () => {
    const dir = fixture({
      'usr/share/doc/gitgit-desktop/copyright': readFileSync(join(REPO_ROOT, 'LICENSE'), 'utf8'),
    });
    try {
      const res = runFinder(dir);
      expect(res.status, 'a package with no notice was accepted').not.toBe(0);
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });

  it('runs a shell bash can parse', () => {
    const res = spawnSync(BASH, ['-n', BUNDLE_SH], { encoding: 'utf8' });
    expect(
      res.status,
      `bash -n rejected ${BUNDLE_SH}:\n${res.stdout ?? ''}${res.stderr ?? ''}`
    ).toBe(0);
  });
});

// ---------------------------------------------------------------------------
// The release body
// ---------------------------------------------------------------------------

describe('the release body carries the Corresponding Source section', () => {
  it('found the release-creating step', () => {
    expect(releaseStep, 'no softprops/action-gh-release step in draft-release').toBeDefined();
    expect(body.length, 'the release step declares no body').toBeGreaterThan(0);
  });

  it('has a Corresponding Source section', () => {
    expect(body, 'the release body has no Corresponding Source section').toContain(
      '## Corresponding Source'
    );
  });

  it('names the tag the artifacts were built from', () => {
    expect(body, 'the section must name the source tag of THIS build').toContain(TAG_OUTPUT);
  });

  it('never falls back to the triggering ref', () => {
    // `github.ref_name` is the BRANCH on a dispatch, and `github.ref` is the
    // triggering ref. Either one would advertise a branch as the source of a
    // tagged release.
    expect(body, 'the body names the triggering ref').not.toMatch(/github\.ref(_name)?\b/);
  });

  it('names the licence and where its text is', () => {
    expect(body, 'the section must name the licence').toContain('AGPL-3.0');
    expect(body, 'the section must say where the licence text is').toContain(
      `${HOMEPAGE}/blob/${TAG_OUTPUT}/LICENSE`
    );
  });

  it('names where the source is offered, at no further charge', () => {
    expect(body, 'the section must name the archive of the source tag').toContain(
      `${HOMEPAGE}/archive/refs/tags/${TAG_OUTPUT}.tar.gz`
    );
    expect(
      body,
      'the section must say the source is offered at no further charge'
    ).toMatch(/no\s+further charge/);
  });

  it('names the notice the installers carry, and the path is the one that is mapped', () => {
    const [debDest] = noticeEntry(conf.bundle?.linux?.deb?.files);
    const [appimageDest] = noticeEntry(conf.bundle?.linux?.appimage?.files);
    expect(debDest).toBe(appimageDest);
    expect(
      body,
      'the body must name the notice the installers carry, so a reader knows where to look for it'
    ).toContain(debDest);
  });

  it('the URLs it names come from the configuration, not from a stale copy', () => {
    // The homepage is what the project publishes itself under, and the
    // designated place is derived from it. If the two ever disagree, the
    // release page would point somewhere the configuration does not.
    expect(
      HOMEPAGE,
      'bundle.homepage is not a GitHub repository URL, so the release body cannot be built ' +
        'from it'
    ).toMatch(/^https:\/\/github\.com\/[^/]+\/[^/]+$/);
    expect(body, 'the body names a repository other than the configured homepage').toContain(
      HOMEPAGE
    );
  });

  it('says what the .msi and the .dmg carry, and it is true', () => {
    // [FACT] The .msi installs the licence RTF next to the executable, and
    // the .dmg carries the licence as a UDIF EULA resource. Neither carries
    // the notice file, and the release page says so rather than implying
    // otherwise.
    expect(body, 'the body must say what the .msi carries').toContain('LICENSE.rtf');
    expect(
      wixMain,
      'the WiX template does not install LICENSE.rtf, so the release body would be wrong'
    ).toMatch(/<File Id="LicenseFile" Source="\{\{license\}\}" Name="LICENSE\.rtf"/);
    expect(body, 'the body must say what the .dmg carries').toContain('.dmg');
    expect(
      bundleRaw,
      'the bundle verifier no longer records that the dmg licence is a UDIF EULA resource'
    ).toMatch(/--eula/);
  });
});

// ---------------------------------------------------------------------------
// The plan document
// ---------------------------------------------------------------------------

describe('the plan document records the facts and leaves the judgement open', () => {
  const tasks = readFileSync(TASKS, 'utf8');

  it('still has the section 6(d) row in the risk register', () => {
    expect(tasks, 'the section 6(d) risk row is gone').toMatch(/AGPL-3\.0 §6\(d\)/);
  });

  it('records the designated place and the facts about it', () => {
    expect(
      tasks,
      'the document does not record the designated place the object code and the source are ' +
        'offered from'
    ).toContain(PLACE);
    expect(
      tasks,
      'the document still says the check was not done'
    ).not.toMatch(/本轮未作检查|本轮未查/);
  });

  it('records what each artifact carries', () => {
    expect(tasks, 'the document does not record the notice file').toContain(NOTICE_REL);
    expect(tasks, 'the document does not record the .msi position').toContain('.msi');
    expect(tasks, 'the document does not record the .dmg position').toContain('.dmg');
  });

  it('leaves the legal judgement to a responsible human', () => {
    expect(
      tasks,
      'the section 6(d) entry must still be marked as a recommendation rather than a verdict'
    ).toContain('[PROPOSAL]');
    // A conclusion would be a judgement this repository is not in a position
    // to make, and it is the exact thing the risk register was opened to
    // avoid.
    expect(
      tasks,
      'the document concludes that the arrangement satisfies section 6(d)'
    ).not.toMatch(/符合\s*AGPL-3\.0\s*§6\(d\)|满足\s*AGPL-3\.0\s*§6\(d\)/);
  });
});
