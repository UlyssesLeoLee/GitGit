<!--
  Conflict-arbitration dialog (ADR-0024 §3.4.3 / §3.7.1).

  `[FACT]` Why a component inside a route and not a modal: the same
  reason `/review` is a page and not a dialog — the V0 desktop shell has
  no merge action to anchor a modal to, and the content is what matters.
  The route is the container; this is the panel a dialog would host.

  `[FACT]` Model output is rendered through a normal text node. Svelte
  escapes it, and this file contains no raw-HTML block, so a provider that
  returns markup or a script tag shows as the characters it emitted.

  `[PROPOSAL]` The four options are always in this order (ADR §3.4.3):
  keep ours, keep theirs, hand-edit, and *then* ask the AI. The three
  that do not depend on a credential are available with no key at all —
  the dialog does not degrade because there is no AI.
-->
<script lang="ts">
  import {
    abandonArbitration,
    canAccept,
    canAskAi,
    conflict,
    loadKeyStatus,
    markDiffSeen,
    resolveChoice,
    setResolution,
    startArbitration,
    stopArbitration,
  } from '$lib/stores/conflict';
  import { AI_PROVIDERS, supportsStreaming, type ConflictRegion } from '$lib/api/types';
  import { diffLines } from '$lib/utils/linediff';
  import { catalog } from '$lib/i18n';

  let { region }: { region: ConflictRegion } = $props();

  let provider = $state('openai');
  let model = $state('');

  /**
   * The manual editor is keyed by the region it belongs to, so opening
   * another region closes it instead of leaving one region's draft on
   * screen under another region's name.
   */
  let editingRegionId = $state<string | null>(null);
  let manualText = $state('');
  let manualError = $state<string | null>(null);

  /**
   * The diff is keyed by the region *and* the suggestion it shows, so a
   * second suggestion for the same region arrives closed again — which
   * is what keeps "accept is disabled until the user has seen the
   * diff" true for every suggestion, not only the first.
   */
  let openedDiffKey = $state<string | null>(null);

  const streaming = $derived(supportsStreaming(provider));
  const diffKey = $derived(`${region.id}:${$conflict.suggestion ?? ''}`);
  const diffOpen = $derived(openedDiffKey === diffKey);
  const manualOpen = $derived(editingRegionId === region.id);
  const rows = $derived(diffLines(region.ours, $conflict.suggestion ?? ''));

  /**
   * Ask the backend whether a key can be resolved for the provider.
   *
   * `[PROPOSAL]` ADR §3.7.1: the "ask AI" option is shown but disabled
   * when there is no key, and that needs the app to know before the
   * request. The webview cannot see the process environment, so the
   * backend answers — with a presence, never with the key.
   */
  $effect(() => {
    void loadKeyStatus(provider);
  });

  function openManual(): void {
    // `[PROPOSAL]` ADR §3.7.4: the editor is prefilled with the ours
    // side's text.
    manualText = region.ours;
    manualError = null;
    editingRegionId = region.id;
  }

  function closeManual(): void {
    editingRegionId = null;
    manualError = null;
  }

  function onSaveManual(): void {
    const outcome = resolveChoice(region, 'manual', manualText);
    if (!outcome.ok) {
      manualError = outcome.reason;
      return;
    }
    // `[PROPOSAL]` ADR §3.7.3: the decision is what produces the audit
    // record, and the region travels with it so the backend can recompute
    // the id.
    setResolution('manual', outcome.text, region);
    closeManual();
  }

  function onKeepOurs(): void {
    const outcome = resolveChoice(region, 'ours');
    if (outcome.ok) setResolution('ours', outcome.text, region);
  }

  function onKeepTheirs(): void {
    const outcome = resolveChoice(region, 'theirs');
    if (outcome.ok) setResolution('theirs', outcome.text, region);
  }

  async function onAskAi(): Promise<void> {
    await startArbitration(region, {
      provider,
      model: model.trim() === '' ? null : model.trim(),
    });
  }

  function openDiff(): void {
    openedDiffKey = diffKey;
    // `[PROPOSAL]` ADR §3.4.3: the accept button unlocks when the user
    // has opened the diff, and not before.
    markDiffSeen();
  }

  function onAccept(): void {
    const suggestion = $conflict.suggestion;
    if (suggestion === null) return;
    setResolution('ai', suggestion, region);
  }

  function onStop(): void {
    void stopArbitration();
  }

  function onLeave(): void {
    abandonArbitration();
  }
