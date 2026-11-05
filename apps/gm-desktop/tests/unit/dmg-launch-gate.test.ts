/**
 * Gates on the macOS launch proof — the part of the `dmg` job that asks
 * whether the application inside the disk image actually STARTS.
 *
 * ## Why this exists
 *
 * The `dmg` job builds the macOS disk image and OPENS it: `hdiutil
 * imageinfo` parses the image, it is mounted read-only, and the `.app`
 * inside it is checked for a real Mach-O binary and for the manifest's
 * version. That is a statement about the PACKAGE. It says nothing about
 * the APPLICATION — and for years the application inside the macOS image
 * had never been started on any machine, which the repository recorded in
 * plain words rather than glossing over.
 *
 * The Windows lane already broke that symmetry: `verify-msi-launch.ps1`
 * installs the packaged application, launches it, and waits for the webview
 * to report that it booted. This file pins the macOS counterpart of that
 * proof, because a proof nobody re-checks is a proof that quietly stops
 * proving anything.
 *
 * ## The failure modes this has to catch
 *
 *   * a job that was quietly stripped of its launch step;
 *   * a launch proof that stops looking for the marker, or that accepts a
 *     marker without this launch's token — a marker left by an earlier
 *     run would satisfy it, and the gate would go green without this
 *     launch having booted anything;
 *   * a check that PASSES when it could not check: a missing image, an
 *     image that cannot be mounted, an unknown executable — those must be
 *     failures, not passes;
 *   * a "red path" that is really a skip: the proof has to be shown to go
 *     red, not merely be capable of going red;
 *   * a workaround for Gatekeeper that would not apply on a user's
 *     machine — the point of the proof is what a user gets.
 */
