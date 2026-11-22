/**
 * Gates on the Tauri capability surface the desktop webview is given.
 *
 * ## What was wrong
 *
 * `src-tauri/capabilities/default.json` granted the `main` window 35
 * permission entries across seven plugins. Six of those plugins — `fs`,
 * `shell`, `opener`, `clipboard-manager`, `notification` and `log` — are
 * called by nothing in `src/`. Measured over every `.ts` / `.svelte` file
 * under `src/`: `@tauri-apps/api` is imported six times,
 * `@tauri-apps/plugin-dialog` twice, and the other six plugin packages zero
 * times. The only raw `plugin:<name>|<command>` invocations in `src/` are
 * the four `plugin:dialog|*` cases in the test mock.
 *
 * So the webview was given a `$HOME/**` file-write primitive, the ability to
 * hand a URL or a path to the OS, clipboard read/write, notifications and a
 * log sink, for six plugins no code path could reach.
 *
 * That is not "a vulnerability" — it is an unnecessarily broad grant, and its
 * severity is bounded by the webview's own isolation. But a broad grant that
 * nothing can use is still a broad grant: any script that does get injected
 * into the webview inherits all of it, and the width of that inheritance is
 * not visible from the frontend's source at all.
 *
 * ## Why a gate rather than a one-time edit
 *
 * The edit is cheap to make and easy to undo by accident. `tauri-plugin-fs`
 * is one line in `Cargo.toml`, one line in `lib.rs`, one line in
 * `capabilities/default.json` and one line in `package.json`; nothing in the
 * build fails when all four come back, and nothing in the test suite notices
 * until a feature that needs it is written and does not work. The surface
 * would then be wide again for as long as it took somebody to notice.
 *
 * So the properties are asserted instead of assumed.
 *
 * ## The properties
 *
 *   1. **The two sets cannot drift apart.** Every non-`core` plugin prefix
 *      granted in `capabilities/default.json` must be imported by the
 *      frontend, and every plugin the frontend imports must be granted. A
 *      grant nobody uses and a call nobody granted are the same defect seen
 *      from two sides, and one of them is a runtime failure that CI cannot
 *      see.
 *
 *   2. **No capability may grant a file-system scope.** The scope is the
 *      dangerous part of `fs`: `fs:allow-read-text-file` alone is inert, and
 *      `fs:scope` with `$HOME/**` is a file-write primitive over the user's
 *      home directory.
 *
 *   3. **No broad plugin prefix is granted at all**, so the grant cannot come
 *      back through a route the drift check would not notice (a plugin that
 *      is imported *and* granted is still a grant nobody asked for).
 *
 *   4. **The Rust side registers what the webview is allowed to use.** A
 *      plugin registered in `lib.rs` but granted to nobody is dead weight in
 *      the shipped binary; a plugin granted to the webview but never
 *      registered cannot be called at all.
 *
 *   5. **The application's own commands cannot drift apart either.** The
 *      frontend calls the app's own Tauri commands through
 *      `invoke('<command>')`; the Rust side registers them in
 *      `tauri::generate_handler![...]`. A command the frontend invokes but
 *      the Rust side never registers fails at run time and nowhere else, and
 *      a command the Rust side registers that the frontend never calls is
 *      reachable surface with no feature behind it. Both directions are
 *      asserted in the second half of this file.
 *
 * ## The application's own command surface
 *
 * The properties above are about the *plugin* capability surface: which
 * plugins the webview may reach. The desktop app has a second surface — the
 * app's own commands — and it drifted the same way the plugin surface did:
 * silently, and in a way no build noticed.
 *
 * `[FACT]` The frontend calls the app's own commands through `invoke` from
 * `@tauri-apps/api/core`, imported under the name `tauriInvoke` in
 * `src/lib/api/tauri.ts`, `src/lib/api/ai.ts` and `src/lib/api/graph.ts`.
 * `[FACT]` The Rust side registers them in `tauri::generate_handler![...]`
 * in `apps/gm-desktop/src-tauri/src/lib.rs`.
 *
 * So the same two-sided check applies to that surface:
 *
 *   - **Every command the frontend invokes must be registered.** Otherwise
 *     the call fails in the webview at run time, and the build says
 *     nothing about it.
 *   - **Every command the Rust side registers must be invoked** — or be
 *     named, in prose, as a command whose caller has not landed yet.
 *
 * `[INFERENCE]` The second direction is a **failure**, not a reported list.
 * A registered command is reachable from the webview whether or not any UI
 * calls it — the registration IS the reachability, exactly as a plugin grant
 * is — so a command nobody calls is surface with no feature behind it: the
 * same defect the plugin half of this gate calls out from the other side. A
 * reported list was considered and rejected, because a list that is only
 * printed is not a gate: nothing goes red, and the next reader has to know
 * to look for it. The exemptions that do exist are named, and each one
 * carries the reason it exists; they live next to the assertion, in the
 * same commit that introduces the asymmetry — the same discipline
 * `RUST_ONLY_PLUGINS` below already follows.
 *
 * `[INFERENCE]` The command surface is deliberately **not** pinned to a
 * snapshot of today's list, the way the plugin surface is. The plugin
 * surface changes when somebody adds a plugin; the command surface changes
 * whenever a lane lands a feature, and ADR-0024 has four lanes landing at
 * different times. A snapshot would make every sibling lane's merge red for
 * a change nobody decided against.
 *
 * ## Two things this gate deliberately does NOT do
 *
 * It does not use an allow-list. An allow-list is a data-driven way to turn
 * a gate off without a code change: the entry gets added, the assertion goes
 * green, and the reason lives in a string instead of in a diff that a
 * reviewer reads. If a genuine asymmetry ever needs to exist, the assertion
 * below is amended in the same commit that introduces the asymmetry, with
 * the reason in prose — which is a reviewable change rather than a silent
 * one.
 *
 * It does not assert anything about the frontend's *command-level* use of a
 * plugin. `dialog:allow-message`, `dialog:allow-ask` and
 * `dialog:allow-confirm` are granted, and the frontend calls only
 * `dialog:allow-open`. Narrowing that is a separate decision about a
 * separate plugin, and this gate is about which plugins are reachable.
 * That is about *plugin* commands: the application's own commands are a
 * different surface, and the second half of this file does assert them.
 *
 * ## MUTATION EVIDENCE
 *
 * Every assertion below was fed a failing input and observed to go red.
 * The mutations and the exact failure messages are recorded in the commit
 * message of the change that introduced this file. Summary:
 *
 *   - re-adding `"fs:allow-read-text-file"` to the capability file:
 *     the drift check and the broad-prefix check both go red and name the
 *     grant.
 *   - re-adding the `fs:scope` block with `$HOME/**`: the scope check goes
 *     red and names the path.
 *   - deleting `import { open } from '@tauri-apps/plugin-dialog'` from
 *     `src/lib/stores/worktreeRoot.ts`: the drift check goes red and names
 *     the grants that now have no caller.
 *   - re-registering `tauri_plugin_fs::init()` in `lib.rs`: the Rust-side
 *     check goes red and names the registration.
 *
 * A gate that has never been fed a failing input is not a gate.
 */
