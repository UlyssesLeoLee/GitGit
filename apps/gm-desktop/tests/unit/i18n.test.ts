import { describe, expect, it } from 'vitest';
import { tFor, type LocaleId } from '../../src/lib/i18n';

describe('i18n', () => {
  it('returns zh-CN translation by default for known keys', () => {
    expect(tFor('zh-CN', 'nav.dashboard')).toBe('概览');
  });

  it('returns the en translation when locale is en', () => {
    expect(tFor('en', 'nav.dashboard')).toBe('Overview');
  });

  it('falls back to en when key is missing in the requested locale', () => {
    // Inject a placeholder locale to force the fallback path.
    const fallback = tFor(('xx' as unknown) as LocaleId, 'common.cancel');
    expect(fallback).toBe('Cancel');
  });

  it('returns the key itself when even en misses it', () => {
    expect(tFor('en', 'totally.missing.key')).toBe('totally.missing.key');
  });
});
