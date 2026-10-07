/**
 * Gates on the Cargo.lock reproducibility check.
 *
 * ## The history, because the current state looks arbitrary otherwise
 *
 * On 2026-10-05 `src-tauri/Cargo.lock` genuinely did not satisfy the root
 * crate it takes by path. `cargo build --release --locked` exited 101, and
 * `docs/reports/2026-10-05-bundle-ci/README.md` §7 recorded it with the
 * evidence. The lock was refreshed in `ab632ae` the same day and the report
 * gained a resolution note re-measuring it with three independent commands.
 *
 * What did not follow is the change that would have made the resolution
 * durable. Two `tauri build` steps in `gm-desktop-bundle.yml` and three in
 * `release.yml` invoked the bundler with nothing asserting the lock, and one
 * of them still explained the omission by citing the stale-lock reason the
 * report had already retired. So reproducibility was *true* — the lock happened
 * to be current — rather than *guaranteed*, and the comment that would have
 * reminded the next reader was itself wrong.
 *
 * ## Why a script and not a flag
 *
 * `tauri build` gives no way to forward `--locked` to the cargo it invokes.
 * The property therefore has to be asserted one step earlier, and six copies
 * of a `run:` block is six things to keep in sync. One script that every
 * bundle build calls is one definition — and one that can be executed locally,
 * which a `run:` block cannot.
 */
