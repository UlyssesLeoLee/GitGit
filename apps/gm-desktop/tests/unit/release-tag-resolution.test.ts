/**
 * Gates on WHICH TAG a release run builds.
 *
 * ## Why this exists
 *
 * `release.yml` has two triggers — a `v*` tag push and a `workflow_dispatch`
 * that names a tag in its `tag` input — and it used to build two different
 * things. Every job checked out with `actions/checkout@v4` and no `ref:`,
 * and with no `ref:` checkout resolves the ref that TRIGGERED the run.
 *
 * On a tag push that ref is the tag, so the tag path worked — by accident,
 * not by design. On a dispatch it is the BRANCH the operator picked, and the
 * `tag` input was read by the preflight and by nothing else. The four build
 * jobs therefore compiled the dispatched branch's tree while the release was
 * labelled with a tag, and the version check compared the BRANCH's
 * `tauri.conf.json` against the TAG's name: a number read out of one tree,
 * compared with a name read out of another.
 *
 * Two more places named the branch for the same reason: the release body
 * interpolated `github.ref_name`, and the concurrency group was keyed on
 * `github.ref`. A dispatch of tag X and a push of tag X landed in different
 * concurrency groups — so they could run against the same tag at once —
 * while two dispatches of two different tags from one branch serialised
 * against each other for no reason.
 *
 * ## What is asserted here, and how
 *
 * Two kinds of assertion, because the defect has two halves:
 *
 *   1. **Structural.** The workflow is parsed with a YAML parser (not with
 *      regexes over the text) and every `actions/checkout` step in it is
 *      located by job, and each one is checked against the ref it is
 *      REQUIRED to carry. A checkout that is not pinned to the tag is
 *      reported by name, so "a `ref:` exists somewhere in the file" cannot
 *      satisfy anything here.
 *
 *   2. **Behavioural.** The tag resolution is EXECUTED, in each of the event
 *      shapes that matter, and what it produces is asserted. The resolution
 *      lives in `apps/gm-desktop/scripts/resolve-release-tag.sh` rather than
 *      in a `run:` block, because a `run:` block is text that only a
 *      workflow ever reads: nothing can execute it, so nothing can prove
 *      that it resolves the tag rather than merely appearing to.
 *
 * The concurrency group is the one thing that cannot be executed here — it
 * is evaluated by GitHub's expression engine at workflow level, before any
 * job has run. It is therefore EVALUATED here, by a small evaluator for the
 * subset of the expression language it uses, against the two event contexts
 * that matter. The evaluator throws on anything it does not understand, so a
 * group written in a form this file cannot read is a red test and not a
 * silently-passing one.
 *
 * [UNVERIFIED-FACT] The evaluator implements the `&&`, `||`, `==` and
 * grouping semantics as GitHub documents them. It cannot prove that GitHub's
 * engine agrees with it, because GitHub's engine cannot be run from here.
 * What this file proves is that the key names the TAG in both event shapes,
 * given the documented semantics.
 */
