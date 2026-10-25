/**
 * Gates on the `msi-launch` job — the one that asks whether the packaged
 * application actually STARTS.
 *
 * ## Why this is a separate job
 *
 * `msi-install` proves the Windows package installs, registers and removes
 * itself, and deliberately does not launch the application. The reason it
 * gives is a Linux one: a webkit/GTK app fails to start in a container with
 * no dbus, no seat and no hardware, for reasons that have nothing to do
 * with packaging, and a gate that reddens for those reasons gets switched
 * off.
 *
 * That reason does not hold on `windows-latest`, which has a real
 * interactive desktop and the WebView2 runtime in the image. So the same
 * question can be asked of the PACKAGED artefact — which is a stronger
 * claim than anything the repository has been able to make so far.
 *
 * The two jobs therefore have different contracts, and the boundary
 * between them is asserted here: the install proof must NOT start the
 * application, and the launch proof must.
 *
 * ## The failure modes this has to catch
 *
 *   * a job that was quietly removed, or one that no longer launches
 *     anything;
 *   * a launch that is not actually waited for — Windows Installer can
 *     return before the work is done, and an application can exit a
 *     second later, so "the process was started" is not "the application
 *     started";
 *   * a check that passes when it could not check: a missing package, an
 *     unknown executable name, a missing `msiexec` — those must be
 *     failures, not passes.
 */
import { describe, expect, it } from 'vitest';
import { existsSync, readFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { parse } from 'yaml';

const REPO_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..', '..', '..', '..');
const WORKFLOW = join(REPO_ROOT, '.github', 'workflows', 'gm-desktop.yml');
const LAUNCH_SCRIPT = join(REPO_ROOT, 'apps', 'gm-desktop', 'scripts', 'verify-msi-launch.ps1');
const INSTALL_SCRIPT = join(REPO_ROOT, 'apps', 'gm-desktop', 'scripts', 'verify-msi-install.ps1');

type Step = {
  name?: string;
  run?: string;
  uses?: string;
  with?: Record<string, unknown>;
};

const doc = parse(readFileSync(WORKFLOW, 'utf8')) as {
  jobs: Record<string, {
    needs?: string | string[];
    'runs-on'?: string;
    steps?: Step[];
  }>;
};

const launchJob = doc.jobs['msi-launch'];
const installJob = doc.jobs['msi-install'];

const scriptExists = existsSync(LAUNCH_SCRIPT);
const script = scriptExists ? readFileSync(LAUNCH_SCRIPT, 'utf8') : '';

const launchRuns = (launchJob?.steps ?? []).map((s) => s.run ?? '').join('\n');
const installRuns = (installJob?.steps ?? []).map((s) => s.run ?? '').join('\n');

describe('msi-launch — the job exists and tests the packaged artefact', () => {
  it('found the job', () => {
    expect(
      Object.keys(doc.jobs),
      'no `msi-launch` job, so nothing proves the packaged application starts'
    ).toContain('msi-launch');
  });

  it('tests the package the msi job built, not a rebuild', () => {
    const needs = launchJob?.needs;
    const list = Array.isArray(needs) ? needs : needs ? [needs] : [];
    expect(
      list,
      '`msi-launch` must need the `msi` job, or it would be testing a different build'
    ).toContain('msi');
  });

  it('runs on windows-latest', () => {
    expect(
      launchJob?.['runs-on'],
      'the launch has to happen on the platform the artefact ships to'
    ).toBe('windows-latest');
  });

  it('runs the launch proof and fails the step when it fails', () => {
    expect(launchRuns, 'the job does not run verify-msi-launch.ps1').toContain(
      'verify-msi-launch.ps1'
    );
    expect(
      launchRuns,
      'the step does not check $LASTEXITCODE, so a failed launch would be reported as a passed step'
    ).toMatch(/\$LASTEXITCODE\s*-ne\s*0/);
  });

  it('downloads the package it is going to launch', () => {
    const download = (launchJob?.steps ?? []).find((s) =>
      (s.uses ?? '').startsWith('actions/download-artifact@')
    );
    expect(download, 'the job does not download the built package').toBeDefined();
    expect(
      String(download?.with?.pattern ?? ''),
      'the download must reach the perMachine package'
    ).toMatch(/perMachine/);
  });
});

describe('msi-launch — the boundary between the two jobs holds', () => {
  it('the install proof does not launch the application', () => {
    // `msi-install`'s contract is "installs and removes; it does not
    // launch". A job that quietly started the app would turn the
    // container's start-up failures into a red gate on the wrong
    // platform.
    expect(
      installRuns,
      'the install proof now launches the application, which is not what it is for'
    ).not.toContain('verify-msi-launch.ps1');
  });

  it('the launch proof is a different script, and the install proof is untouched', () => {
    expect(scriptExists, 'apps/gm-desktop/scripts/verify-msi-launch.ps1 is missing').toBe(true);
    expect(
      existsSync(INSTALL_SCRIPT),
      'the install proof script is missing — the boundary cannot be asserted without it'
    ).toBe(true);
  });
});

describe('msi-launch — the script checks the right thing', () => {
  it('launches the INSTALLED executable, and waits for a window', () => {
    // "The process was started" is not "the application started": a
    // process can exit a second later, and a window can be created and
    // never shown.
    expect(
      script,
      'the script never starts the installed executable'
    ).toMatch(/Start-Process\s+-FilePath\s+\$exe/);
    expect(
      script,
      'the script does not wait for a top-level window to appear'
    ).toMatch(/MainWindowHandle/);
  });

  it('fails when the application exits without a window', () => {
    expect(
      script,
      'the script does not distinguish "the application died" from "the application is running"'
    ).toMatch(/started and then exited/);
  });

  it('a setup problem is a failure, not a pass', () => {
    // A missing package, an unknown executable name: these must not
    // produce a green job that checked nothing.
    expect(
      script,
      'the script has no distinct setup-error exit code'
    ).toMatch(/exit 2/);
    expect(
      script,
      'the script can report success without ever launching anything'
    ).not.toMatch(/exit 0\s*\r?\n\s*Write-Ok 'PASS[^']*'/);
  });

  it('searches for the executable BY ITS FILE NAME, extension and all', () => {
    // [FACT] `Get-ChildItem -Filter` matches the WHOLE file name, so
    // `-Filter 'gm-desktop'` does NOT match 'gm-desktop.exe'. The first CI
    // run of this job failed on exactly that: the install had succeeded,
    // the file was sitting in Program Files, and the check reported that
    // no executable had been found. The application was never even
    // started.
    expect(
      script,
      'the script does not derive the executable FILE name from the crate name, so the file ' +
        "search can never match the file that is actually installed"
    ).toMatch(/\$exeFile\s*=\s*if\s*\(\$ExeName\s+-like\s+'\*\.exe'\)/);
    expect(
      script,
      'the file search does not use the executable FILE name'
    ).toMatch(/-Filter\s+\$exeFile/);
  });

  it('removes the package and stops the process afterwards', () => {
    expect(
      script,
      'the script does not stop the application it started'
    ).toMatch(/\.Kill\(\)/);
    expect(
      script,
      'the script does not uninstall the package it installed'
    ).toMatch(/'\/x'/);
  });
});