import { describe, expect, it } from 'vitest';
import { existsSync, readFileSync, statSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { parse } from 'yaml';

const REPO_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..', '..', '..', '..');
const WORKFLOW_DIR = join(REPO_ROOT, '.github', 'workflows');
const SCRIPT = join(REPO_ROOT, 'apps', 'gm-desktop', 'scripts', 'assert-lock-current.sh');

type Step = { name?: string; run?: string; uses?: string; 'working-directory'?: string };
type Workflow = {
  jobs: Record<string, { steps?: Step[] }>;
};

const workflowFiles = ['gm-desktop-bundle.yml', 'release.yml'];

/** Every step that invokes the Tauri bundler, paired with the step above it. */
const buildSteps = workflowFiles.flatMap((file) => {
  const doc = parse(readFileSync(join(WORKFLOW_DIR, file), 'utf8')) as Workflow;
  return Object.entries(doc.jobs).flatMap(([jobId, job]) =>
    (job.steps ?? [])
      .map((step, i) => ({ file, jobId, step, index: i, previous: (job.steps ?? [])[i - 1] }))
      .filter((s) => /pnpm tauri build|cargo tauri build/.test(s.step.run ?? ''))
  );
});

const scriptText = existsSync(SCRIPT) ? readFileSync(SCRIPT, 'utf8') : '';

/**
 * The script with its own prose removed.
 *
 * This is not defensive tidiness — it is the fix for a gate that read green
 * while broken. The first version of the `--locked` assertion matched the raw
 * file, and the mutation that deletes `--locked` from the cargo invocation
 * **passed**, because the word also appears in the comment directly above it:
 *
 *     # `--locked` means "assert this lock will not change". ...
 *     cargo metadata --locked --format-version 1 >/dev/null
 *
 * Remove the flag from the command and the comment still satisfies the pattern.
 * The script explains itself at length — that is the point of it — so every
 * assertion about what the script *does* reads this instead. A gate that reads
 * green because of a sentence is worse than no gate, because it is a gate
 * nobody can trust.
 *
 * Full-line comments only. Stripping trailing comments is unsound in shell: `#`
 * is ordinary inside a quoted string, and deleting from a `#` to end-of-line
 * without regard for quoting would silently change what a line means.
 */
const scriptCode = scriptText
  .split('\n')
  .filter((line) => !/^\s*#/.test(line))
  .join('\n');

describe('every bundle build proves it is using the committed lock', () => {
  it('found the bundler invocations (a scan that finds none passes vacuously)', () => {
    // Guards the guard. A path typo, a renamed workflow, or a `paths` typo
    // makes every assertion below pass while checking nothing. Measured by
    // narrowing the filename list once.
    expect(
      buildSteps.map((s) => `${s.file} :: ${s.jobId}`),
      'no `tauri build` step found in gm-desktop-bundle.yml or release.yml'
    ).not.toEqual([]);
    expect(
      buildSteps.length,
      'expected six bundler invocations: three bundle builds and three release builds'
    ).toBe(6);
  });

  it('each one is immediately preceded by the lock check', () => {
    // Immediately, not somewhere in the same job. A check that runs after the
    // build is a report on what happened; a check that runs before it is a
    // gate on what is about to happen, and it fails ten minutes earlier.
    const unguarded = buildSteps
      .filter((s) => !/assert-lock-current\.sh/.test(s.previous?.run ?? ''))
      .map((s) => `${s.file} :: ${s.jobId} :: ${s.step.name}`);

    expect(
      unguarded,
      'these bundler invocations are not guarded. Without the check, `tauri build` ' +
        'resolves with whatever the registry offers and the artifact is not the one ' +
        'the commit describes.'
    ).toEqual([]);
  });

  it('the check runs from the repository root, not from inside the crate', () => {
    // `working-directory` on the guarding step would be a red herring: the
    // script derives the crate from its own location, so it is correct from any
    // cwd, and a step that changed directory to compensate would be relying on
    // a property the script does not have.
    for (const s of buildSteps) {
      expect(
        s.previous?.['working-directory'],
        `${s.file} :: ${s.jobId} sets working-directory on the lock check; the script ` +
          'resolves the crate from its own path and must be called as written'
      ).toBeUndefined();
    }
  });
});

describe('the check itself', () => {
  it('exists', () => {
    expect(existsSync(SCRIPT), `missing ${SCRIPT}`).toBe(true);
    expect(statSync(SCRIPT).size, 'the script is empty').toBeGreaterThan(0);
  });

  it('asserts with --locked, which is the only thing that makes the check meaningful', () => {
    // Without `--locked`, `cargo metadata` would cheerfully update the lock and
    // exit 0 — which is precisely the behaviour being guarded against.
    expect(
      scriptCode,
      'the cargo invocation must pass --locked. Without it cargo updates the lock and ' +
        'exits 0, and the check becomes a report rather than a gate.'
    ).toMatch(/cargo metadata --locked/);
  });

  it('fails closed rather than repairing', () => {
    expect(
      scriptCode,
      'a gate that refreshes the lock on the runner would make the next build ' +
        'reproducible and the first one not, and the refresh is never committed'
    ).toMatch(/exit 1/);
  });

  it('resolves the crate from its own location, so cwd cannot change the answer', () => {
    // Measured: invoked from an unrelated directory it still found the lock and
    // reported the same SHA-256.
    expect(scriptCode).toMatch(/BASH_SOURCE/);
  });

  it('keeps the real commands after comment stripping', () => {
    // The stripping above is a transformation, and a transformation that eats
    // the code would make every assertion above vacuously true. These are the
    // lines the gate depends on, checked on the SAME transformed text.
    expect(scriptCode, 'stripping removed the cargo invocation').toMatch(/cargo metadata/);
    expect(scriptCode, 'stripping removed the hash comparison').toMatch(/sha256sum/);
    expect(
      scriptCode.split('\n').filter((l) => l.trim() && !/^\s*#/.test(l)).length,
      'stripping left no executable lines at all'
    ).toBeGreaterThan(10);
  });

  it('is LF, because a CRLF shebang does not run on a Linux runner', () => {
    // Windows editors love to do this, and the failure mode is `bash: ...: /bin/sh^M:
    // bad interpreter` on the only machines that matter here.
    expect(
      scriptText.includes('\r\n'),
      'the script has CRLF line endings; a bash script in a Linux-only path must be LF'
    ).toBe(false);
  });
});

/*
 * Mutation evidence for the script itself, run against a fixture whose lock
 * omits a dependency its manifest requires:
 *
 *   stale lock   -> exit 1, with cargo's own "cannot update the lock file …
 *                   because --locked was passed" above the message, and the lock
 *                   byte-identical afterwards (SHA-256 unchanged). It refuses; it
 *                   does not repair.
 *   no lock file -> exit 1, naming the path it looked for.
 *   consistent    -> exit 0, printing the lock's SHA-256.
 *
 * Two fixture mistakes happened on the way to those results, and both are worth
 * recording because each produced a green that meant nothing.
 *
 * The first fixture had no build target, so the gate failed on "no targets
 * specified in the manifest" — it proved only that the script exits non-zero on
 * any breakage, not that it recognises a stale lock. The second run of the
 * fixture did have a target, but the test harness ran a "control"
 * `cargo metadata` WITHOUT `--locked` first, which silently regenerated the
 * lock, after which the gate correctly reported a healthy crate.
 *
 * The common shape: the fixture, not the gate, was wrong, and a green result
 * from a broken fixture is the failure mode this whole exercise exists to
 * prevent. Rebuild the mutation input on every run; whatever ran before it may
 * have consumed it.
 */
