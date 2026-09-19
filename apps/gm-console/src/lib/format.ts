/**
 * Format a byte count as a human-readable string using base-1024 units.
 *
 * Negative numbers are normalised to 0 before formatting so the UI
 * never prints "NaN B" or "-1 B".
 */
export function formatBytes(n: number | null | undefined): string {
  if (n == null || !Number.isFinite(n)) return '—';
  const safe = Math.max(0, Math.floor(n));
  if (safe < 1024) return `${safe} B`;
  const units = ['KB', 'MB', 'GB', 'TB'] as const;
  let value = safe / 1024;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }
  return `${value.toFixed(value >= 10 ? 0 : 1)} ${units[unit]}`;
}

/**
 * Format a unix-ms timestamp as a locale-aware short date+time string.
 * Returns `'—'` for nullish or non-finite inputs.
 */
export function formatUnixMs(ms: number | null | undefined, locale: string = 'zh-CN'): string {
  if (ms == null || !Number.isFinite(ms)) return '—';
  try {
    return new Intl.DateTimeFormat(locale, {
      year: 'numeric',
      month: '2-digit',
      day: '2-digit',
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit',
      hour12: false,
    }).format(new Date(ms));
  } catch {
    return new Date(ms).toISOString();
  }
}

/**
 * Truncate a SHA string for display. Returns the first `head` chars,
 * an ellipsis, then the last `tail` chars. Inputs shorter than
 * `head + tail + 1` are returned verbatim.
 */
export function shortenSha(sha: string | null | undefined, head: number = 7, tail: number = 0): string {
  if (!sha) return '—';
  if (sha.length <= head + tail + 1) return sha;
  if (tail === 0) return sha.slice(0, head);
  return `${sha.slice(0, head)}…${sha.slice(-tail)}`;
}

/** Compute a relative time string ("3 minutes ago"). */
export function relativeFromNow(ms: number | null | undefined, locale: string = 'zh-CN'): string {
  if (ms == null || !Number.isFinite(ms)) return '—';
  const diff = Date.now() - ms;
  const abs = Math.abs(diff);
  const rtf = new Intl.RelativeTimeFormat(locale, { numeric: 'auto' });
  if (abs < 60_000) return rtf.format(Math.round(-diff / 1000), 'second');
  if (abs < 3_600_000) return rtf.format(Math.round(-diff / 60_000), 'minute');
  if (abs < 86_400_000) return rtf.format(Math.round(-diff / 3_600_000), 'hour');
  return rtf.format(Math.round(-diff / 86_400_000), 'day');
}

/** Format a numeric `+N` / `-N` delta. */
export function formatDelta(n: number | null | undefined): string {
  if (n == null || !Number.isFinite(n)) return '—';
  if (n === 0) return '0';
  return n > 0 ? `+${n}` : `${n}`;
}