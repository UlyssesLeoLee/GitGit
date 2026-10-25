/**
 * Gates on `scripts/ci/changed-areas.ps1` — the script that decides which of
 * this repository's five gated areas a change touches.
 *
 * ## Why this file exists
 *
 * The five pull-request workflows in this repository were path-filtered at the
 * WORKFLOW level. That is why none of them could ever be a required status
 * check: GitHub's own documentation says a workflow skipped by path filtering
 * leaves its checks in a "Pending" state, and a required check that is Pending
 * blocks the pull request for ever. So the deep gates were advisory — a red
 * `fmt + clippy + test` did not stop a merge.
 *
 * The fix moves the filter INSIDE the jobs. A job whose `if:` condition is
 * false is reported as "Success", which satisfies a required check. The
 * decision of *which* jobs run is delegated to this script, and a gate that
 * decides the wrong thing is worse than the filter it replaced: it would skip
 * a gate that should have run, and report "Success" while checking nothing.
 *
 * ## What each group of cases is for
 *
 *   * the MATCHING — which area a changed file belongs to, including the
 *     root-anchoring rule that is easiest to get wrong (`Cargo.toml` is the
 *     ROOT `Cargo.toml`, not any `Cargo.toml`);
 *   * the DELETED FILE — a file that no longer exists still has to count,
 *     because "delete a file and the build breaks" is a real failure mode;
 *   * the CONSERVATIVE DEFAULT — with no baseline every area is treated as
 *     changed, because a gate that runs too often is cheap and a gate that
 *     does not run when it should is not;
 *   * the LOUD FAILURES — a change set that cannot be computed and a pattern
 *     list that cannot be evaluated are both hard errors. A gate that quietly
 *     concludes "nothing changed" while checking nothing is worse than no
 *     gate at all;
 *   * the READ-ONLY PROMISE — the script writes files only where it says it
 *     does, and never touches the repository.
 *
 * ## A note on one expectation
 *
 * The brief for this work says a `src/main.rs` change should leave "the other
 * areas" false. It cannot, and the reason is in the pattern lists themselves:
 * `gm-desktop-bundle`'s list contains `src/**`, because the Tauri desktop
 * build compiles the Rust backend. The lists are the contract — they are the
 * workflows' `paths:` blocks, verbatim — so the lists win and the assertion
 * below states what the lists actually imply.
 */
import { afterAll, describe, expect, it } from 'vitest';
import { execFileSync, spawnSync } from 'node:child_process';
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const REPO_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..', '..', '..', '..');
const SCRIPT = join(REPO_ROOT, 'scripts', 'ci', 'changed-areas.ps1');

/** The five areas, in the order the script emits them. */
const AREAS = [
  'rust_backend',
  'gm_console',
  'gm_desktop',
  'gm_desktop_bundle',
  'frontend_audit',
] as const;

/**
 * A scratch directory OUTSIDE the repository.
 *
 * It is removed in `afterAll`, and only this directory: the test never
 * creates or deletes anything inside the repository.
 */
const TEMP_DIR = mkdtempSync(join(tmpdir(), 'gitgit-changed-areas-'));

afterAll(() => {
  rmSync(TEMP_DIR, { recursive: true, force: true });
});

type Run = { status: number; stdout: string; stderr: string };

type AreaFlags = Record<string, string>;

/**
 * Run the script.
 *
 * The GitHub-provided variables are cleared for every run: a developer's
 * machine may have `GITHUB_BASE_REF` set from an earlier experiment, and a
 * test whose result depends on the ambient environment is not a test.
 */
function runScript(args: string[], extraEnv: Record<string, string> = {}): Run {
  const result = spawnSync('pwsh', ['-NoProfile', '-File', SCRIPT, ...args], {
    cwd: REPO_ROOT,
    encoding: 'utf8',
    env: {
      ...process.env,
      GITHUB_BASE_REF: '',
      GITHUB_EVENT_BEFORE: '',
      GITHUB_OUTPUT: '',
      ...extraEnv,
    },
  });

  if (result.error) {
    throw result.error;
  }

  return {
    status: result.status ?? -1,
    stdout: result.stdout ?? '',
    stderr: result.stderr ?? '',
  };
}

/**
 * Pull the `area=true|false` lines out of a block of text.
 *
 * The lines are read from the plain `key=value` form, which is the same form
 * the script writes to `$GITHUB_OUTPUT` and to `-OutputPath`. A table line
 * (`[area] rust_backend   HIT   src/main.rs`) does not match and is ignored.
 */
