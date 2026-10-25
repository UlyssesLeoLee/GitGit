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
 *     job reports "Success" — so every job a required context needs must
 *     itself be a required context, or a red dependency turns into a green
 *     required check;
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
 * These cases cover all five gated workflows and all fourteen contexts
 * `scripts/ci/required-checks.json` declares: the three rust-backend.yml
 * jobs, the one gm-console.yml job, the one frontend-audit.yml job, the
 * three gm-desktop.yml jobs and the four gm-desktop-bundle.yml jobs.
 *
 * Three of those fourteen are CONSUMER jobs — `msi-install`, `msi-launch`
 * and `deb-install` — and they carry `needs:` ON PURPOSE: they test the
 * artefact the producer job built, not the source it was built from. So the
 * rule below is not "no `needs:`" — that would be wrong for them — but the
 * stronger one that does fit them: every job a required context needs must
 * itself be a required context.
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

/**
 * The time budget for the test that spawns the script.
 *
 * [FACT] Vitest's default per-test timeout is 5s. This script is a PowerShell
 * program, and on a GitHub-hosted ubuntu runner a cold `pwsh` start alone
 * costs several seconds: the whole script was measured at ~17s inside this
 * test on 2026-10-25 (the case failed with "Test timed out in 5000ms" after
 * 16987ms), against 0.7s on a developer machine. The 5s default is therefore
 * a property of the MACHINE, not of the code under test.
 *
 * [FACT] Only the BUDGET is raised. The assertion is untouched: the pattern
 * lists the script evaluates must still equal the recorded ones, list by
 * list. A gate that fails for a reason unrelated to what it asserts is a gate
 * that gets switched off.
 */
const SPAWN_TIMEOUT_MS = 60_000;

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
  outputs?: Record<string, string>;
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
 * The job keys a job needs, as a list.
 *
 * [FACT] `needs:` is either one key or a list of them, and the assertions
 * below read both shapes the same way.
 */
function needsList(job: Job): string[] {
  const needs = job.needs;
  if (needs === undefined) return [];
  return Array.isArray(needs) ? needs : [needs];
}

/**
 * Which area each gated workflow's jobs belong to.
 *
 * [FACT] The area keys are the ones `scripts/ci/changed-areas.ps1` emits, and
 * they are the ones the workflows' `if:` conditions read.
 */
const AREA_BY_WORKFLOW: Record<string, string> = {
  'rust-backend.yml': 'rust_backend',
  'gm-console.yml': 'gm_console',
  'frontend-audit.yml': 'frontend_audit',
  'gm-desktop.yml': 'gm_desktop',
  'gm-desktop-bundle.yml': 'gm_desktop_bundle',
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

  it('a required context that needs a job needs a REQUIRED context', () => {
    // [FACT] This replaces an earlier rule that said a required context must
    // not sit in a `needs:` chain at all. That rule was wrong for the three
    // consumer jobs (`msi-install`, `msi-launch`, `deb-install`), which carry
    // `needs:` on purpose because they test the artefact the producer built.
    //
    // [FACT] The rule that does hold is this one, and it is STRONGER: if a
    // required context has `needs:`, then every job it needs must itself be
    // a required context.
    //
    // [INFERENCE] This is exactly what closes the hole the old rule was
    // reaching for. A job whose dependency is skipped is itself skipped, and
    // a skipped job reports "Success" — so a red dependency would turn into a
    // green required check and let the merge through. With this rule the
    // dependency is itself a required check: it is red on its own account and
    // blocks the merge by itself, and the dependent's own colour is then
    // irrelevant.
    //
    // [FACT] GitHub's documentation, "Using jobs in a workflow", states the
    // dependency rule this rests on: "If a job fails or is skipped, all jobs
    // that need it are skipped unless the jobs use a conditional expression
    // that causes the job to continue."
    for (const context of CONTEXTS) {
      const owner = allJobs.find((entry) => entry.job.name === context);
      expect(owner, `no job is named "${context}"`).toBeDefined();

      for (const dependency of needsList(owner!.job)) {
        const upstream = allJobs.find((entry) => entry.file === owner!.file && entry.key === dependency);
        expect(
          upstream,
          `${owner!.file}:${owner!.key} needs "${dependency}", which is not a job in that workflow`
        ).toBeDefined();

        expect(
          CONTEXTS,
          `${owner!.file}:${owner!.key} is the required context "${context}" and it needs ${owner!.file}:${dependency}, whose name is "${upstream!.job.name}". That dependency is not a required context, so when it fails this job is SKIPPED — and a skipped job reports "Success", which turns a red dependency into a green required check and lets the merge through. Every job a required context needs must itself be a required context.`
        ).toContain(upstream!.job.name);
      }
    }
  });
});

