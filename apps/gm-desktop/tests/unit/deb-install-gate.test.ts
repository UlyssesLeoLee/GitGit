/**
 * Gates on the `deb-install` job.
 *
 * ## Why this job exists
 *
 * Everything the bundle workflow already checked about the `.deb` is
 * structural: control fields parse, the version matches `tauri.conf.json`,
 * `usr/bin/<bin>` is large, a `.desktop` entry exists. None of that finds out
 * whether the package installs on a machine that does not already have the
 * GTK/WebKit stack — which is the only machine a user has.
 *
 * ## Why the container is load-bearing rather than incidental
 *
 * Running the install on the hosted `ubuntu-latest` runner would prove
 * nothing, and the reason is easy to miss: the `deb` BUILD job installs
 * `libwebkit2gtk-4.1-dev` and friends on that same machine. A package with a
 * missing dependency would install happily, because the dependency is
 * already there. `ubuntu:24.04` has none of it.
 *
 * That makes the `container:` block the whole point of the job, and makes its
 * removal the single most likely way for this job to silently become theatre.
 * These gates exist to make that removal loud.
 *
 * What is deliberately NOT claimed here: that the GUI opens. A webkit app
 * under `xvfb-run` on a container with no dbus or seat fails for reasons
 * unrelated to packaging, and a gate that reddens for those reasons gets
 * switched off. What is proven is what packaging controls — the package
 * installs on a bare machine, its dependency list is sufficient to LINK the
 * binary, and its launch metadata is well-formed.
 */
import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { parse } from 'yaml';

const REPO_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..', '..', '..', '..');
const DOC = parse(
  readFileSync(join(REPO_ROOT, '.github', 'workflows', 'gm-desktop-bundle.yml'), 'utf8')
) as {
  jobs: Record<string, {
    needs?: string;
    /** The literal YAML key is `runs-on`. Not `runsOn` — see the note below. */
    'runs-on'?: string;
    container?: { image?: string };
    outputs?: Record<string, string>;
    steps?: Array<{ name?: string; run?: string; uses?: string; with?: Record<string, unknown> }>;
  }>;
};

const job = DOC.jobs['deb-install'];
const steps = job?.steps ?? [];

/**
 * The `run:` bodies with comments stripped.
 *
 * Both the positive and the negative assertions read THIS, not the raw text.
 * Measured while writing this gate: the step explains itself in comments —
 * "NOT `dpkg -i`", "`apt-get -f install` afterwards would …" — and a check over
 * the raw text both false-fires on the explanation and, far worse, can be
 * SATISFIED by an explanation. A gate that a comment can satisfy is worse than
 * no gate, because it reads green.
 */