function parseAreaFlags(text: string): AreaFlags {
  const flags: AreaFlags = {};
  for (const line of text.split(/\r?\n/)) {
    const match = /^(rust_backend|gm_console|gm_desktop|gm_desktop_bundle|frontend_audit)=(true|false)$/.exec(
      line.trim()
    );
    if (match) {
      flags[match[1]] = match[2];
    }
  }
  return flags;
}

let scratchCounter = 0;

function scratchPath(suffix: string): string {
  scratchCounter += 1;
  return join(TEMP_DIR, `scratch-${scratchCounter}-${suffix}`);
}

/**
 * Compute the areas for a given changed-file list.
 *
 * The list is written to a file and handed to the script with
 * `-ChangedFilesPath`, so git is never called and the result depends only on
 * the list.
 */
function computeAreas(files: string[]): { run: Run; flags: AreaFlags; outputFile: string } {
  const listPath = scratchPath('changed.txt');
  const outputPath = scratchPath('output.txt');
  writeFileSync(listPath, `${files.join('\n')}\n`, 'utf8');

  const run = runScript(['-ChangedFilesPath', listPath, '-OutputPath', outputPath]);

  expect(
    run.status,
    `the script failed on a changed-file list of ${JSON.stringify(files)}:\n${run.stdout}\n${run.stderr}`
  ).toBe(0);

  return {
    run,
    flags: parseAreaFlags(run.stdout),
    outputFile: readFileSync(outputPath, 'utf8'),
  };
}

describe('changed-areas — the script computes the five areas', () => {
  it('reports every area, and only the areas', () => {
    const { flags } = computeAreas(['src/main.rs']);

    expect(
      Object.keys(flags).sort(),
      'the script did not report exactly the five areas, so a workflow could read an output that does not exist'
    ).toEqual([...AREAS].sort());
  });

  it('sends a Rust source change to the Rust gate', () => {
    const { flags } = computeAreas(['src/main.rs']);

    expect(
      flags.rust_backend,
      'a change under src/** must hit the Rust gate, or the gate would not run on the change it exists for'
    ).toBe('true');

    expect(
      flags.gm_console,
      'a change under src/** must not hit the console gate — the console gate watches apps/gm-console/**'
    ).toBe('false');

    expect(
      flags.gm_desktop,
      'a change under src/** must not hit the desktop gate — the desktop gate watches apps/gm-desktop/**'
    ).toBe('false');

    expect(
      flags.frontend_audit,
      'a change under src/** must not hit the frontend advisory gate'
    ).toBe('false');

    // [FACT] `gm_desktop_bundle` DOES hit, and it has to: its pattern list
    // contains `src/**`, because the Tauri desktop build compiles the Rust
    // backend. A test that asserted "false" here would have required
    // changing the list, and the lists are the workflows' `paths:` blocks
    // verbatim — they are the contract, not a parameter.
    expect(
      flags.gm_desktop_bundle,
      "gm_desktop_bundle's pattern list contains `src/**`, so a Rust source change must hit it"
    ).toBe('true');
  });

  it('anchors a literal pattern at the repository root', () => {
    // [FACT] GitHub documents that path patterns match the whole path
    // starting from the repository root, so `Cargo.toml` is the ROOT
    // `Cargo.toml` and not `apps/gm-console/Cargo.toml`.
    //
    // This is the assertion most easily written wrong, and it is the one
    // that decides whether a console change drags the whole Rust suite in.
    const { flags } = computeAreas(['apps/gm-console/Cargo.toml']);

    expect(
      flags.rust_backend,
      'apps/gm-console/Cargo.toml must NOT hit the Rust gate — `Cargo.toml` matches the root file only'
    ).toBe('false');

    expect(
      flags.gm_console,
      'apps/gm-console/Cargo.toml must hit the console gate'
    ).toBe('true');

    expect(
      flags.frontend_audit,
      'apps/gm-console/** is in the frontend advisory gate’s list, so it must hit'
    ).toBe('true');
  });

  it('sends a desktop source change to the three frontend gates', () => {
    const { flags } = computeAreas(['apps/gm-desktop/src/routes/+page.svelte']);

    for (const area of ['gm_desktop', 'gm_desktop_bundle', 'frontend_audit']) {
      expect(
        flags[area],
        `apps/gm-desktop/** is in ${area}’s pattern list, so the change must hit it`
      ).toBe('true');
    }

    expect(
      flags.rust_backend,
      'a desktop source change must not hit the Rust gate'
    ).toBe('false');

    expect(
      flags.gm_console,
      'a desktop source change must not hit the console gate'
    ).toBe('false');
  });

  it('counts a file that no longer exists', () => {
    // [FACT] GitHub's path filter counts DELETED files as matches, and so
    // must this: deleting a file is exactly how a build breaks.
    const { flags } = computeAreas(['src/this-file-was-deleted-long-ago.rs']);

    expect(
      flags.rust_backend,
      'a deleted file under src/** must still hit the Rust gate — deleting a file is how a build breaks'
    ).toBe('true');
  });

  it('treats every area as changed when there is no baseline', () => {
    // A `schedule` or `workflow_dispatch` event has no base to diff
    // against. Running every gate is expensive; running none of them is
    // unsafe. The script must take the expensive side.
    const run = runScript([]);
    const flags = parseAreaFlags(run.stdout);

    expect(run.status, `the script failed without a baseline:\n${run.stdout}\n${run.stderr}`).toBe(0);

    for (const area of AREAS) {
      expect(
        flags[area],
        `${area} must be true when there is no baseline — a gate that runs too often is cheap, one that does not run is not`
      ).toBe('true');
    }

    expect(
      run.stdout,
      'the script must say WHY every area was treated as changed, so the CI log explains the run'
    ).toMatch(/no baseline/i);
  });
});

