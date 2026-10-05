/**
 * Tests for `src/lib/utils/clipboard.ts`.
 *
 * This file was 0% covered (lines 9-37). The helper has two paths and
 * the interesting one is the second: `navigator.clipboard` is
 * unavailable or refuses in exactly the environments this desktop app
 * ships into — an insecure origin, a WebView2 build without clipboard
 * permission, a Linux sandbox — and then the hidden-textarea fallback
 * is the only thing standing between the user and a dead copy button.
 *
 * Both `navigator.clipboard` and `document.execCommand` are absent
 * from jsdom, which makes the fallback path the *natural* one to test
 * here rather than a contrived one.
 */
import { afterEach, describe, expect, it, vi } from 'vitest';
import { copyText } from '../../src/lib/utils/clipboard';

/** Install an async clipboard, or one that refuses. */
function stubClipboard(writeText: (text: string) => Promise<void>) {
  const fn = vi.fn(writeText);
  Object.defineProperty(navigator, 'clipboard', { value: { writeText: fn }, configurable: true });
  return fn;
}

/** Install a clipboard object with no `writeText` on it. */
function stubPartialClipboard(): void {
  Object.defineProperty(navigator, 'clipboard', { value: {}, configurable: true });
}

/**
 * Install `document.execCommand`, which jsdom does not implement.
 * `body` is what the call returns; pass a function to make it throw.
 */
function stubExecCommand(body: boolean | (() => boolean)): ReturnType<typeof vi.fn> {
  const fn = vi.fn(typeof body === 'function' ? body : () => body);
  Object.defineProperty(document, 'execCommand', { value: fn, configurable: true, writable: true });
  return fn;
}

/**
 * Capture the scratch textarea while it is still in the document.
 * The helper removes it in a `finally`, so the only moment it can be
 * inspected is from inside the `execCommand` call.
 */
function captureScratchTextarea(execCommand: ReturnType<typeof vi.fn>): () => HTMLTextAreaElement | null {
  let seen: HTMLTextAreaElement | null = null;
  execCommand.mockImplementation(() => {
    seen = document.querySelector('textarea');
    return true;
  });
  return () => seen;
}

afterEach(() => {
  delete (navigator as unknown as Record<string, unknown>)['clipboard'];
  delete (document as unknown as Record<string, unknown>)['execCommand'];
  vi.restoreAllMocks();
  // A scratch textarea that outlived its copy would be visible on the
  // page; the helper must never leave one behind.
  for (const stray of Array.from(document.querySelectorAll('textarea'))) stray.remove();
});

describe('utils / clipboard — the async clipboard path', () => {
  it('copies the text and reports success', async () => {
    const writeText = stubClipboard(async () => {});
    await expect(copyText('REQ-GRF-001')).resolves.toBe(true);
    expect(writeText).toHaveBeenCalledWith('REQ-GRF-001');
  });

  it('passes the text through byte for byte', async () => {
    // A command name, a path and a newline have to survive intact; a
    // trimmed or re-encoded string would be pasted wrong.
    const writeText = stubClipboard(async () => {});
    const text = '  git commit -m "feat: add x"\r\n\nsrc/lib/stores/graph.ts  ';
    await copyText(text);
    expect(writeText).toHaveBeenCalledWith(text);
  });

  it('copies an empty string without treating it as a failure', async () => {
    const writeText = stubClipboard(async () => {});
    // `''` is a legitimate clipboard value; the old
    // `if (text) copy()` guard shape is why this is worth pinning.
    expect(await copyText('')).toBe(true);
    expect(writeText).toHaveBeenCalledWith('');
  });
});

describe('utils / clipboard — the legacy fallback', () => {
  it('falls back to execCommand when the clipboard refuses', async () => {
    // A denied clipboard permission is the common case, and it
    // arrives as a rejected promise rather than a missing API.
    stubClipboard(async () => {
      throw new Error('Write permission denied');
    });
    const execCommand = stubExecCommand(true);

    expect(await copyText('copy me')).toBe(true);
    expect(execCommand).toHaveBeenCalledWith('copy');
  });

  it('uses the fallback when there is no clipboard at all', async () => {
    delete (navigator as unknown as Record<string, unknown>)['clipboard'];
    const execCommand = stubExecCommand(true);

    expect(await copyText('copy me')).toBe(true);
    expect(execCommand).toHaveBeenCalledWith('copy');
  });

  it('uses the fallback when the clipboard object has no writeText', async () => {
    stubPartialClipboard();
    const execCommand = stubExecCommand(true);

    expect(await copyText('copy me')).toBe(true);
    expect(execCommand).toHaveBeenCalledWith('copy');
  });

  it('reports failure when the fallback refuses too', async () => {
    // Honesty matters here: `false` is what lets the caller show
    // "could not copy" instead of pretending the text is on the
    // clipboard.
    stubClipboard(async () => {
      throw new Error('denied');
    });
    stubExecCommand(false);

    expect(await copyText('copy me')).toBe(false);
  });

  it('reports failure instead of throwing when execCommand throws', async () => {
    stubExecCommand(() => {
      throw new Error('execCommand is not implemented here');
    });

    // A rejected promise from a copy button is an unhandled error in
    // every caller; `false` is a value they can render.
    await expect(copyText('copy me')).resolves.toBe(false);
  });

  it('reports failure when neither mechanism exists', async () => {
    delete (navigator as unknown as Record<string, unknown>)['clipboard'];
    // No `execCommand` at all: the call throws a TypeError, which the
    // helper's own `try/catch` has to absorb.
    expect(await copyText('copy me')).toBe(false);
  });
});

describe('utils / clipboard — the scratch textarea', () => {
  it('stages the text in a selected, off-screen, read-only element', async () => {
    const execCommand = stubExecCommand(true);
    const capture = captureScratchTextarea(execCommand);
    const select = vi.spyOn(HTMLTextAreaElement.prototype, 'select');

    await copyText('staged text');
    const scratch = capture();

    // These three attributes are what make the fallback invisible: the
    // element must not flash on screen, steal focus visibly, or be
    // edited while it holds the selection.
    expect(scratch).not.toBeNull();
    expect(scratch?.value).toBe('staged text');
    expect(scratch?.getAttribute('readonly')).toBe('true');
    expect(scratch?.style.position).toBe('absolute');
    expect(scratch?.style.left).toBe('-9999px');
    expect(select).toHaveBeenCalled();
  });

  it('removes the scratch element after a successful copy', async () => {
    stubExecCommand(true);
    await copyText('copy me');
    // A textarea left in the body would be a visible artefact of a
    // copy the user never asked to see.
    expect(document.querySelector('textarea')).toBeNull();
  });

  it('removes the scratch element even when the copy throws', async () => {
    // The `finally` is the reason the helper cannot leak a stray node
    // on the failure path, which is the path that actually happens in
    // the environments that need this fallback.
    stubExecCommand(() => {
      throw new Error('nope');
    });

    expect(await copyText('copy me')).toBe(false);
    expect(document.querySelector('textarea')).toBeNull();
  });

  it('puts the text into the scratch element verbatim', async () => {
    const execCommand = stubExecCommand(true);
    const capture = captureScratchTextarea(execCommand);

    await copyText('a "quoted" <value> & more');
    // `value` is assigned as a property, so no escaping can corrupt it.
    expect(capture()?.value).toBe('a "quoted" <value> & more');
  });
});