describe('the deep gates inside the required jobs', () => {
  /**
   * Is this required context a CONSUMER job?
   *
   * [FACT] A consumer job is one that carries `needs:` — it tests the
   * artefact another job produced. Three of the fourteen contexts are
   * consumers: `msi-install` and `msi-launch` (which need `msi`) and
   * `deb-install` (which needs `deb`).
   *
   * [FACT] A consumer job carries NO detection step and NO area condition on
   * its steps, and that is deliberate. The area decision belongs to the
   * producer it needs: the producer is the job that reads the changed-file
   * set and decides whether the area applies at all, and a consumer that
   * re-decided the area for itself would be a second place where the
   * decision is made — and the two could disagree.
   *
   * [FACT] The consumer is skipped as a WHOLE, by a job-level `if:` that
   * requires the producer's published area decision. It is NOT skipped by
   * its `needs:` — see the assertion below for why that distinction is the
   * whole point.
   */
  function isConsumer(job: Job): boolean {
    return job.needs !== undefined;
  }

  it('carry the area condition on every step that does the work', () => {
    for (const context of CONTEXTS) {
      const owner = allJobs.find((entry) => entry.job.name === context);
      expect(owner, `no job is named "${context}"`).toBeDefined();

      if (isConsumer(owner!.job)) {
        // A consumer job is not conditioned on the area; see `isConsumer`.
        continue;
      }

      const area = AREA_BY_WORKFLOW[owner!.file];
      expect(
        area,
        `${owner!.file} owns the required context "${context}" but is not one of the five gated workflows, so this file does not know which area it belongs to`
      ).toBeDefined();

      const condition = `steps.detect.outputs.${area} == 'true'`;

      for (const step of owner!.job.steps ?? []) {
        const uses = step.uses ?? '';
        const isCheckout = uses.startsWith('actions/checkout@');
        const isDetection = step.id === 'detect' || step.name === 'Detect changed areas';
        // [FACT] A step whose own condition is `always()` or `failure()` is a
        // DIAGNOSTIC upload: it exists to hand over the logs, the reports or
        // the build directory of a step that FAILED, and it carries
        // `if-no-files-found: warn` (or `ignore`), so it cannot turn the job
        // red. Adding the area condition to it would remove the artefact of
        // exactly the failure it was uploaded for.
        //
        // [FACT] The exemption is limited to uploads. A step that does the
        // work and happens to carry `always()` or `failure()` is still a
        // step that does the work, and conditioning it is what keeps the
        // gate from running when it does not apply.
        const isDiagnosticUpload = (() => {
          const own = (step.if ?? '').trim();
          if (own !== 'always()' && own !== 'failure()') return false;
          return uses.startsWith('actions/upload-artifact@');
        })();

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

  it('a consumer job has no detection step of its own', () => {
    // [FACT] The counterpart of the rule above: a consumer job must not
    // carry a detection step. If it did, it would be deciding the area a
    // second time, in a job whose whole purpose is to test what another job
    // produced — and the two decisions could disagree, which is the class of
    // defect this file exists to make loud.
    for (const context of CONTEXTS) {
      const owner = allJobs.find((entry) => entry.job.name === context);
      expect(owner, `no job is named "${context}"`).toBeDefined();

      if (!isConsumer(owner!.job)) continue;

      const hasDetection = (owner!.job.steps ?? []).some(
        (step) => step.id === 'detect' || step.name === 'Detect changed areas'
      );

      expect(
        hasDetection,
        `${owner!.file}:${owner!.key} is the consumer of another job's artefact and carries a detection step. The area decision belongs to the producer it needs; a second decision here could disagree with it.`
      ).toBe(false);

      // [INFERENCE] And the corollary, which is a silent failure if it ever
      // happens: a consumer that carried an area condition on its steps
      // would be evaluating a `steps.detect.outputs.<area>` that no step in
      // that job ever sets — so the condition would always be false, every
      // step would be skipped, and the job would report green while testing
      // NOTHING. That is exactly the class of green-but-blind gate this
      // repository has been bitten by before.
      const area = AREA_BY_WORKFLOW[owner!.file];
      expect(area, `${owner!.file} is not one of the five gated workflows`).toBeDefined();

      for (const step of owner!.job.steps ?? []) {
        expect(
          step.if ?? '',
          `${owner!.file}:${owner!.key} — the step "${step.name ?? step.uses ?? ''}" carries an area condition, but this job has no detection step to evaluate it. The condition would always be false, every step would be skipped, and the job would report green while testing nothing. The area decision belongs to the producer.`
        ).not.toContain(`steps.detect.outputs.${area}`);
      }
    }
  });

  it('a consumer job is skipped whole by the producer\'s published area decision', () => {
    // [FACT] This is the second time in this work that a DOCUMENTED GitHub
    // behaviour was not what the design assumed, so it is written down here
    // rather than left in a comment.
    //
    // [FACT] The producer is never SKIPPED. When the area does not match, the
    // producer's steps are individually skipped and the JOB concludes
    // SUCCESS. GitHub's documentation, "Using jobs in a workflow", says a
    // dependent job is skipped when the job it needs "fails or is skipped" —
    // a dependency that merely SUCCEEDS does not skip its dependents.
    //
    // [INFERENCE] So a `needs:` chain alone does NOT skip a consumer: the
    // consumer would run, and then fail on an artefact that was never
    // uploaded — three red contexts on every pull request that does not
    // touch the area. The producer therefore publishes the area decision as
    // a job output, and the consumer's job-level condition requires it. A
    // job-level `if:` that is false means the job is SKIPPED, and a skipped
    // job reports "Success" — which is what a required status check needs
    // when the gate does not apply.
    for (const context of CONTEXTS) {
      const owner = allJobs.find((entry) => entry.job.name === context);
      expect(owner, `no job is named "${context}"`).toBeDefined();

      if (!isConsumer(owner!.job)) continue;

      const condition = owner!.job['if'];
      expect(
        condition,
        `${owner!.file}:${owner!.key} is a consumer job and carries no job-level condition. Its producer is never skipped — it runs, its steps are skipped, and it concludes SUCCESS — so a \`needs:\` chain alone leaves this job to run and then to fail on an artefact that was never uploaded. The consumer must be skipped by a job-level condition that requires the producer's area decision.`
      ).toBeDefined();

      for (const dependency of needsList(owner!.job)) {
        const producer = allJobs.find((entry) => entry.file === owner!.file && entry.key === dependency);
        expect(
          producer,
          `${owner!.file}:${owner!.key} needs "${dependency}", which is not a job in that workflow`
        ).toBeDefined();

        expect(
          condition!,
          `${owner!.file}:${owner!.key} must require the producer's published area decision: \`needs.${dependency}.outputs.applies == 'true'\`. Without it the consumer runs on every pull request and fails on an artefact that was never uploaded.`
        ).toContain(`needs.${dependency}.outputs.applies == 'true'`);

        expect(
          producer!.job.outputs?.applies,
          `${owner!.file}:${dependency} publishes no \`applies\` output, so the consumer that requires it cannot be skipped.`
        ).toBeDefined();

        const area = AREA_BY_WORKFLOW[producer!.file];
        expect(area, `${producer!.file} is not one of the five gated workflows`).toBeDefined();

        expect(
          producer!.job.outputs!.applies,
          `${owner!.file}:${dependency} publishes an \`applies\` output that is not the area flag \`steps.detect.outputs.${area}\`, so it does not say whether the area applies.`
        ).toContain(`steps.detect.outputs.${area}`);
      }
    }
  });
});

describe('the area patterns the script evaluates', { timeout: SPAWN_TIMEOUT_MS }, () => {
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
