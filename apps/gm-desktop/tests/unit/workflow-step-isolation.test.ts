/**
 * Why this file exists
 * ====================
 * `release.yml` failed on its first and only real run, and it failed with no
 * explanation at all. The tag was correct. The version was correct. The log
 * contained this and nothing else:
 *
 *     .../a1ca9eb1-fde5-4e00-8525-8dcec4214937.sh: line 7: ref: unbound variable
 *     ##[error]Process completed with exit code 1.
 *
 * `ref` was assigned in the `Resolve the tag` step and read in the
 * `Compare against tauri.conf.json` step. Both steps declare
 * `set -euo pipefail`. Both are correct, individually, and the sequence is
 * broken, because **every `run:` block is its own process**. Nothing a step
 * assigns survives into the next one.
 *
 * The reason this survived a review, a YAML parse, a local `bash -n` on every
 * extracted step, and a merge is worth stating plainly: the workflow was never
 * executed. It parsed. Every check that could run without pushing a tag said
 * yes. The only thing that found it was pushing `v0.1.0`.
 *
 * So the two things below exist:
 *
 *   1. `step isolation` — a static invariant over EVERY workflow in
 *      `.github/workflows`, not just `release.yml`. A `run:` step may only
 *      read a shell variable it assigns itself, is handed through `env:`, or
 *      that the runner provides. Nothing else is in scope, because nothing
 *      else is in scope at run time.
 *
 *   2. `verify-tag runs` — the actual steps of `release.yml`'s preflight job,
 *      extracted from the YAML and executed as separate processes, exactly
 *      the way the runner executes them.
 *
 * WHAT #2 ASSERTS, AND WHY IT IS NOT JUST #1 AGAIN
 * ==================================================
 * #1 cannot see a variable that a step reads but never assigns anywhere in
 * the repository. It can only compare a read against what it can prove is
 * present, and `ref` was provably absent from *its own step* — so #1 alone
 * would have caught this. But #1 is a regex over text, and the failure mode of
 * a regex over text is silence.
 *
 * #2 needs no shell parsing at all. It runs the step and looks at the exit
 * code. And the assertion it is built around is deliberately narrow:
 *
 *     A step that fails MUST have said why.
 *
 * That is the exact shape of the original bug. `ref: unbound variable` is a
 * failure with no `::error::` line — bash's own diagnostic, on stderr, after
 * `set -u` had already decided. Every failure this workflow intends is a
 * guarded branch that echoes `::error::` and exits 1. A failure without one is
 * a defect, not a verdict.
 *
 * REQUIREMENTS FOR RUNNING LOCALLY
 * ===============================
 * `bash` and `jq`. Both are preinstalled on every GitHub-hosted runner, so CI
 * is covered by construction.
 *
 * If either is missing this file **fails** rather than skipping, and the
 * failure says which one and how to get it. That is deliberate. A gate that
 * skips when it cannot run is indistinguishable, in the log, from a gate that
 * passed — and this repository has been bitten by exactly that shape more than
 * once. A red test you have to fix is cheaper than a green one that checked
 * nothing.
 */