describe('changed-areas — the script fails loudly rather than silently', () => {
  it('fails when the change set cannot be computed', () => {
    // The case the whole script is written around: a base ref that exists
    // but cannot be diffed (a shallow clone without it, a typo) must NOT be
    // reported as "nothing changed".
    const run = runScript(['-BaseRef', 'refs/heads/definitely-not-a-real-ref']);

    expect(
      run.status,
      'a base ref that cannot be diffed must fail the step — a gate that reports Success while checking nothing is worse than no gate'
    ).not.toBe(0);

    const said = `${run.stdout}\n${run.stderr}`;
    expect(
      said,
      'the failure must name the base ref that could not be used'
    ).toContain('definitely-not-a-real-ref');

    expect(
      said,
      'the failure must say that this is NOT the same as "nothing changed"'
    ).toMatch(/not the same as/i);
  });

  it('refuses a pattern list it cannot evaluate', () => {
    // A wildcard the script does not implement would be compared literally,
    // which matches the wrong files while looking like a working gate. The
    // script has to refuse it.
    const patternsPath = scratchPath('patterns.json');
    writeFileSync(
      patternsPath,
      JSON.stringify({ rust_backend: ['src/**', 'Cargo.toml', 'src/*.rs'] }),
      'utf8'
    );

    const run = runScript(['-BaseRef', 'HEAD', '-PatternsPath', patternsPath]);

    expect(
      run.status,
      'a wildcard in a pattern list must fail the step, not be treated as a literal'
    ).not.toBe(0);

    const said = `${run.stdout}\n${run.stderr}`;
    expect(
      said,
      'the failure must name the pattern that cannot be evaluated'
    ).toContain('src/*.rs');

    expect(
      said,
      'the failure must name the area the pattern belongs to'
    ).toContain('rust_backend');
  });
});

describe('changed-areas — the script writes what it says it writes', () => {
  it('writes the same flags to the output file as to stdout', () => {
    const { flags, outputFile } = computeAreas(['apps/gm-console/src/App.tsx']);

    const fromFile = parseAreaFlags(outputFile);
    expect(
      fromFile,
      'the file written by -OutputPath must carry the same flags as stdout'
    ).toEqual(flags);

    expect(
      Object.keys(fromFile),
      'the output file must carry one entry per area and nothing else'
    ).toEqual([...AREAS]);
  });

  it('writes to $GITHUB_OUTPUT when GitHub sets it', () => {
    const githubOutput = scratchPath('github-output.txt');
    const run = runScript(['-BaseRef', 'HEAD'], { GITHUB_OUTPUT: githubOutput });

    expect(run.status, `the script failed:\n${run.stdout}\n${run.stderr}`).toBe(0);

    const written = readFileSync(githubOutput, 'utf8');
    expect(
      parseAreaFlags(written),
      'GitHub reads the job outputs from $GITHUB_OUTPUT, so the flags have to be there'
    ).toEqual(parseAreaFlags(run.stdout));
  });

  it('does not modify the repository', () => {
    const before = execFileSync('git', ['status', '--porcelain'], {
      cwd: REPO_ROOT,
      encoding: 'utf8',
    });

    // `-BaseRef HEAD` exercises the git path: the diff is computed against
    // the current commit, which every checkout has.
    const run = runScript(['-BaseRef', 'HEAD']);
    expect(run.status, `the script failed:\n${run.stdout}\n${run.stderr}`).toBe(0);

    const after = execFileSync('git', ['status', '--porcelain'], {
      cwd: REPO_ROOT,
      encoding: 'utf8',
    });

    expect(
      after,
      'the script changed the repository state. It must only read: a gate that switches a branch or writes a file can leave the checkout in a state the next step does not expect.'
    ).toBe(before);
  });
});
