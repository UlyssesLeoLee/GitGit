/**
 * Gates on the `msi-install` job.
 *
 * ## Why this job exists
 *
 * `verify-msi.ps1` (the `msi` job) reads the package: it extracts the FILE
 * table with `msiexec /a` and reads the MSI tables through Windows Installer
 * COM automation. It never installs anything, and its own last line says so:
 *
 *     PASS: 4 msi verified (payload + scope, no install, no elevation)
 *
 * So the repository proves that the `.deb` INSTALLS — the
 * `deb installs and links on a stock Ubuntu` job runs a real `apt-get install`
 * in a bare `ubuntu:24.04`, a machine that has nothing on it — and proves only
 * that the `.msi` CONTAINS what it should. The Windows primary artefact is
 * the only shipping artefact with no install proof, and for a commercial
 * Windows product "it installs and it uninstalls cleanly" is the most basic
 * release property there is.
 *
 * ## What these gates are for
 *
 * A gate that has never been fed a failing input is not proven, and a gate
 * whose assertion can be satisfied by an unrelated occurrence is not a gate.
 * The failure mode this repository has actually hit — an assertion about "the
 * licence block calls `die`" being satisfied by a second, unrelated `die` in
 * the same branch — shapes every assertion below:
 *
 *   * the workflow is PARSED as YAML, and only `run:` bodies are scanned, so a
 *     YAML comment cannot satisfy an assertion;
 *   * shell comments are stripped from those bodies before scanning, because
 *     this file's own comments explain at length what the job does NOT do;
 *   * the PowerShell script is scanned with BOTH its block comments
 *     (`<# ... #>`) and its line comments removed — the script's header
 *     mentions `gm-desktop.exe` by name, and a grep that can see that
 *     paragraph would "prove" anything about whether the app is launched.
 *
 * ## What is deliberately NOT claimed
 *
 * That the GUI opens. The desktop application has never been observed
 * running, and this repository records that as a deliberate, explained
 * boundary (`.github/CI.md`, `docs/plan/v0-tasks.md`). What is proven is what
 * packaging controls. The job does not launch the app, and the gates below
 * make that structural rather than a promise in a comment.
 */
