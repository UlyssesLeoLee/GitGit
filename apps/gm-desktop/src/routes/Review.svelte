<!--
  AI review page (V0 task T9).

  Why a route and not the "push 前弹窗" (pre-push dialog) the task text
  names: the V0 desktop shell has no push action to anchor a dialog to —
  `commands/repos.rs` exposes list / detail / clone-url / open-in-shell
  and nothing that pushes. Inventing a push button to hang a dialog off
  would be a larger change than T9 is, and the streaming contract the
  dialog exists to show is identical either way. So the dialog's content
  ships as a page of its own, and it is the same panel a dialog would
  host.

  Model output is rendered through a normal text node. Svelte escapes it,
  and this file contains no `{@html}`, so a provider that returns markup
  or a script tag shows as the characters it emitted.
-->
<script lang="ts">
  import { review, startReview, stopReview, isBusy } from '$lib/stores/review';
  import { catalog } from '$lib/i18n';
  import { AI_PROVIDERS } from '$lib/api/types';

  let diff = $state('');
  let provider = $state<string>('openai');
  let model = $state('');
  let baseUrl = $state('');
  /** Local, pre-flight validation only. The Rust side validates again. */
  let localError = $state<string | null>(null);

  const busy = $derived(isBusy($review));

  /**
   * Reject a base URL that cannot be parsed as an http(s) URL, and any
   * scheme that is not http(s).
   *
   * `[FACT]` `base_url` decides where the review diff — untrusted user
   * content that may contain code, paths, and (despite the redaction
   * pass) residual secrets — is transmitted. `ProviderSpec::with_base_url`
   * accepts any string and the request goes to `{base_url}/chat/completions`,
   * so a typo here sends it somewhere real rather than nowhere.
   *
   * `[FACT]` `file://` and `javascript:` are not merely useless: reqwest
   * would fail on them, but only after the user waited for the round trip
   * and read an error that names the scheme rather than the typo. The
   * blank case is the common one and must stay valid — it means "use the
   * preset's own base URL", which is what every provider without an
   * override should do.
   */
  function validateBaseUrl(raw: string): string | null {
    const trimmed = raw.trim();
    if (trimmed === '') return null;
    let parsed: URL;
    try {
      parsed = new URL(trimmed);
    } catch {
      return $catalog['review.invalidBaseUrl'];
    }
    if (parsed.protocol !== 'http:' && parsed.protocol !== 'https:') {
      return $catalog['review.invalidBaseUrl'];
    }
    return null;
  }

  async function onStart(): Promise<void> {
    if (diff.trim().length === 0) {
      localError = $catalog['review.invalidDiff'];
      return;
    }
    const baseUrlError = validateBaseUrl(baseUrl);
    if (baseUrlError !== null) {
      localError = baseUrlError;
      return;
    }
    localError = null;
    const trimmedBase = baseUrl.trim();
    await startReview(diff, {
      provider,
      model: model.trim() === '' ? null : model.trim(),
      // Blank means "the preset's own base URL", which is the Rust
      // contract: `None` keeps `ProviderSpec` unmodified.
      baseUrl: trimmedBase === '' ? null : trimmedBase,
    });
  }

  async function onStop(): Promise<void> {
    await stopReview();
  }
</script>