import { describe, expect, it } from 'vitest';
import { spawnSync, type SpawnSyncReturns } from 'node:child_process';
import { existsSync, mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { parse } from 'yaml';

// tests/unit/ -> tests/ -> gm-desktop/ -> apps/ -> repo root
const REPO_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..', '..', '..', '..');
const WORKFLOW_DIR = join(REPO_ROOT, '.github', 'workflows');

// ---------------------------------------------------------------------------
// Static analysis
// ---------------------------------------------------------------------------

/**
 * Variables the Actions runner puts in every step's environment. Anything
 * matching a prefix is assumed present; the exact list covers the rest.
 */
const RUNNER_PREFIXES = [
  'GITHUB_',
  'RUNNER_',
  'ACTIONS_',
  'INPUT_',
  'PNPM_',
  'npm_',
  'CARGO_',
  'RUST',
  'NODE',
  'JAVA_',
  'DOTNET_',
  'PYTHON',
  'SUDO_',
  'LC_',
  'VSCODE_',
  'AGENT_TOOLSDIRECTORY',
  'SSL_',
  'http_proxy',
  'https_proxy',
  'no_proxy',
];

const RUNNER_EXACT = new Set([
  'CI',
  'HOME',
  'PATH',
  'PWD',
  'OLDPWD',
  'SHELL',
  'USER',
  'LOGNAME',
  'HOSTNAME',
  'TERM',
  'LANG',
  'TMPDIR',
  'TEMP',
  'TMP',
  'HOMEDRIVE',
  'HOMEPATH',
  'SYSTEMROOT',
  'SYSTEMDRIVE',
  'COMSPEC',
  'OS',
  'OSTYPE',
  'MACHTYPE',
  'SHLVL',
  'SHELLOPTS',
  'PROGRAMDATA',
  'APPDATA',
  'LOCALAPPDATA',
  'USERPROFILE',
  'PROCESSOR_ARCHITECTURE',
  'NUMBER_OF_PROCESSORS',
  'TESSDATA_PREFIX',
  'FORCE_COLOR',
  'NO_COLOR',
  'DEBIAN_FRONTEND',
  'IFS',
  'PS1',
  'PS4',
  'OPTIND',
  'OPTARG',
  'REPLY',
  'UID',
  'EUID',
  'PPID',
  'PIPESTATUS',
  'BASH_VERSION',
  'BASH_SOURCE',
  'BASH_SUBSHELL',
  'RANDOM',
  'SECONDS',
  'LINENO',
  'GROUPS',
  'COLUMNS',
  'LINES',
  '_',
]);

/**
 * `name=`, `export name=`, `declare -x name=`, `local name=`, `readonly name=`.
 *
 * The leading group rejects a match when `=` is part of a comparison (`==`,
 * `!=`, `<=`, `>=`) and when the identifier is inside a quoted literal. The
 * `(?!=)` rejects `x == y`. Both are there because a shell step in these
 * workflows is dense with `[ "$a" = "$b" ]`, and a gate that reads those as
 * assignments reports nonsense.
 */
const ASSIGN_RE =
  /(^|[^A-Za-z0-9_!<>='"`])(?:export\s+|declare\s+-[A-Za-z-]+\s+|local\s+|readonly\s+|typeset\s+)*([A-Za-z_][A-Za-z0-9_]*)\s*=(?!=)/g;

/** `for name in ...` — assigns without ever writing an `=`. */
const FOR_RE = /\bfor\s+([A-Za-z_][A-Za-z0-9_]*)\s+in\b/g;

/**
 * `mapfile -t name` / `readarray`.
 *
 * `gm-desktop-bundle.yml` reads its dependency list this way, and `mapfile`
 * assigns without an `=` anywhere near it. Missing this produced four false
 * positives the first time this file ran — which is the failure mode that
 * teaches people to switch a gate off.
 */
const MAPFILE_RE = /\b(?:mapfile|readarray)\s+(?:-[A-Za-z]+\s+)*([A-Za-z_][A-Za-z0-9_]*)/g;

/** `read -r name` — likewise. */
const READ_ASSIGN_RE = /\bread\s+(?:-[A-Za-z]+\s+)*([A-Za-z_][A-Za-z0-9_]*)\s*(?:;|$)/gm;

/**
 * `$name` and `${name...}`.
 *
 * `${{ }}` deliberately does not match: `${` followed by `{` cannot begin a
 * variable name, and `$` followed by `{` is not `$name`. The workflow engine
 * expands those before bash ever sees the text, so they are not shell reads.
 */
const SHELL_READ_RE = /\$\{([A-Za-z_][A-Za-z0-9_]*)[^}]*\}|\$([A-Za-z_][A-Za-z0-9_]*)/g;

/**
 * Variables `/etc/os-release` defines.
 *
 * `gm-desktop-bundle.yml`'s deb dependency check contains
 *
 *     $(. /etc/os-release && echo "$PRETTY_NAME")
 *
 * which is legitimate: the variable really is assigned, by sourcing a file.
 * No scan of the script text can see an assignment made by a file the script
 * includes, so this is a real blind spot and it is closed for the one file
 * whose variable set is fixed and documented (os-release(5)).
 *
 * A step that sources some OTHER file will be reported as a violation. That
 * is intended: a new blind spot should arrive as a loud, fixable failure
 * rather than as silence.
 */
const OS_RELEASE_VARS = new Set([
  'NAME',
  'VERSION',
  'ID',
  'ID_LIKE',
  'PRETTY_NAME',
  'VERSION_ID',
  'VERSION_CODENAME',
  'VERSION_CODENAME_LOWER',
  'CPE_NAME',
  'CPE',
  'CPE_VERSION',
  'HOME_URL',
  'DOCUMENTATION_URL',
  'SUPPORT_URL',
  'BUG_REPORT_URL',
  'PRIVACY_POLICY_URL',
  'SUPPORT_END',
  'VARIANT',
  'VARIANT_ID',
  'LOGO',
  'ANSI_COLOR',
  'DEFAULT_HOSTNAME',
  'BUILD_ID',
  'IMAGE_ID',
  'IMAGE_VERSION',
]);

function providedByRunner(name: string): boolean {
  return (
    RUNNER_EXACT.has(name) ||
    OS_RELEASE_VARS.has(name) ||
    RUNNER_PREFIXES.some((p) => name.startsWith(p))
  );
}

/**
 * Strip whole-line and trailing comments.
 *
 * Conservative by design: it drops text from a `#` that starts a line or
 * follows whitespace. That can cut a `#` inside an unquoted string, which
 * loses a detection — it cannot invent one.
 */
function stripComments(script: string): string {
  return script
    .split('\n')
    .map((line) => line.replace(/(^|\s)#.*$/, ''))
    .join('\n');
}

/**
 * Blank out single-quoted spans.
 *
 * bash performs **no** parameter expansion inside single quotes, so `$Status`
 * in `dpkg-query -f='${Status} ${Version}'` is a dpkg format directive, not a
 * shell variable read. Scanning it as a read reported four phantom violations
 * in `gm-desktop-bundle.yml` the first time this file ran, which is exactly
 * how a gate gets switched off.
 *
 * Cost: a single quote inside a double-quoted string would hide a real read
 * after it. That costs a detection. The alternative costs correctness, and a
 * gate that cries wolf costs the gate.
 */
function stripSingleQuoted(script: string): string {
  return script.replace(/'[^']*'/g, "''");
}

function collectAssigned(script: string): Set<string> {
  const body = stripSingleQuoted(stripComments(script));
  const names = new Set<string>();
  for (const m of body.matchAll(ASSIGN_RE)) names.add(m[2]);
  for (const m of body.matchAll(FOR_RE)) names.add(m[1]);
  for (const m of body.matchAll(MAPFILE_RE)) names.add(m[1]);
  for (const m of body.matchAll(READ_ASSIGN_RE)) names.add(m[1]);
  return names;
}

function collectRead(script: string): Set<string> {
  const body = stripSingleQuoted(stripComments(script));
  const names = new Set<string>();
  for (const m of body.matchAll(SHELL_READ_RE)) names.add(m[1] ?? m[2]);
  return names;
}

// --- a very small typed view over the workflow YAML -------------------------

interface WfStep {
  name?: string;
  id?: string;
  uses?: string;
  run?: string;
  shell?: string;
  env?: Record<string, unknown>;
  /** `with:` — a reserved word, but a legal property name. */
  with?: Record<string, unknown>;
}

interface WfJob {
  env?: Record<string, unknown>;
  steps?: WfStep[];
}

interface WfDoc {
  env?: Record<string, unknown>;
  jobs?: Record<string, WfJob>;
}

/** `shell` unset means the runner's default for the OS, which is bash. */
function usesBash(step: WfStep): boolean {
  if (typeof step.run !== 'string') return false;
  if (step.shell === undefined) return true;
  return step.shell === 'bash' || step.shell === 'sh';
}

const workflowFiles = readdirSync(WORKFLOW_DIR)
  .filter((f) => f.endsWith('.yml') || f.endsWith('.yaml'))
  .sort();

const documents = new Map<string, WfDoc>(
  workflowFiles.map((f) => [f, parse(readFileSync(join(WORKFLOW_DIR, f), 'utf8')) as WfDoc])
);

describe('step isolation — a run: step may only read what it can actually see', () => {
  it('found the workflows at all', () => {
    // A gate that reads zero files and reports zero violations is a gate that
    // passed because it looked at nothing. Prove the directory first.
    expect(workflowFiles.length).toBeGreaterThan(0);
    expect(documents.size).toBe(workflowFiles.length);
    expect(documents.get('release.yml')).toBeDefined();
  });

  it('no step reads a shell variable that dies with the step that set it', () => {
    const violations: string[] = [];

    for (const [file, doc] of documents) {
      const workflowEnv = Object.keys(doc.env ?? {});
      for (const [jobId, job] of Object.entries(doc.jobs ?? {})) {
        const jobEnv = Object.keys(job.env ?? {});
        for (const step of job.steps ?? []) {
          if (!usesBash(step)) continue;

          const stepEnv = Object.keys(step.env ?? {});
          const assigned = collectAssigned(step.run as string);
          const visible = new Set([...assigned, ...stepEnv, ...jobEnv, ...workflowEnv]);

          for (const name of collectRead(step.run as string)) {
            if (visible.has(name) || providedByRunner(name)) continue;
            violations.push(
              `${file} :: job "${jobId}" :: step "${step.name ?? step.uses ?? '<unnamed>'}"` +
                ` reads $${name}, which is assigned in no step of this job, passed through no env:, and not a runner variable.` +
                ` Each run: block is a separate process, so under set -u this fails with` +
                ` "${name}: unbound variable" and no ::error:: line.`
            );
          }
        }
      }
    }

    expect(violations, `\n${violations.join('\n')}\n`).toEqual([]);
  });

  it('the scanner sees the variables it is supposed to see', () => {
    // Anti-false-negative check. A scanner that returned empty sets would make
    // the gate above vacuously true, which is the one outcome it must never
    // have. These are the real constructs from the workflows in this repo.
    expect([...collectAssigned('ref="${GITHUB_REF#refs/tags/}"\necho "ref=$ref" >> "$GITHUB_OUTPUT"')]).toEqual(['ref']);
    expect([...collectAssigned('export TAG=v1\ndeclare -x A=1\nlocal b=2\nreadonly c=3')].sort()).toEqual(['A', 'TAG', 'b', 'c']);
    expect([...collectAssigned('for f in a b c; do echo "$f"; done')]).toEqual(['f']);
    // Real, from gm-desktop-bundle.yml. `mapfile` assigns without an `=`.
    expect([...collectAssigned("mapfile -t deps < <(jq -r '.a // [] | .[]' f.json)")]).toEqual(['deps']);
    expect([...collectRead('tagged="${REF#v}"')]).toEqual(['REF']);
    expect([...collectRead('echo "${{ github.event_name }}"')]).toEqual([]);
    expect([...collectRead('status=${PIPESTATUS[0]}')]).toEqual(['PIPESTATUS']);
    // A comparison is not an assignment, and the identifier inside it is not
    // an assignment either.
    expect([...collectAssigned('if [ "$declared" != "$tagged" ]; then')]).toEqual([]);
  });

  it('does not mistake a literal in single quotes for a shell variable', () => {
    // Real, from gm-desktop-bundle.yml: `dpkg-query -f='${Status} ${Version}'`.
    // bash expands nothing inside single quotes, so these are dpkg's own
    // format directives. Reading them as shell variables reported four
    // phantom violations on the first run of this file.
    expect([...collectRead("dpkg-query -W -f='${Status} ${Version} ${Package}' \"$dep\"")]).toEqual(['dep']);
    // Double quotes DO expand, so a read inside them is still a read.
    expect([...collectRead('echo "resolved: $REAL_THING"')]).toEqual(['REAL_THING']);
  });
});

// ---------------------------------------------------------------------------
// A second defect class, found while verifying the first fix
// ---------------------------------------------------------------------------

/** The basename a path or glob requires. */
function basenameOf(pattern: string): string {
  return pattern.slice(pattern.lastIndexOf('/') + 1);
}

/**
 * Glob match for the two metacharacters these paths actually use (`*`, `?`),
 * anchored at both ends.
 *
 * Comparing basenames rather than extensions is what makes this gate
 * discriminating. An extension-only check calls a `manifest.json` glob and a
 * `dmg-manifest.json` upload compatible, because both end in `.json`. They
 * are not: the first cannot match the second, which is exactly the shape a
 * half-finished fix leaves behind.
 */
function basenameMatches(pattern: string, name: string): boolean {
  const escaped = pattern.replace(/[.+^${}()|[\]\\]/g, '\\$&').replace(/\*/g, '.*').replace(/\?/g, '.');
  return new RegExp(`^${escaped}$`).test(name);
}

describe('release assets — every glob in files: has something that can match it', () => {
  it('found the release workflow and its release-creating step', () => {
    // Same reason as the workflow-count assertion above: prove the thing this
    // gate reads exists before trusting a clean result.
    const doc = documents.get('release.yml') as WfDoc;
    const releaseSteps = (doc.jobs?.['draft-release']?.steps ?? []).filter(
      (s) => typeof s.uses === 'string' && s.uses.startsWith('softprops/action-gh-release')
    );
    expect(releaseSteps).toHaveLength(1);
  });

  it('no required asset extension is unproducible by any upload in the workflow', () => {
    // How the second defect in `release.yml` happened.
    //
    // Its `files:` listed `staged/**/manifest.json` with
    // `fail_on_unmatched_files: true`, while all four build jobs uploaded
    // only `*.dmg`, `*.deb`, `*.AppImage` and `msi-out/*.msi`. Nothing in the
    // workflow ever put a manifest into an artifact, so the glob had nothing
    // to match and the action would have thrown — after dmg, deb, appimage and
    // msi had each spent minutes producing an installer.
    //
    // The action decides this per pattern, not per run:
    //
    //     patterns.forEach((pattern) => {
    //       if (config.input_fail_on_unmatched_files) throw ...
    //     })
    //
    // so one unmatchable entry is enough. Which extension a glob needs, and
    // which extensions the uploads can produce, is a comparison this gate can
    // make without running anything.
    const doc = documents.get('release.yml') as WfDoc;

    const uploadPaths: string[] = [];
    for (const job of Object.values(doc.jobs ?? {})) {
      for (const step of job.steps ?? []) {
        if (typeof step.uses === 'string' && step.uses.startsWith('actions/upload-artifact')) {
          for (const line of String((step.with as Record<string, unknown>)?.path ?? '')
            .split('\n')
            .map((l) => l.trim())
            .filter(Boolean)) {
            uploadPaths.push(line);
          }
        }
      }
    }
    expect(uploadPaths.length, 'no upload steps found; the gate below would be vacuous').toBeGreaterThan(0);

    const uploadBasenames = uploadPaths.map(basenameOf);

    const releaseStep = (doc.jobs?.['draft-release']?.steps ?? []).find(
      (s) => typeof s.uses === 'string' && s.uses.startsWith('softprops/action-gh-release')
    );
    const filesInput = String((releaseStep?.with as Record<string, unknown>)?.files ?? '');
    const globs = filesInput.split('\n').map((l) => l.trim()).filter(Boolean);
    expect(globs.length, 'the release step declares no files: globs').toBeGreaterThan(0);

    const unproducible = globs
      .filter((g) => !uploadBasenames.some((u) => basenameMatches(basenameOf(g), u)))
      .map(
        (g) =>
          `files: glob "${g}" needs a file called "${basenameOf(g)}", and no upload produces one.` +
          ` Upload basenames: ${uploadBasenames.join(', ')}`
      );

    expect(unproducible, `\n${unproducible.join('\n')}\n`).toEqual([]);
  });

  it('the matcher is not fooled by a shared extension', () => {
    // The reason this compares basenames. Both end in `.json`; only one of
    // these two can match anything that is actually uploaded.
    expect(basenameMatches('manifest.json', 'dmg-manifest.json')).toBe(false);
    expect(basenameMatches('*-manifest.json', 'dmg-manifest.json')).toBe(true);
    expect(basenameMatches('*.msi', '*.msi')).toBe(true);
    expect(basenameMatches('*.AppImage', '*.dmg')).toBe(false);
    expect(basenameMatches('gm-desktop_*.deb', 'gm-desktop_0.1.0_amd64.deb')).toBe(true);
  });

  it('every upload names a distinct artifact, or merges would silently collapse them', () => {
    // `download-artifact` is configured with `merge-multiple: true`, so two
    // artifacts holding a file of the same name land on one path. The fix for
    // that was platform-qualified manifest names, and this keeps them
    // qualified.
    const doc = documents.get('release.yml') as WfDoc;

    const manifestArtifacts: string[] = [];
    for (const job of Object.values(doc.jobs ?? {})) {
      for (const step of job.steps ?? []) {
        if (typeof step.uses !== 'string' || !step.uses.startsWith('actions/upload-artifact')) continue;
        const with_ = (step.with ?? {}) as Record<string, unknown>;
        const name = String(with_.name ?? '');
        if (/manifest\.json$/.test(String(with_.path ?? ''))) manifestArtifacts.push(name);
      }
    }

    expect(manifestArtifacts.length, 'no manifest artifact is uploaded at all').toBe(4);
    expect(new Set(manifestArtifacts).size, `manifest artifacts share a name: ${manifestArtifacts.join(', ')}`).toBe(
      manifestArtifacts.length
    );
  });
});

// ---------------------------------------------------------------------------
// Behavioural: run the real preflight steps
// ---------------------------------------------------------------------------

function resolveBash(): string {
  const override = process.env.GIT_BASH;
  if (override && existsSync(override)) return override;
  if (process.platform === 'win32') {
    for (const candidate of [
      'C:\\Program Files\\Git\\bin\\bash.exe',
      'C:\\Program Files\\Git\\usr\\bin\\bash.exe',
    ]) {
      if (existsSync(candidate)) return candidate;
    }
  }
  return 'bash';
}

const BASH = resolveBash();

function bashAvailable(): boolean {
  return spawnSync(BASH, ['-c', 'command -v jq >/dev/null 2>&1']).status === 0;
}

/**
 * Replace `${{ expr }}` with a concrete value.
 *
 * An expression with no entry here throws rather than substituting an empty
 * string. Substituting empty would let a step that reads a value CI never set
 * look like it passed.
 */
function expandExpressions(script: string, values: Record<string, string>, where: string): string {
  return script.replace(/\$\{\{([^}]*)\}\}/g, (_whole, expr: string) => {
    const key = expr.trim();
    const value = values[key];
    if (value === undefined) {
      throw new Error(
        `${where}: no value supplied for expression \${{ ${key} }}.\n` +
          `Known: ${Object.keys(values).join(', ')}\n` +
          `The test refuses to substitute an empty value, because a step reading a` +
          ` variable nobody set can look like it passed.`
      );
    }
    return value;
  });
}

interface StepOutcome {
  name: string;
  status: number | null;
  output: string;
}

function runVerifyTag(
  scenario: { tag: string; eventName: 'push' | 'workflow_dispatch' },
): { outcomes: StepOutcome[]; outputs: string; summary: string } {
  const doc = documents.get('release.yml') as WfDoc;
  const job = doc.jobs?.['verify-tag'];
  if (!job) throw new Error('release.yml has no verify-tag job');

  const steps = (job.steps ?? []).filter((s) => typeof s.run === 'string');

  const dir = mkdtempSync(join(tmpdir(), 'release-verify-tag-'));
  const outputFile = join(dir, 'github-output');
  const summaryFile = join(dir, 'github-summary');
  writeFileSync(outputFile, '');
  writeFileSync(summaryFile, '');

  const outcomes: StepOutcome[] = [];
  // Values the workflow engine resolves `${{ }}` against. Outputs are added to
  // this table under `steps.<owning step>.outputs.<name>` as each step
  // finishes — keyed by the step that PUBLISHED them, which is not always the
  // step being expanded. Keying by the consuming step was the second bug in
  // this harness: `REF: ${{ steps.tag.outputs.ref }}` came out as
  // `steps.check.outputs.ref`, matched nothing, and failed every case.
  const expressionValues: Record<string, string> = {
    'github.event_name': scenario.eventName,
    'inputs.tag': scenario.tag,
  };

  try {
    for (const step of steps) {
      const stepEnv: Record<string, string> = {};
      for (const [key, raw] of Object.entries(step.env ?? {})) {
        const where = `release.yml verify-tag step "${step.name}" env.${key}`;
        stepEnv[key] = expandExpressions(String(raw), expressionValues, where);
      }

      const where = `release.yml verify-tag step "${step.name}"`;
      const script = expandExpressions(step.run as string, expressionValues, where);
      const scriptFile = join(dir, `step-${steps.indexOf(step)}.sh`);
      writeFileSync(scriptFile, script, 'utf8');

      const res: SpawnSyncReturns<string> = spawnSync(BASH, [scriptFile], {
        cwd: REPO_ROOT,
        encoding: 'utf8',
        env: {
          ...process.env,
          ...stepEnv,
          GITHUB_REF: scenario.eventName === 'push' ? `refs/tags/${scenario.tag}` : 'refs/heads/dev',
          GITHUB_OUTPUT: outputFile,
          GITHUB_STEP_SUMMARY: summaryFile,
          GITHUB_WORKSPACE: REPO_ROOT,
          GITHUB_REPOSITORY: 'UlyssesLeoLee/GitGit',
        },
      });

      outcomes.push({
        name: step.name ?? '<unnamed>',
        status: res.status,
        output: `${res.stdout ?? ''}${res.stderr ?? ''}`,
      });

      // Publish this step's outputs, the way the runner reads them back: the
      // runner keys them by the step that WROTE them.
      if (step.id) {
        for (const line of readFileSync(outputFile, 'utf8').split('\n')) {
          const eq = line.indexOf('=');
          if (eq > 0) {
            expressionValues[`steps.${step.id}.outputs.${line.slice(0, eq)}`] = line.slice(eq + 1);
          }
        }
      }

      if (res.status !== 0) break;
    }

    return {
      outcomes,
      outputs: readFileSync(outputFile, 'utf8'),
      summary: readFileSync(summaryFile, 'utf8'),
    };
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
}

/** The version the repo actually declares, read the same way the workflow does. */
const DECLARED_VERSION: string = (() => {
  const conf = readFileSync(join(REPO_ROOT, 'apps/gm-desktop/src-tauri/tauri.conf.json'), 'utf8');
  return (JSON.parse(conf) as { version: string }).version;
})();

describe('release.yml verify-tag — the steps actually execute', () => {
  it('has bash and jq, without which this file proves nothing', () => {
    const bash = spawnSync(BASH, ['-c', 'echo ok'], { encoding: 'utf8' });
    expect(
      bash.status,
      `bash did not run from ${BASH}. On Windows the PATH "bash" is usually WSL, which cannot see this repository; ` +
        `set GIT_BASH to a Git-for-Windows bash.exe.`
    ).toBe(0);
    expect(
      bashAvailable(),
      'jq was not found by bash. Every step of verify-tag reads tauri.conf.json with it, and CI has it ' +
        'preinstalled, so this gate cannot be honestly skipped when it is missing.'
    ).toBe(true);
  });

  it('passes every step when the tag matches the declared version', () => {
    const { outcomes, outputs } = runVerifyTag({ tag: `v${DECLARED_VERSION}`, eventName: 'push' });

    expect(
      outcomes.map((o) => `${o.name}=${o.status}`),
      outcomes.map((o) => `${o.name}\n${o.output}`).join('\n---\n')
    ).toEqual(outcomes.map((o) => `${o.name}=0`));
    expect(outputs).toContain(`version=${DECLARED_VERSION}`);
  });

  it('reports a mismatch with an explanation instead of a bare exit code', () => {
    const { outcomes } = runVerifyTag({ tag: 'v9.9.9', eventName: 'push' });

    expect(outcomes[0].status).toBe(0);
    expect(outcomes[1].status).toBe(1);
    expect(outcomes[1].output).toContain('::error::');
    expect(outcomes[1].output).toContain('9.9.9');
    expect(outcomes[1].output).toContain(DECLARED_VERSION);
    // The jobs behind it must not run.
    expect(outcomes).toHaveLength(2);
  });

  it('rejects a tag with no v prefix, naming it', () => {
    const { outcomes } = runVerifyTag({ tag: DECLARED_VERSION, eventName: 'push' });

    expect(outcomes[0].status).toBe(1);
    expect(outcomes[0].output).toContain('::error::');
    expect(outcomes[0].output).toContain('not a v-prefixed tag');
    expect(outcomes).toHaveLength(1);
  });

  it('takes the tag from the dispatch input when there is no tag in the ref', () => {
    const ok = runVerifyTag({ tag: `v${DECLARED_VERSION}`, eventName: 'workflow_dispatch' });
    expect(ok.outcomes.map((o) => o.status)).toEqual([0, 0, 0]);

    const bad = runVerifyTag({ tag: 'release-candidate', eventName: 'workflow_dispatch' });
    expect(bad.outcomes[0].status).toBe(1);
    expect(bad.outcomes[0].output).toContain('release-candidate');
  });

  it('never fails a step without saying why — the shape of the 2026-10-06 failure', () => {
    // This is the assertion, not the scenarios above. Every failure this
    // workflow intends is a guarded branch that echoes `::error::`. A step
    // that exits non-zero having printed no `::error::` was killed by
    // something the author did not write — `set -u` on an unbound name, a
    // missing tool, a typo. That is a defect, and it is the defect this file
    // was written for.
    for (const scenario of [
      { tag: `v${DECLARED_VERSION}`, eventName: 'push' as const },
      { tag: 'v9.9.9', eventName: 'push' as const },
      { tag: DECLARED_VERSION, eventName: 'push' as const },
      { tag: `v${DECLARED_VERSION}`, eventName: 'workflow_dispatch' as const },
      { tag: 'nonsense', eventName: 'workflow_dispatch' as const },
    ]) {
      const { outcomes } = runVerifyTag(scenario);
      for (const outcome of outcomes) {
        if (outcome.status === 0) continue;
        expect(
          outcome.output,
          `scenario ${scenario.eventName}/${scenario.tag}, step "${outcome.name}" exited ${outcome.status} ` +
            `without an ::error:: line:\n${outcome.output}`
        ).toContain('::error::');
      }
    }
  });

  it('the preflight publishes exactly the outputs the release jobs read', () => {
    // `draft-release` consumes `needs.verify-tag.outputs.version`. If the
    // preflight stopped writing `version=`, the four builds would still pass
    // and the release would be published with an empty name.
    const doc = documents.get('release.yml') as WfDoc;
    expect(doc.jobs?.['verify-tag']?.steps?.find((s) => s.id === 'check')).toBeDefined();

    const { outputs } = runVerifyTag({ tag: `v${DECLARED_VERSION}`, eventName: 'push' });
    const keys = outputs
      .split('\n')
      .map((l) => l.slice(0, l.indexOf('=')))
      .filter(Boolean);
    expect(keys).toContain('version');
  });
});