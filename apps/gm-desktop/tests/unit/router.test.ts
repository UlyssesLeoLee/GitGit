/**
 * Tests for the local hash router in `src/lib/router.ts`.
 *
 * This module replaced `svelte-spa-router@4.0.2`, which could not be used
 * under Svelte 5 runes mode. The five pre-existing test files cover
 * debounce / format / graph-parser / i18n / service only — none of them
 * touched routing — so the swap had no regression net until now.
 */
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { href, location, matchPath, matchRoute, navigate, readPathForTest } from '../../src/lib/router';
import type { RouteMap } from '../../src/lib/router';

function setHash(hash: string): void {
  window.location.hash = hash;
}

const routes: RouteMap = {
  '/': { component: 'Home' as never },
  '/repos': { component: 'Repos' as never },
  '/repos/:name': { component: 'RepoDetail' as never },
  '/vault': { component: 'Vault' as never },
  '*': { component: 'NotFound' as never },
};

beforeEach(() => {
  window.location.hash = '';
});

afterEach(() => {
  window.location.hash = '';
});

describe('readPath', () => {
  it('treats an empty hash as the root path', () => {
    expect(readPathForTest()).toBe('/');
  });

  it('reads a bare "#" as the root path', () => {
    setHash('#');
    expect(readPathForTest()).toBe('/');
  });

  it('reads "#/" as the root path', () => {
    setHash('#/');
    expect(readPathForTest()).toBe('/');
  });

  it('reads a hash path', () => {
    setHash('#/repos/alpha');
    expect(readPathForTest()).toBe('/repos/alpha');
  });

  it('normalises a hash without a leading slash', () => {
    setHash('#repos');
    expect(readPathForTest()).toBe('/repos');
  });
});

describe('href', () => {
  it('prefixes a plain path with the hash marker', () => {
    expect(href('/repos')).toBe('#/repos');
  });

  it('leaves an already-hashed path untouched', () => {
    expect(href('#/repos')).toBe('#/repos');
  });
});

describe('navigate', () => {
  it('writes the hash so the router observes a hashchange', () => {
    navigate('/vault');
    expect(window.location.hash).toBe('#/vault');
  });
});

describe('matchPath', () => {
  it('matches an exact pattern', () => {
    expect(matchPath('/vault', '/vault')).toEqual({});
  });

  it('captures a single named segment', () => {
    expect(matchPath('/repos/:name', '/repos/alpha')).toEqual({ name: 'alpha' });
  });

  it('percent-decodes the captured value', () => {
    expect(matchPath('/repos/:name', '/repos/my%20repo')).toEqual({ name: 'my repo' });
  });

  it('rejects a literal mismatch', () => {
    expect(matchPath('/vault', '/settings')).toBeNull();
  });

  it('rejects a segment-count mismatch', () => {
    expect(matchPath('/repos', '/repos/alpha')).toBeNull();
  });

  it('matches the root against a single empty pattern', () => {
    expect(matchPath('/', '/')).toEqual({});
  });
});

describe('matchRoute', () => {
  it('resolves the root route', () => {
    expect(matchRoute(routes, '/')?.component).toBe('Home');
  });

  it('resolves a static route', () => {
    expect(matchRoute(routes, '/vault')?.component).toBe('Vault');
  });

  it('resolves a param route and returns its params', () => {
    const match = matchRoute(routes, '/repos/alpha');
    expect(match?.component).toBe('RepoDetail');
    expect(match?.params).toEqual({ name: 'alpha' });
  });

  it('prefers the param route over the sibling static route', () => {
    // `/repos/:name` must not be shadowed by `/repos`.
    expect(matchRoute(routes, '/repos/alpha')?.component).toBe('RepoDetail');
  });

  it('falls back to the catch-all for an unknown path', () => {
    const match = matchRoute(routes, '/nope/deep');
    expect(match?.component).toBe('NotFound');
    expect(match?.params).toEqual({});
  });

  it('returns null when there is no catch-all and nothing matches', () => {
    const noFallback: RouteMap = { '/': { component: 'Home' as never } };
    expect(matchRoute(noFallback, '/nope')).toBeNull();
  });
});

describe('location store', () => {
  it('emits the current path on subscribe', () => {
    setHash('#/vault');
    const seen: string[] = [];
    const unsub = location.subscribe((v) => seen.push(v));
    expect(seen).toEqual(['/vault']);
    unsub();
  });

  it('updates when the hash changes', async () => {
    setHash('#/');
    const seen: string[] = [];
    const unsub = location.subscribe((v) => seen.push(v));

    setHash('#/repos');
    // jsdom fires hashchange asynchronously.
    await new Promise((r) => setTimeout(r, 0));

    expect(seen[seen.length - 1]).toBe('/repos');
    unsub();
  });

  it('stops emitting after unsubscribe', async () => {
    setHash('#/');
    const seen: string[] = [];
    const unsub = location.subscribe((v) => seen.push(v));
    const countAtUnsub = seen.length;

    unsub();
    setHash('#/settings');
    await new Promise((r) => setTimeout(r, 0));

    expect(seen.length).toBe(countAtUnsub);
  });
});
