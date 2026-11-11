/**
 * Tiny set of formatting helpers used across the UI.
 *
 * Pure functions only — no DOM access — so they're trivially
 * unit-testable under Vitest.
 */

export function formatBytes(n: number): string {
  if (!Number.isFinite(n) || n < 0) return '0';
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  if (n < 1024 * 1024 * 1024) return `${(n / (1024 * 1024)).toFixed(1)} MB`;
  return `${(n / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}

export function formatUptime(seconds: number | null | undefined): string {
  if (seconds == null) return '—';
  if (seconds < 60) return `${seconds}s`;
  const m = Math.floor(seconds / 60);
  const s = seconds % 60;
  if (m < 60) return `${m}m ${s}s`;
  const h = Math.floor(m / 60);
  const mm = m % 60;
  return `${h}h ${mm}m`;
}

/**
 * `String.prototype.replace` with `{key}` placeholders, conservative
 * — only handles `{key}` (no nested paths, no positional args).
 */
export function interpolate(
  template: string,
  values: Readonly<Record<string, string | number>>,
): string {
  return template.replace(/\{(\w+)\}/g, (whole, key: string) => {
    if (Object.prototype.hasOwnProperty.call(values, key)) {
      return String(values[key]);
    }
    return whole;
  });
}

export function shortSha(sha: string, len = 8): string {
  if (!sha) return '';
  return sha.length <= len ? sha : sha.slice(0, len);
}
