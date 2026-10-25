/**
 * Gates on the always-running gate workflow.
 *
 * ## Why this file exists
 *
 * Until 2026-10-25, every other workflow here was path-filtered, and that was
 * precisely why those workflows could not be required status checks. GitHub's
 * own documentation says it: a workflow skipped by path filtering leaves its
 * checks in a "Pending" state, and a required check that is Pending blocks the
 * pull request for ever. A documentation-only pull request would wait for a
 * Rust check that is never going to run.
 *
 * [FACT] That has changed for three of the five other workflows. The filter
 * was moved OUT of the workflow and INTO its jobs, because a job skipped by
 * its own `if:` condition is reported as "Success" — which satisfies a
 * required check — whereas a workflow that never starts reports nothing.
 * `apps/gm-desktop/tests/unit/workflow-gate.test.ts` pins that arrangement,
 * and `docs/plan/deep-gate-enforcement.md` records it.
 *
 * [FACT] `gate.yml` keeps its place as the always-running workflow, and for
 * the same reason it had it: it is the one whose checks are true of every
 * pull request regardless of which files it touches, and whose checks can go
 * red ONLY because of the pull request's own content.
 *
 * So the branch protection requires the check of a workflow that always runs.
 * That makes this workflow load-bearing, and load-bearing is exactly the
 * kind of thing that gets edited away by someone who does not know what it is
 * for. These cases pin the properties that make it work.
 *
 * ## What each case is for
 *
 *   * the file exists at all;
 *   * it has NO `paths:` filter — adding one turns it into another
 *     path-filtered workflow and every pull request it did not match would
 *     block on a check that never reports;
 *   * its job carries the exact name the branch protection requires, because
 *     a renamed job is a required check that never reports — the same
 *     deadlock, one string away;
 *   * the checks inside it cannot go red for a reason that has nothing to do
 *     with the pull request, because a required check that does that gets
 *     bypassed and then is theatre.
 */
import { describe, expect, it } from 'vitest';
import { existsSync, readFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { parse } from 'yaml';

const REPO_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..', '..', '..', '..');
const GATE = join(REPO_ROOT, '.github', 'workflows', 'gate.yml');

/**
 * The status-check context the branch protection requires.
 *
 * It is the JOB NAME, not the workflow name: GitHub reports a check under the
 * job's name. Changing it here without changing the protection leaves the
 * protection requiring a check that is never reported — the same deadlock,
 * one string away. The value is written down in `.github/CI.md` too.
 */
const REQUIRED_CONTEXT = 'repo gate (always runs)';

type Workflow = {
  on?: Record<string, unknown>;
  jobs?: Record<string, { name?: string; steps?: Array<{ run?: string; uses?: string }> }>;
};

const exists = existsSync(GATE);
const doc = exists
  ? (parse(readFileSync(GATE, 'utf8')) as Workflow)
  : ({} as Workflow);

describe('the always-running gate', () => {
  it('exists', () => {
    expect(
      exists,
      '.github/workflows/gate.yml is missing, so the branch protection requires a check that ' +
        'is never reported and every pull request blocks'
    ).toBe(true);
  });

  it('has no path filter, or it would block the pull requests it does not match', () => {
    const on = doc.on ?? {};

    for (const [event, config] of Object.entries(on)) {
      if (typeof config !== 'object' || config === null) continue;
      const paths = (config as { paths?: unknown }).paths;
      expect(
        paths,
        `the gate's "${event}" trigger has a paths filter, which makes it another ` +
          'path-filtered workflow — and a path-filtered workflow cannot be a required check'
      ).toBeUndefined();
    }
  });

  it('runs on pull requests to dev and main', () => {
    const pr = doc.on?.pull_request as { branches?: string[] } | undefined;
    expect(pr, 'the gate does not run on pull requests').toBeDefined();
    expect(pr?.branches ?? [], 'the gate does not run on pull requests to dev and main').toEqual(
      expect.arrayContaining(['dev', 'main'])
    );
  });

  it('carries the exact job name the branch protection requires', () => {
    const names = Object.values(doc.jobs ?? {}).map((j) => j.name);
    expect(
      names,
      'the gate job was renamed, so the branch protection requires a check that is never ' +
        `reported. The required context is "${REQUIRED_CONTEXT}".`
    ).toContain(REQUIRED_CONTEXT);
  });

  it('carries no check that can go red for a reason unrelated to the pull request', () => {
    // The advisory ratchets are the trap: they go red when the advisory
    // database grows, which is a fact about the world and not about this
    // pull request. A required check that does that gets bypassed.
    const runs = (doc.jobs?.gate?.steps ?? [])
      .map((s) => s.run ?? '')
      .join('\n');

    expect(
      runs,
      'the gate runs an advisory ratchet, which turns red when the advisory database grows — ' +
        'unrelated to the pull request, and a required check like that gets bypassed'
    ).not.toMatch(/check-advisories|frontend-advisory-check|audit/);
  });
});
