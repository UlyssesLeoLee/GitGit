/**
 * Tests for the 404 catch-all (`src/routes/NotFound.svelte`), and for the
 * one thing that makes it reachable.
 *
 * This file was 0% covered. `NotFound.svelte` is only ever mounted by the
 * router, through the `'*'` entry in `App.svelte`'s route table — and
 * `App.svelte` currently cannot mount at all (see `routes-app.test.ts` for
 * the measured reason). The page is therefore rendered here through the
 * real `Router` component with the real catch-all entry, which is the
 * contract that matters: *every* unregistered path lands on it.
 *
 * `matchRoute`'s own fallback arithmetic is already covered by
 * `router.test.ts`; what is pinned here is that the router hands the
 * catch-all a component that renders, and that what the user is offered
 * next is a way out.
 */
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { cleanup, render, screen, waitFor } from '@testing-library/svelte';
import Router from '../../src/lib/components/Router.svelte';
import ErrorBoundary from '../../src/lib/components/ErrorBoundary.svelte';
import NotFound from '../../src/routes/NotFound.svelte';
import { cat, gotoPath, resetRouteStores } from './routes-harness';
import type { RouteMap } from '../../src/lib/router';

/**
 * A route table shaped like the real one: two registered patterns and the
 * catch-all. `ErrorBoundary` stands in for a registered page because it
 * mounts synchronously, has a stable test id, and is itself covered by
 * `components-chrome.test.ts` — the fixture needs a *distinguishable*
 * component, not a second thing to test.
 */
const ROUTES: RouteMap = {
  '/': { component: ErrorBoundary },
  '/repos/:name': { component: ErrorBoundary },
  '*': { component: NotFound },
};

beforeEach(() => {
  resetRouteStores();
});

afterEach(() => {
  cleanup();
});

describe('route / 404 — the page itself', () => {
  it('states the status code and the catalogue’s own empty string', () => {
    render(NotFound);
    const card = screen.getByTestId('not-found');
    // "404" is the one string on this page that is not a translation —
    // it is a status code.
    expect(card.querySelector('h1')?.textContent?.trim()).toBe('404');
    // The rest comes from the catalogue, so it follows the locale rather
    // than being an English sentence baked into the template.
    expect(card.textContent).toContain(cat('common.empty'));
    expect(card.textContent).not.toContain('common.empty');
  });

  it('offers exactly one way out, and it is a hash link', () => {
    // A 404 with no way back is a dead end. And the link has to carry
    // the `#`, or it is a full page load that throws the app state away.
    render(NotFound);
    const links = Array.from(screen.getByTestId('not-found').querySelectorAll('a'));
    expect(links).toHaveLength(1);
    const home = links[0] as HTMLAnchorElement;
    expect(home.getAttribute('href')).toBe('#/');
    expect(home.textContent?.trim()).toBe(cat('nav.dashboard'));
  });
});

describe('route / 404 — reaching it through the router', () => {
  it('is what the router mounts for an unregistered path', async () => {
    // The catch-all is the whole point of the component: an unknown path
    // must render it rather than an empty page.
    await gotoPath('#/no-such-page');
    render(Router, { props: { routes: ROUTES } });

    expect(await screen.findByTestId('not-found')).toBeTruthy();
  });

  it('is what the router mounts for a path with too many segments', async () => {
    // `matchPath` rejects a segment-count mismatch, so a deep link past
    // every registered pattern falls through to the catch-all rather
    // than to nothing.
    await gotoPath('#/repos/demo/extra/segments');
    render(Router, { props: { routes: ROUTES } });

    await waitFor(() => expect(screen.getByTestId('not-found')).toBeTruthy());
  });

  it('follows a hash change out of the 404 and back into it', async () => {
    await gotoPath('#/no-such-page');
    render(Router, { props: { routes: ROUTES } });
    await screen.findByTestId('not-found');

    // Following the link's own target must clear the 404 — otherwise the
    // "go back to the dashboard" affordance does nothing.
    await gotoPath('#/');
    await waitFor(() => expect(screen.queryByTestId('not-found')).toBeNull());
    expect(screen.getByTestId('error-boundary')).toBeTruthy();

    await gotoPath('#/still-nope');
    await waitFor(() => expect(screen.getByTestId('not-found')).toBeTruthy());
  });
});
