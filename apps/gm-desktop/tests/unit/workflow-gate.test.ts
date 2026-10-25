/**
 * Gates on the structure of the workflows that own the required status checks.
 *
 * ## Why this file exists
 *
 * The branch protection is expected to require the five contexts listed in
 * `scripts/ci/required-checks.json`. GitHub reports a check under a JOB's
 * `name:`, so those five strings are the context strings. Everything this
 * file pins is a way that list can stop matching reality — silently, and
 * without anything else in the repository noticing:
 *
 *   * a renamed job is a required check that is never reported — the same
 *     Pending-forever deadlock a path-filtered workflow causes, one string
 *     away;
 *   * a workflow-level `paths:` filter on the pull_request trigger is exactly
 *     the deadlock this lane exists to close, and putting it back would
 *     re-open it;
 *   * a `needs:` chain is the subtler version of the same trap: a job that
 *     needs another one is SKIPPED when that one is skipped, and a skipped
 *     job reports "Success";
 *   * a changed area pattern silently changes WHAT a gate covers, and the
 *     gate keeps reporting — it just stops covering the thing it was
 *     covering.
 *
 * ## What is deliberately NOT claimed
 *
 * These cases read the workflow files. They do not run them, and they do not
 * prove that GitHub's own workflow parser accepts the files — that is GitHub's
 * parser, and it is verified by a pull request, not by a test. What these
 * cases prove is that the STRUCTURE the required checks depend on is still
 * there.
 *
 * ## Scope
 *
 * These cases cover lane L1-a: the three rust-backend.yml jobs, the one
 * gm-console.yml job and the one frontend-audit.yml job — the five contexts
 * `scripts/ci/required-checks.json` declares. `gm-desktop.yml` and
 * `gm-desktop-bundle.yml` are lane L1-b: their `msi` / `msi-launch` jobs
 * were only stabilised in PR #57 on 2026-10-25, they still carry
 * workflow-level path filters, and their `msi-install` / `msi-launch` jobs
 * carry `needs: msi` ON PURPOSE. Asserting "no `needs:`" over the whole
 * repository would be wrong, and asserting it over the wrong set would be
 * worse.
 */