<section class="mx-auto max-w-3xl space-y-6" data-testid="review-page">
  <header>
    <h1 class="text-xl font-semibold" data-testid="review-heading">
      {$catalog['review.heading']}
    </h1>
    <p class="mt-1 text-sm text-slate-500 dark:text-slate-400">
      {$catalog['review.subhead']}
    </p>
  </header>

  <div class="card space-y-4">
    <div>
      <label class="block text-sm font-medium" for="review-diff">
        {$catalog['review.diffLabel']}
      </label>
      <textarea
        id="review-diff"
        class="mt-1 h-40 w-full rounded-md border border-slate-300 p-2 font-mono text-xs dark:border-slate-600 dark:bg-slate-900"
        placeholder={$catalog['review.diffPlaceholder']}
        bind:value={diff}
        data-testid="review-diff"
      ></textarea>
    </div>

    <div class="flex flex-wrap gap-4">
      <div>
        <label class="block text-sm font-medium" for="review-provider">
          {$catalog['review.providerLabel']}
        </label>
        <select
          id="review-provider"
          class="mt-1 rounded-md border border-slate-300 px-2 py-1 text-sm dark:border-slate-600 dark:bg-slate-900"
          bind:value={provider}
          data-testid="review-provider"
        >
          {#each AI_PROVIDERS as key (key)}
            <option value={key}>{key}</option>
          {/each}
        </select>
      </div>

      <div class="flex-1">
        <label class="block text-sm font-medium" for="review-model">
          {$catalog['review.modelLabel']}
        </label>
        <input
          id="review-model"
          class="mt-1 w-full rounded-md border border-slate-300 px-2 py-1 text-sm dark:border-slate-600 dark:bg-slate-900"
          placeholder={$catalog['review.modelPlaceholder']}
          bind:value={model}
          data-testid="review-model"
        />
      </div>

      <!-- Base URL override. Shown unconditionally rather than only for
           the local presets: a self-hosted OpenAI-compatible gateway, a
           corporate proxy, or a gateway on a non-default port is just as
           real a deployment, and the registry has no notion of "which
           providers might need this". The placeholder carries the
           default so leaving it blank is the obvious path. -->
      <div class="w-full">
        <label class="block text-sm font-medium" for="review-base-url">
          {$catalog['review.baseUrlLabel']}
        </label>
        <input
          id="review-base-url"
          type="url"
          class="mt-1 w-full rounded-md border border-slate-300 px-2 py-1 text-sm dark:border-slate-600 dark:bg-slate-900"
          placeholder={$catalog['review.baseUrlPlaceholder']}
          bind:value={baseUrl}
          disabled={busy}
          data-testid="review-base-url"
        />
      </div>
    </div>

    <div class="flex items-center gap-2">
      <button
        class="rounded-md bg-accent-600 px-3 py-1.5 text-sm font-medium text-white disabled:opacity-50"
        disabled={busy}
        onclick={onStart}
        data-testid="review-start"
      >
        {$catalog['review.start']}
      </button>
      <button
        class="rounded-md border border-slate-300 px-3 py-1.5 text-sm font-medium disabled:opacity-50 dark:border-slate-600"
        disabled={!busy}
        onclick={onStop}
        data-testid="review-stop"
      >
        {$catalog['review.stop']}
      </button>

      <span class="text-sm text-slate-500" data-testid="review-status">
        {#if $review.status === 'starting'}
          {$catalog['review.starting']}
        {:else if $review.status === 'streaming'}
          {$catalog['review.streaming']}
        {:else if $review.status === 'done'}
          {$catalog['review.done']}
        {:else if $review.status === 'cancelled'}
          {$catalog['review.cancelled']}
        {/if}
      </span>
    </div>

    {#if localError}
      <p class="text-sm text-red-600" data-testid="review-local-error">{localError}</p>
    {/if}

    {#if $review.redactions > 0}
      <p class="text-xs text-amber-600" data-testid="review-redactions">
        {$catalog['review.redacted'].replace('{n}', String($review.redactions))}
      </p>
    {/if}
  </div>

  <!-- Unsupported provider. Anthropic genuinely has no streaming
       implementation, so this is a refusal, not a fallback: the review
       is not sent at all rather than arriving in one lump behind a
       spinner. -->
  {#if $review.status === 'unsupported'}
    <div
      class="card border-l-4 border-l-amber-500 text-sm"
      data-testid="review-unsupported"
    >
      {$catalog['review.unsupported'].replace(
        '{provider}',
        $review.unsupportedProvider ?? provider,
      )}
    </div>
  {/if}

  {#if $review.status === 'error'}
    <div class="card border-l-4 border-l-red-500 text-sm" data-testid="review-error">
      <p class="font-medium">
        {$catalog[`errors.kind.${$review.errorKind ?? 'Internal'}`] ??
          $catalog['errors.routeTitle']}
      </p>
      {#if $review.errorKind === 'AiReviewNoKey'}
        <p class="mt-1 text-slate-600 dark:text-slate-300">
          {$catalog['review.noKey']}
        </p>
      {/if}
      <p class="mt-1 break-words font-mono text-xs text-slate-500" data-testid="review-error-detail">
        {$review.errorMessage}
      </p>
    </div>
  {/if}

  <div class="card" data-testid="review-output-card">
    <div class="mb-2 flex items-center justify-between text-xs text-slate-500">
      <span>{$catalog['review.outputLabel']}</span>
      <span class="flex gap-3">
        {#if $review.model}
          <span data-testid="review-model-served">
            {$catalog['review.modelServed'].replace('{model}', $review.model)}
          </span>
        {/if}
        <span data-testid="review-token-count">
          {$catalog['review.tokenCount'].replace('{n}', String($review.tokenCount))}
        </span>
      </span>
    </div>

    {#if $review.truncated}
      <p class="mb-1 text-xs text-amber-600" data-testid="review-truncated">
        {$catalog['review.truncated']}
      </p>
    {/if}

    <pre
      class="max-h-96 min-h-24 overflow-y-auto whitespace-pre-wrap break-words font-mono text-xs"
      data-testid="review-output"
      >{$review.text === '' ? $catalog['review.empty'] : $review.text}</pre
    >
  </div>
</section>
