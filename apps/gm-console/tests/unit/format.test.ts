import { describe, expect, it } from 'vitest';
import {
  formatBytes,
  formatDelta,
  formatUnixMs,
  relativeFromNow,
  shortenSha,
} from '@/lib/format';

describe('formatBytes', () => {
  it('formats raw bytes', () => {
    expect(formatBytes(512)).toBe('512 B');
  });
  it('promotes to KB / MB', () => {
    expect(formatBytes(2048)).toBe('2.0 KB');
    expect(formatBytes(5 * 1024 * 1024)).toBe('5.0 MB');
  });
  it('clamps negative values to 0', () => {
    expect(formatBytes(-1)).toBe('0 B');
  });
  it('returns placeholder for nullish or non-finite inputs', () => {
    expect(formatBytes(null)).toBe('\u2014');
    expect(formatBytes(undefined)).toBe('\u2014');
    expect(formatBytes(NaN)).toBe('\u2014');
  });
});

describe('shortenSha', () => {
  const sha = 'abcdef0123456789';
  it('truncates long shas to head chars by default', () => {
    expect(shortenSha(sha)).toBe('abcdef0');
  });
  it('returns verbatim when input is short', () => {
    expect(shortenSha('abc')).toBe('abc');
  });
  it('returns placeholder for nullish', () => {
    expect(shortenSha(null)).toBe('\u2014');
  });
});

describe('formatDelta', () => {
  it('prefixes positive numbers with +', () => {
    expect(formatDelta(3)).toBe('+3');
  });
  it('keeps sign on negative numbers', () => {
    expect(formatDelta(-7)).toBe('-7');
  });
  it('renders zero as 0', () => {
    expect(formatDelta(0)).toBe('0');
  });
  it('returns placeholder for nullish', () => {
    expect(formatDelta(null)).toBe('\u2014');
  });
});

describe('formatUnixMs', () => {
  it('formats a known timestamp as a locale-aware string', () => {
    const text = formatUnixMs(0, 'en-US');
    // The exact pattern depends on the runtime ICU data, but it must
    // include the year 1970 (the unix epoch) and never throw.
    expect(text).toContain('1970');
  });
  it('returns placeholder for nullish', () => {
    expect(formatUnixMs(null)).toBe('\u2014');
  });
});

describe('relativeFromNow', () => {
  it('returns placeholder for nullish', () => {
    expect(relativeFromNow(null)).toBe('\u2014');
  });
  it('produces a non-empty string for recent timestamps', () => {
    expect(relativeFromNow(Date.now() - 60_000, 'en-US').length).toBeGreaterThan(0);
  });
});