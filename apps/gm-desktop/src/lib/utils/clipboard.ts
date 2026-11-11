/**
 * Clipboard helper. Uses `navigator.clipboard.writeText` when
 * available; otherwise falls back to a hidden textarea + execCommand
 * path so legacy WebView2 builds (and some sandboxed Linux
 * environments) still work.
 */

export async function copyText(text: string): Promise<boolean> {
  if (typeof navigator !== 'undefined' && navigator.clipboard?.writeText) {
    try {
      await navigator.clipboard.writeText(text);
      return true;
    } catch (_err) {
      // fall through to the legacy path
    }
  }
  return legacyCopy(text);
}

function legacyCopy(text: string): boolean {
  if (typeof document === 'undefined') return false;
  const el = document.createElement('textarea');
  el.value = text;
  el.setAttribute('readonly', 'true');
  el.style.position = 'absolute';
  el.style.left = '-9999px';
  document.body.appendChild(el);
  let ok = false;
  try {
    el.select();
    ok = document.execCommand('copy');
  } catch (_err) {
    ok = false;
  } finally {
    document.body.removeChild(el);
  }
  return ok;
}
