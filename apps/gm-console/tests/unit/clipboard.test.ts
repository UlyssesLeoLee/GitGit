import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { copyToClipboard } from '@/lib/clipboard';

const originalExecCommand = document.execCommand;

function setSecureContext(value: boolean) {
  Object.defineProperty(window, 'isSecureContext', {
    writable: true,
    configurable: true,
    value,
  });
}

function setClipboard(writeText: (() => Promise<void>) | undefined) {
  Object.defineProperty(navigator, 'clipboard', {
    writable: true,
    configurable: true,
    value: writeText ? { writeText } : undefined,
  });
}

beforeEach(() => {
  setSecureContext(true);
  setClipboard(undefined);
  document.body.innerHTML = '';
});

afterEach(() => {
  setSecureContext(false);
  setClipboard(undefined);
  document.execCommand = originalExecCommand;
  document.body.innerHTML = '';
});

describe('copyToClipboard — modern Clipboard API path', () => {
  it('writes through navigator.clipboard and reports success', async () => {
    const writeText = vi.fn().mockResolvedValue(undefined);
    setClipboard(writeText);
    await expect(copyToClipboard('sk-test-123')).resolves.toBe(true);
    expect(writeText).toHaveBeenCalledWith('sk-test-123');
  });

  it('does not touch the DOM when the modern API succeeds', async () => {
    setClipboard(vi.fn().mockResolvedValue(undefined));
    await copyToClipboard('hello');
    expect(document.querySelector('textarea')).toBeNull();
  });

  it('falls back to the legacy path when writeText rejects', async () => {
    const writeText = vi.fn().mockRejectedValue(new Error('denied'));
    setClipboard(writeText);
    document.execCommand = vi.fn().mockReturnValue(true);

    await expect(copyToClipboard('hello')).resolves.toBe(true);
    expect(writeText).toHaveBeenCalled();
    expect(document.execCommand).toHaveBeenCalledWith('copy');
  });

  it('skips the modern API entirely in an insecure context', async () => {
    const writeText = vi.fn().mockResolvedValue(undefined);
    setClipboard(writeText);
    setSecureContext(false);
    document.execCommand = vi.fn().mockReturnValue(true);

    await expect(copyToClipboard('hello')).resolves.toBe(true);
    expect(writeText).not.toHaveBeenCalled();
    expect(document.execCommand).toHaveBeenCalledWith('copy');
  });
});

describe('copyToClipboard — legacy execCommand path', () => {
  it('copies through a hidden textarea and cleans it up', async () => {
    setSecureContext(false);
    document.execCommand = vi.fn().mockReturnValue(true);

    await expect(copyToClipboard('legacy text')).resolves.toBe(true);
    // The textarea must not be left behind in the document.
    expect(document.querySelector('textarea')).toBeNull();
  });

  it('puts the text into the temporary textarea', async () => {
    setSecureContext(false);
    let captured = '';
    document.execCommand = vi.fn().mockReturnValue(true);
    const originalCreate = document.createElement.bind(document);
    vi.spyOn(document, 'createElement').mockImplementation((tag: string) => {
      const el = originalCreate(tag);
      if (tag === 'textarea') {
        const origSelect = el.select.bind(el);
        el.select = () => {
          captured = (el as HTMLTextAreaElement).value;
          origSelect();
        };
      }
      return el;
    });

    await copyToClipboard('captured-value');
    expect(captured).toBe('captured-value');
    vi.restoreAllMocks();
  });

  it('reports failure when execCommand returns false', async () => {
    setSecureContext(false);
    document.execCommand = vi.fn().mockReturnValue(false);
    await expect(copyToClipboard('nope')).resolves.toBe(false);
  });

  it('reports failure instead of throwing when the DOM call blows up', async () => {
    setSecureContext(false);
    document.execCommand = vi.fn().mockReturnValue(true);
    vi.spyOn(document.body, 'appendChild').mockImplementation(() => {
      throw new Error('detached document');
    });

    await expect(copyToClipboard('boom')).resolves.toBe(false);
    vi.restoreAllMocks();
  });

  it('reports failure when no copy mechanism exists at all', async () => {
    setSecureContext(false);
    setClipboard(undefined);
    document.execCommand = vi.fn().mockReturnValue(false);
    await expect(copyToClipboard('nothing works')).resolves.toBe(false);
  });
});
