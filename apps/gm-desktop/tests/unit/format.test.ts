import { describe, expect, it } from 'vitest';
import { formatBytes, formatUptime, interpolate, shortSha } from '../../src/lib/utils/format';

describe('format / formatBytes', () => {
  it('returns bytes for small values', () => {
    expect(formatBytes(0)).toBe('0 B');
    expect(formatBytes(1023)).toBe('1023 B');
  });
  it('switches to KB / MB / GB at thresholds', () => {
    expect(formatBytes(1024)).toBe('1.0 KB');
    expect(formatBytes(1024 * 1024)).toBe('1.0 MB');
    expect(formatBytes(1024 * 1024 * 1024)).toBe('1.00 GB');
  });
  it('handles negative / non-finite input safely', () => {
    expect(formatBytes(-1)).toBe('0');
    expect(formatBytes(Number.NaN)).toBe('0');
  });
});

describe('format / formatUptime', () => {
  it('returns dash for nullish values', () => {
    expect(formatUptime(null)).toBe('—');
    expect(formatUptime(undefined)).toBe('—');
  });
  it('formats seconds / minutes / hours', () => {
    expect(formatUptime(5)).toBe('5s');
    expect(formatUptime(60)).toBe('1m 0s');
    expect(formatUptime(3600)).toBe('1h 0m');
    expect(formatUptime(3661)).toBe('1h 1m');
  });
});

describe('format / interpolate', () => {
  it('replaces {key} tokens from the values map', () => {
    expect(interpolate('v{n} was created by {user}', { n: 3, user: 'Ulysses' })).toBe(
      'v3 was created by Ulysses',
    );
  });
  it('keeps the original token when the value is missing', () => {
    expect(interpolate('{missing} here', {})).toBe('{missing} here');
  });
});

describe('format / shortSha', () => {
  it('returns input unchanged when shorter than the cap', () => {
    expect(shortSha('abc', 8)).toBe('abc');
  });
  it('truncates to the cap when longer', () => {
    expect(shortSha('abcdef0123456789', 8)).toBe('abcdef01');
  });
  it('returns empty for empty input', () => {
    expect(shortSha('')).toBe('');
  });
});