import { describe, expect, it } from 'vitest';
import { spawnSync } from 'node:child_process';
import { readdirSync, readFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { parse } from 'yaml';

const REPO_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..', '..', '..', '..');
const WORKFLOW_DIR = join(REPO_ROOT, '.github', 'workflows');
const REQUIRED_CHECKS = join(REPO_ROOT, 'scripts', 'ci', 'required-checks.json');
const SCRIPT = join(REPO_ROOT, 'scripts', 'ci', 'changed-areas.ps1');

type Step = {
  name?: string;
  id?: string;
  if?: string;
  run?: string;
  uses?: string;
};

type Job = {
  name?: string;
  needs?: string | string[];
  'if'?: string;
  steps?: Step[];
};

type Workflow = {
  on?: Record<string, unknown>;
  jobs?: Record<string, Job>;
};

const workflowFiles = readdirSync(WORKFLOW_DIR)
  .filter((file) => file.endsWith('.yml'))
  .sort();

const workflows = new Map<string, Workflow>(
  workflowFiles.map((file) => [
    file,
    parse(readFileSync(join(WORKFLOW_DIR, file), 'utf8')) as Workflow,
  ])
);

/** Every job of every workflow, with the file it came from. */
const allJobs: Array<{ file: string; key: string; job: Job }> = [];
for (const [file, workflow] of workflows) {
  for (const [key, job] of Object.entries(workflow.jobs ?? {})) {
    allJobs.push({ file, key, job });
  }
}

const requiredChecks = JSON.parse(readFileSync(REQUIRED_CHECKS, 'utf8')) as {
  contexts: string[];
};

const CONTEXTS = requiredChecks.contexts;

/**
 * Which area each L1-a workflow's jobs belong to.
 *
 * [FACT] The area keys are the ones `scripts/ci/changed-areas.ps1` emits, and
 * they are the ones the workflows' `if:` conditions read.
 */
const AREA_BY_WORKFLOW: Record<string, string> = {
  'rust-backend.yml': 'rust_backend',
  'gm-console.yml': 'gm_console',
  'frontend-audit.yml': 'frontend_audit',
};

/**
 * The area patterns, as measured from the workflows' `paths:` blocks.
 *
 * [FACT] These lists are the DRIFT ANCHOR. They were measured from
 * `origin/dev` on 2026-10-25 and they are what the gate covers. Changing an
 * area's patterns changes what the gate covers, so the change has to change
 * this file too — and a change here shows up in the diff, where a person can
 * see that the coverage moved.
 */
const RECORDED_AREA_PATTERNS: Record<string, string[]> = {
  rust_backend: [
    'src/**',
    'Cargo.toml',
    'Cargo.lock',
    'scripts/**',
    '.github/workflows/rust-backend.yml',
  ],
  gm_console: [
    'apps/gm-console/**',
    'src/server/api.rs',
    '.github/workflows/gm-console.yml',
  ],
  gm_desktop: [
    'apps/gm-desktop/**',
    '.github/workflows/gm-desktop.yml',
    '.github/workflows/release.yml',
    '.github/workflows/gate.yml',
    'Cargo.toml',
    'Cargo.lock',
    'scripts/advisory-baseline.json',
    'scripts/check-advisories.ps1',
  ],
  gm_desktop_bundle: [
    'apps/gm-desktop/**',
    '.github/workflows/gm-desktop-bundle.yml',
    '.github/workflows/gm-desktop.yml',
    'Cargo.toml',
    'Cargo.lock',
    'src/**',
  ],
  frontend_audit: [
    'apps/gm-desktop/**',
    'apps/gm-console/**',
    'scripts/frontend-advisory-baseline.json',
    'scripts/frontend-advisory-check.ps1',
    '.github/workflows/frontend-audit.yml',
  ],
};

describe('the required status-check contexts', () => {
  it('are declared at all', () => {
    expect(
      Array.isArray(CONTEXTS) && CONTEXTS.length > 0,
      'scripts/ci/required-checks.json declares no contexts, so nothing declares which checks the branch protection is expected to require'
    ).toBe(true);
  });

  it('each name exactly one job, and no job twice', () => {
    for (const context of CONTEXTS) {
      const owners = allJobs.filter((entry) => entry.job.name === context);

      expect(
        owners.length,
        `the context "${context}" is the name of ${owners.length} jobs (${owners
          .map((owner) => `${owner.file}:${owner.key}`)
          .join(', ')}). GitHub matches required checks by name, and two jobs with the same name make it undefined which one counts — and neither of them is a name the branch protection would fail loudly about.`
      ).toBe(1);
    }
  });

  it('contain no expression, because GitHub does not document what a skipped job with an expression in its name reports', () => {
    for (const context of CONTEXTS) {
      expect(
        context,
        `the context "${context}" contains an expression. GitHub documents that a skipped job reports "Success", but says nothing about a job whose NAME is an expression — and an expression's value is not a stable context string to require.`
      ).not.toContain('${{');
    }
  });
});

describe('the workflows that own a required context', () => {
  it('do not filter the pull_request trigger at the workflow level', () => {
    for (const context of CONTEXTS) {
      const owner = allJobs.find((entry) => entry.job.name === context);
      expect(owner, `no job is named "${context}"`).toBeDefined();

      const on = workflows.get(owner!.file)?.on ?? {};
      const pullRequest = (on as Record<string, unknown>).pull_request;

      if (pullRequest === undefined) {
        // A workflow that does not run on pull requests at all reports
        // nothing on a pull request, which is the same deadlock.
        expect(
          pullRequest,
          `${owner!.file} owns the required context "${context}" but has no pull_request trigger`
        ).toBeDefined();
        continue;
      }

      const config = (pullRequest ?? {}) as { paths?: unknown; 'paths-ignore'?: unknown };

      expect(
        config.paths,
        `${owner!.file} has a workflow-level paths filter on its pull_request trigger. A workflow that the filter excludes never starts, so its checks are never reported and stay Pending — which blocks the pull request for ever instead of enforcing anything. The filter belongs inside the jobs.`
      ).toBeUndefined();

      expect(
        config['paths-ignore'],
        `${owner!.file} has a workflow-level paths-ignore filter on its pull_request trigger, with the same Pending-forever consequence as a paths filter.`
      ).toBeUndefined();
    }
  });

  it('do not depend on another job, because a skipped dependency skips the dependent', () => {
    // [FACT] Scope: lane L1-a's contexts only. `gm-desktop.yml`'s
    // `msi-install` and `msi-launch` jobs carry `needs: msi` on purpose and
    // belong to lane L1-b; they are not in CONTEXTS, so this assertion does
    // not reach them.
    for (const context of CONTEXTS) {
      const owner = allJobs.find((entry) => entry.job.name === context);
      expect(owner, `no job is named "${context}"`).toBeDefined();

      expect(
        owner!.job.needs,
        `${owner!.file}:${owner!.key} carries a needs: chain. A job whose dependency is skipped is itself skipped, and a skipped job reports "Success" — so a red dependency becomes a green required check.`
      ).toBeUndefined();
    }
  });
});

describe('the deep gates inside the required jobs', () => {
  it('carry the area condition on every step that does the work', () => {
    for (const context of CONTEXTS) {
      const owner = allJobs.find((entry) => entry.job.name === context);
      expect(owner, `no job is named "${context}"`).toBeDefined();

      const area = AREA_BY_WORKFLOW[owner!.file];
      expect(
        area,
        `${owner!.file} owns the required context "${context}" but is not one of the lane L1-a workflows, so this file does not know which area it belongs to`
      ).toBeDefined();

      const condition = `steps.detect.outputs.${area} == 'true'`;

      for (const step of owner!.job.steps ?? []) {
        const uses = step.uses ?? '';
        const isCheckout = uses.startsWith('actions/checkout@');
        const isDetection = step.id === 'detect' || step.name === 'Detect changed areas';
        // [FACT] A step whose own condition is `always()` is a diagnostic
        // upload: it exists to hand over logs or reports from a step that
        // FAILED, and it carries `if-no-files-found: warn`, so it cannot turn
        // the job red. Adding the area condition to it would remove the
        // artefact of exactly the failure it was uploaded for.
        const isDiagnosticUpload = (step.if ?? '').trim() === 'always()';

        if (isCheckout || isDetection || isDiagnosticUpload) {
          continue;
        }

        expect(
          step.if,
          `${owner!.file}:${owner!.key} — the step "${step.name ?? uses}" has no condition. Without one it runs on every pull request, which is the behaviour the path filter used to provide; and a step that runs unconditionally is a step whose cost nobody chose.`
        ).toBeDefined();

        expect(
          step.if,
          `${owner!.file}:${owner!.key} — the step "${step.name ?? uses}" is not conditioned on the area. It must carry \`${condition}\`, or the gate runs when it does not apply and skips when it does.`
        ).toContain(condition);
      }
    }
  });
});

describe('the area patterns the script evaluates', () => {
  it('are exactly the lists the workflows used to filter on', () => {
    // The script prints its effective pattern lists in a machine-readable
    // form. Reading them back from the SCRIPT — rather than from its source
    // text — is what makes this a drift anchor rather than a second copy of
    // the same constants.
    const result = spawnSync(
      'pwsh',
      ['-NoProfile', '-File', SCRIPT, '-SelfCheck', '-BaseRef', 'HEAD'],
      {
        cwd: REPO_ROOT,
        encoding: 'utf8',
        env: { ...process.env, GITHUB_BASE_REF: '', GITHUB_EVENT_BEFORE: '', GITHUB_OUTPUT: '' },
      }
    );

    expect(
      result.status,
      `the script's self-check failed:\n${result.stdout}\n${result.stderr}`
    ).toBe(0);

    const effective: Record<string, string[]> = {};
    for (const line of (result.stdout ?? '').split(/\r?\n/)) {
      const match = /^\[pattern\]\s+(\S+)\s+::\s+(.*)$/.exec(line.trim());
      if (match) {
        effective[match[1]] = effective[match[1]] ?? [];
        effective[match[1]].push(match[2]);
      }
    }

    expect(
      Object.keys(effective).sort(),
      'the script did not print all five area pattern lists, so the drift anchor would compare against nothing'
    ).toEqual(Object.keys(RECORDED_AREA_PATTERNS).sort());

    for (const [area, recorded] of Object.entries(RECORDED_AREA_PATTERNS)) {
      expect(
        effective[area],
        `the pattern list of "${area}" changed. The list decides what the gate covers, so changing it must be a deliberate change — and it must change this file too, so the diff shows the coverage moving.`
      ).toEqual(recorded);
    }
  });
});
