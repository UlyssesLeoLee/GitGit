/**
 * Gates on the MSI signing wiring.
 *
 * ## Why this exists
 *
 * `release.yml` shipped a draft release whose body said, unconditionally,
 *
 *     **Unsigned.** No code-signing certificate is configured for this project
 *
 * That was true when it was written. The moment signing was switched on the
 * sentence became a lie, and a draft release that misdescribes its own
 * artifacts is worse than one that says nothing: the sentence is what a human
 * reads before pressing Publish.
 *
 * The failure this file guards against is not "someone forgot to sign". It is
 * the quieter one: the workflow believing it signed, or the release claiming
 * something the build did not establish. Both are checked here rather than
 * trusted.
 *
 * ## What each gate actually holds
 *
 * 1. `release.yml` signs with `-Required`. Without it a missing secret is a
 *    printed NOTE and exit 0 — correct for a pull request, and exactly wrong
 *    for the one job whose artifacts get distributed.
 * 2. `gm-desktop.yml` signs WITHOUT `-Required`. The mirror image: secrets are
 *    withheld from fork pull requests, so requiring them would redden every
 *    such PR while adding no safety.
 * 3. Signing runs before `verify-msi.ps1` in both jobs. Signing changes the
 *    bytes, and the verifier cross-checks `manifest.json` against the files on
 *    disk, so verifying first would compare signed files against the hashes
 *    recorded for unsigned ones.
 * 4. The release body takes its signing claim from a step output. A hardcoded
 *    "Unsigned" is a stale claim the moment signing works, and a hardcoded
 *    "Signed" would be worse — it would be a claim nothing verified.
 *
 * These are structural assertions over YAML. Nothing here executes a signing
 * operation; `sign-msi.ps1` proves itself by running, and this proves the
 * workflows call it the way they have to.
 */