import { describe, expect, it } from 'vitest';
import { existsSync, readFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { parse } from 'yaml';

const REPO_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..', '..', '..', '..');
const WORKFLOW = join(REPO_ROOT, '.github', 'workflows', 'gm-desktop.yml');
const SCRIPT = join(REPO_ROOT, 'apps', 'gm-desktop', 'scripts', 'verify-msi-install.ps1');

type Step = {
  name?: string;
  run?: string;
  uses?: string;
  with?: Record<string, unknown>;
  shell?: string;
};

const DOC = parse(readFileSync(WORKFLOW, 'utf8')) as {
  jobs: Record<string, {
    needs?: string | string[];
    'runs-on'?: string;
    steps?: Step[];
  }>;
};

const job = DOC.jobs['msi-install'];
const steps = job?.steps ?? [];

/**
 * The `run:` bodies with shell comments stripped.
 *
 * A `#` at the start of a line, or after whitespace, is a comment in these
 * bodies. This file's own comments explain what the job must not do, so a
 * check over the raw text would both false-fire on the explanation and, far
 * worse, could be SATISFIED by it.
 */
const jobRuns = steps
  .map((s) => s.run ?? '')
  .join('\n')
  .split('\n')
  .map((line) => line.replace(/(^|\s)#.*$/, ''))
  .join('\n');

/**
 * The PowerShell script with BOTH kinds of comment removed.
 *
 * `<# ... #>` is a block comment that can run to many lines, and this
 * script's header — which names `gm-desktop.exe` several times, in prose —
 * is one. A scan that can see it would "prove" the app is launched, or
 * fail to prove it is not, depending on which way the assertion pointed.
 */
function stripPowerShellComments(source: string): string {
  const withoutBlocks = source.replace(/<#[\s\S]*?#>/g, '');
  return withoutBlocks
    .split('\n')
    .map((line) => line.replace(/(^|\s)#.*$/, ''))
    .join('\n');
}

const scriptSource = existsSync(SCRIPT) ? readFileSync(SCRIPT, 'utf8') : '';
const scriptCode = stripPowerShellComments(scriptSource);

describe('msi-install — the job exists and tests the artefact that was built', () => {
  it('found the job', () => {
    expect(
      Object.keys(DOC.jobs),
      'no `msi-install` job in gm-desktop.yml, so nothing proves that the MSI installs'
    ).toContain('msi-install');
  });

  it('needs the msi job, so the artefact under test is the one that was built', () => {
    const needs = job?.needs;
    const list = Array.isArray(needs) ? needs : needs ? [needs] : [];
    expect(
      list,
      '`msi-install` must need the `msi` job; a second build would be a different set of bytes'
    ).toContain('msi');
  });

  it('runs on windows-latest', () => {
    expect(
      job?.['runs-on'],
      'the proof has to run on the platform the artefact ships to'
    ).toBe('windows-latest');
  });

  it('downloads the built MSIs and the manifest into one directory', () => {
    const download = steps.find((s) => (s.uses ?? '').startsWith('actions/download-artifact@'));

    expect(download, 'the job does not download the artefact the msi job built').toBeDefined();

    const with_ = download?.with ?? {};
    expect(
      String(with_.pattern ?? ''),
      'the download pattern must cover the per-scope installers and the manifest'
    ).toMatch(/^gm-desktop-msi-/);

    expect(
      with_['merge-multiple'],
      'the installers and the manifest have to land in ONE directory, because the script ' +
        'reads the manifest from the same directory as the packages'
    ).toBe(true);
  });

  it('runs the install proof, and fails the step when it fails', () => {
    expect(
      jobRuns,
      'the job does not run verify-msi-install.ps1, so nothing is installed'
    ).toContain('verify-msi-install.ps1');

    expect(
      jobRuns,
      'the script is invoked without the directory holding the artefacts'
    ).toMatch(/verify-msi-install\.ps1[^\n]*-Dir\s+msi-out/);

    // A `pwsh -command` step that only runs a failing native command reports
    // the step as PASSED, because the failure lands in $LASTEXITCODE and
    // nobody looks. The explicit check is what makes it live code.
    expect(
      jobRuns,
      'the step does not check $LASTEXITCODE, so a failed install would be reported as a ' +
        'passed step'
    ).toMatch(/\$LASTEXITCODE\s*-ne\s*0/);
  });
});

describe('msi-install — the script is committed and does what the job says', () => {
  it('the script exists in the repository, not only in a worktree', () => {
    expect(
      existsSync(SCRIPT),
      'verify-msi-install.ps1 is not in the repository, so the job would fail on a clean checkout'
    ).toBe(true);
  });

  it('installs and uninstalls for real', () => {
    expect(
      scriptCode,
      'the script never calls msiexec, so nothing is installed'
    ).toMatch(/msiexec\.exe/);

    expect(
      scriptCode,
      'the script never uninstalls, so a package that cannot be removed would pass'
    ).toMatch(/'\/x'/);
  });

  it('does not launch the application', () => {
    // The script's only `Start-Process` is the msiexec client. Anything else
    // would be the job starting the app, which is the boundary this
    // repository deliberately draws.
    const startProcessTargets = [
      ...scriptCode.matchAll(/Start-Process[^\n]*?-FilePath\s+'([^']+)'/g),
    ].map((m) => m[1]);

    expect(
      startProcessTargets,
      'the only process the script may start is msiexec.exe'
    ).toEqual(['msiexec.exe']);

    // And the boundary is ENFORCED, not just stated: after an install the
    // script asserts that the application is not running.
    //
    // The assertion is on the COMPARISON, not on the prose. An earlier
    // version of this gate matched the words "is running after the install" —
    // which live in the `throw` message — so replacing the real check
    // `if ($running.Count -gt 0)` with `if ($false)` left the gate green.
    // The message is the explanation; the comparison is the check.
    expect(
      scriptCode,
      'the script does not assert that the application is not running after an install, so the ' +
        '"installs, does not launch" boundary is a promise rather than a check'
    ).toMatch(/Get-Process[\s\S]{0,200}?\$running\.Count\s*-gt\s*0/);
  });

  it('the job does not launch the application either', () => {
    expect(
      jobRuns,
      'the job launches the application, which this repository deliberately does not do'
    ).not.toMatch(/gm-desktop\.exe/);
  });

  it('reads its expectations out of the package rather than hardcoding them', () => {
    // The proof is only worth something if it checks the artefact in front of
    // it. A hardcoded install path would pass on a package whose layout
    // changed.
    expect(
      scriptCode,
      'the script never reads the MSI tables, so its expectations are hardcoded rather than ' +
        'taken from the package under test'
    ).toMatch(/OpenView/);

    expect(
      scriptCode,
      'the script does not read the manifest, so it cannot check that the artefact under test ' +
        'is the one that was built'
    ).toMatch(/manifest\.json/);
  });

  it('parses as PowerShell', () => {
    // [FACT] This gate exists because the first version of the script did not
    // parse, and nothing here noticed: 48 KB of PowerShell with 36 invalid
    // variable references, every one of them of the form `"$Label: ..."`. In
    // PowerShell `$Label:` is not a variable reference — the parser reads the
    // colon as part of the name and stops with "Variable reference is not
    // valid", so the file was dead on arrival. The CI job caught it in twelve
    // seconds; nothing local had, because nothing local had RUN it.
    //
    // The check is a pattern rather than a parse, because the unit suite must
    // not require a PowerShell host to run. It targets the one syntactic trap
    // this script walked into, and the parse itself is the CI job's job.
    const invalidVariableReferences = [
      ...scriptCode.matchAll(/"\$[A-Za-z_][A-Za-z0-9_]*:/g),
    ].map((m) => m[0]);

    expect(
      invalidVariableReferences,
      'these are not variable references in PowerShell — the parser reads the colon as part ' +
        'of the name and the file does not parse at all. Write ${Var}: instead.'
    ).toEqual([]);
  });
});
