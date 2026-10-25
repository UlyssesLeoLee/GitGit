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
  }))
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
        'permissions array is empty'
    ).toBeGreaterThan(0);
  });

  it('still grants the Tauri built-ins, so the parser is reading real content', () => {
    expect(
      permissions.filter((p) => permissionPrefix(p) === 'core').length,
      'no `core:` permission found — the capability file no longer grants the Tauri ' +
        'built-ins, so this gate is reading something it does not understand'
    ).toBeGreaterThan(0);
  });
});

describe('the frontend was actually scanned', () => {
  it('found source files', () => {
    expect(
      frontendFiles.length,
      `no source files found under ${SRC_DIR} — the scan is looking in the wrong place`
    ).toBeGreaterThan(0);
  });

  it('found at least one plugin import', () => {
    // Without this, "the granted set equals the imported set" would also be
    // satisfied by both being empty.
    expect(
      [...usage.imported],
      `no \`@tauri-apps/plugin-*\` import found under ${SRC_DIR} — the import scan is ` +
        'not reading real code, so the drift check below would pass vacuously'
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
    expect(
      scan("import {\n  open,\n} from '@tauri-apps/plugin-dialog';\n")
    ).toEqual(['dialog']);
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
    expect(scan(" * `open()` from `@tauri-apps/plugin-dialog` is one `invoke` away,\n")).toEqual(
      []
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
      scan("import './side-effect';\nconst note = \" from '@tauri-apps/plugin-dialog'\";\n")
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
        'the shipped webview inherits it, and the only way to tell is to read both sides.'
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
        'is a bug the build cannot see.'
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
        'the import scan cannot see.'
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
        '`$HOME/**` is a file-read/write primitive over the user\'s home directory, handed ' +
        'to any script that reaches the webview.'
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
        'and `opener` in particular reach the filesystem and the OS on the webview\'s ' +
        'behalf. Remove the grant, and remove the plugin registration and the npm ' +
        'dependency that goes with it.'
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
        'it, which is a Rust-side facility and needs no webview capability.'
    ).toEqual([]);
  });

  it('grants no plugin the Rust side does not register', () => {
    const grantedOnly = [...grantedPrefixes].filter((prefix) => !rustPlugins.has(prefix)).sort();

    expect(
      grantedOnly,
      'the capability file grants these plugins, but lib.rs never registers the matching ' +
        'crate, so the grant cannot reach an implementation.'
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
        'without anyone deciding that it should.'
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