import { describe, expect, it } from 'vitest';
import { spawnSync } from 'node:child_process';
import { existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { parse } from 'yaml';

const REPO_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..', '..', '..', '..');
const RELEASE = join(REPO_ROOT, '.github', 'workflows', 'release.yml');

/** The script that resolves the tag, relative to the repository root. */
const RESOLVER_REL = 'apps/gm-desktop/scripts/resolve-release-tag.sh';
const RESOLVER = join(REPO_ROOT, ...RESOLVER_REL.split('/'));

/** The expression that pins a checkout to the tag the preflight resolved. */
const TAG_OUTPUT = '${{ needs.verify-tag.outputs.tag }}';
/** The same expression, spelled from inside the `verify-tag` job itself. */
const TAG_STEP_OUTPUT = '${{ steps.tag.outputs.tag }}';

/**
 * The one checkout in this workflow that is NOT pinned to the tag: the one
 * that fetches the resolver script, because the tag is the thing being
 * resolved and therefore cannot be the source of the resolver.
 *
 * It names a COMMIT on both paths — the head commit of the dispatched
 * branch, or the tag itself — so no checkout in this file resolves to a
 * branch.
 */
const RESOLVER_FETCH_REF =
  "${{ github.event_name == 'workflow_dispatch' && github.sha || github.ref }}";

interface WfStep {
  name?: string;
  id?: string;
  uses?: string;
  run?: string;
  env?: Record<string, string>;
  with?: Record<string, unknown>;
}

interface WfJob {
  needs?: string | string[];
  outputs?: Record<string, string>;
  steps?: WfStep[];
}

interface WfDoc {
  concurrency?: { group?: string; 'cancel-in-progress'?: boolean };
  jobs: Record<string, WfJob>;
}

const DOC = parse(readFileSync(RELEASE, 'utf8')) as WfDoc;

/** Every `actions/checkout` step in the workflow, with the job it belongs to. */
const checkouts = Object.entries(DOC.jobs).flatMap(([jobId, job]) =>
  (job.steps ?? [])
    .map((step, index) => ({ jobId, index, step }))
    .filter((c) => typeof c.step.uses === 'string' && c.step.uses.startsWith('actions/checkout'))
);

const verifyTagSteps = DOC.jobs['verify-tag']?.steps ?? [];

/** The step that resolves the tag. */
const resolveStep = verifyTagSteps.find((s) => s.id === 'tag');

/** The step that publishes the version. */
const checkStep = verifyTagSteps.find((s) => s.id === 'check');

/** The step that creates the release. */
const releaseStep = (DOC.jobs['draft-release']?.steps ?? []).find((s) =>
  typeof s.uses === 'string' && s.uses.startsWith('softprops/action-gh-release')
);

/**
 * The `run:` bodies of the whole workflow, with comments removed.
 *
 * Same reason as every other gate in this suite that reads a shell body: a
 * comment can say anything, and a check that a comment can satisfy is worse
 * than no check, because it reads green.
 */
const runBodies = Object.values(DOC.jobs)
  .flatMap((job) => job.steps ?? [])
  .map((step) => step.run ?? '')
  .map((body) =>
    body
      .split('\n')
      .map((line) => line.replace(/(^|\s)#.*$/, ''))
      .join('\n')
  )
  .filter((body) => body.trim() !== '');

// ---------------------------------------------------------------------------
// Structural: every checkout is pinned to the tag
// ---------------------------------------------------------------------------

describe('every checkout in the release workflow is pinned to a tag', () => {
  it('found the checkouts it is about to judge', () => {
    // Guards the guard. A path typo or a filter that matches nothing makes
    // every assertion below pass vacuously, and the suite still reports
    // green.
    expect(checkouts.length, 'no actions/checkout step found in release.yml').toBeGreaterThan(0);
    expect(
      Object.keys(DOC.jobs)
        .filter((id) =>
          (DOC.jobs[id].steps ?? []).some((s) =>
            typeof s.uses === 'string' && s.uses.startsWith('actions/checkout')
          )
        )
        .sort(),
      'the set of jobs that check something out changed. A new job that checks out the ' +
        'triggering ref would build the wrong tree, so the list is asserted rather than ' +
        'discovered.'
    ).toEqual(['appimage', 'deb', 'dmg', 'draft-release', 'msi', 'verify-tag']);
  });

  it('no checkout falls back to the ref that triggered the run', () => {
    // With no `ref:`, `actions/checkout` resolves the triggering ref: the
    // tag on a tag push, and the BRANCH on a dispatch. That single omission
    // is the whole defect, so a missing `ref:` is reported by job.
    const loose = checkouts
      .filter((c) => {
        const ref = c.step.with?.['ref'];
        return typeof ref !== 'string' || ref.trim() === '';
      })
      .map((c) => `${c.jobId} (step #${c.index})`);

    expect(
      loose,
      'these checkouts carry no `ref:`, so they resolve the ref that triggered the run — ' +
        'which on a workflow_dispatch is a branch, not the tag.'
    ).toEqual([]);
  });

  it('the five build jobs check out the tag the preflight resolved', () => {
    const buildJobs = ['dmg', 'deb', 'appimage', 'msi', 'draft-release'];
    const wrong = buildJobs.flatMap((jobId) => {
      const steps = DOC.jobs[jobId]?.steps ?? [];
      const found = steps.filter((s) => typeof s.uses === 'string' && s.uses.startsWith('actions/checkout'));
      if (found.length !== 1) {
        return [`${jobId} has ${found.length} checkouts, expected exactly 1`];
      }
      const ref = found[0].with?.['ref'];
      return ref === TAG_OUTPUT ? [] : [`${jobId} checks out '${String(ref)}', expected '${TAG_OUTPUT}'`];
    });

    expect(
      wrong,
      'a build job that checks out anything other than the resolved tag builds a tree that ' +
        'is not the tag the release is published under.'
    ).toEqual([]);
  });

  it('verify-tag resolves the tag BEFORE it checks the tag out', () => {
    // The order is the fix. The checkout cannot name the tag until the
    // resolver has published it, and the resolver cannot publish it until it
    // has run, so the checkout has to come second.
    const resolveAt = verifyTagSteps.findIndex((s) => s.id === 'tag');
    const checkoutAt = verifyTagSteps.findIndex(
      (s) => s.with?.['ref'] === TAG_STEP_OUTPUT
    );

    expect(resolveAt, 'verify-tag has no step with `id: tag`').toBeGreaterThanOrEqual(0);
    expect(
      checkoutAt,
      `verify-tag has no checkout pinned to '${TAG_STEP_OUTPUT}'`
    ).toBeGreaterThan(-1);
    expect(
      checkoutAt,
      'the checkout of the tag comes before the step that resolves the tag, so it cannot ' +
        'know which tag to check out'
    ).toBeGreaterThan(resolveAt);
  });

  it('the version is compared against the TAG\'s tauri.conf.json', () => {
    // The comparison reads a file out of the checked-out tree. If the
    // checkout happened after it, the number it compares was read from the
    // branch while the name it is compared with came from the tag.
    const checkoutAt = verifyTagSteps.findIndex((s) => s.with?.['ref'] === TAG_STEP_OUTPUT);
    const compareAt = verifyTagSteps.findIndex((s) => s.id === 'check');

    expect(compareAt, 'verify-tag has no step with `id: check`').toBeGreaterThan(-1);
    expect(
      compareAt,
      'the version is compared before the tag is checked out, so the version comes from the ' +
        'wrong tree'
    ).toBeGreaterThan(checkoutAt);
  });

  it('exactly one checkout is not pinned to the tag, and it only fetches the resolver', () => {
    // The exception is real and it is one checkout. It exists because the
    // resolver is a file in the repository and the repository is not on disk
    // until something checks it out — the tag cannot be the source of the
    // thing that resolves the tag. It is pinned to a COMMIT, so no checkout
    // in this file resolves to a branch.
    const notPinnedToTag = checkouts.filter(
      (c) => c.step.with?.['ref'] !== TAG_OUTPUT && c.step.with?.['ref'] !== TAG_STEP_OUTPUT
    );

    expect(
      notPinnedToTag.map((c) => `${c.jobId} (step #${c.index})`),
      'a checkout that is not pinned to the tag builds whatever the triggering ref was'
    ).toEqual(['verify-tag (step #0)']);

    expect(
      notPinnedToTag[0]?.step.with?.['ref'],
      'the resolver fetch must name a commit, never a branch'
    ).toBe(RESOLVER_FETCH_REF);

    // And it is the FIRST step, so the resolver is on disk before anything
    // tries to run it.
    expect(notPinnedToTag[0]?.index, 'the resolver fetch is not the first step').toBe(0);
  });

  it('the preflight publishes the tag as a job output', () => {
    // Every job below reads `needs.verify-tag.outputs.tag`. A preflight
    // that stopped publishing it would leave every checkout unpinned, and
    // the builds would go green again.
    expect(
      DOC.jobs['verify-tag']?.outputs?.['tag'],
      'verify-tag publishes no `tag` output, so the build jobs have nothing to check out'
    ).toBe(TAG_STEP_OUTPUT);
    expect(
      DOC.jobs['verify-tag']?.outputs?.['version'],
      'verify-tag stopped publishing `version`'
    ).toBe('${{ steps.check.outputs.version }}');
  });
});

// ---------------------------------------------------------------------------
// Structural: the release metadata names the tag
// ---------------------------------------------------------------------------

describe('the release names the tag, not the branch', () => {
  it('found the release-creating step', () => {
    expect(releaseStep, 'no softprops/action-gh-release step in draft-release').toBeDefined();
  });

  it('the release is tagged with the resolved tag', () => {
    // The old value was
    //   needs.verify-tag.outputs.version && format('v{0}', ...) || github.ref_name
    // whose fallback is the BRANCH on a dispatch.
    expect(String(releaseStep?.with?.['tag_name'] ?? '')).toBe(TAG_OUTPUT);
  });

  it('the body names the resolved tag', () => {
    const body = String(releaseStep?.with?.['body'] ?? '');
    expect(body.length, 'the release step declares no body').toBeGreaterThan(0);
    expect(
      body,
      'the body must name the tag the artifacts were built from'
    ).toContain(TAG_OUTPUT);
    expect(
      body,
      'the body names `github.ref_name`, which is the BRANCH on a dispatch: the release ' +
        'would advertise a branch while being tagged with a version'
    ).not.toContain('github.ref_name');
  });

  it('no release metadata falls back to the triggering ref', () => {
    const metadata = JSON.stringify(releaseStep?.with ?? {});
    expect(
      metadata,
      'the release metadata mentions `github.ref`, which is the triggering ref: the branch ' +
        'on a dispatch'
    ).not.toMatch(/github\.ref(_name)?\b/);
  });
});

// ---------------------------------------------------------------------------
// The concurrency group, evaluated
// ---------------------------------------------------------------------------

type ExprValue = string | boolean;

/**
 * The context the concurrency group's expression is evaluated against.
 *
 * Typed as a plain record so that an expression naming a key this file does
 * not know about is a thrown error rather than a silently-empty value.
 */
type ExprContext = Record<string, string>;

type Token =
  | { kind: 'ident'; text: string }
  | { kind: 'string'; text: string }
  | { kind: 'op'; text: string };

/**
 * Tokenise the subset of the expression language the group uses: dotted
 * identifiers, single-quoted strings, `==`, `!=`, `&&`, `||`, `!` and
 * parentheses.
 *
 * Anything else throws. A gate that silently skipped what it did not
 * understand would read green on a group it never evaluated.
 */
function tokenize(source: string): Token[] {
  const tokens: Token[] = [];
  let i = 0;

  while (i < source.length) {
    const ch = source[i];

    if (/\s/.test(ch)) {
      i += 1;
      continue;
    }

    if (ch === "'") {
      let j = i + 1;
      let text = '';
      let closed = false;
      while (j < source.length) {
        if (source[j] === "'") {
          if (source[j + 1] === "'") {
            text += "'";
            j += 2;
            continue;
          }
          closed = true;
          break;
        }
        text += source[j];
        j += 1;
      }
      if (!closed) throw new Error(`unterminated string in expression: ${source}`);
      tokens.push({ kind: 'string', text });
      i = j + 1;
      continue;
    }

    if (ch === '(' || ch === ')') {
      tokens.push({ kind: 'op', text: ch });
      i += 1;
      continue;
    }

    if (source.startsWith('==', i) || source.startsWith('!=', i)) {
      tokens.push({ kind: 'op', text: source.slice(i, i + 2) });
      i += 2;
      continue;
    }

    if (source.startsWith('&&', i) || source.startsWith('||', i)) {
      tokens.push({ kind: 'op', text: source.slice(i, i + 2) });
      i += 2;
      continue;
    }

    if (/[A-Za-z0-9_.-]/.test(ch)) {
      let j = i;
      while (j < source.length && /[A-Za-z0-9_.-]/.test(source[j])) j += 1;
      tokens.push({ kind: 'ident', text: source.slice(i, j) });
      i = j;
      continue;
    }

    throw new Error(`unsupported character '${ch}' in expression: ${source}`);
  }

  return tokens;
}

/** GitHub's truthiness: an empty string is false, a non-empty string is true. */
function isTruthy(value: ExprValue): boolean {
  if (typeof value === 'boolean') return value;
  return value !== '';
}

/**
 * Evaluate a GitHub expression.
 *
 * The semantics implemented here are the documented ones: `&&` and `||`
 * return an OPERAND rather than a boolean, `==` compares strings
 * case-insensitively, and parentheses group.
 */
function evaluateExpression(source: string, ctx: ExprContext): ExprValue {
  const tokens = tokenize(source);
  let pos = 0;

  const peek = (): Token | undefined => tokens[pos];

  function parseOr(): ExprValue {
    let left = parseAnd();
    while (peek()?.kind === 'op' && peek()?.text === '||') {
      pos += 1;
      const right = parseAnd();
      if (!isTruthy(left)) left = right;
    }
    return left;
  }

  function parseAnd(): ExprValue {
    let left = parseComparison();
    while (peek()?.kind === 'op' && peek()?.text === '&&') {
      pos += 1;
      const right = parseComparison();
      if (isTruthy(left)) left = right;
    }
    return left;
  }

  function parseComparison(): ExprValue {
    const left = parseUnary();
    const token = peek();
    if (token?.kind === 'op' && (token.text === '==' || token.text === '!=')) {
      pos += 1;
      const right = parseUnary();
      const equal = String(left).toLowerCase() === String(right).toLowerCase();
      return token.text === '==' ? equal : !equal;
    }
    return left;
  }

  function parseUnary(): ExprValue {
    const token = peek();
    if (token?.kind === 'op' && token.text === '!') {
      pos += 1;
      return !isTruthy(parseUnary());
    }
    return parsePrimary();
  }

  function parsePrimary(): ExprValue {
    const token = tokens[pos];
    pos += 1;
    if (!token) throw new Error(`expression ended unexpectedly: ${source}`);

    if (token.kind === 'string') return token.text;

    if (token.kind === 'op' && token.text === '(') {
      const value = parseOr();
      const close = tokens[pos];
      pos += 1;
      if (!close || close.kind !== 'op' || close.text !== ')') {
        throw new Error(`expected ')' in expression: ${source}`);
      }
      return value;
    }

    if (token.kind === 'ident') {
      if (token.text === 'true') return true;
      if (token.text === 'false') return false;
      if (token.text === 'null') return '';
      const value = ctx[token.text];
      if (value === undefined) {
        throw new Error(
          `expression reads '${token.text}', which this evaluator does not know. ` +
            `Add it to the context rather than letting the gate pass on an expression it ` +
            `could not read.`
        );
      }
      return value;
    }

    throw new Error(`unexpected token in expression: ${JSON.stringify(token)}`);
  }

  const result = parseOr();
  if (pos !== tokens.length) {
    throw new Error(`expression has trailing tokens: ${source}`);
  }
  return result;
}

/** The literal prefix and the expression inside a group's `${{ }}`. */
function splitGroup(group: string): { prefix: string; expression: string } | null {
  const match = /^(.*?)\$\{\{([^}]*)\}\}$/.exec(group);
  if (!match) return null;
  return { prefix: match[1], expression: match[2].trim() };
}

/** The context of a manual run: the operator names the tag. */
const DISPATCH_CONTEXT: ExprContext = {
  'github.event_name': 'workflow_dispatch',
  'github.ref': 'refs/heads/dev',
  'github.ref_name': 'dev',
  'inputs.tag': 'v1.2.3',
};

/** The context of a tag push: the ref IS the tag. */
const PUSH_CONTEXT: ExprContext = {
  'github.event_name': 'push',
  'github.ref': 'refs/tags/v1.2.3',
  'github.ref_name': 'v1.2.3',
  'inputs.tag': '',
};

describe('the concurrency group names the tag', () => {
  const group = DOC.concurrency?.group ?? '';
  const split = splitGroup(group);

  it('the group is an expression, not a literal', () => {
    expect(
      split,
      `the concurrency group is '${group}'. It has to be an expression, because the tag is ` +
        `not known until the preflight has run — and the group is evaluated before that.`
    ).not.toBeNull();
  });

  it('the evaluator computes what GitHub documents', () => {
    // The evaluator is asserted against itself, because a broken evaluator
    // makes every assertion below vacuously true.
    expect(evaluateExpression("'a' || 'b'", {})).toBe('a');
    expect(evaluateExpression("'' || 'b'", {})).toBe('b');
    expect(evaluateExpression("'a' && 'b'", {})).toBe('b');
    expect(evaluateExpression("'' && 'b'", {})).toBe('');
    expect(evaluateExpression("'A' == 'a'", {})).toBe(true);
    expect(evaluateExpression("'A' != 'a'", {})).toBe(false);
    expect(evaluateExpression("('x' || 'y') || 'z'", {})).toBe('x');
    expect(evaluateExpression("('' || 'y') || 'z'", {})).toBe('y');
    expect(
      () => evaluateExpression('github.nothing', {}),
      'an unknown identifier must throw rather than evaluate to nothing'
    ).toThrow();
  });

  it('a manual run and a tag push of the same tag land in the same group', () => {
    // Otherwise they run against the same tag at the same time.
    const key = (ctx: ExprContext): string =>
      split!.prefix + String(evaluateExpression(split!.expression, ctx));

    expect(key(DISPATCH_CONTEXT)).toBe(key(PUSH_CONTEXT));
    expect(key(DISPATCH_CONTEXT)).toBe('release-v1.2.3');
    expect(key(PUSH_CONTEXT)).toBe('release-v1.2.3');
  });

  it('two different tags do not serialise against each other', () => {
    const key = (ctx: ExprContext): string =>
      split!.prefix + String(evaluateExpression(split!.expression, ctx));

    const other: ExprContext = { ...DISPATCH_CONTEXT, 'inputs.tag': 'v2.0.0' };
    expect(key(other)).toBe('release-v2.0.0');
    expect(key(other)).not.toBe(key(DISPATCH_CONTEXT));
  });

  it('a manual run never names the branch', () => {
    const key = (ctx: ExprContext): string =>
      split!.prefix + String(evaluateExpression(split!.expression, ctx));

    // The empty-input case is the one that bites: an empty string is FALSY
    // in a GitHub expression, so a group written as
    //   event == 'workflow_dispatch' && inputs.tag || github.ref_name
    // falls through to the BRANCH when the operator sends an empty tag.
    const empty: ExprContext = { ...DISPATCH_CONTEXT, 'inputs.tag': '' };
    expect(
      key(empty),
      'the group names the branch when the dispatch carried no tag. A run that fails must ' +
        'not also say the wrong thing about which tag it was for.'
    ).not.toContain('dev');
  });
});

// ---------------------------------------------------------------------------
// Behavioural: the resolver, executed
// ---------------------------------------------------------------------------

/**
 * The bash this machine runs.
 *
 * Same lookup as `workflow-step-isolation.test.ts`: on Windows the `bash`
 * on PATH is usually WSL, which cannot see this repository.
 */
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

interface ResolverOutcome {
  status: number | null;
  output: string;
  outputs: string;
  summary: string;
}

/** Run the resolver with the given environment and report what it did. */
function runResolver(env: Record<string, string>): ResolverOutcome {
  const dir = mkdtempSync(join(tmpdir(), 'release-resolve-tag-'));
  const outputFile = join(dir, 'github-output');
  const summaryFile = join(dir, 'github-summary');
  writeFileSync(outputFile, '');
  writeFileSync(summaryFile, '');

  try {
    const res = spawnSync(BASH, [RESOLVER], {
      cwd: REPO_ROOT,
      encoding: 'utf8',
      env: {
        ...process.env,
        ...env,
        GITHUB_OUTPUT: outputFile,
        GITHUB_STEP_SUMMARY: summaryFile,
      },
    });

    return {
      status: res.status,
      output: `${res.stdout ?? ''}${res.stderr ?? ''}`,
      outputs: readFileSync(outputFile, 'utf8'),
      summary: readFileSync(summaryFile, 'utf8'),
    };
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
}

describe('the resolver resolves the tag it is given', () => {
  it('the script exists where the workflow calls it', () => {
    expect(existsSync(RESOLVER), `missing ${RESOLVER}`).toBe(true);
    expect(readFileSync(RESOLVER, 'utf8').length, 'the resolver is empty').toBeGreaterThan(0);
  });

  it('is a shell script bash can parse', () => {
    const res = spawnSync(BASH, ['-n', RESOLVER], { encoding: 'utf8' });
    expect(
      res.status,
      `bash -n rejected ${RESOLVER}:\n${res.stdout ?? ''}${res.stderr ?? ''}`
    ).toBe(0);
  });

  it('is LF, because a CRLF shebang does not run on a Linux runner', () => {
    expect(
      readFileSync(RESOLVER, 'utf8').includes('\r\n'),
      'the resolver has CRLF line endings; a bash script run on a Linux runner must be LF'
    ).toBe(false);
  });

  it('a tag push resolves to the tag in the ref', () => {
    const res = runResolver({ EVENT_NAME: 'push', GITHUB_REF: 'refs/tags/v1.2.3' });

    expect(res.status, res.output).toBe(0);
    expect(res.outputs, 'the resolver published no `tag=` output').toContain('tag=v1.2.3');
    expect(res.output.trim().split('\n').pop()).toBe('v1.2.3');
    expect(res.summary).toContain('building v1.2.3');
  });

  it('a manual run resolves to the tag in the input, not the branch in the ref', () => {
    const res = runResolver({
      EVENT_NAME: 'workflow_dispatch',
      INPUT_TAG: 'v1.2.3',
      GITHUB_REF: 'refs/heads/dev',
    });

    expect(res.status, res.output).toBe(0);
    expect(res.outputs).toContain('tag=v1.2.3');
    expect(res.outputs, 'the resolver published the branch').not.toContain('dev');
  });

  it('a manual run with a tag that is not v-prefixed fails loudly', () => {
    const res = runResolver({
      EVENT_NAME: 'workflow_dispatch',
      INPUT_TAG: '1.2.3',
      GITHUB_REF: 'refs/heads/dev',
    });

    expect(res.status, 'the resolver accepted a tag that is not v-prefixed').toBe(1);
    expect(res.output, 'the failure did not say what was wrong').toContain('::error::');
    expect(res.output).toContain('not a v-prefixed tag');
    expect(res.output, 'the failure did not name the value it refused').toContain('1.2.3');
    expect(res.outputs, 'a refused tag must not be published').not.toContain('tag=');
  });

  it('a manual run with an empty tag fails loudly, and does not fall back to the branch', () => {
    const res = runResolver({
      EVENT_NAME: 'workflow_dispatch',
      INPUT_TAG: '',
      GITHUB_REF: 'refs/heads/dev',
    });

    expect(res.status, 'an empty tag was accepted').toBe(1);
    expect(res.output, 'the failure did not say what was wrong').toContain('::error::');
    expect(res.outputs, 'an empty tag must not be published').not.toContain('tag=');
    expect(
      res.output,
      'the failure names the branch it refused to fall back to, which means the branch ' +
        'reached the resolver as an answer'
    ).not.toContain('refs/heads/dev');
  });

  it('a push that is not a tag push fails loudly', () => {
    const res = runResolver({ EVENT_NAME: 'push', GITHUB_REF: 'refs/heads/dev' });

    expect(res.status, 'a branch push was accepted as a release').toBe(1);
    expect(res.output).toContain('::error::');
    expect(res.outputs).not.toContain('tag=');
  });

  it('an event it does not know fails loudly', () => {
    const res = runResolver({ EVENT_NAME: 'schedule', GITHUB_REF: 'refs/tags/v1.2.3' });

    expect(res.status, 'an unknown event was accepted').toBe(1);
    expect(res.output).toContain('::error::');
    expect(res.outputs).not.toContain('tag=');
  });
});

// ---------------------------------------------------------------------------
// The workflow calls the script, and does not re-implement it
// ---------------------------------------------------------------------------

describe('the workflow calls the resolver rather than re-implementing it', () => {
  it('found the step that resolves the tag', () => {
    expect(resolveStep, 'verify-tag has no step with `id: tag`').toBeDefined();
  });

  it('the step runs the script in the repository', () => {
    expect(
      resolveStep?.run ?? '',
      'the Resolve the tag step does not run the resolver script'
    ).toContain(RESOLVER_REL);
  });

  it('the script the workflow runs is the script this file runs', () => {
    // The path in the `run:` body, resolved against the repository root,
    // must be the very file this test executed. A copy under another name
    // would be a second definition, and the test would be proving the
    // wrong one.
    const called = (resolveStep?.run ?? '').match(/[\w./-]*resolve-release-tag\.sh/)?.[0];
    expect(called, 'the Resolve the tag step names no resolver script').toBeDefined();
    expect(called).toBe(RESOLVER_REL);
    expect(existsSync(join(REPO_ROOT, called as string))).toBe(true);
  });

  it('the resolution is not duplicated in the workflow', () => {
    // A second copy of the logic is a second definition that nothing
    // executes, and the copy is the one a future edit would change.
    const inlined = runBodies.filter(
      (body) => body.includes('refs/tags/') || /\bv\*\)/.test(body)
    );

    expect(
      inlined,
      'the tag resolution is written out in a `run:` body. The script is the single ' +
        'definition; a copy here is a copy nothing executes.'
    ).toEqual([]);
  });

  it('the resolver step reads the event and the input through the environment', () => {
    // `inputs.tag` is operator-supplied text. Interpolated into a command
    // it would be a word to be split on rather than a tag to be checked.
    const env = resolveStep?.env ?? {};
    expect(Object.keys(env).sort()).toEqual(['EVENT_NAME', 'INPUT_TAG']);
    expect(String(env['EVENT_NAME'])).toContain('github.event_name');
    expect(String(env['INPUT_TAG'])).toContain('inputs.tag');
    expect(
      resolveStep?.run ?? '',
      'the operator-supplied tag is pasted into the command instead of being passed ' +
        'through the environment'
    ).not.toContain('${{');
  });

  it('the compare step reads the resolved tag through the environment', () => {
    // The 2026-10-06 failure: a `run:` block is its own process, so a step
    // that read `$ref` instead of receiving it through `env:` died on
    // `ref: unbound variable` with no `::error::` line at all.
    expect(checkStep, 'verify-tag has no step with `id: check`').toBeDefined();
    expect(String(checkStep?.env?.['REF'] ?? '')).toBe(TAG_STEP_OUTPUT);
  });
});

/*
 * Mutation evidence, run against the ACTUAL `release.yml` and the ACTUAL
 * resolver script — one property at a time, with the file restored between
 * runs (`git checkout --`). Each mutation reverts one property of the fix;
 * each one has to turn this file red, and it has to turn it red for the
 * RIGHT reason — the assertion that names the property, not some unrelated
 * one that happens to be nearby.
 *
 * A. Removed the `ref:` from verify-tag's TAG checkout.
 *    -> 3 failed / 28 passed.
 *       "these checkouts carry no `ref:`, so they resolve the ref that
 *        triggered the run ...: expected [ 'verify-tag (step #2)' ] to deeply
 *        equal []"
 *       "verify-tag has no checkout pinned to '${{ steps.tag.outputs.tag }}':
 *        expected -1 to be greater than -1"
 *       "a checkout that is not pinned to the tag builds whatever the
 *        triggering ref was: expected [ 'verify-tag (step #0)', ...(1) ] to
 *        deeply equal [ 'verify-tag (step #0)' ]"
 *
 * B. Removed the `ref:` from the resolver-fetch checkout.
 *    -> 2 failed / 29 passed.
 *       "...: expected [ 'verify-tag (step #0)' ] to deeply equal []"
 *       "the resolver fetch must name a commit, never a branch: expected
 *        undefined to be '${{ github.event_name == 'workflow_dispatch' &&
 *        github.sha || github.ref }}'"
 *
 * C. Removed the `ref:` from the `dmg` job's checkout.
 *    -> 3 failed / 28 passed.
 *       "dmg checks out 'undefined', expected '${{
 *        needs.verify-tag.outputs.tag }}'"
 *       (plus the two whole-file checks above)
 *
 * D. Restored `github.ref_name` in the release body.
 *    -> 2 failed / 29 passed.
 *       "the body must name the tag the artifacts were built from: expected
 *        'Automated draft for `${{ github.ref_n...' to contain '${{
 *        needs.verify-tag.outputs.tag }}'"
 *       "the release metadata mentions `github.ref`, which is the triggering
 *        ref: the branch on a dispatch"
 *
 * E. Restored the old concurrency key `release-${{ github.ref }}`.
 *    -> 3 failed / 28 passed.
 *       "expected 'release-refs/heads/dev' to be 'release-refs/tags/v1.2.3'"
 *       "expected 'release-refs/heads/dev' to be 'release-v2.0.0'"
 *       "the group names the branch when the dispatch carried no tag ...
 *        expected 'release-refs/heads/dev' not to contain 'dev'"
 *
 * F. Made the resolver return the branch name instead of the tag on dispatch.
 *    -> 2 failed / 29 passed.
 *       "a manual run resolves to the tag in the input, not the branch in the
 *        ref: expected 1 to be +0" — the resolver exited 1 with
 *        "::error::'dev' is not a v-prefixed tag"
 *       "the failure did not name the value it refused: expected
 *        '::error::\'dev\' is not a v-prefixed ...' to contain '1.2.3'"
 *
 * G. Restored the old `tag_name` expression.
 *    -> 2 failed / 29 passed.
 *       "expected '${{ needs.verify-tag.outputs.version …' to be '${{
 *        needs.verify-tag.outputs.tag }}'"
 *       (plus the whole-metadata check)
 *
 * H. Removed the `tag` job output from verify-tag.
 *    -> 1 failed / 30 passed.
 *       "verify-tag publishes no `tag` output, so the build jobs have nothing
 *        to check out: expected undefined to be '${{ steps.tag.outputs.tag }}'"
 *
 * Baseline with the fix in place: 31 passed / 31.
 */
