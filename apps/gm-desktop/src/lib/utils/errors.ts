/**
 * One place that turns a rejection into something a page can show.
 *
 * `[FACT]` Tauri rejects with the serialized `AppError` from
 * `src-tauri/src/error.rs`, which is a **plain object**
 * `{ kind, message, source }` — not an `Error` instance. So the obvious
 * `e instanceof Error ? e.message : String(e)` is wrong twice over: the
 * `instanceof` arm never matches, and `String({...})` is
 * `"[object Object]"`. Measured on the graph page, which rendered
 * exactly that when the backend refused.
 *
 * Before this module the codebase had three partial answers: two
 * near-identical normalisers in the review and worktree stores with
 * *different* fallbacks, and a third translated-message helper inlined
 * into `Home.svelte`. Four other call sites had none at all.
 *
 * `[FACT]` The previous version of this comment ended "This is the
 * single implementation; the stores and the components all route through
 * it", and that was false when it was written: `worktree.ts`'s
 * `readError`, `review.ts`'s `asAppError` and `ErrorBoundary.svelte`'s
 * `toMessage` each still had their own normalizer, and each could render
 * `[object Object]`. All three now call the functions below, and the two
 * store-local helpers are deleted rather than delegated, so this module
 * is the only place a rejection is turned into text.
 */
import { t } from '$lib/i18n';

export interface NormalizedError {
  /** The `AppError` variant name, e.g. `NotAWorkTree`. */
  kind: string;
  message: string;
}

/** `[FACT]` `Internal` rather than `Unknown`: it is the one fallback that
 *  has a catalogue entry (`errors.kind.Internal`), so a translation is
 *  available instead of degrading to the raw text. */
const FALLBACK_KIND = 'Internal';

function hasString(o: Record<string, unknown>, field: 'kind' | 'message'): boolean {
  return typeof o[field] === 'string' && (o[field] as string).length > 0;
}

/** `[FACT]` A plain object that is *neither* of those is not an
 *  `AppError`, and stringifying it would give `[object Object]`. */
function isShaped(e: unknown): e is Record<string, unknown> {
  return typeof e === 'object' && e !== null;
}

/** Keep `kind` and `message` from whatever the command rejected with. */
export function normalizeError(e: unknown): NormalizedError {
  if (isShaped(e)) {
    const o = e as Record<string, unknown>;
    return {
      kind: hasString(o, 'kind') ? (o.kind as string) : FALLBACK_KIND,
      message: hasString(o, 'message') ? (o.message as string) : '',
    };
  }
  if (e instanceof Error) return { kind: FALLBACK_KIND, message: e.message };
  return { kind: FALLBACK_KIND, message: typeof e === 'string' ? e : String(e) };
}

/**
 * A single line for a toast or a status bar.
 *
 * `[FACT]` The catalogue template is used only when the `kind` actually
 * came from the payload **and** has an entry. Falling back to
 * `FALLBACK_KIND` and then looking that up was tried and is wrong:
 * `errors.kind.Internal` has a translation, so a rejection carrying a
 * perfectly good `message` but no `kind` — which the mock layer and any
 * non-Tauri caller produce — showed the generic "Internal error" and
 * dropped the words the backend actually said. The backend's own message
 * is more useful than a generic label, so it wins.
 */
export function friendlyError(e: unknown): string {
  if (isShaped(e)) {
    const o = e as Record<string, unknown>;
    const kind = hasString(o, 'kind') ? (o.kind as string) : null;
    const message = hasString(o, 'message') ? (o.message as string) : '';
    if (kind) {
      const tmplKey = `errors.kind.${kind}`;
      // `t` returns the key itself when there is no entry, which is how a
      // miss is detected — a real translation can never equal its own key.
      const tmpl = t(tmplKey);
      if (tmpl !== tmplKey) return tmpl;
    }
    if (message) return message;
    // Nothing usable in the payload at all. Say so rather than render
    // `[object Object]`, which is what the old `String(e)` did.
    return t(`errors.kind.${FALLBACK_KIND}`);
  }
  if (e instanceof Error) return e.message;
  return typeof e === 'string' ? e : String(e);
}
