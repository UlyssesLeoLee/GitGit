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
 * `[FACT]` This file originally recorded a defect rather than a passing
 * suite: `App.svelte` put an `<html>` element inside `<svelte:head>`,
 * which the HTML parser drops, so the root component threw
 * `TypeError: Cannot read properties of null (reading 'cloneNode')` and
 * the app could not start. `src/**` was read-only for the lane that
 * wrote this, so the crash was pinned as the expected behaviour and the
 * report asked for the fix. The fix has since landed; the cases below
 * are the regression test, and the mechanism case is kept so the trap
 * cannot be re-introduced silently.
 *
 * Still untested and worth a follow-up: the boot spinner, the pre-warm
 * ordering inside the `allSettled`, and the boot-failure card (whose
 * `bootError` branch is currently unreachable, since `initTheme` and
 * `initLocale` are no-ops and nothing in the try block can reject).
 * `matchRoute`'s own fallback *is* covered by `router.test.ts`.
 */
import { beforeEach, describe, expect, it } from 'vitest';
import { get } from 'svelte/store';
import { cleanup, render, screen, waitFor } from '@testing-library/svelte';
import App from '../../src/App.svelte';

beforeEach(() => {
  cleanup();
});

describe('app shell — the root component mounts', () => {
  // `[FACT]` These two cases previously asserted the opposite: that
  // `render(App)` throws `TypeError: Cannot read properties of null
  // (reading 'cloneNode')`. They were correct at the time and were
  // written as a defect record, which is how the bug was found at all.
  // The fix removed `<svelte:head><html lang=... data-theme=...>` from
  // `App.svelte` — the HTML parser drops a nested `<html>` start tag, so
  // that element never reached the document and the head code dereferenced
  // a null `firstChild`. These cases are now the regression test for it.

  it('mounts without throwing, and renders the shell', () => {
    render(App);
    expect(screen.getByTestId('app-shell')).toBeTruthy();
    expect(screen.getByTestId('main-panel')).toBeTruthy();
  });

  it('publishes lang and data-theme on the document element', async () => {
    // The `<html>` element in `<svelte:head>` never applied these, and
    // `lang` in particular had no other writer — it was simply never set.
    // Both are now written by their stores (`stores/locale` mirrors
    // `stores/theme`), so the attributes are asserted on the real
    // document element rather than on a component subtree.
    //
    // The expected locale comes from the store, not a literal:
    // `pickInitial()` falls back to `navigator.language`, which is
    // `en-US` under jsdom and so resolves to `en`. Hard-coding either
    // value would make this a test of the environment, not the wiring.
    const { locale, setLocale } = await import('../../src/lib/stores/locale');
    render(App);
    const html = document.documentElement;

    expect(html.getAttribute('lang')).toBe(get(locale));

    setLocale('zh-CN');
    await waitFor(() => expect(html.getAttribute('lang')).toBe('zh-CN'));
    setLocale('en');
    await waitFor(() => expect(html.getAttribute('lang')).toBe('en'));

    // `auto` is the default theme and resolves through `matchMedia`,
    // which jsdom does not provide, so the resolved value is one of the
    // two concrete palettes rather than the literal `auto`.
    expect(['light', 'dark']).toContain(html.getAttribute('data-theme'));
  });

  it('reproduces the mechanism: an <html> element does not survive template parsing', () => {
    // The diagnosis, as an executable check. Svelte parses its head
    // block with `template.innerHTML`, and the parser drops the element,
    // so the compiled `get_first_child` returns null. If a future
    // runtime ever keeps it, this case fails and the fix above needs
    // re-checking — which is the point of having it.
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
