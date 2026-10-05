/**
 * The React Router version floor, and the path-segment invariant that makes
 * that floor the *second* line of defence rather than the only one.
 *
 * `[FACT]` `react-router-dom` 6.30.6 carried two advisories, and they were
 * the only ones anywhere in this repository's **runtime** dependency tree —
 * everything else in `pnpm audit` output is a devDependency that never ships
 * to an installed user:
 *
 *   - open redirect through a backslash in `<Link>` / `useNavigate`
 *     (`>=6.0.0 <7.18.0`)
 *   - arbitrary constructor injection via `deserializeErrors()` during SSR
 *     hydration (`>=6.4.0 <7.18.0`)
 *
 * The second is unreachable here: this app renders through `BrowserRouter`
 * with no SSR and never calls `deserializeErrors()`. It is cleared anyway
 * because the floor is a range, not a patch list.
 *
 * `[FACT]` The upgrade required no source change. The whole public surface
 * this app uses is nine symbols across nine files, all of which kept their
 * v6 signatures in v7.
 *
 * ## Why the app is not actually exploitable at 6.x
 *
 * The `<Link to>` and `navigate()` targets that carry server-supplied data
 * all interpolate through `encodeURIComponent`:
 *
 *     `Home.tsx:47`           `/repos/${encodeURIComponent(r.name)}`
 *     `Vault.tsx:37`          `/vault/${encodeURIComponent(k.key)}`
 *     `VaultKeyDetail.tsx:63` `/vault/${encodeURIComponent(decoded)}/diff`
 *     `VaultDiff.tsx:58`      `/vault/${encodeURIComponent(decoded)}`
 *
 * `encodeURIComponent('\\evil.example')` is `%5Cevil.example` — the
 * backslash never reaches the attribute, so there is nothing left for the
 * URL parser to reinterpret. An operator who got a repository named
 * `\evil.example` into that list got a dead link, not an off-site one.
 *
 * So the honest framing is: the pin is defence in depth. The first layer is
 * the encoder, enforced by the source-scan below. The second is the
 * version floor, enforced by the first test.
 *
 * ## Measured behaviour, both versions
 *
 * `[FACT]` Rendered `href` for a raw (un-encoded) `to`, in jsdom:
 *
 *     to                     6.30.6             7.18.4
 *     "\evil.example"        "/\evil.example"   "/evil.example"
 *     "repos/\evil.example"  "/repos/\evil…"    "/repos/evil.example"
 *     "\\evil.example"       "/\\evil.example"  "\evil.example"
 *     "/\evil.example"       "/\evil.example"   "/\evil.example"   <-- unchanged
 *
 * That last row is a **negative result worth recording**: 7.18.4 does not
 * normalise a leading `/\`, so the version floor is not by itself a complete
 * fix for that input. It does not matter here — the encoder in the rows
 * above removes the backslash before the value ever reaches `to` — but an
 * earlier draft of this file asserted that 7.x normalised *every*
 * backslash, and it failed. Recorded so the next person does not re-derive it.
 *
 * A browser resolving `href="/repos/\evil.example"` applies the WHATWG rule
 * that a backslash in a path is treated as `/`, collapsing it to
 * `//evil.example` — a protocol-relative URL pointing off-site.
 */