const stepRuns = steps
  .map((s) => s.run ?? '')
  .join('\n')
  .split('\n')
  .map((line) => line.replace(/(^|\s)#.*$/, ''))
  .join('\n');

describe('deb-install — the job exists and depends on a real artifact', () => {
  it('found the job', () => {
    expect(DOC.jobs, 'no deb-install job in gm-desktop-bundle.yml').toBeDefined();
    expect(job).toBeDefined();
    expect(steps.length, 'deb-install has no steps').toBeGreaterThan(0);
  });

  it('needs the deb job, and that job publishes the version it names the artifact by', () => {
    expect(job?.needs, 'deb-install does not depend on the build it is testing').toBe('deb');
    expect(
      DOC.jobs['deb']?.outputs?.version,
      'the deb job publishes no `version` output. A glob would match the -manifest artifact too.'
    ).toMatch(/steps\.meta\.outputs\.version/);

    // The reference is in the download step's `with.name`, not in a `run:`
    // body — an earlier version of this check scanned only the scripts and
    // failed on a workflow that was correct.
    const download = steps.find((s) => s.uses?.startsWith('actions/download-artifact'));
    expect(download, 'deb-install never downloads the artifact it is testing').toBeDefined();
    expect(
      String(download?.with?.name ?? ''),
      'the artifact must be named by version, not by a glob (a glob also matches the -manifest artifact)'
    ).toMatch(/^gm-desktop-deb-\$\{\{ needs\.deb\.outputs\.version \}\}$/);
  });

  it('downloads the artifact rather than rebuilding it', () => {
    expect(steps.some((s) => s.uses?.startsWith('actions/download-artifact'))).toBe(true);
    expect(
      stepRuns,
      'deb-install must not rebuild; it is testing the artifact deb produced'
    ).not.toMatch(/tauri build/);
  });
});

describe('deb-install — it runs somewhere bare', () => {
  it('runs in a minimal container, not on the build runner', () => {
    expect(job?.container, 'deb-install has no `container:`. On ubuntu-latest the deb BUILD job has already installed libwebkit2gtk on that machine, so a missing dependency would still install and this job would prove nothing.').toBeDefined();

    const image = job?.container?.image ?? '';
    expect(image, 'the container image must be named').not.toBe('');
    expect(
      image,
      `container image is "${image}". It must be a plain, minimal Ubuntu - the build ` +
        `runner's image already contains the stack this job exists to prove absent.`
    ).not.toMatch(/ubuntu-latest/);
    expect(image).toMatch(/^ubuntu:/);

    // And the host runner is not the container image; a job-level `container:`
    // replaces the filesystem, which is exactly the point.
    //
    // The key is literally `runs-on`. Reading it as `runsOn` — the idiomatic
    // camelCase an interface would suggest — yields undefined, and an
    // assertion written against `undefined` fails on a workflow that is right.
    //
    // `[FACT]` This one is not hypothetical: the interface declared `runsOn`
    // and the check read `job['runs-on']`, which `vitest` and `eslint` both
    // accept silently because neither type-checks. Only `pnpm check`
    // (svelte-check) rejected it, and that is the command CI runs and this
    // file's author did not.
    expect(
      job?.['runs-on'],
      'runs-on must stay ubuntu-latest so the container is what executes'
    ).toBe('ubuntu-latest');
  });
});

describe('deb-install — it installs the way a user does', () => {
  it('uses apt to resolve Depends, not a bare dpkg -i', () => {
    expect(
      stepRuns,
      'no apt-get install of the package. `dpkg -i` refuses to configure a package whose ' +
        'dependencies are unmet and leaves it half-registered, which is not what a user gets.'
    ).toMatch(/apt-get install[^\n]*\.\/pkg\/\*\.deb/);

    expect(
      stepRuns,
      'a `dpkg -i` here would either be the whole test (proving nothing about dependency ' +
        'resolution) or be repaired by an `apt-get -f install` afterwards, which hides exactly ' +
        'the defect being looked for.'
    ).not.toMatch(/dpkg -i/);
    expect(stepRuns).not.toMatch(/apt-get\s+-f\s+install/);
  });

  it('asserts the package is registered, not merely unpacked', () => {
    expect(stepRuns).toMatch(/dpkg-query/);
    expect(
      stepRuns,
      'must assert the exact "install ok installed" status; anything looser accepts a package ' +
        'that is unpacked but unconfigured'
    ).toMatch(/install ok installed/);
  });

  it('asserts the binary the package names is present and executable', () => {
    expect(stepRuns).toMatch(/dpkg -L/);
    expect(stepRuns).toMatch(/usr\/bin/);
    expect(stepRuns, 'must assert the binary is executable').toMatch(/-x "?\$bin"?|! -x/);
  });
});

describe('deb-install — it catches the defect the build job cannot see', () => {
  it('checks that every shared library the binary needs was actually installed', () => {
    // This is the assertion that earns the job its runner minutes.
    //
    // With `bundle.linux.deb.depends` empty or incomplete, `ldd` prints
    // `=> not found` for each missing library. The app fails to start on a
    // clean machine while EVERY structural check in `verify-bundle.sh` still
    // passes. Without this step the job would be "it installed" and nothing
    // more, which is a weaker claim than the build already made.
    expect(stepRuns, 'no ldd check in deb-install').toMatch(/ldd/);
    expect(stepRuns, 'no `not found` assertion on ldd output').toMatch(/not found/);
    expect(
      stepRuns,
      'the unresolved-library branch must fail the job'
    ).toMatch(/exit 1/);
  });

  it('validates the freedesktop entry rather than only counting it', () => {
    expect(stepRuns, 'the .desktop entry is checked only for existence').toMatch(
      /desktop-file-validate/
    );
    expect(stepRuns).toMatch(/desktop-file-utils/);
  });

  it('does not claim the GUI was observed running', () => {
    // The honest counterweight. A job that installs a package and then asserts
    // the window opens would be red for reasons packaging does not control, so
    // the boundary is recorded in the file rather than left implied.
    expect(
      stepRuns,
      'deb-install must not shell out to xvfb or otherwise assert the GUI starts; that fails ' +
        'for reasons unrelated to packaging and the job would be switched off.'
    ).not.toMatch(/xvfb-run/);
  });
});