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

/**
 * The root crate's lockfile gate.
 *
 * `src-tauri/Cargo.lock` has the script above, called by every bundle build.
 * The ROOT crate's `Cargo.lock` had no equivalent, and the cargo invocations
 * in `.github/workflows/rust-backend.yml` that resolve the root crate's graph
 * all re-resolved it: a CI run could rewrite the committed lock and then
 * build an artefact that is not the one the commit describes — the exact
 * failure the Tauri gate was written to prevent, with nothing on the root
 * side to prevent it.
 *
 * Unlike the Tauri crate this needs no script. `cargo` takes `--locked`
 * directly and there is no wrapper to forward it through, so the flag goes
 * on the invocation itself.
 *
 * Three properties of the assertions below are load-bearing, and each one
 * comes from the failure mode recorded at the top of this file — a gate that
 * reads green because of a sentence rather than a command:
 *
 *   1. The workflow is parsed as YAML and only the `run:` strings are
 *      scanned, so a YAML-level comment is not part of any command.
 *   2. Full-line shell comments inside a `run:` block are removed before
 *      scanning. The prose in this workflow says "cargo test" out loud, and
 *      that prose is not an invocation.
 *   3. Each invocation is judged on its OWN flag tokens. The file mentioning
 *      `--locked` repeatedly is not evidence that any one command carries it.
 */
const RUST_BACKEND_WORKFLOW = 'rust-backend.yml';
const rustBackendPath = join(WORKFLOW_DIR, RUST_BACKEND_WORKFLOW);
const rustBackendText = readFileSync(rustBackendPath, 'utf8');
const rustBackendDoc = parse(rustBackendText) as Workflow;

/**
 * The cargo subcommands that resolve the current crate's dependency graph,
 * and may therefore rewrite that crate's `Cargo.lock`.
 *
 * Two subcommands that appear in the workflow are deliberately absent:
 *
 *   `fmt`    — resolves no dependency graph at all, so it cannot rewrite the
 *              lock. It also *cannot* carry the flag: cargo-fmt rejects it
 *              outright (`error: unexpected argument '--locked' found`,
 *              exit 2), so adding it would break the job rather than
 *              protect it.
 *   `audit`  — an external subcommand (cargo-audit). Cargo performs no
 *              resolution for external subcommands, and cargo-audit only
 *              reads `Cargo.lock`.
 */
const CARGO_SUBCOMMANDS = new Set([
  'build',
  'check',
  'test',
  'bench',
  'clippy',
  'run',
  'doc',
  'metadata',
  'tree',
  'fetch',
  'update',
  'generate-lockfile',
  'vendor',
  'package',
  'publish',
  'fix',
]);

type CargoInvocation = {
  jobId: string;
  stepName: string;
  /** The command text, comments removed. */
  command: string;
  /** The cargo subcommand, e.g. `test`. */
  subcommand: string;
  /** The command's own flag tokens — `cargo` and the subcommand excluded. */
  flags: string[];
};

/**
 * The cargo invocations in a `run:` block.
 *
 * Comments are removed first, full-line comments only — the same rule the
 * script gate uses, for the same reason: `#` is ordinary inside a quoted
 * string in shell, so stripping trailing comments would change what a line
 * means.
 *
 * A cargo invocation is a `cargo` in *command* position: at the start of a
 * line, or after `&&`, `||` or `;`. A mention of cargo inside a string, a
 * path such as `/tmp/cargo-test.txt`, or a comment is not an invocation.
 */