import { describe, expect, it } from 'vitest';
import { readFileSync, readdirSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { parse } from 'yaml';

const REPO_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..', '..', '..', '..');
const WORKFLOW_DIR = join(REPO_ROOT, '.github', 'workflows');
const APP_DIR = join(REPO_ROOT, 'apps', 'gm-desktop');

interface WfStep {
  name?: string;
  run?: string;
  shell?: string;
  uses?: string;
  with?: Record<string, unknown>;
  env?: Record<string, unknown>;
}

interface WfJob {
  steps?: WfStep[];
  outputs?: Record<string, unknown>;
}

interface WfDoc {
  jobs?: Record<string, WfJob>;
}

function load(file: string): WfDoc {
  return parse(readFileSync(join(WORKFLOW_DIR, file), 'utf8')) as WfDoc;
}

/** The steps of one job, in order. */
function jobSteps(doc: WfDoc, job: string): WfStep[] {
  const steps = doc.jobs?.[job]?.steps;
  if (!steps) throw new Error(`${job} has no steps`);
  return steps;
}

/** Index of the first step whose `run:` invokes the given script, or -1. */
function indexOfScript(steps: WfStep[], script: string): number {
  return steps.findIndex((s) => typeof s.run === 'string' && s.run.includes(script));
}

const RELEASE = load('release.yml');
const DESKTOP = load('gm-desktop.yml');
const SCRIPT = join(APP_DIR, 'scripts', 'sign-msi.ps1');
const SIGN_SCRIPT = 'sign-msi.ps1';
const VERIFY_SCRIPT = 'verify-msi.ps1';

describe('msi signing — the release path requires it', () => {
  it('release.yml signs the MSIs with -Required', () => {
    const steps = jobSteps(RELEASE, 'msi');
    const i = indexOfScript(steps, SIGN_SCRIPT);
    expect(i, `release.yml msi job never calls ${SIGN_SCRIPT}`).toBeGreaterThanOrEqual(0);

    const run = steps[i].run as string;
    expect(
      run,
      'release.yml signs without -Required, so a missing certificate is a NOTE and exit 0. ' +
        'That is right for a pull request and wrong for the job whose artifacts are distributed.'
    ).toMatch(/-Required\b/);
  });

  it('the certificate reaches the script as an environment variable, not an argument', () => {
    const steps = jobSteps(RELEASE, 'msi');
    const i = indexOfScript(steps, SIGN_SCRIPT);
    const env = steps[i].env ?? {};

    // `signtool /p <password>` would put the passphrase in the process command
    // line. The script avoids that by importing the pfx and signing by
    // thumbprint, which only works if the password arrives through env.
    expect(Object.keys(env), 'the signing step declares no env').toEqual(
      expect.arrayContaining(['GITGIT_MSI_PFX_BASE64', 'GITGIT_MSI_PFX_PASSWORD'])
    );
    expect(Object.values(env).join('\n')).toMatch(/secrets\./);
    expect(
      steps[i].run as string,
      'the passphrase must not appear in the step body; it is read from the environment'
    ).not.toMatch(/PASSWORD\s*=\s*['"][^'"]/i);
  });

  it('signing happens before verification, in both msi jobs', () => {
    for (const [label, doc] of [
      ['release.yml', RELEASE],
      ['gm-desktop.yml', DESKTOP],
    ] as const) {
      const steps = jobSteps(doc, 'msi');
      const sign = indexOfScript(steps, SIGN_SCRIPT);
      const verify = indexOfScript(steps, VERIFY_SCRIPT);

      expect(sign, `${label} msi job does not sign`).toBeGreaterThanOrEqual(0);
      expect(verify, `${label} msi job does not verify`).toBeGreaterThanOrEqual(0);
      expect(
        sign,
        `${label} verifies before it signs. Signing changes the bytes, and verify-msi.ps1 ` +
          `cross-checks manifest.json against the files on disk, so it would compare signed ` +
          `files against the hashes recorded before signing.`
      ).toBeLessThan(verify);
    }
  });
});

describe('msi signing — the pull-request path does not require it', () => {
  it('gm-desktop.yml signs WITHOUT -Required', () => {
    const steps = jobSteps(DESKTOP, 'msi');
    const i = indexOfScript(steps, SIGN_SCRIPT);
    expect(i, `gm-desktop.yml msi job never calls ${SIGN_SCRIPT}`).toBeGreaterThanOrEqual(0);
    expect(
      steps[i].run as string,
      'the pull-request job must not pass -Required: GitHub withholds secrets from fork pull ' +
        'requests, so this would redden every such PR while adding no safety.'
    ).not.toMatch(/-Required\b/);
  });
});

describe('msi signing — the release body states what was built', () => {
  const releaseStep = (RELEASE.jobs?.['draft-release']?.steps ?? []).find(
    (s) => typeof s.uses === 'string' && s.uses.startsWith('softprops/action-gh-release')
  );

  it('found the release-creating step', () => {
    expect(releaseStep).toBeDefined();
  });

  it('the body contains no hardcoded signing claim', () => {
    const body = String((releaseStep?.with as Record<string, unknown>)?.body ?? '');
    expect(body.length, 'the release step declares no body').toBeGreaterThan(0);

    // Either spelling is a claim. Whichever is written becomes false the
    // moment the configuration changes, and nothing re-checks it.
    expect(
      body,
      'the release body hardcodes a signing claim. A draft release that misdescribes its own ' +
        'artifacts is worse than one that says nothing, because that sentence is what a human ' +
        'reads before pressing Publish. Derive it from a step output instead.'
    ).not.toMatch(/\*\*(Un)?[Ss]igned\.?\*\*/);

    expect(
      body,
      'the body must interpolate the notice derived from the manifest that was built'
    ).toContain('${{ steps.signing.outputs.notice }}');
  });

  it('the notice is derived from the manifest, and an unsigned result is refused', () => {
    const steps = jobSteps(RELEASE, 'draft-release');
    const i = steps.findIndex((s) => s.name?.includes('signing notice'));
    expect(i, 'no step derives the signing notice').toBeGreaterThanOrEqual(0);

    const run = steps[i].run as string;
    expect(run, 'the notice must read the staged manifest').toContain('msi-manifest.json');
    expect(run, 'the notice must read the signing block').toMatch(/\.signing\.signed/);
    // Fail-closed: an unsigned release is refused rather than published with a
    // softer caption.
    expect(
      run,
      'the signing notice must fail the job when the manifest records no signature'
    ).toMatch(/exit 1/);
  });
});

describe('the signing script itself', () => {
  const source = readFileSync(SCRIPT, 'utf8');

  it('exists where the workflows look for it', () => {
    expect(source.length, `${SIGN_SCRIPT} is missing or empty`).toBeGreaterThan(0);
  });

  it('has the -Required switch the workflows pass', () => {
    expect(source).toMatch(/\[switch\]\$Required/);
  });

  it('reads the credential from the environment, never from a parameter', () => {
    expect(source).toMatch(/\$env:GITGIT_MSI_PFX_BASE64/);
    expect(source).toMatch(/\$env:GITGIT_MSI_PFX_PASSWORD/);
    expect(
      source,
      'the script must not accept the pfx or passphrase as a parameter; that would put them ' +
        'where every other process on the runner can read them'
    ).not.toMatch(/\[string\]\$PfxPassword/);
    expect(source).not.toMatch(/\[string\]\$Pfx(Path|File)?\b/);
  });

  it('refuses to continue when -Required and no certificate is configured', () => {
    // The exact branch a silent skip would live in. Anchored on the head of
    // the branch rather than on its printed message: `if ($Required)` comes
    // BEFORE the NOTE string, so a window opened at the message misses it and
    // the gate fails on a script that is behaving correctly.
    const head = source.indexOf('IsNullOrWhiteSpace($pfxBase64)');
    expect(head, 'the missing-credential branch is gone').toBeGreaterThanOrEqual(0);
    const branch = source.slice(head, head + 1400);

    // Both outcomes must be present in the same branch: fail when required,
    // exit cleanly when not.
    expect(branch, 'no -Required check in the missing-credential branch').toMatch(/if \(\$Required\)/);
    expect(branch, 'no failure when signing is required but unavailable').toMatch(/throw/);
    expect(branch, 'no clean exit when signing is optional and unavailable').toMatch(/exit 0/);

    // And the throw has to come first, or `-Required` is decorative.
    expect(
      branch.indexOf('throw'),
      'the throw comes after the exit 0, so -Required never takes effect'
    ).toBeLessThan(branch.indexOf('exit 0'));
  });

  it('fails rather than reporting success when signtool is missing', () => {
    // A signing step that cannot find its signer and reports success anyway
    // is the same shape as the defect this whole file came out of.
    const idx = source.indexOf('signtool.exe was not found');
    expect(idx, 'the missing-signtool failure is gone').toBeGreaterThanOrEqual(0);
    expect(source.slice(Math.max(0, idx - 400), idx)).toMatch(/throw/);
  });

  it('refreshes manifest.json, because signing changes the bytes', () => {
    expect(
      source,
      'the script must rewrite manifest.json after signing. build-msi.ps1 records the hash of ' +
        'the UNSIGNED files; without this the manifest would describe installers that no longer ' +
        'exist, and verify-msi.ps1 would compare signed files against those stale hashes.'
    ).toMatch(/manifest\.json/);
    expect(source).toMatch(/Get-FileHash/);
    expect(source).toMatch(/Add-Member\s+-NotePropertyName signing/);
  });

  it('removes the certificate and the .pfx on every exit path', () => {
    const finallyIdx = source.lastIndexOf('finally');
    expect(finallyIdx, 'the script has no finally block').toBeGreaterThanOrEqual(0);
    const tail = source.slice(finallyIdx);
    expect(tail, 'the certificate is left in Cert:\\CurrentUser\\My').toMatch(
      /Cert:\\CurrentUser\\My/
    );
    expect(tail, 'the temporary .pfx is left on disk').toMatch(/Remove-Item/);
  });

  it('documents the exit codes, like the other scripts in this directory', () => {
    // verify-msi.ps1 and build-msi.ps1 both do this, and a script that can
    // return a non-zero code with no stated meaning is a script the workflow
    // cannot wrap correctly.
    expect(source).toMatch(/\.NOTES/);
    expect(source).toMatch(/Exit codes/);
    expect(source).toMatch(/#requires -Version 7/);
  });
});

describe('the workflows the gate reads are the ones on disk', () => {
  it('found all five workflows', () => {
    // A gate that silently reads nothing is a gate that passed.
    const files = readdirSync(WORKFLOW_DIR).filter((f) => f.endsWith('.yml'));
    expect(files.length).toBeGreaterThanOrEqual(5);
    expect(files).toContain('release.yml');
    expect(files).toContain('gm-desktop.yml');
  });
});
