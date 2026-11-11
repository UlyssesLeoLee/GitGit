/**
 * Minimal hash router for the desktop shell.
 *
 * Replaces `svelte-spa-router@4.0.2`, which cannot be used here: that
 * library imports `afterUpdate`, and Svelte 5's runes mode forbids it, so
 * `vite build` failed outright. It also drove navigation through the URL
 * hash while the app emitted plain `<a href="/repos">` anchors, so the two
 * contracts did not agree.
 *
 * This module keeps the same external surface the app already used —
 * a `location` store and a `routes` map keyed by path pattern — so the
 * swap is contained to the router component plus the anchor call sites.
 */
import { readable } from 'svelte/store';
import type { Component } from 'svelte';

export type RouteParams = Record<string, string>;

export interface RouteDef {
  component: Component;
}
export type RouteMap = Record<string, RouteDef>;

export interface RouteMatch {
  component: Component;
  params: RouteParams;
}

/** Read the current in-app path out of the URL hash. */
function readPath(): string {
  if (typeof window === 'undefined') return '/';
  const raw = window.location.hash;
  if (!raw || raw === '#' || raw === '#/') return '/';
  const path = raw.startsWith('#') ? raw.slice(1) : raw;
  return path.startsWith('/') ? path : `/${path}`;
}

/**
 * Same as the internal reader, exported so the unit tests can exercise the
 * hash-parsing edge cases without mutating global router state.
 */
export const readPathForTest = readPath;

/**
 * Current path, e.g. `/repos/alpha`. Emits immediately on subscribe and
 * again on every `hashchange`.
 */
export const location = readable(readPath(), (set) => {
  if (typeof window === 'undefined') return;
  const update = (): void => set(readPath());
  window.addEventListener('hashchange', update);
  update();
  return () => window.removeEventListener('hashchange', update);
});

/**
 * Anchor href for a route path. The `#` prefix is what the router listens
 * to; without it the browser would do a real page load.
 */
export function href(path: string): string {
  return path.startsWith('#') ? path : `#${path}`;
}

/** Programmatic navigation. */
export function navigate(path: string): void {
  if (typeof window === 'undefined') return;
  window.location.hash = href(path);
}

/**
 * Match one path against one pattern.
 * Returns the extracted params on a hit, or `null` on a miss.
 * Segments starting with `:` capture; everything else must match exactly.
 */
export function matchPath(pattern: string, path: string): RouteParams | null {
  const patternParts = pattern.split('/').filter(Boolean);
  const pathParts = path.split('/').filter(Boolean);
  if (patternParts.length !== pathParts.length) return null;

  const params: RouteParams = {};
  for (let i = 0; i < patternParts.length; i++) {
    const segment = patternParts[i] as string;
    const value = pathParts[i] as string;
    if (segment.startsWith(':')) {
      params[segment.slice(1)] = decodeURIComponent(value);
    } else if (segment !== value) {
      return null;
    }
  }
  return params;
}

/**
 * Resolve a path against a route map. Exact/param patterns win in
 * declaration order; `*` is the fallback.
 */
export function matchRoute(routes: RouteMap, path: string): RouteMatch | null {
  for (const [pattern, def] of Object.entries(routes)) {
    if (pattern === '*') continue;
    const params = matchPath(pattern, path);
    if (params) return { component: def.component, params };
  }
  const fallback = routes['*'];
  return fallback ? { component: fallback.component, params: {} } : null;
}
