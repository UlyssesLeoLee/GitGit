/**
 * Tests for the root component (`src/App.svelte`).
 *
 * `[FACT]` This file does not test the shell's behaviour, because the
 * shell cannot be mounted: `render(App)` throws
 * `TypeError: Cannot read properties of null (reading 'cloneNode')` before
 * a single node reaches the document. The cause is at
 * `src/App.svelte:70-72`:
 *
 *     <svelte:head>
 *       <html lang={$locale} data-theme={$theme}></html>
 *     </svelte:head>
 *
 * Svelte compiles a `<svelte:head>` block to
 * `$.from_html('<html></html>')` (measured: `svelte/compiler` 5.57.1 on
 * this file emits `var root = $.from_html(`<html></html>`)`). `from_html`
 * parses that string with `template.innerHTML = …` and then takes
 * `get_first_child(fragment)` — and an `<html>` start tag inside template
 * content is *ignored* by the HTML parser, in every conformant
 * implementation. The fragment therefore has no first child, `node`
 * becomes `null`, and `node.cloneNode(true)` in `from_html` throws.
 *
 * This is not a jsdom quirk: "in template" insertion mode ignoring an
 * `<html>` start tag is HTML-spec behaviour, and the Tauri WebView runs
 * the same parser. `readPath`-style unit tests for the route table pass
 * because they never touch the component.
 *
 * The cases below pin the two facts a reader needs — that the root
 * component throws, and why — and the consequence that matters most for
 * a failed mount: nothing is left behind in the document.
 *
 * `[FACT]` The defect is reported, not fixed: `src/**` is read-only for
 * this lane. When `App.svelte` stops putting an `<html>` element inside
 * `<svelte:head>` (moving `lang` / `data-theme` onto
 * `document.documentElement` from the existing theme subscription, which
 * already does the `data-theme` half), the first case here has to be
 * replaced by the shell tests it was standing in for — the boot spinner,
 * the pre-warm order, the route table and the boot-failure card are all
 * still untested, and `matchRoute`'s own fallback *is* covered by
 * `router.test.ts`.
 */
import { beforeEach, describe, expect, it } from 'vitest';
import { cleanup, render, screen } from '@testing-library/svelte';
import App from '../../src/App.svelte';

beforeEach(() => {
  cleanup();
});

describe('app shell — the root component cannot mount', () => {
  it('throws while mounting, and the failure names the null clone', () => {
    // Asserted as the behaviour it is today. `render` is where the
    // throw surfaces: Svelte evaluates the `<svelte:head>` block while
    // creating the component's nodes, before anything is inserted.
    expect(() => render(App)).toThrow(TypeError);
    expect(() => render(App)).toThrow(/cloneNode/);
  });

  it('leaves no half-rendered shell behind when the mount fails', () => {
    // A root component that threw half way through would otherwise
    // leave a page with no way to navigate and no error shown.
    expect(() => render(App)).toThrow();
    expect(screen.queryByTestId('app-shell')).toBeNull();
    expect(screen.queryByTestId('page-host')).toBeNull();
    expect(screen.queryByTestId('boot-spinner')).toBeNull();
    expect(document.body.querySelector('[data-testid]')).toBeNull();
  });

  it('reproduces the mechanism: an <html> element does not survive template parsing', () => {
    // The diagnosis, as an executable check. Svelte parses its head
    // block with `template.innerHTML`, and the parser drops the element,
    // so the compiled `get_first_child` returns null. If a future
    // runtime ever keeps it, this case fails and the first case above
    // needs re-checking — which is the point of having it.
    const t = document.createElement('template');
    t.innerHTML = '<html lang="en"></html>';
    expect(t.content.childNodes.length).toBe(0);

    // The control: an ordinary element in the same position survives,
    // so this is about the tag name and not about the parser or the
    // surrounding string.
    const ok = document.createElement('template');
    ok.innerHTML = '<div lang="en"></div>';
    expect(ok.content.childNodes.length).toBe(1);
  });
});