import { readFileSync, readdirSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';
import { render, screen } from '@testing-library/react';
import { MemoryRouter, Link } from 'react-router-dom';

const APP_ROOT = process.cwd();
const pkg = JSON.parse(readFileSync(resolve(APP_ROOT, 'package.json'), 'utf-8'));

/**
 * `[FACT]` Asserts on the rendered `href`, not on a re-implementation of the
 * WHATWG parser. Re-parsing here would test this file's own idea of the
 * rule rather than the string the browser will actually see.
 */
function hrefOf(to: string): string {
  const { unmount } = render(
    <MemoryRouter>
      <Link to={to}>go</Link>
    </MemoryRouter>,
  );
  const href = screen.getByRole('link', { name: 'go' }).getAttribute('href') ?? '';
  unmount();
  return href;
}

/**
 * Repository and vault-key names the server could plausibly return. Each
 * one is a way of saying "go somewhere other than here".
 */
const HOSTILE_NAMES = [
  '\\evil.example',
  '\\\\evil.example',
  '/\\evil.example',
  '//evil.example',
  '../../evil',
  '?next=//evil.example',
];

describe('react-router version floor', () => {
  it('pins react-router-dom to a range that clears both advisories', () => {
    // Both advisories close at 7.18.0. The floor sits above that with room
    // to spare, because a bare `>=7.18.0` would silently re-admit
    // GHSA-qwww-vcr4-c8h2 (RSC-mode CSRF, `>=7.12.0 <7.18.2`) the next
    // time the range is rewritten by hand.
    const range: string = pkg.dependencies['react-router-dom'];
    const min = Number(/(\d+)\.(\d+)\.(\d+)/.exec(range)?.[1] ?? '0');
    const installed: string = JSON.parse(
      readFileSync(resolve(APP_ROOT, 'node_modules/react-router-dom/package.json'), 'utf-8'),
    ).version;

    expect(min).toBeGreaterThanOrEqual(7);
    expect(installed).toMatch(/^7\./);

    // The installed tree is what the advisory scan actually reads, so a
    // manifest bump that left the lockfile behind would clear the first
    // two assertions and ship the old build. A floor comparison rather
    // than string equality, so a later patch release does not trip it.
    const [maj, min2, patch] = installed.split('.').map(Number);
    expect(maj).toBe(7);
    expect(maj * 1e6 + min2 * 1e3 + patch).toBeGreaterThanOrEqual(7 * 1e6 + 18 * 1e3 + 4);
  });
});

describe('a server-supplied name cannot become an off-site link', () => {
  for (const name of HOSTILE_NAMES) {
    it(`stays same-origin for ${JSON.stringify(name)}`, () => {
      // The real splice, as `Home.tsx:47` performs it.
      const href = hrefOf(`/repos/${encodeURIComponent(name)}`);

      // Nothing for the URL parser to reinterpret as a protocol-relative
      // prefix, and nothing that resolves off-origin.
      expect(href).not.toContain('\\');
      expect(href).not.toMatch(/^\/\//);
      expect(href.startsWith('/repos/')).toBe(true);
    });
  }

  it('leaves ordinary names untouched', () => {
    // A fix that escaped every separator would pass the cases above.
    expect(hrefOf('/repos/alpha')).toBe('/repos/alpha');
    expect(hrefOf(`/repos/${encodeURIComponent('org/repo.git')}`)).toBe(
      '/repos/org%2Frepo.git',
    );
  });
});

/**
 * The encoder is the layer that actually holds. This scan is what stops
 * someone deleting it: a future route that interpolates a server-supplied
 * value straight into `to` is a silent open redirect, and nothing else in
 * the suite would notice.
 */
describe('every interpolated route target encodes its path segment', () => {
  const ROUTE_DIR = resolve(APP_ROOT, 'src/routes');

  // `to={...}` and `navigate(...)` both take a path; only a template
  // literal carrying an interpolation can smuggle a separator in. The
  // `\{` matters: `<Link to={`...`}>` puts a brace between `to=` and the
  // backtick, so a pattern that omits it silently scans only the
  // `navigate()` call sites and reports a clean run over a third of the
  // surface. The count assertion below is what caught that.
  const TARGET = /(?:to=\{|navigate\(\s*)`([^`]*)`/g;

  const files = readdirSync(ROUTE_DIR)
    .filter((f) => f.endsWith('.tsx'))
    .flatMap((file) => {
      const source = readFileSync(resolve(ROUTE_DIR, file), 'utf-8');
      // Comments describe the rule rather than follow it, and a commented
      // out `to={`/repos/${name}`}` would otherwise read as a violation.
      const code = source.replace(/\/\*[\s\S]*?\*\//g, '').replace(/^\s*\/\/.*$/gm, '');
      return [...code.matchAll(TARGET)].map((m) => ({ file, target: m[1] }));
    });

  it('found the route targets to check', () => {
    // A regex that matches nothing is indistinguishable from a suite
    // that found nothing wrong.
    expect(files.length).toBeGreaterThanOrEqual(8);
  });

  it.each(files)('$file: $target', ({ target }) => {
    const interpolations = [...target.matchAll(/\$\{([^}]*)\}/g)].map((m) => m[1].trim());
    if (interpolations.length === 0) return;

    // The first interpolation is the path segment; anything after it is
    // query-string material (`?base=${...}&head=${...}`), which cannot
    // become a host and is the route's business, not this file's.
    expect(interpolations[0]).toMatch(/^encodeURIComponent\(/);
  });
});