function cargoInvocationsIn(run: string): Omit<CargoInvocation, 'jobId' | 'stepName'>[] {
  const found: Omit<CargoInvocation, 'jobId' | 'stepName'>[] = [];
  const lines = run.split('\n').filter((line) => !/^\s*#/.test(line));

  for (const line of lines) {
    const re = /(?:^|(?:&&|\|\||;)\s*)cargo\s+([A-Za-z][A-Za-z0-9_-]*)/g;
    let match: RegExpExecArray | null;
    while ((match = re.exec(line)) !== null) {
      const start = match.index + match[0].lastIndexOf('cargo');
      const rest = line.slice(start);
      const boundary = rest.match(/&&|\|\||;/);
      const command = (boundary ? rest.slice(0, boundary.index) : rest).trim();
      found.push({
        command,
        subcommand: match[1],
        flags: command.split(/\s+/).slice(2),
      });
    }
  }

  return found;
}

const rustBackendInvocations: CargoInvocation[] = Object.entries(rustBackendDoc.jobs).flatMap(
  ([jobId, job]) =>
    (job.steps ?? []).flatMap((step) =>
      cargoInvocationsIn(step.run ?? '').map((invocation) => ({
        ...invocation,
        jobId,
        stepName: step.name ?? '(unnamed step)',
      }))
    )
);

/**
 * Whether this invocation can rewrite the root crate's `Cargo.lock`.
 */
const canRewriteLock = (invocation: CargoInvocation): boolean => {
  // `cargo install --path .` installs THIS crate and does resolve its
  // graph; `cargo install <crate>` installs a published crate from the
  // registry and never touches this crate's lockfile.
  if (invocation.subcommand === 'install') return /--path\b/.test(invocation.command);
  return CARGO_SUBCOMMANDS.has(invocation.subcommand);
};

describe('the root crate lock is pinned in rust-backend.yml', () => {
  it('found the cargo invocations (a scan that finds none passes vacuously)', () => {
    expect(
      rustBackendInvocations.map((i) => `${i.jobId} :: ${i.stepName} :: ${i.command}`),
      `no cargo invocation found in ${RUST_BACKEND_WORKFLOW}`
    ).not.toEqual([]);
    expect(
      rustBackendInvocations.length,
      'the inventory of cargo invocations changed; a new invocation must be classified ' +
        'before it is assumed safe. The inventory is now:\n' +
        rustBackendInvocations
          .map((i) => `  ${i.jobId} :: ${i.stepName} :: ${i.command}`)
          .join('\n')
    ).toBe(8);
  });

  it("classifies the integration job's registry warm-up as lock-rewriting", () => {
    // `cargo fetch` downloads the dependency set and, when the lock is out
    // of date, rewrites it. It arrived with the integration job's offline
    // tiers, which need a warm registry before they can run `--offline`,
    // and it is why the inventory above is 8 rather than 7.
    //
    // [FACT] This case exists so that arrival is classified rather than
    // absorbed into a number. The count assertion is a tripwire: it is
    // supposed to fire when the inventory changes, and the human who
    // responds to it has to decide what the new invocation does. Editing
    // `toBe(7)` to `toBe(8)` and moving on would have turned the tripwire
    // back off.
    const fetches = rustBackendInvocations.filter((i) => i.subcommand === 'fetch');

    expect(
      fetches.map((i) => `${i.jobId} :: ${i.command}`),
      'expected the integration job to warm the registry with cargo fetch'
    ).not.toEqual([]);

    expect(
      fetches.filter((i) => !i.flags.includes('--locked')).map((i) => `${i.jobId} :: ${i.command}`),
      'cargo fetch rewrites Cargo.lock when the lock is out of date, so it must carry --locked'
    ).toEqual([]);

    expect(
      fetches.filter((i) => !canRewriteLock(i)).map((i) => `${i.jobId} :: ${i.command}`),
      'cargo fetch must be classified as lock-rewriting, or the per-command gate ' +
        'would skip it while still reporting the job as covered'
    ).toEqual([]);
  });

  it('every invocation that can rewrite the lock carries --locked', () => {
    const unguarded = rustBackendInvocations
      .filter(canRewriteLock)
      .filter((i) => !i.flags.includes('--locked'))
      .map((i) => `${i.jobId} :: ${i.stepName} :: ${i.command}`);

    expect(
      unguarded,
      'these cargo invocations resolve the root crate graph without --locked. Without it a ' +
        'CI run may re-resolve the dependency graph and rewrite Cargo.lock, and the artefact ' +
        'the job builds is then not the one the commit describes.'
    ).toEqual([]);
  });

  it('does not let a flag elsewhere in the file stand in for the one a command needs', () => {
    // The gate is per-command. This is the shape that has bitten this repo
    // before: the file mentions `--locked` many times, and a check that
    // asked "does the file mention --locked?" would pass with every single
    // invocation unguarded.
    const sample = [
      'set -o pipefail',
      'cargo test --all-targets --locked 2>&1 | tee /tmp/cargo-test.txt',
      '# cargo build --release --locked   <- prose, not an invocation',
      'cargo build --release',
    ].join('\n');

    const found = cargoInvocationsIn(sample);

    expect(
      found.map((i) => i.command),
      'the comment must not be counted as an invocation'
    ).toEqual(['cargo test --all-targets --locked 2>&1 | tee /tmp/cargo-test.txt', 'cargo build --release']);

    expect(
      found[0].flags,
      'the guarded command must be recognised as guarded'
    ).toContain('--locked');

    expect(
      found[1].flags,
      'the unguarded command must be recognised as unguarded even though its ' +
        'sibling in the same block carries --locked'
    ).not.toContain('--locked');
  });

  it('leaves the fmt step alone, because cargo-fmt rejects the flag', () => {
    // `cargo fmt --locked` exits 2 with "unexpected argument '--locked'
    // found" (measured against cargo 1.98.1). A well-meaning contributor
    // adding the flag to make the file look uniform would break the job,
    // so the exclusion is asserted rather than merely explained.
    const fmtSteps = rustBackendInvocations.filter((i) => i.subcommand === 'fmt');

    expect(fmtSteps.length, 'expected exactly one cargo fmt invocation').toBe(1);
    expect(
      fmtSteps[0].flags,
      'cargo fmt cannot carry --locked; cargo-fmt rejects the flag outright'
    ).not.toContain('--locked');
  });
});

/**
 * The `integration` job.
 *
 * Its cargo invocations do not live in the workflow — they live in the tier
 * scripts it calls, which is why the job's own `run:` blocks contain no
 * cargo at all. The gate therefore follows the call graph rather than
 * trusting a list of file names, so a tier that starts invoking cargo is
 * covered without this file being edited.
 */
function scriptsReachableFrom(jobId: string): string[] {
  const seen = new Set<string>();
  const queue: string[] = [];

  const collect = (text: string): void => {
    for (const match of text.matchAll(/scripts\/[A-Za-z0-9._-]+\.ps1/g)) {
      const relative = match[0].replace(/\\/g, '/');
      if (!seen.has(relative)) {
        seen.add(relative);
        queue.push(relative);
      }
    }
  };

  const job = rustBackendDoc.jobs[jobId];
  for (const step of job?.steps ?? []) collect(step.run ?? '');

  while (queue.length > 0) {
    const relative = queue.shift() as string;
    const absolute = join(REPO_ROOT, ...relative.split('/'));
    if (!existsSync(absolute)) continue;
    collect(readFileSync(absolute, 'utf8'));
  }

  return [...seen];
}

/** Split the body of a PowerShell `@(...)` array into its elements. */
function parsePsArray(body: string): string[] {
  return body
    .split(',')
    .map((s) => s.trim().replace(/^['"]/, '').replace(/['"]$/, ''))
    .filter((s) => s.length > 0);
}

/**
 * The cargo argument arrays in a PowerShell script.
 *
 * Two shapes exist in this repo and both are covered:
 *
 *   -ArgumentList @('build', '--locked', '--quiet')
 *       the subcommand is the first element and the array goes straight to
 *       cargo;
 *
 *   $argList = @('--color=never', '--offline', '--locked')
 *       the array holds only flags, and the subcommand is supplied at the
 *       call site (`cargo test $argList`).
 */
function cargoArgumentArrays(scriptText: string): string[][] {
  const arrays: string[][] = [];

  for (const match of scriptText.matchAll(/-ArgumentList\s*@\(([^)]*)\)/g)) {
    const items = parsePsArray(match[1]);
    if (items.length > 0 && CARGO_SUBCOMMANDS.has(items[0])) arrays.push(items);
  }

  if (/\$argList\s*=/.test(scriptText) && /\bcargo\b/i.test(scriptText)) {
    for (const match of scriptText.matchAll(/\$argList\s*=\s*@\(([^)]*)\)/g)) {
      arrays.push(parsePsArray(match[1]));
    }
  }

  return arrays;
}

describe('the integration job reaches cargo only through --locked invocations', () => {
  const reachable = scriptsReachableFrom('integration');

  const cargoArraysPerScript = reachable.map((relative) => ({
    relative,
    arrays: cargoArgumentArrays(readFileSync(join(REPO_ROOT, ...relative.split('/')), 'utf8')),
  }));

  it('found the tiers (a scan that finds none passes vacuously)', () => {
    expect(reachable.length, 'no script reachable from the integration job').toBeGreaterThan(0);
    expect(
      reachable,
      'the integration job must still reach the IT and ST tiers'
    ).toEqual(
      expect.arrayContaining(['scripts/regression-it.ps1', 'scripts/regression-st.ps1'])
    );
    expect(
      cargoArraysPerScript.filter((s) => s.arrays.length > 0).map((s) => s.relative),
      'no cargo argument array found in any tier — the scan is broken, not the tiers ' +
        'are safe'
    ).not.toEqual([]);
  });

  it('every cargo argument array carries --locked', () => {
    const unguarded = cargoArraysPerScript
      .filter((s) => s.arrays.length > 0)
      .flatMap((s) =>
        s.arrays
          .filter((items) => !items.includes('--locked'))
          .map((items) => `${s.relative} :: ${items.join(' ')}`)
      );

    expect(
      unguarded,
      'these tiers run cargo at the repository root without --locked. Without it a run ' +
        'may re-resolve the dependency graph and rewrite the committed Cargo.lock, and ' +
        'the artefact the tier exercises is then not the one the commit describes.'
    ).toEqual([]);
  });
});

describe('the local UT tier resolves the root crate graph', () => {
  const UT_TIER = join(REPO_ROOT, 'scripts', 'regression-ut.ps1');

  it('runs cargo with --locked', () => {
    const arrays = cargoArgumentArrays(readFileSync(UT_TIER, 'utf8'));

    expect(
      arrays.length,
      'no cargo argument array found in scripts/regression-ut.ps1 — the scan is broken, ' +
        'not the tier is safe'
    ).toBeGreaterThan(0);

    expect(
      arrays.filter((items) => !items.includes('--locked')),
      'the UT tier runs cargo at the repository root, so it resolves the root crate graph. ' +
        'Without --locked it may re-resolve and rewrite the committed Cargo.lock, and the ' +
        'test counts it reports would then describe a dependency set the commit does not name.'
    ).toEqual([]);
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
