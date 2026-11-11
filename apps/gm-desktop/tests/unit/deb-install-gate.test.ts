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
import { readdirSync, readFileSync } from 'node:fs';
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

/* -------------------------------------------------------------------------- *
 * The shell this job actually gets
 * -------------------------------------------------------------------------- */

/**
 * Constructs that `/bin/sh` cannot parse.
 *
 * Each entry pairs a pattern with the name it is reported under, so a failure
 * says WHICH construct and WHERE instead of only "some step is not bash".
 *
 * `[[`, `mapfile`, `shopt` and `source` are here for the same reason `pipefail`
 * is: they are extremely common in shell that was written on a hosted Ubuntu
 * image, where bash is the default and nothing has ever complained.
 */
const BASH_ONLY: ReadonlyArray<{ name: string; re: RegExp }> = [
  { name: 'set -o pipefail', re: /set\s+[^\n]*\bpipefail\b/ },
  { name: 'process substitution <(', re: /<\(/ },
  { name: 'here-string <<<', re: /<<</ },
  { name: '[[ ]] conditional', re: /\[\[/ },
  { name: 'mapfile/readarray', re: /\b(?:mapfile|readarray)\b/ },
  { name: 'shopt', re: /\bshopt\b/ },
  { name: 'source (POSIX uses `.`)', re: /^\s*source\s+\S/m },
  { name: '$RANDOM', re: /\$RANDOM\b/ },
  { name: 'printf -v', re: /printf\s+-v\b/ },
  { name: '(( )) arithmetic command', re: /(?:^|[;&|(]|\b(?:if|while|until|elif)\s+)\(\(/m },
  { name: 'shell parameter case-modification', re: /\$\{[A-Za-z_][A-Za-z0-9_]*(?:,,\}|\^\^})/ },
];

type Job = {
  'runs-on'?: string | string[];
  container?: unknown;
  defaults?: { run?: { shell?: string } };
  steps?: Array<{ name?: string; run?: string; shell?: string }>;
};

type Workflow = { defaults?: { run?: { shell?: string } }; jobs: Record<string, Job> };

/**
 * Strip comments the way a shell would: from `#` to end of line, but only when
 * `#` starts the line or follows whitespace.
 *
 * This is deliberately the same treatment `stepRuns` above gets, for the same
 * reason. A workflow that explains `set -o pipefail` in a comment must not be
 * reported as using it — and, far worse, must not be able to SATISFY a check by
 * mentioning it.
 */
const stripComments = (text: string): string =>
  text
    .split('\n')
    .map((line) => line.replace(/(^|\s)#.*$/, ''))
    .join('\n');

/**
 * The shell GitHub Actions will use for a `run:` step, with the precedence the
 * runner actually applies: step `shell:` > job `defaults.run.shell` >
 * workflow `defaults.run.shell` > platform default.
 *
 * [FACT] The platform default is the whole point of this function. A job with
 * `container:` gets `/bin/sh` — on `ubuntu:24.04` that is dash — while the same
 * YAML without the `container:` block gets bash. Nothing in the step body
 * changes, so nothing in the step body warns you.
 */
function effectiveShell(
  job: Job,
  step: { shell?: string },
  workflowDefault?: string
): string {
  const explicit = step.shell ?? job.defaults?.run?.shell ?? workflowDefault;
  if (explicit) return explicit;

  if (job.container) return 'sh';
  const runsOn = job['runs-on'];
  const label = Array.isArray(runsOn) ? runsOn.join(',') : (runsOn ?? '');
  return /windows/i.test(label) ? 'pwsh' : 'bash';
}

const WORKFLOW_DIR = join(REPO_ROOT, '.github', 'workflows');
const workflowFiles = readdirSync(WORKFLOW_DIR)
  .filter((f) => /\.ya?ml$/.test(f))
  .sort();

/** Every step in every workflow, paired with the shell it will really get. */
const allRunSteps = workflowFiles.flatMap((file) => {
  const doc = parse(readFileSync(join(WORKFLOW_DIR, file), 'utf8')) as Workflow;
  return Object.entries(doc.jobs ?? {}).flatMap(([jobId, job]) =>
    (job.steps ?? [])
      .filter((s): s is { name?: string; run: string; shell?: string } =>
        typeof s.run === 'string' && s.run.trim() !== ''
      )
      .map((step) => ({
        file,
        jobId,
        name: step.name ?? step.run.split('\n')[0].slice(0, 60),
        body: stripComments(step.run),
        shell: effectiveShell(job, step, doc.defaults?.run?.shell),
        isContainer: Boolean(job.container),
      }))
  );
});

/** Container jobs across the repo, for the scan-coverage assertions below. */
const containerJobs = workflowFiles.flatMap((file) => {
  const doc = parse(readFileSync(join(WORKFLOW_DIR, file), 'utf8')) as Workflow;
  return Object.entries(doc.jobs ?? {})
    .filter(([, job]) => job.container)
    .map(([jobId, job]) => ({
      file,
      jobId,
      shell: job.defaults?.run?.shell,
      overrides: (job.steps ?? []).map((s) => s.shell).filter((s): s is string => Boolean(s)),
    }));
});

describe('every `run:` step runs under a shell that can parse it', () => {
  /*
   * Mutation evidence, recorded because a gate that has never been fed a real
   * failure is not a gate:
   *
   * 1. Delete `defaults.run.shell: bash` from the deb-install job.
   *    -> 2 red / 468 green. Both failures name the offending step and the
   *       construct, e.g. `gm-desktop-bundle.yml :: deb-install :: The
   *       freedesktop entry a package manager would show is valid` using
   *       `set -o pipefail` and `process substitution <(`.
   * 2. Add an unrelated workflow with its own container job and one
   *    bash-only step, deb-install untouched and still correct.
   *    -> 2 red / 468 green, both pointing at the new file. This is the
   *       mutation that shows the invariant is repo-wide rather than fitted to
   *       the job that happened to have the bug.
   * 3. Keep that container job but move the bash-only syntax into a comment.
   *    -> 1 red / 469 green: the declaration gate still fails, the syntax gate
   *       goes green. Prose that says `set -o pipefail` is not code that uses
   *       it, and the two tests are pinning different properties.
   */

  it('scanned the workflows it claims to scan', () => {
    // Guards the guard. A path typo or a `readdirSync` filter that matches
    // nothing makes every assertion below pass vacuously, and the suite still
    // reports green. Measured by deliberately narrowing the filter once.
    expect(workflowFiles.length, 'no workflow files found').toBeGreaterThan(0);
    expect(allRunSteps.length, 'no `run:` steps found in any workflow').toBeGreaterThan(0);
    expect(
      containerJobs.length,
      'expected at least the deb-install job to have a `container:`. If this is zero, ' +
        'the container was removed — which is exactly the change that would make ' +
        'deb-install prove nothing — or this scan is not looking where it thinks it is.'
    ).toBeGreaterThan(0);
  });

  it('recognises bash-only syntax (or the recogniser is broken, not the workflows)', () => {
    // The scanner's own coverage, asserted against itself.
    //
    // Every construct in BASH_ONLY gets fed to the same detection used on the
    // workflows. If a pattern is wrong, this goes red here instead of the
    // workflows quietly scanning as clean forever.
    const samples: ReadonlyArray<readonly [string, string]> = [
      ['set -euo pipefail', 'set -euo pipefail'],
      ['done < <(dpkg -L "$pkg")', 'done < <(dpkg -L x)'],
      ['cat <<< "$x"', 'cat <<< "$x"'],
      ['if [[ -n "$x" ]]; then', 'if [[ -n "$x" ]]; then'],
      ['mapfile -t deps < <(x)', 'mapfile -t deps < <(x)'],
      ['shopt -s nullglob', 'shopt -s nullglob'],
      ['source /etc/os-release', 'source /etc/os-release'],
      ['echo $RANDOM', 'echo $RANDOM'],
      ['printf -v out %s x', 'printf -v out %s x'],
      ['if (( n > 1 )); then', 'if (( n > 1 )); then'],
      ['echo "${v^^}"', 'echo "${v^^}"'],
    ];

    const missed = samples.filter(([, sample]) => !BASH_ONLY.some((c) => c.re.test(sample)));
    expect(
      missed.map(([name]) => name),
      'BASH_ONLY no longer detects these. A broken pattern makes every workflow look clean.'
    ).toEqual([]);
  });

  it('no step uses bash-only syntax while resolving to sh', () => {
    // [FACT] This gate is not hypothetical. Run 37551742758, job
    // `deb installs and links on a stock Ubuntu`, died on its first line:
    //
    //   /__w/_temp/1b0f7435-....sh: 1: set: Illegal option -o pipefail
    //   ##[error]Process completed with exit code 2.
    //
    // The job is reported as failing on `apt-get install`, but the package was
    // never touched. The `deb` job above it had already gone green, and every
    // local gate over this YAML passed, because the step body is valid bash and
    // a machine with no `container:` runs bash by default.
    const offenders = allRunSteps
      .filter((s) => s.shell !== 'bash')
      .map((s) => ({
        where: `${s.file} :: ${s.jobId} :: ${s.name}`,
        shell: s.shell,
        used: BASH_ONLY.filter((c) => c.re.test(s.body)).map((c) => c.name),
      }))
      .filter((o) => o.used.length > 0);

    expect(
      offenders,
      'these steps use bash-only syntax but do not run under bash. A job with `container:` ' +
        'gets /bin/sh (dash on ubuntu:24.04), which cannot parse them.'
    ).toEqual([]);
  });

  it('declares bash for the container jobs it found, at job level', () => {
    // Job-level rather than per-step, so a step added later inherits bash
    // instead of silently reverting to dash. Step-level overrides are allowed
    // but must not exist here: they are the mechanism by which the guarantee
    // quietly stops applying to a single step.
    expect(
      containerJobs.map((j) => ({ where: `${j.file} :: ${j.jobId}`, shell: j.shell })),
      'a container job runs `run:` steps under /bin/sh unless it says otherwise'
    ).toEqual(
      containerJobs.map((j) => ({ where: `${j.file} :: ${j.jobId}`, shell: 'bash' }))
    );
    expect(
      containerJobs.flatMap((j) => j.overrides).filter((s) => s !== 'bash'),
      'a step-level `shell:` in a container job overrides the job default and can drop a step back to sh'
    ).toEqual([]);
  });
});