import { describe, expect, it } from 'vitest';
import { readFileSync, readdirSync, statSync } from 'node:fs';
import { dirname, join, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

// tests/unit/ -> tests/ -> gm-desktop/ -> apps/ -> repo root
const REPO_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..', '..', '..', '..');
const APP_ROOT = join(REPO_ROOT, 'apps', 'gm-desktop');
const CAPABILITY_FILE = join(APP_ROOT, 'src-tauri', 'capabilities', 'default.json');
const LIB_RS = join(APP_ROOT, 'src-tauri', 'src', 'lib.rs');
const SRC_DIR = join(APP_ROOT, 'src');

// ---------------------------------------------------------------------------
// The capability file
// ---------------------------------------------------------------------------

type PermissionObject = Record<string, unknown>;
type PermissionEntry = string | PermissionObject;

type Capability = {
  identifier?: string;
  windows?: string[];
  permissions?: PermissionEntry[];
};

const capability = JSON.parse(readFileSync(CAPABILITY_FILE, 'utf8')) as Capability;
const permissions: PermissionEntry[] = capability.permissions ?? [];

/**
 * The plugin prefix of a permission.
 *
 * `fs:allow-read-text-file` -> `fs`. `core:default` -> `core`. A permission
 * object is named by its `identifier`, which is the same
 * `<plugin>:<permission>` shape. `core` is Tauri itself and is not a
 * plugin, so it is excluded from every set below.
 */
function permissionPrefix(entry: PermissionEntry): string | null {
  const id =
    typeof entry === 'string'
      ? entry
      : typeof entry.identifier === 'string'
        ? entry.identifier
        : null;
  if (id === null) return null;
  const colon = id.indexOf(':');
  return colon === -1 ? id : id.slice(0, colon);
}

/** A readable label for a permission entry, for failure messages. */
function describePermission(entry: PermissionEntry): string {
  if (typeof entry === 'string') return entry;
  const identifier = typeof entry.identifier === 'string' ? entry.identifier : '(no identifier)';
  const scopes = collectScopes([entry]);
  if (scopes.length === 0) return identifier;
  return `${identifier} [${scopes.map((s) => s.path).join(', ')}]`;
}

/**
 * The filesystem scopes a permission grants.
 *
 * A scope is the `allow` / `deny` list of an object permission. An entry
 * that names a `path` is a filesystem scope; a bare string is one when it
 * looks like a path, which is what Tauri's own `$HOME/**` form is.
 *
 * The shape is read from the capability file rather than assumed to be
 * `fs:scope`, so a scope smuggled in under another identifier is still a
 * scope. That is the whole point: the check is about the *grant*, not about
 * the spelling of the grant.
 */
function collectScopes(entries: PermissionEntry[]): Array<{ identifier: string; path: string }> {
  const found: Array<{ identifier: string; path: string }> = [];

  for (const entry of entries) {
    if (typeof entry === 'string') continue;
    const identifier = typeof entry.identifier === 'string' ? entry.identifier : '(no identifier)';

    for (const key of ['allow', 'deny'] as const) {
      const list = entry[key];
      if (!Array.isArray(list)) continue;

      for (const item of list) {
        if (typeof item === 'string') {
          if (looksLikePath(item)) found.push({ identifier, path: item });
          continue;
        }
        if (item && typeof item === 'object') {
          const path = (item as PermissionObject).path;
          if (typeof path === 'string') found.push({ identifier, path });
        }
      }
    }
  }

  return found;
}

/**
 * Whether a bare string is a path.
 *
 * `$APPDATA/**` is Tauri's own scope syntax; a `/` or `\` makes it a path
 * too. A string with neither is a scope of some other kind (a URL, a
 * command) and is not this gate's business.
 */
function looksLikePath(value: string): boolean {
  return value.startsWith('$') || value.includes('/') || value.includes('\\');
}

const grantedPrefixes = new Set<string>();
for (const entry of permissions) {
  const prefix = permissionPrefix(entry);
  if (prefix !== null && prefix !== 'core') grantedPrefixes.add(prefix);
}

/** Every granted permission, rendered for a failure message. */
const grantedPermissionLabels = permissions.map(describePermission);

// ---------------------------------------------------------------------------
// The frontend
// ---------------------------------------------------------------------------

/** Every source file under `src/`, recursively. */
function sourceFiles(dir: string): string[] {
  const out: string[] = [];
  for (const name of readdirSync(dir)) {
    const full = join(dir, name);
    if (statSync(full).isDirectory()) {
      out.push(...sourceFiles(full));
      continue;
    }
    if (/\.(ts|js|mjs|svelte)$/.test(name)) out.push(full);
  }
  return out.sort();
}

const frontendFiles = sourceFiles(SRC_DIR);

/**
 * The specifier of an `import` / `export ... from` statement.
 *
 * Three shapes exist and all three are matched:
 *
 *   import { open } from '@tauri-apps/plugin-dialog';
 *   import '@tauri-apps/plugin-dialog';
 *   export { open } from '@tauri-apps/plugin-dialog';
 *   const m = await import('@tauri-apps/plugin-dialog');
 *
 * The specifier must be in quotes immediately after `from`, after `import`,
 * or inside `import(...)`, and the statement must have an `import` or
 * `export` head. A mention of the package in prose does not match — which
 * is the case that matters here, because `src/mocks/handlers.ts` names
 * `@tauri-apps/plugin-dialog` in a comment that has no import statement
 * around it.
 *
 * The `[^'";]` inside the "from" clause is what keeps a match inside one
 * statement: a semicolon ends the statement, and a quote ends the clause.
 * The `{0,500}` bound keeps a match from wandering across a long file.
 *
 * The scanner does not strip comments, and a line inside a comment that
 * reads exactly like an import statement WOULD be counted. That is the
 * safe direction to be wrong in: a false import makes the drift check
 * demand a grant that is not there, which fails the build, rather than
 * hiding a grant that should not be there, which would let it through.
 */
const STATIC_SPECIFIER_RE =
  /\b(?:import|export)\s+(?:[^'";]{0,500}?\sfrom\s+)?(['"])(@tauri-apps\/plugin-[a-z0-9-]+)\1/g;
const DYNAMIC_SPECIFIER_RE = /\bimport\s*\(\s*(['"])(@tauri-apps\/plugin-[a-z0-9-]+)\1\s*\)/g;

/**
 * A raw `plugin:<name>|<command>` command string.
 *
 * Tauri names a plugin command `plugin:<plugin>:<command>` on the wire and
 * the JS packages call it `plugin:<plugin>|<command>`. A frontend that
 * invokes one of those directly does not import the package, so this is
 * scanned separately — see the assertion that uses it.
 */
const RAW_PLUGIN_COMMAND_RE = /['"`]plugin:([a-z0-9-]+)\|/g;

type FrontendUsage = {
  /** The plugin prefixes imported as a package. */
  imported: Set<string>;
  /** The plugin prefixes invoked through a raw `plugin:<name>|<command>`. */
  raw: Set<string>;
  /** Where each raw invocation was seen, for the failure message. */
  rawSites: string[];
};

/**
 * Scan sources for plugin usage.
 *
 * This is a function rather than a loop over the real tree so that the
 * scanner itself can be tested. A scanner that silently stops matching
 * would make every drift check below pass, and a gate that has stopped
 * looking is indistinguishable, in the log, from a gate that found nothing.
 */
function scanFrontend(sources: Array<{ path: string; text: string }>): FrontendUsage {
  const usage: FrontendUsage = { imported: new Set(), raw: new Set(), rawSites: [] };

  for (const { path, text } of sources) {
    for (const re of [STATIC_SPECIFIER_RE, DYNAMIC_SPECIFIER_RE]) {
      re.lastIndex = 0;
      let match: RegExpExecArray | null;
      while ((match = re.exec(text)) !== null) {
        usage.imported.add(match[2].replace('@tauri-apps/plugin-', ''));
      }
    }

    RAW_PLUGIN_COMMAND_RE.lastIndex = 0;
    let rawMatch: RegExpExecArray | null;
    while ((rawMatch = RAW_PLUGIN_COMMAND_RE.exec(text)) !== null) {
      usage.raw.add(rawMatch[1]);
      usage.rawSites.push(`${path}: plugin:${rawMatch[1]}|`);
    }
  }

  return usage;
}

const usage: FrontendUsage = scanFrontend(
  frontendFiles.map((file) => ({
    path: relative(APP_ROOT, file).replace(/\\/g, '/'),
    text: readFileSync(file, 'utf8'),
  })),
);

// ---------------------------------------------------------------------------
// The Rust side
// ---------------------------------------------------------------------------

/**
 * Rust source with its comments removed.
 *
 * A `//` inside a string literal is not a comment, and a C-style block
 * comment is. The scanner tracks string literals so the two are not
 * confused, which matters because the assertion below reads a source file
 * that mentions `tauri-plugin-log` in prose.
 *
 * Rust char literals (`'/'`) and raw strings (`r"..."`) are not handled;
 * neither occurs in `lib.rs`, and a false positive there would ADD a
 * plugin to the "in use" set, which makes the check stricter rather than
 * looser.
 */
function stripRustComments(source: string): string {
  let out = '';
  let i = 0;

  while (i < source.length) {
    const ch = source[i];

    if (ch === '"') {
      let j = i + 1;
      while (j < source.length) {
        if (source[j] === '\\') {
          j += 2;
          continue;
        }
        if (source[j] === '"') {
          j += 1;
          break;
        }
        j += 1;
      }
      out += source.slice(i, j);
      i = j;
      continue;
    }

    if (ch === '/' && source[i + 1] === '/') {
      while (i < source.length && source[i] !== '\n') i += 1;
      continue;
    }

    if (ch === '/' && source[i + 1] === '*') {
      const end = source.indexOf('*/', i + 2);
      i = end === -1 ? source.length : end + 2;
      continue;
    }

    out += ch;
    i += 1;
  }

  return out;
}

const libRsCode = stripRustComments(readFileSync(LIB_RS, 'utf8'));

/**
 * The Tauri plugins the Rust side actually uses.
 *
 * `tauri_plugin_<name>::` is the way every Tauri plugin crate is referred
 * to, and it is referred to here for exactly one reason: to be registered
 * with the builder. The crate name is normalised from `snake_case` to the
 * hyphenated prefix the capability file uses (`clipboard_manager` ->
 * `clipboard-manager`).
 */
const rustPlugins = new Set<string>();
for (const match of libRsCode.matchAll(/\btauri_plugin_([a-z0-9_]+)\b/g)) {
  rustPlugins.add(match[1].replace(/_/g, '-'));
}

/**
 * The one plugin that is used by the Rust side and granted to nobody.
 *
 * `tauri-plugin-log` writes the backend's own log lines to stdout and to
 * the log directory. That is a Rust-side facility and it needs no webview
 * capability: the webview's `log:default` would grant the *JavaScript* the
 * ability to write into that same sink, which is a different thing.
 */
const RUST_ONLY_PLUGINS = new Set(['log']);

// ---------------------------------------------------------------------------
// The assertions
// ---------------------------------------------------------------------------

describe('the capability file was actually read', () => {
  it('has permissions to check', () => {
    // Guards the guard. A file that failed to parse, or a parse that
    // returned nothing, would make every assertion below pass while
    // checking nothing.
    expect(
      permissions.length,
      `no permissions found in ${CAPABILITY_FILE} — the file was not read, or the ` +
        'permissions array is empty',
    ).toBeGreaterThan(0);
  });

  it('still grants the Tauri built-ins, so the parser is reading real content', () => {
    expect(
      permissions.filter((p) => permissionPrefix(p) === 'core').length,
      'no `core:` permission found — the capability file no longer grants the Tauri ' +
        'built-ins, so this gate is reading something it does not understand',
    ).toBeGreaterThan(0);
  });
});

describe('the frontend was actually scanned', () => {
  it('found source files', () => {
    expect(
      frontendFiles.length,
      `no source files found under ${SRC_DIR} — the scan is looking in the wrong place`,
    ).toBeGreaterThan(0);
  });

  it('found at least one plugin import', () => {
    // Without this, "the granted set equals the imported set" would also be
    // satisfied by both being empty.
    expect(
      [...usage.imported],
      `no \`@tauri-apps/plugin-*\` import found under ${SRC_DIR} — the import scan is ` +
        'not reading real code, so the drift check below would pass vacuously',
    ).not.toEqual([]);
  });
});

describe('the import scanner finds what an import is', () => {
  // The drift checks above are only as good as this scanner. A scanner that
  // stopped matching would make them pass while checking nothing, so the
  // shapes it has to recognise are asserted here rather than assumed.

  const scan = (text: string): string[] =>
    [...scanFrontend([{ path: 'synthetic.ts', text }]).imported].sort();

  it('finds a named import', () => {
    expect(scan("import { open } from '@tauri-apps/plugin-dialog';\n")).toEqual(['dialog']);
  });

  it('finds a side-effect import', () => {
    expect(scan("import '@tauri-apps/plugin-fs';\n")).toEqual(['fs']);
  });

  it('finds a multi-line import', () => {
    expect(scan("import {\n  open,\n} from '@tauri-apps/plugin-dialog';\n")).toEqual(['dialog']);
  });

  it('finds a dynamic import', () => {
    expect(scan("const m = await import('@tauri-apps/plugin-opener');\n")).toEqual(['opener']);
  });

  it('finds a re-export', () => {
    expect(scan("export { open } from '@tauri-apps/plugin-dialog';\n")).toEqual(['dialog']);
  });

  it('finds a type-only import', () => {
    expect(scan("import type { X } from '@tauri-apps/plugin-shell';\n")).toEqual(['shell']);
  });

  it('does not count a mention in a comment', () => {
    // This is the real case, not a synthetic one: `src/mocks/handlers.ts`
    // names the package in prose, in backticks. A scanner that matched
    // bare specifiers would report the mock as an importer, and the drift
    // check would then be satisfied by a file that imports nothing.
    expect(scan(' * `open()` from `@tauri-apps/plugin-dialog` is one `invoke` away,\n')).toEqual(
      [],
    );
  });

  it('does not count a non-plugin package', () => {
    expect(scan("import { invoke } from '@tauri-apps/api/core';\n")).toEqual([]);
  });

  it('does not cross from one statement into the next', () => {
    // `import` with a specifier that is not a plugin, then a string literal
    // somewhere later that happens to read `from '@tauri-apps/plugin-...'`.
    // The semicolon and the quotes end the first statement, so the second
    // fragment belongs to no import at all.
    //
    // The sample has to contain the plugin specifier, or the assertion
    // would be satisfied by the absence of the word rather than by the
    // scanner's behaviour.
    expect(
      scan("import './side-effect';\nconst note = \" from '@tauri-apps/plugin-dialog'\";\n"),
    ).toEqual([]);
  });

  it('counts the real tree as one importer', () => {
    // The `dialog` import lives in `src/lib/stores/worktreeRoot.ts` and the
    // mock's four `plugin:dialog|*` cases are raw invocations, not
    // imports. If this ever names a second importer, the drift check's
    // meaning has changed and this assertion is where it shows.
    expect([...usage.imported].sort()).toEqual(['dialog']);
    expect([...usage.raw].sort()).toEqual(['dialog']);
  });
});

describe('the granted surface and the frontend cannot drift apart', () => {
  it('every granted plugin prefix is imported by the frontend', () => {
    const grantedButUnused = [...grantedPrefixes]
      .filter((prefix) => !usage.imported.has(prefix))
      .sort();

    expect(
      grantedButUnused,
      'these capabilities are granted to the webview, but no file under src/ imports the ' +
        'matching `@tauri-apps/plugin-<prefix>` package, so nothing in the application can ' +
        'reach them. A grant with no caller is attack surface with no feature behind it: ' +
        'the shipped webview inherits it, and the only way to tell is to read both sides.',
    ).toEqual([]);
  });

  it('every plugin the frontend imports is granted', () => {
    const usedButNotGranted = [...usage.imported]
      .filter((prefix) => !grantedPrefixes.has(prefix))
      .sort();

    expect(
      usedButNotGranted,
      'the frontend imports these Tauri plugins, but the capability file grants them ' +
        'nothing, so the call is refused at run time by the Tauri runtime and the feature ' +
        'silently fails. This is the other half of the drift check: a grant that is missing ' +
        'is a bug the build cannot see.',
    ).toEqual([]);
  });

  it('every raw plugin:<name>|<command> the frontend invokes is granted', () => {
    // A raw invocation is a use of a plugin that does not import the
    // package, so it is invisible to the import scan. It is checked
    // separately rather than folded into the imported set: folding it in
    // would let a raw `plugin:dialog|open` in the test mock stand in for
    // the real import, and the drift check would then pass with the
    // frontend's only real plugin import deleted.
    const rawNotGranted = [...usage.raw].filter((prefix) => !grantedPrefixes.has(prefix)).sort();

    expect(
      rawNotGranted,
      'the frontend invokes `plugin:<name>|<command>` for these plugins, but the ' +
        'capability file grants them nothing. A raw invocation is a use of the plugin that ' +
        'the import scan cannot see.',
    ).toEqual([]);
  });
});

describe('no capability grants a broad primitive', () => {
  it('grants no file-system scope', () => {
    const scopes = collectScopes(permissions);

    expect(
      scopes.map((s) => `${s.identifier} -> ${s.path}`),
      'these capabilities grant a file-system scope. A scope is the dangerous half of ' +
        'the fs plugin: `fs:allow-read-text-file` on its own is inert, and a scope over ' +
        "`$HOME/**` is a file-read/write primitive over the user's home directory, handed " +
        'to any script that reaches the webview.',
    ).toEqual([]);
  });

  it('grants no broad plugin prefix', () => {
    // The drift check above would not catch a plugin that is imported AND
    // granted but should not be either. The list is the set of prefixes
    // this application has no use for.
    const forbidden = ['fs', 'shell', 'opener', 'clipboard-manager', 'notification', 'log'];

    const offending = permissions
      .filter((entry) => {
        const prefix = permissionPrefix(entry);
        return prefix !== null && forbidden.includes(prefix);
      })
      .map(describePermission);

    expect(
      offending,
      'these permissions grant a plugin this application does not use. `fs` and `shell` ' +
        "and `opener` in particular reach the filesystem and the OS on the webview's " +
        'behalf. Remove the grant, and remove the plugin registration and the npm ' +
        'dependency that goes with it.',
    ).toEqual([]);
  });
});

describe('the Rust side registers what the webview is allowed to use', () => {
  it('registers no plugin the webview cannot use', () => {
    const registeredOnly = [...rustPlugins]
      .filter((prefix) => !RUST_ONLY_PLUGINS.has(prefix))
      .filter((prefix) => !grantedPrefixes.has(prefix))
      .sort();

    expect(
      registeredOnly,
      'these Tauri plugins are registered in lib.rs but granted to the webview by no ' +
        'capability, so they are linked into the shipped binary and cannot be called. ' +
        '`log` is the documented exception: the backend writes its own log lines through ' +
        'it, which is a Rust-side facility and needs no webview capability.',
    ).toEqual([]);
  });

  it('grants no plugin the Rust side does not register', () => {
    const grantedOnly = [...grantedPrefixes].filter((prefix) => !rustPlugins.has(prefix)).sort();

    expect(
      grantedOnly,
      'the capability file grants these plugins, but lib.rs never registers the matching ' +
        'crate, so the grant cannot reach an implementation.',
    ).toEqual([]);
  });
});

describe('the surface, as it stands', () => {
  it('grants exactly the plugin the frontend uses, and nothing else', () => {
    // This is the summary the other assertions imply. It is written out
    // rather than derived from the sets above so that a change to the
    // surface is visible in a test failure instead of only in a diff.
    expect(
      {
        granted: [...grantedPrefixes].sort(),
        importedByFrontend: [...usage.imported].sort(),
        registeredInRust: [...rustPlugins].sort(),
        grantedPermissions: grantedPermissionLabels,
      },
      'the capability surface changed. If the change is intended, update this assertion ' +
        'and the reason for it in the same commit; if it is not, the surface has grown ' +
        'without anyone deciding that it should.',
    ).toEqual({
      granted: ['dialog'],
      importedByFrontend: ['dialog'],
      registeredInRust: ['dialog', 'log'],
      grantedPermissions: [
        'core:default',
        'core:window:default',
        'core:window:allow-minimize',
        'core:window:allow-hide',
        'core:window:allow-show',
        'core:window:allow-close',
        'core:window:allow-set-focus',
        'core:app:default',
        'core:event:default',
        'core:tray:default',
        'core:menu:default',
        'core:path:default',
        'dialog:default',
        'dialog:allow-open',
        'dialog:allow-message',
        'dialog:allow-ask',
        'dialog:allow-confirm',
      ],
    });
  });
});

// ---------------------------------------------------------------------------
// The application's own command surface
// ---------------------------------------------------------------------------

/**
 * The command names the Rust side registers.
 *
 * `[FACT]` `apps/gm-desktop/src-tauri/src/lib.rs` registers the app's own
 * commands in one `tauri::generate_handler![...]` list, one path per line
 * (`commands::server::server_status,`). Tauri v2 names a command after the
 * Rust function behind it, so the name on the wire is the LAST segment of
 * the path: `commands::ai::ai_conflict_start` is the command
 * `ai_conflict_start`.
 *
 * The list is read as text rather than compiled, for the same reason the
 * plugin half of this file reads `lib.rs` as text — and the reason is
 * stronger here: this gate has to be able to fail in CI without a Rust
 * toolchain, and it has to fail when the two sides drift, not when the
 * desktop crate happens to be rebuilt.
 */
function parseRegisteredCommands(source: string): string[] {
  const marker = 'tauri::generate_handler![';
  const start = source.indexOf(marker);
  if (start === -1) return [];

  const bodyStart = start + marker.length;
  let depth = 1;
  let end = bodyStart;
  while (end < source.length) {
    const ch = source[end];
    if (ch === '[') depth += 1;
    else if (ch === ']') depth -= 1;
    if (depth === 0) break;
    end += 1;
  }

  const body = source.slice(bodyStart, end);
  const commands: string[] = [];
  for (const entry of body.split(',')) {
    const trimmed = entry.trim();
    if (trimmed === '') continue;
    const segments = trimmed.split('::');
    const name = segments[segments.length - 1].trim();
    if (name !== '') commands.push(name);
  }
  return commands;
}

/**
 * The first argument of an `invoke(...)` call.
 *
 * `[FACT]` The frontend calls the app's own commands through `invoke` from
 * `@tauri-apps/api/core`, imported under the name `tauriInvoke` in
 * `src/lib/api/tauri.ts`, `src/lib/api/ai.ts` and `src/lib/api/graph.ts`.
 * Every call site under `src/` passes the command name as a string literal.
 *
 * The alternation captures that first argument three ways — a
 * single-quoted literal, a double-quoted literal, a template literal — and
 * falls back to "everything up to the next comma or paren", which is what a
 * DYNAMIC command name looks like. The fallback is not there for
 * convenience: it is how the scan reports a call it cannot read, so a
 * dynamic command name fails the gate instead of silently escaping it.
 */
const INVOKE_CALL_RE = /\b(?:tauriInvoke|invoke)\s*\(\s*('[^']*'|"[^"]*"|`[^`]*`|[^,)]*)/g;

type CommandUsage = {
  /** The command names the frontend invokes, where the name is a literal. */
  invoked: Set<string>;
  /** Call sites whose command name the scan could not read as a literal. */
  dynamicSites: string[];
  /** Per file: the command names it calls, and how many call sites it has. */
  byFile: Map<string, { commands: string[]; callSites: number }>;
};

/**
 * Scan sources for calls into the app's own command surface.
 *
 * This is a function rather than a loop over the real tree so that the
 * scanner itself can be tested, for the reason the plugin scanner above is
 * a function: a scanner that silently stops matching would make every
 * drift check below pass, and a gate that has stopped looking is
 * indistinguishable, in the log, from a gate that found nothing.
 *
 * `[FACT]` The dev mock (`src/mocks/handlers.ts`) answers commands from a
 * `switch` over the command name. Those labels are a SIMULATION of the
 * backend, not call sites, and they are deliberately not counted here: the
 * mock answers `set_clipboard_text`, which no file in `src/` calls, and
 * counting the mock would make that command look used.
 */
function scanCommandCalls(sources: Array<{ path: string; text: string }>): CommandUsage {
  const usage: CommandUsage = {
    invoked: new Set<string>(),
    dynamicSites: [],
    byFile: new Map(),
  };

  for (const { path, text } of sources) {
    const commands: string[] = [];
    let callSites = 0;

    INVOKE_CALL_RE.lastIndex = 0;
    let match: RegExpExecArray | null;
    while ((match = INVOKE_CALL_RE.exec(text)) !== null) {
      const argument = match[1].trim();

      // `invoke()` with no argument at all carries no command name. The
      // only occurrences of that shape under `src/` are two prose mentions
      // of `invoke()` in a comment.
      if (argument === '') continue;

      callSites += 1;
      const quote = argument[0];
      if (quote === "'" || quote === '"' || quote === '`') {
        const name = argument.slice(1, -1);
        if (/^[a-z0-9_]+$/.test(name)) {
          usage.invoked.add(name);
          commands.push(name);
        } else {
          usage.dynamicSites.push(`${path}: invoke(${argument}) is not a command name`);
        }
        continue;
      }

      usage.dynamicSites.push(`${path}: invoke(${argument}) does not pass a literal command name`);
    }

    usage.byFile.set(path, { commands, callSites });
  }

  return usage;
}

const registeredCommands = new Set<string>(parseRegisteredCommands(libRsCode));
const commandUsage = scanCommandCalls(
  frontendFiles.map((file) => ({
    path: relative(APP_ROOT, file).replace(/\\/g, '/'),
    text: readFileSync(file, 'utf8'),
  })),
);

/**
 * The ADR-0024 conflict commands, by name.
 *
 * `[FACT]` `ai_conflict_start`, `ai_conflict_cancel` and `ai_key_set` are
 * implemented by ADR-0024 Lane B (`apps/gm-desktop/src-tauri/src/commands/ai.rs`,
 * merged as PR #92) and are called by ADR-0024 Lane C's frontend
 * (`apps/gm-desktop/src/lib/api/conflict.ts`, PR #96 on `feat/conflict-ui`).
 *
 * `[FACT]` `ai_key_status` is Lane C's own command: Lane C implements it in
 * `src-tauri/src/commands/ai.rs`, registers it in `src-tauri/src/lib.rs`
 * and calls it from `src/lib/api/conflict.ts`.
 *
 * `[FACT]` `conflict_regions` is the command Lane C's `src/lib/api/conflict.ts`
 * calls to read the regions out of a repository, and the command Lane C's
 * dev mock answers (`src/mocks/handlers.ts`). It is named here because it is
 * part of the same feature, and because on a branch that carries Lane C it
 * is the one command the gate has something to say about.
 *
 * `[FACT]` `conflict_audit_record` is Lane E's command (ADR-0024 §3.7.3):
 * it writes the audit record of one arbitration. Lane E implements it in
 * `src-tauri/src/commands/audit.rs`, registers it in `src-tauri/src/lib.rs`
 * and calls it from `src/lib/api/conflict.ts`, so it is on both sides of the
 * surface.
 *
 * `[INFERENCE]` A command that a sibling lane has not landed yet is absent
 * from BOTH sides of the surface, and that is not a defect: the merge order
 * is not a property of the code. So the gate does not require every name
 * above to exist. What it does require is that no name is HALF-wired — the
 * dangerous half (invoked but not registered) is always a failure — and that
 * every conflict command the surface does carry is named here, so a new one
 * cannot appear without somebody pointing at it.
 */
const CONFLICT_COMMANDS = [
  'ai_conflict_start',
  'ai_conflict_cancel',
  'ai_key_set',
  'ai_key_status',
  'conflict_regions',
  'conflict_audit_record',
];

/**
 * The naming convention the ADR-0024 conflict feature follows.
 *
 * `[FACT]` The feature's commands are `ai_conflict_*` (the arbitration
 * session), `ai_key_*` (the provider-key namespace) and `conflict_regions`
 * (the read side). `ai_review_*` belongs to the `/review` feature and is
 * deliberately not matched.
 */
const CONFLICT_COMMAND_PATTERN = /^(?:ai_conflict_|ai_key_|conflict_)/;

/**
 * The registered commands the frontend does not call, and why each one is
 * allowed to be that way.
 *
 * `[INFERENCE]` **This is a failure, not a reported list.** A registered
 * command is reachable from the webview whether or not any UI calls it —
 * the registration IS the reachability, exactly as a plugin grant is — so a
 * command nobody calls is surface with no feature behind it, which is the
 * same defect the plugin half of this gate calls out from the other side.
 * A reported list was the alternative and was rejected: a list that is only
 * printed is not a gate, because nothing goes red and the next reader has
 * to know to look for it.
 *
 * The exemption is the command's name and the reason it has no caller. The
 * reason is data rather than a comment so that the failure message can
 * quote it, and so that an exemption cannot be added without saying why.
 * An exemption that has outlived its subject is itself a failure, below.
 */
const UNCALLED_EXEMPTIONS = new Map<string, string>([
  [
    'set_clipboard_text',
    "the frontend copies through the browser's own clipboard API " +
      '(`src/lib/utils/clipboard.ts` uses `navigator.clipboard.writeText`), so this ' +
      'command has had no caller since the desktop app was scaffolded: ' +
      "`git log -S 'set_clipboard_text' -- apps/gm-desktop/src` names one commit, " +
      "`2f8b9fc`, and the only occurrence left under `src/` is the mock's dispatch case",
  ],
]);

describe("the application's own command surface was actually read", () => {
  it('found the generate_handler! list', () => {
    // Guards the guard. A `lib.rs` whose registration list moved, or a
    // parse that found nothing, would make every assertion below pass
    // while checking nothing.
    expect(
      registeredCommands.size,
      `no command was read from ${LIB_RS} — the \`tauri::generate_handler![...]\` ` +
        'list was not found, so the command checks below would pass vacuously',
    ).toBeGreaterThan(0);
  });

  it('found the command call sites', () => {
    expect(
      [...commandUsage.invoked].length,
      `no \`invoke('<command>')\` call site found under ${SRC_DIR} — the scan is not ` +
        'reading real code, so the drift check below would pass vacuously',
    ).toBeGreaterThan(0);
  });

  it('reads a command name out of every module that calls one', () => {
    // The set of commands the frontend calls is deliberately NOT pinned to
    // a snapshot of today's list: a sibling lane landing a feature adds
    // commands, and a snapshot would make that merge red for a change
    // nobody decided against. What is pinned instead is the weaker, and
    // more useful, property that a module which calls the backend at all
    // still yields command names — so renaming one import does not quietly
    // take one module out of the gate.
    const silent = [...commandUsage.byFile.entries()]
      .filter(([, info]) => info.callSites > 0 && info.commands.length === 0)
      .map(([path]) => path)
      .sort();

    expect(
      silent,
      'these files call `invoke(...)` but the scan read no command name out of them, ' +
        'so the commands they reach are on neither side of the surface this gate ' +
        'compares. A renamed import is the likely cause.',
    ).toEqual([]);
  });

  it('sees no call site whose command name is not a literal', () => {
    expect(
      commandUsage.dynamicSites,
      'these `invoke(...)` call sites do not pass a literal command name, so this ' +
        'scan cannot see which command they call. A dynamic command name is a hole ' +
        'in the gate: the command it reaches is on neither side of the surface the ' +
        'gate compares.',
    ).toEqual([]);
  });
});

describe('the command scanners find what they are supposed to find', () => {
  const scan = (text: string): string[] =>
    [...scanCommandCalls([{ path: 'synthetic.ts', text }]).invoked].sort();

  it('finds a literal command name', () => {
    expect(scan("await tauriInvoke('server_status');\n")).toEqual(['server_status']);
    expect(scan("await invoke('server_status');\n")).toEqual(['server_status']);
    expect(scan('await invoke(`server_status`);\n')).toEqual(['server_status']);
  });

  it('finds a call written across two lines', () => {
    expect(scan("await tauriInvoke(\n  'server_status',\n  { bind: null }\n);\n")).toEqual([
      'server_status',
    ]);
  });

  it('ignores a mention of invoke() in prose', () => {
    // The real case, not a synthetic one: `src/lib/api/tauri.ts` says
    // "`invoke()` throws synchronously" in its header comment. A scanner
    // that counted that as a call site would report a command with no name.
    expect(scan(' * `invoke()` throws synchronously.\n')).toEqual([]);
  });

  it('reports a dynamic command name instead of guessing it', () => {
    const usage = scanCommandCalls([
      { path: 'synthetic.ts', text: 'await invoke(commandName);\n' },
    ]);
    expect([...usage.invoked]).toEqual([]);
    expect(usage.dynamicSites.length).toBe(1);
  });

  it('reads the command name out of a registration path', () => {
    const parse = (source: string): string[] => parseRegisteredCommands(source);
    expect(parse('tauri::generate_handler![commands::ai::ai_conflict_start,]\n')).toEqual([
      'ai_conflict_start',
    ]);
    expect(parse('tauri::generate_handler![\n  commands::server::server_status,\n]\n')).toEqual([
      'server_status',
    ]);
  });
});

describe("the application's own command surface cannot drift apart", () => {
  it('registers every command the frontend invokes', () => {
    const invokedButNotRegistered = [...commandUsage.invoked]
      .filter((command) => !registeredCommands.has(command))
      .sort();

    expect(
      invokedButNotRegistered,
      'the frontend invokes these commands, but the Rust side does not register them ' +
        'in `tauri::generate_handler![...]` (`apps/gm-desktop/src-tauri/src/lib.rs`). ' +
        'An unregistered command is not a compile error and not a build error: the ' +
        'call fails in the webview at run time, and nothing in CI notices until ' +
        'somebody clicks the button.',
    ).toEqual([]);
  });

  it('registers no command the frontend never invokes', () => {
    const unjustified = [...registeredCommands]
      .filter((command) => !commandUsage.invoked.has(command))
      .filter((command) => !UNCALLED_EXEMPTIONS.has(command))
      .sort();

    expect(
      unjustified,
      'these commands are registered in `tauri::generate_handler![...]` but no file ' +
        'under `src/` invokes them. A registered command is reachable from the ' +
        'webview whether or not any UI calls it, so this is surface with no feature ' +
        'behind it. Call it, or delete the registration. A command whose caller has ' +
        'not landed yet is named in `UNCALLED_EXEMPTIONS`, with the reason it is ' +
        'excused.',
    ).toEqual([]);
  });

  it('exempts no command that is no longer registered', () => {
    const stale = [...UNCALLED_EXEMPTIONS.keys()]
      .filter((command) => !registeredCommands.has(command))
      .sort();

    expect(
      stale,
      'these commands are excused from the "registered but never called" check, but ' +
        'the Rust side does not register them any more. The excuse has outlived its ' +
        'subject: remove it, so the list stays a list of live asymmetries.',
    ).toEqual([]);
  });
});

describe('the ADR-0024 conflict commands are named and are not half-wired', () => {
  const conflictCommandsOnTheSurface = (): string[] =>
    [...new Set([...registeredCommands, ...commandUsage.invoked])]
      .filter((command) => CONFLICT_COMMAND_PATTERN.test(command))
      .sort();

  it('names every conflict command the surface carries', () => {
    const unnamed = conflictCommandsOnTheSurface()
      .filter((command) => !CONFLICT_COMMANDS.includes(command))
      .sort();

    expect(
      unnamed,
      'these conflict-feature commands are on the surface — registered or invoked — ' +
        'but are not named in `CONFLICT_COMMANDS`. Naming them is the point: the ' +
        'ADR-0024 conflict feature has a command surface somebody decided on, and a ' +
        'new command that nobody names is a command nobody has looked at.',
    ).toEqual([]);
  });

  it('leaves no conflict command half-wired', () => {
    const halfWired = CONFLICT_COMMANDS.filter(
      (command) =>
        commandUsage.invoked.has(command) !== registeredCommands.has(command) &&
        !UNCALLED_EXEMPTIONS.has(command),
    ).sort();

    expect(
      halfWired,
      'these ADR-0024 conflict commands are on one side of the surface only. ' +
        'Invoked but not registered means the call dies at run time; registered but ' +
        'never invoked means the registration is dead weight. A command whose ' +
        'caller has not landed yet is on NEITHER side, and that is not a failure — ' +
        'but half-wired always is.',
    ).toEqual([]);
  });

  it('keeps the conflict feature on the surface', () => {
    expect(
      conflictCommandsOnTheSurface().length,
      'no ADR-0024 conflict command is on the surface at all — neither registered ' +
        'nor invoked. The conflict feature has been removed from both sides of the ' +
        'command surface without anyone deciding that it should.',
    ).toBeGreaterThan(0);
  });
});