import { describe, expect, it } from 'vitest';
import { existsSync, readFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { parse } from 'yaml';

const REPO_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..', '..', '..', '..');
const WORKFLOW = join(REPO_ROOT, '.github', 'workflows', 'gm-desktop-bundle.yml');
const LAUNCH_SCRIPT = join(REPO_ROOT, 'apps', 'gm-desktop', 'scripts', 'verify-dmg-launch.ps1');
const REQUIRED_CHECKS = join(REPO_ROOT, 'scripts', 'ci', 'required-checks.json');

type Step = {
  name?: string;
  run?: string;
  uses?: string;
  shell?: string;
  if?: string;
};

const doc = parse(readFileSync(WORKFLOW, 'utf8')) as {
  jobs: Record<string, {
    name?: string;
    'runs-on'?: string;
    needs?: string | string[];
    steps?: Step[];
  }>;
};

const dmgJob = doc.jobs['dmg'];
const debInstallJob = doc.jobs['deb-install'];

const scriptExists = existsSync(LAUNCH_SCRIPT);
const script = scriptExists ? readFileSync(LAUNCH_SCRIPT, 'utf8') : '';

const dmgSteps = dmgJob?.steps ?? [];
const dmgRuns = dmgSteps.map((s) => s.run ?? '').join('\n');
const redPathStep = dmgSteps.find((s) => (s.run ?? '').includes('-SelfTestRedPath'));
// The launch step is the one that runs the proof WITHOUT the self-test
// mode; both steps name the same script, so the self-test marker is what
// tells them apart.
const launchStep = dmgSteps.find(
  (s) =>
    (s.run ?? '').includes('verify-dmg-launch.ps1') &&
    !(s.run ?? '').includes('-SelfTestRedPath')
);

const requiredChecks = JSON.parse(readFileSync(REQUIRED_CHECKS, 'utf8')) as {
  contexts: string[];
};

describe('dmg — the job exists, keeps its name, and runs on the macOS runner', () => {
  it('found the job', () => {
    expect(
      Object.keys(doc.jobs),
      'no `dmg` job, so nothing proves the macOS artefact was ever built'
    ).toContain('dmg');
  });

  it('keeps the exact name the branch protection requires', () => {
    // [FACT] The job names are the pinned required status-check contexts
    // (`scripts/ci/required-checks.json`). Renaming one makes the branch
    // protection wait for a check that is never reported, and the pull
    // request deadlocks.
    expect(dmgJob?.name, 'the `dmg` job was renamed').toBe('dmg (unsigned, ad-hoc app)');
    expect(
      requiredChecks.contexts,
      'the required-checks file no longer names the `dmg` job, so the branch protection and the job disagree'
    ).toContain('dmg (unsigned, ad-hoc app)');
  });

  it('runs on macos-latest', () => {
    expect(
      dmgJob?.['runs-on'],
      'the launch has to happen on the platform the artefact ships to'
    ).toBe('macos-latest');
  });

  it('adds no new required context — the proof rides on the existing job', () => {
    // [FACT] A NEW job would have to be added to the required-checks file
    // AND to the branch protection in the same change, and this lane is
    // forbidden from touching the branch protection. Extending the
    // existing job is what makes the proof enforceable without either.
    expect(
      requiredChecks.contexts.length,
      'the number of required contexts changed, so a job was added or removed'
    ).toBe(14);
  });
});

describe('dmg — the job launches the application and checks the exit code', () => {
  it('runs the launch proof', () => {
    expect(dmgRuns, 'the job does not run verify-dmg-launch.ps1').toContain(
      'verify-dmg-launch.ps1'
    );
  });

  it('fails the step when the proof fails', () => {
    // [FACT] A `pwsh -File` step whose child exits non-zero does NOT fail
    // the step by itself: the step's own exit code is the child's, unless
    // the step checks it. Without the check the step would report success
    // with a red proof inside it.
    expect(
      launchStep?.run ?? '',
      'the launch step does not check $LASTEXITCODE, so a failed launch would be reported as a passed step'
    ).toMatch(/\$LASTEXITCODE\s*-ne\s*0/);
  });

  it('the launch step and the red-path step are different steps', () => {
    expect(launchStep, 'no step runs the launch proof').toBeDefined();
    expect(redPathStep, 'no step proves the launch proof can fail').toBeDefined();
    expect(redPathStep?.name).not.toBe(launchStep?.name);
  });

  it('the red path REQUIRES a failure, and does not accept a skip', () => {
    // [FACT] A gate that has never been shown to fail is not known to be a
    // gate. The red-path step runs the proof WITHOUT starting the
    // application and requires a non-zero exit: the only process that could
    // write this launch's marker was never started, so the proof must go
    // red.
    const red = redPathStep?.run ?? '';
    expect(red, 'the red-path step does not ask for the self-test mode').toContain(
      '-SelfTestRedPath'
    );
    expect(
      red,
      'the red-path step does not fail the build when the proof PASSES without starting the application'
    ).toMatch(/\$code\s+-eq\s+0/);
    expect(
      red,
      'the red-path step does not require the failure to carry the LAUNCH PROOF FAILED message'
    ).toMatch(/LAUNCH PROOF FAILED/);
    expect(
      red,
      'the red-path step accepts any non-zero exit, so a setup error would be read as the expected proof failure'
    ).toMatch(/\$code\s+-ne\s+1/);
  });

  it('both steps are conditioned on the area, like every other step of this job', () => {
    for (const step of [redPathStep, launchStep]) {
      expect(
        step?.if,
        `the step "${step?.name}" has no area condition, so it runs on every pull request`
      ).toContain("steps.detect.outputs.gm_desktop_bundle == 'true'");
    }
  });

  it('both steps decide their OWN verdict, with an explicit exit code', () => {
    // [FACT] The FIRST CI run of the red-path step ended with exit code 1
    // even though every one of its checks had passed: the only non-zero
    // exit code in that step was the child's, which is SUPPOSED to be 1
    // (the application is deliberately not started, so the proof must
    // fail). The raw log carried no error record at all between the
    // proof's own last line and GitHub's "Process completed with exit code
    // 1". Whether the host PowerShell propagates that as the session's
    // exit code is a property of the runner's PowerShell version and not
    // of this repository — so the step must not depend on it.
    for (const step of [redPathStep, launchStep]) {
      expect(
        step?.run ?? '',
        `the step "${step?.name}" has no explicit exit code, so its verdict is whatever the host PowerShell happens to propagate`
      ).toMatch(/^\s*exit 0\s*$/m);
    }
  });

  it('the red-path step does not let the child\'s expected failure become a terminating error', () => {
    // [FACT] The child is SUPPOSED to exit non-zero here. Under
    // `$ErrorActionPreference = 'Stop'` a native command that exits
    // non-zero raises a terminating error when the native-error preference
    // is on, and the step would abort before it could read the code that
    // is the whole point of the step. The repository's own PowerShell
    // scripts switch that preference off for the same reason.
    expect(
      redPathStep?.run ?? '',
      'the red-path step does not switch off the native-error preference'
    ).toMatch(/\$PSNativeCommandUseErrorActionPreference\s*=\s*\$false/);
  });
});

describe('dmg — the script checks the right thing', () => {
  it('exists', () => {
    expect(scriptExists, 'apps/gm-desktop/scripts/verify-dmg-launch.ps1 is missing').toBe(true);
  });

  it('mounts the image READ-ONLY and launches out of the mounted volume', () => {
    expect(
      script,
      'the script does not mount the image read-only'
    ).toMatch(/hdiutil attach -readonly/);
    expect(
      script,
      'the script does not launch the application out of the mounted volume'
    ).toMatch(/Contents' \(Join-Path 'MacOS'/);
    expect(
      script,
      'the script does not start the application it mounted'
    ).toMatch(/Start-Process\s+-FilePath\s+\$appExec/);
  });

  it('requires the marker to carry THIS launch\'s token', () => {
    // [FACT] The application's log file lives in the user's profile and
    // survives a launch, so a marker written by an EARLIER run can be
    // sitting in it. A check that greps for a constant marker can be
    // satisfied by history.
    expect(script, 'the script does not mint a one-shot token').toMatch(/New-BootNonce/);
    expect(
      script,
      'the script does not search for the marker with that token'
    ).toMatch(/Get-BootMarkerText/);
    expect(
      script,
      'the script does not look for the marker in the application\'s own log files'
    ).toMatch(/Find-BootMarkerInLogs/);
  });

  it('fails, with a message, when the marker does not arrive', () => {
    expect(
      script,
      'the script has no distinct proof-failure exit code'
    ).toMatch(/exit \$exitCode/);
    expect(
      script,
      'the script does not print a message that says what failed'
    ).toMatch(/LAUNCH PROOF FAILED/);
    expect(
      script,
      'the script does not have a distinct setup-error exit code'
    ).toMatch(/exit 2/);
  });

  it('the failure message says what was EXPECTED and what was FOUND', () => {
    expect(
      script,
      'the failure message does not say what the marker should have contained'
    ).toMatch(/the webview never reported that the UI booted/);
    expect(
      script,
      'the failure message does not report where the log was looked for'
    ).toMatch(/the application's log directories, and what each of them holds/);
  });

  it('stops the application and unmounts the image afterwards', () => {
    expect(
      script,
      'the script does not stop the application it started'
    ).toMatch(/\.Kill\(\)/);
    expect(
      script,
      'the script does not unmount the disk image it mounted'
    ).toMatch(/hdiutil detach/);
  });

  it('does not work around Gatekeeper', () => {
    // [FACT] The `.dmg` is ad-hoc signed and not notarized, and that is
    // what a user receives. A runner with Gatekeeper switched off would
    // prove something a user's machine does not have, so the proof has to
    // run the application exactly as it is and report what macOS says.
    //
    // The COMMANDS are what is forbidden, not the words: the script's
    // prose says out loud that it does not clear the quarantine attribute,
    // and that sentence must not be mistaken for the thing it forbids.
    const forbidden = [/spctl\s+--master-disable/, /\bxattr\b/, /\bcodesign\b/];
    for (const pattern of forbidden) {
      expect(
        script,
        `the script runs ${pattern}, which is a Gatekeeper workaround that would not apply on a user's machine`
      ).not.toMatch(pattern);
    }
  });
});

describe('dmg — the Linux container case is untouched', () => {
  it('the deb-install job still does not start a window', () => {
    // [FACT] That boundary is deliberate and is written into the job: a
    // webkit/GTK application fails in a container with no dbus, no seat and
    // no hardware, for reasons that have nothing to do with packaging, and
    // a gate that reddens for those reasons gets switched off. This lane
    // is about macOS only.
    const debRuns = (debInstallJob?.steps ?? []).map((s) => s.run ?? '').join('\n');
    expect(
      debRuns,
      'the Linux install job now starts a window, which is not what it is for'
    ).not.toContain('verify-dmg-launch.ps1');
    expect(
      debRuns,
      'the Linux install job now starts a window, which is not what it is for'
    ).not.toContain('verify-msi-launch.ps1');
  });
});