</script>

<section class="space-y-4" data-testid="conflict-dialog" data-region-id={region.id}>
  <header class="flex flex-wrap items-baseline gap-x-3 gap-y-1">
    <h2 class="text-lg font-semibold" data-testid="conflict-path">{region.path}</h2>
    <span class="text-xs text-slate-500" data-testid="conflict-region-index">
      {$catalog['conflict.regionIndex'].replace('{n}', String(region.index + 1))}
    </span>
    {#if region.scopeLabel}
      <span class="text-xs text-slate-500" data-testid="conflict-scope">
        {$catalog['conflict.scope'].replace('{scope}', region.scopeLabel)}
      </span>
    {/if}
  </header>

  <!-- The three sides. `[PROPOSAL]` ADR §3.2: the base side is the
       user's only reference for "what it was before", so it is shown
       next to the two candidates rather than hidden. -->
  <div class="grid gap-3 md:grid-cols-3">
    <div class="card">
      <h3 class="mb-1 text-xs font-semibold uppercase" data-testid="conflict-ours-label">
        {$catalog['conflict.ours']}
      </h3>
      <pre class="max-h-48 overflow-auto whitespace-pre-wrap break-words font-mono text-xs"
        data-testid="conflict-ours">{region.ours}</pre>
    </div>
    <div class="card">
      <h3 class="mb-1 text-xs font-semibold uppercase" data-testid="conflict-base-label">
        {$catalog['conflict.base']}
      </h3>
      <pre class="max-h-48 overflow-auto whitespace-pre-wrap break-words font-mono text-xs"
        data-testid="conflict-base">{region.base}</pre>
    </div>
    <div class="card">
      <h3 class="mb-1 text-xs font-semibold uppercase" data-testid="conflict-theirs-label">
        {$catalog['conflict.theirs']}
      </h3>
      <pre class="max-h-48 overflow-auto whitespace-pre-wrap break-words font-mono text-xs"
        data-testid="conflict-theirs">{region.theirs}</pre>
    </div>
  </div>

  <!-- `[PROPOSAL]` The four options, in the ADR's order. The AI one is last. -->
  <div class="card space-y-3" data-testid="conflict-options">
    <div class="flex flex-wrap gap-2">
      <button
        type="button"
        class="btn-secondary"
        data-testid="conflict-keep-ours"
        onclick={onKeepOurs}
        disabled={$conflict.status === 'starting' || $conflict.status === 'streaming'}
      >
        {$catalog['conflict.keepOurs']}
      </button>
      <button
        type="button"
        class="btn-secondary"
        data-testid="conflict-keep-theirs"
        onclick={onKeepTheirs}
        disabled={$conflict.status === 'starting' || $conflict.status === 'streaming'}
      >
        {$catalog['conflict.keepTheirs']}
      </button>
      <button
        type="button"
        class="btn-secondary"
        data-testid="conflict-manual"
        onclick={openManual}
        disabled={$conflict.status === 'starting' || $conflict.status === 'streaming'}
      >
        {$catalog['conflict.manualEdit']}
      </button>

      <!--
        `[PROPOSAL]` ADR §3.4.1: the "ask AI" option is only offered on
        a provider that can stream, so it is not rendered at all for a
        provider that cannot — with the reason stated next to it.
      -->
      {#if streaming}
        <button
          type="button"
          class="btn-primary"
          data-testid="conflict-ask-ai"
          onclick={onAskAi}
          disabled={!canAskAi($conflict, streaming)}
        >
          {$catalog['conflict.askAi']}
        </button>
        {#if $conflict.status === 'starting' || $conflict.status === 'streaming'}
          <button type="button" class="btn-secondary" data-testid="conflict-stop" onclick={onStop}>
            {$catalog['conflict.stop']}
          </button>
        {/if}
      {:else}
        <p class="w-full text-sm text-amber-700" data-testid="conflict-unsupported">
          {$catalog['conflict.unsupported'].replace('{provider}', provider)}
        </p>
      {/if}
    </div>

    <!--
      `[PROPOSAL]` ADR §3.7.1: with no key the option is present but
      disabled, and it says where the key goes. The dialog does not
      degrade: the three options above stay available.
    -->
    {#if streaming && $conflict.keyStatus === 'absent'}
      <p class="text-sm text-amber-700" data-testid="conflict-no-key">
        {$catalog['conflict.noKey']}
      </p>
    {/if}

    <p class="text-sm" data-testid="conflict-status">
      {#if $conflict.status === 'starting'}
        {$catalog['conflict.status.starting']}
      {:else if $conflict.status === 'streaming'}
        {$catalog['conflict.status.streaming']}
      {:else if $conflict.status === 'done'}
        {$catalog['conflict.status.done']}
      {:else if $conflict.status === 'cancelled'}
        {$catalog['conflict.status.cancelled']}
      {:else if $conflict.status === 'unsupported'}
        {$catalog['conflict.unsupported'].replace('{provider}', $conflict.unsupportedProvider ?? provider)}
      {/if}
    </p>

    {#if $conflict.redactions > 0}
      <p class="text-xs text-amber-600" data-testid="conflict-redactions">
        {$catalog['conflict.redacted'].replace('{n}', String($conflict.redactions))}
      </p>
    {/if}

    {#if $conflict.model}
      <p class="text-xs text-slate-500" data-testid="conflict-model-served">
        {$catalog['conflict.modelServed'].replace('{model}', $conflict.model)}
      </p>
    {/if}

    {#if $conflict.status === 'error'}
      <p class="text-sm text-red-600" data-testid="conflict-error">
        {$conflict.errorMessage}
      </p>
    {/if}

    <!--
      `[PROPOSAL]` ADR §3.4.4: a refused answer is shown as refused,
      with its reason. The user can still hand-edit.
    -->
    {#if $conflict.outcome === 'rejected' && $conflict.rejectionReason}
      <p class="text-sm text-red-600" data-testid="conflict-rejected">
        {$catalog['conflict.rejected'].replace('{reason}', $conflict.rejectionReason)}
      </p>
    {/if}
  </div>

  <!-- The provider and model the arbitration would use. -->
  <div class="card flex flex-wrap gap-4">
    <div>
      <label class="block text-sm font-medium" for="conflict-provider">
        {$catalog['conflict.providerLabel']}
      </label>
      <select
        id="conflict-provider"
        class="mt-1 rounded-md border border-slate-300 px-2 py-1 text-sm dark:border-slate-600 dark:bg-slate-900"
        bind:value={provider}
        data-testid="conflict-provider"
      >
        {#each AI_PROVIDERS as key (key)}
          <option value={key}>{key}</option>
        {/each}
      </select>
    </div>
    <div class="flex-1">
      <label class="block text-sm font-medium" for="conflict-model">
        {$catalog['conflict.modelLabel']}
      </label>
      <input
        id="conflict-model"
        class="mt-1 w-full rounded-md border border-slate-300 p-2 text-sm dark:border-slate-600 dark:bg-slate-900"
        placeholder={$catalog['conflict.modelPlaceholder']}
        bind:value={model}
        data-testid="conflict-model"
      />
    </div>
  </div>

  <!--
    `[PROPOSAL]` ADR §3.7.4: the hand-edit. The editor is prefilled with
    the ours side's text, and the saved text goes through the same checks
    as an AI suggestion.
  -->
  {#if manualOpen}
    <div class="card space-y-2" data-testid="conflict-manual-editor">
      <label class="block text-sm font-medium" for="conflict-manual-text">
        {$catalog['conflict.manualLabel']}
      </label>
      <textarea
        id="conflict-manual-text"
        class="h-40 w-full rounded-md border border-slate-300 p-2 font-mono text-xs dark:border-slate-600 dark:bg-slate-900"
        bind:value={manualText}
        data-testid="conflict-manual-text"
      ></textarea>
      {#if manualError}
        <p class="text-sm text-red-600" data-testid="conflict-manual-error">{manualError}</p>
      {/if}
      <div class="flex gap-2">
        <button type="button" class="btn-primary" data-testid="conflict-manual-save" onclick={onSaveManual}>
          {$catalog['common.save']}
        </button>
        <button type="button" class="btn-secondary" data-testid="conflict-manual-cancel" onclick={closeManual}>
          {$catalog['common.cancel']}
        </button>
      </div>
    </div>
  {/if}

  <!--
    `[PROPOSAL]` ADR §3.4.3: the suggestion is always shown as a diff.
    The diff starts closed, and the accept button unlocks when the user
    opens it — that is a state-machine constraint, not a style.
  -->
  {#if $conflict.outcome === 'accepted' && $conflict.suggestion !== null}
    <div class="card space-y-2" data-testid="conflict-suggestion">
      {#if !diffOpen}
        <button type="button" class="btn-secondary" data-testid="conflict-show-diff" onclick={openDiff}>
          {$catalog['conflict.showDiff']}
        </button>
      {:else}
        <div class="flex items-center justify-between text-xs text-slate-500">
          <span data-testid="conflict-diff-ours-label">{$catalog['conflict.diffOursLabel']}</span>
          <span data-testid="conflict-diff-suggestion-label">
            {$catalog['conflict.diffSuggestionLabel']}
          </span>
        </div>
        <div class="grid grid-cols-2 gap-px overflow-x-auto" data-testid="conflict-diff">
          <div class="min-w-0" data-testid="conflict-diff-left">
            {#each rows as row, i (i)}
              <div
                class="whitespace-pre-wrap break-words px-2 py-0.5 font-mono text-xs"
                class:bg-red-50={row.op === 'remove'}
                class:dark:bg-red-950={row.op === 'remove'}
                data-op={row.op}
              >
                {row.left ?? ''}
              </div>
            {/each}
          </div>
          <div class="min-w-0" data-testid="conflict-diff-right">
            {#each rows as row, i (i)}
              <div
                class="whitespace-pre-wrap break-words px-2 py-0.5 font-mono text-xs"
                class:bg-green-50={row.op === 'add'}
                class:dark:bg-green-950={row.op === 'add'}
                data-op={row.op}
              >
                {row.right ?? ''}
              </div>
            {/each}
          </div>
        </div>
      {/if}
      <div class="flex gap-2">
        {#if diffOpen}
          <button type="button" class="btn-secondary" data-testid="conflict-hide-diff" onclick={() => (openedDiffKey = null)}>
            {$catalog['conflict.hideDiff']}
          </button>
        {/if}
        <!--
          `[FACT]` The accept button is present as soon as there is a
          suggestion, and disabled until the diff has been opened. That is
          the state-machine constraint ADR-0024 §3.4.3 asks for — not a
          style.
        -->
        <button
          type="button"
          class="btn-primary"
          data-testid="conflict-accept"
          disabled={!canAccept($conflict)}
          onclick={onAccept}
        >
          {$catalog['conflict.accept']}
        </button>
      </div>
    </div>
  {/if}

  <!-- The answer as it arrived, for the record. -->
  {#if $conflict.text !== ''}
    <details class="card" data-testid="conflict-raw">
      <summary class="text-xs font-semibold">{$catalog['conflict.rawAnswer']}</summary>
      <pre class="mt-2 max-h-48 overflow-auto whitespace-pre-wrap break-words font-mono text-xs"
        data-testid="conflict-raw-text">{$conflict.text}</pre>
    </details>
  {/if}

  <!--
    The replacement the user chose. `[PROPOSAL]` ADR §3.4.4: nothing is
    written from here — the write-back path is a separate step.
  -->
  {#if $conflict.resolution}
    <div class="card border-l-4 border-l-accent-500" data-testid="conflict-resolution">
      <p class="text-sm font-medium">
        {$catalog[`conflict.resolution.${$conflict.resolution.choice}`]}
      </p>
      <pre class="mt-1 max-h-48 overflow-auto whitespace-pre-wrap break-words font-mono text-xs"
        data-testid="conflict-resolution-text">{$conflict.resolution.text}</pre>
    </div>
  {/if}

  <button type="button" class="btn-secondary" data-testid="conflict-leave" onclick={onLeave}>
    {$catalog['conflict.close']}
  </button>
</section>
