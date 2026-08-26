# AI-Native Engineering Platform — Requirements Research: Phase 12

**Status:** v1.0. **Scope:** Phase 12 only (UX Red-Team Review, per original spec §47). Persona for this document: a senior Developer Experience Designer whose job is to find where the platform's own stated UX principles (`00-requirements-definition.md` §13, §34) conflict with usability fundamentals when applied to the actual MVP loop (`phase9-mvp-reduction.md` §6 Definition of Done), and to answer the spec's own question directly: **是否为了"涌现式 UI"破坏了稳定性?** ("did emergent UI get built at the cost of stability?"). This document does not revise the master requirements doc — that is Phase 13's job. It produces findings and open questions only.

**Tagging convention** (inherited unchanged): `[FACT]` / `[UNVERIFIED-FACT]` / `[INFERENCE]` / `[PROPOSAL]` / `[TBD]`.

**Inputs read in full:** `00-requirements-definition.md` §13 (Product Principles), §34 (UX Requirements), §50 (User Journeys); `phase9-mvp-reduction.md` (MVP loop and Definition of Done, §6); `phase11-red-team.md` (Angle 9 — Emergent UX, and Angle 17/RT-17 — the 10x check). This document is written as a direct response to both.

**Headline answer, stated up front so it isn't buried:** Phase 11's Angle 9 found that "Stable Skeleton + Emergent Context" does not exist anywhere in the requirements text — only UX-REQ-002's unbounded "context-assembled pages" promise, with zero acceptance criteria constraining *how much* a page may vary. This phase's job, per the task brief, was to land that promise on a concrete, falsifiable Stable Skeleton + Emergent Context split. §2 and §3 below do that. It is a genuine tightening, not a relabeling — see §9 for the direct comparison against Phase 11's finding.

---

## 1. Walkthrough of the MVP loop as UX

Evaluated against Phase 9 §6's Definition of Done, step by step. Each step is scored on: operation count, page/context jumps, information density, cognitive load, AI interruption points, discoverability, predictability. Where the requirements corpus does not contain enough detail to answer honestly, this document says `[TBD] — needs wireframe` rather than asserting a positive outcome.

### Step 1 — Issue creation (against a Repository)

- **Operation count:** `[PROPOSAL]` 1 primary action (an "intent" entry — UX-REQ-001) plus however many fields the Issue Node's required properties demand. If UX-REQ-001 is implemented shallowly (a search box that dispatches to a create-Issue form), this is effectively 2 operations: type intent, fill form. If implemented as specified (resolve intent to a structured View/action), it could be fewer. `[TBD] — needs wireframe` for the actual form.
- **Page/context jumps:** 0, if UX-REQ-001's entry point is global and persistent (see Stable Skeleton, §2). 1, if intent-resolution requires navigating to a separate "create" page first — which is exactly the "fixed menu/page hierarchy" UX-REQ-001 is written to prevent. `[INFERENCE]` The requirement text is silent on whether the *result* of intent resolution renders in-place or navigates; this is a real gap.
- **Information density:** Low at this step by nature — an Issue doesn't yet have graph context to display. No overload risk here.
- **Cognitive load:** Low — this is the one step closest to a conventional, well-understood UI pattern (issue-tracker creation forms are a solved problem industry-wide).
- **AI interruption points:** None required. This step can be, and should be, 100% Ambient/manual.
- **Discoverability:** Depends entirely on whether the intent-first entry point is visually obvious (a persistent input, not a hidden command palette). `[TBD] — needs wireframe.`
- **Predictability:** High risk if intent resolution is genuinely NLP-driven and non-deterministic — the same typed phrase should resolve to the same action every time, or Human Authority's "actionable control" promise (§34) becomes unpredictable at the very first step. `[PROPOSAL]` This must be a hard requirement (see §6/§7 Findings).

### Step 2 — Context retrieval (bounded, logged View invocation)

- **Operation count:** 0 from the human's perspective — this happens automatically once an agent is assigned (Phase 9 DoD step 2 says "without further human input").
- **Page/context jumps:** 0 for the human. For the agent, this is its own internal step, not a UI concern.
- **Information density:** This is where UX-REQ-002 and UX-REQ-004 are in direct tension, and Phase 11 Angle 9 already flagged this pairing as risky-with-zero-teeth. If the human wants to *inspect* what context the agent saw (an auditability affordance CTX-REQ-002 explicitly enables), that inspection surface's density is unconstrained by any current requirement. `[TBD] — needs wireframe` for what a "View invocation, logged" actually looks like rendered.
- **Cognitive load:** None for the human at this point unless they choose to inspect (progressive disclosure applies correctly here — inspection should be opt-in, not default).
- **AI interruption points:** None — this is definitionally Ambient (background context assembly, no human input required per the DoD text itself).
- **Discoverability:** The *existence* of an inspectable context-log is not discoverable unless the Stable Skeleton (§2) gives it a fixed location. `[PROPOSAL]` — see §2.
- **Predictability:** High by construction — CTX-REQ-002 requires the invocation itself to be logged and reconstructable, which is a predictability guarantee at the data layer. Whether the *UI* surfaces it predictably is `[TBD]`.

### Step 3 — Agent Branch (scoped credential, isolated workspace)

- **Operation count:** 0 human-facing — agent-initiated per the loop.
- **Page/context jumps:** 0, unless the human wants to watch it happen live, in which case: does watching require navigating to a separate "AgentRun" page, or does it surface in-context on the Issue/PR the agent is working from? `[TBD] — needs wireframe.` This is the first real test of "Context > Page": if a human has to leave the Issue view to see the Agent Branch's existence, UX-REQ-002 has already failed its own principle at the third loop step.
- **Information density / cognitive load:** N/A unless watched.
- **AI interruption points:** None required — Ambient by design (no approval gate exists yet at this step; the approval gate is later, at Human Approval).
- **Discoverability:** Same open question as Step 2 — is there always a fixed place to see "what is the agent doing right now for this Issue"? `[PROPOSAL]` — this should be part of the Stable Skeleton (§2): a persistent "Agent Activity" affordance attached to whatever Node an agent is working against.
- **Predictability:** High at the mechanism level (SEC-REQ-002/AGT-REQ-005 make isolation guarantees testable). UI predictability `[TBD]`.

### Step 4 — Code Change (commit/diff captured, linked via `produced` Edge)

- **Operation count:** 0 human-facing during production. When the human later reviews it: at minimum 1 (open the diff).
- **Page/context jumps:** This is the step most exposed to Phase 11's Angle 9 risk. A diff review UI that pulls in every connected Edge (per UX-REQ-002's literal wording — "linked Requirement, failing tests, blocking Policy") without a bound is an overload risk the requirement itself already names as a known risk ("over-loading a page with 'everything connected' recreates information overload"). §2/§3 below draw the concrete boundary this step needs.
- **Information density:** High-risk zone. A code diff plus full graph context plus AI-authorship indicators plus CI state is a lot to hold on one screen. Progressive Disclosure (UX-REQ-004) is the only requirement that constrains this, and its own acceptance criterion explicitly defers the "right default bound" to this phase. §5 below sets concrete levels.
- **Cognitive load:** High if density isn't managed; this is the step where "does the user need to hold in their head" matters most — a reviewer must simultaneously track: whose change is this (human or agent), what does it claim to satisfy, what evidence backs that claim, and what's still open.
- **AI interruption points:** Should be Ambient (a diff annotated with AI-origin markers) not Chat (no reason this step needs a chat modal). `[PROPOSAL]` if any implementation surfaces "ask the agent about this diff" as a blocking chat window rather than an ambient annotation, that is an Ambient-AI-budget violation — see §4.
- **Discoverability:** Diff review is a well-understood pattern; low risk here on its own. Risk is entirely in what's layered around it.
- **Predictability:** Same Node type (PR/Commit) must render the same layout skeleton every time — this is the literal fix this phase proposes for Phase 11 Angle 9's finding.

### Step 5 — CI

- **Operation count:** 0 human-facing (automatic).
- **Page/context jumps:** 0 if CI state renders inline on the PR/Commit view (which UX-REQ-002 explicitly names as required content). Real risk if CI results require a separate "pipeline" page — common in existing tools (GitHub, GitLab) and an easy default to fall back into without an explicit Stable Skeleton rule against it.
- **Information density:** Low — pass/fail plus a drill-in is a solved pattern.
- **AI interruption points:** None — Ambient (a status indicator, not a conversation).
- **Discoverability / predictability:** High if inline; `[TBD] — needs wireframe` to confirm inline placement is actually implemented rather than assumed.

### Step 6 — AI Review

- **Operation count:** 0 to produce (agent-initiated); at minimum 1 (open findings) for a human to consume.
- **Page/context jumps:** This is the second sharpest test of the whole document. Phase 9 DoD explicitly requires review output be "retrievable as an artifact/Edge off an AgentRun Node, not a UI-only ephemeral comment" — but "not ephemeral" is a data-durability requirement, not a UX placement requirement. Nothing currently requires AI Review findings to render *inline* on the same PR view a human is already looking at, as opposed to a separate "AgentRun" or "Agent Activity" page the human must navigate to. `[PROPOSAL]` — Findings list item below.
- **Information density:** High-risk if all findings (including low-confidence/low-severity ones) render with equal visual weight — this is a Progressive Disclosure test case, addressed concretely in §5.
- **Cognitive load:** The reviewer must distinguish AI-generated findings from human review comments, and must be able to tell at a glance which findings are backed by Evidence Edges (e.g., linked to a failing test) versus which are unverified AI assertions ("Why > What" — the finding should explain *why* it flagged something, not just assert a problem).
- **AI interruption points:** This is the step most likely to skew toward Chat if implemented naively (a chat-style "AI reviewer commented" thread). §4 stress-tests this directly.
- **Discoverability:** Should be automatic (findings appear without the human seeking them out) — but "automatic appearance" and "Ambient, non-interruptive appearance" are not the same thing; a modal popup announcing findings would be automatic AND a Chat-style interruption. This distinction matters and is not resolved by existing requirement text.
- **Predictability:** Same finding-rendering pattern must apply regardless of which agent/provider produced the review (AI-REQ-001 is provider-abstracted; the UI must not leak provider identity into layout).

### Step 7 — Human Approval

- **Operation count:** UX-REQ-003 requires the approve/reject control be usable "from the same view without navigating to a separate audit screen" — so, ideally, 1 operation (click approve) from the PR/Commit view already open. This is the single most load-bearing UX requirement in the whole loop and is scrutinized in full in §6.
- **Page/context jumps:** 0 is the requirement's own bar. Whether this is actually achievable depends on whether the approval control's *authority check* (does this human have Policy-granted authority — SEC-REQ-001 via GRF-REQ-004) can be resolved fast enough to render inline without a loading-state jump. `[TBD] — needs wireframe/prototype.`
- **Information density:** Must show enough to approve responsibly (what changed, what evidence backs it, who/what proposed it) without becoming a second full-page review. Tension with Step 4/6's already-heavy density — likely the same panel, not a new one.
- **Cognitive load:** This is the step where a human is legally/organizationally accountable for the decision — cognitive load here should be *deliberately* not minimized below a threshold; a "one-click, no context shown" approval button would be a UX anti-pattern that undermines Human Authority by making approval too easy to rubber-stamp. Flagged as a real tension in §8.
- **AI interruption points:** This is the loop's one legitimate blocking point — the loop is *supposed* to stop here. The distinction to test in §4 is whether the block is presented as "Suggested" (a clear actionable control, still user-paced) versus "Chat" (a conversational back-and-forth the human must navigate to resolve).
- **Discoverability:** Must be visually distinct from a completed/human-only action per UX-REQ-003's literal acceptance criterion. `[TBD] — needs wireframe` for the actual visual treatment (color, badge, iconography) — no visual language is specified anywhere in the corpus.
- **Predictability:** An unauthorized human's approval attempt must be rejected consistently (already covered as a negative test in Phase 9 DoD) — but what does the *rejection* look like in the UI? Silently hidden control, vs. visible-but-disabled, vs. visible-and-clickable-then-erroring are three very different UX outcomes with different discoverability/trust implications. `[PROPOSAL]` — Findings list.

### Step 8 — Merge

- **Operation count:** 1 (click merge), gated on step 7 having completed.
- **Page/context jumps:** 0 — should be the same view.
- **Information density / cognitive load:** Low — this is a well-understood, low-risk UI moment once approval has already happened.
- **AI interruption points:** None.
- **Discoverability / predictability:** High — standard pattern, low risk.

### Step 9 — Engineering Graph update

- **Operation count:** 0 for the human in the base loop; 1 (a query) if they choose to verify the full chain (Phase 9 DoD step 9's "one query" requirement).
- **Page/context jumps:** This is where the platform's actual differentiator (per Phase 11 RT-17) either lands as a visible UX moment or stays invisible backend plumbing. If the only way to see "Issue → ... → Merge, fully traced" is a separate graph-query tool a user must consciously open, this differentiator is real but *undiscoverable in the loop itself*. §9 returns to this directly, since it's the crux of RT-17.
- **Information density:** A full traversal result (potentially many Nodes/Edges) is a textbook Progressive Disclosure case — §5 sets levels for this specifically.
- **AI interruption points:** None — purely informational.
- **Discoverability:** `[TBD] — needs wireframe/user testing.` This is the single biggest open discoverability question in the whole loop.
- **Predictability:** High at the data layer (GRF-REQ-008 is a deterministic query); UI predictability depends on whether the same query always renders the same visualization, which is unconstrained today.

---

## 2. The Stable Skeleton, defined concretely

`[PROPOSAL]` This is Phase 12's answer to Phase 11 Angle 9's "no falsifiable bound exists" finding. The following is fixed, unchanging, and identical regardless of Node type, user role, or AI state. Each item is written to be falsifiable — a screen either matches it or it's a defect, not a matter of taste.

1. **Global intent-entry affordance** (UX-REQ-001's realization): a single, persistently visible input/action point exists in the same screen location on every page, for every role, whether or not AI is enabled/reachable. It never disappears, never relocates, and its absence-of-AI-availability degrades gracefully to plain search/navigation rather than vanishing (this also satisfies OPS-REQ-002/GIT-REQ-010's "AI-independent operation" principle at the UI layer, which no current requirement explicitly does — see Findings).
2. **Fixed page-region layout per Node type**: every rendered page for a given Node type (Issue, PR, Commit, Review, AgentRun, Repository, ADR, Incident) uses the same named regions in the same positions across every instance of that type — e.g., a PR page always has a "Diff" region, a "Linked Context" region, an "Evidence/CI" region, and an "Approval" region in fixed positions. Only the *content populated inside* a region varies. This is the literal codification Phase 11 Angle 9 asked for: "the same Node type's page uses the same layout skeleton across all instances; only the content populated within named regions varies by context."
3. **A fixed, single location for the Human Authority approval control**: whenever an Action is pending approval on a Node, the approval control renders in the same named region (from item 2) every time, with the same visual treatment (see §6 for the concrete pattern proposed), regardless of what kind of Action it is or which agent proposed it.
4. **A fixed, single location for "Why"**: per Product Principle 7 (Evidence-Native) and the spec's Why>What principle, every claim, finding, or approval-worthy Action has one predictable place to see its justification/Evidence trail — never a different affordance per Node type.
5. **A fixed keyboard-shortcut scheme, global and role-independent**: the same key always does the same thing everywhere it's active, and a shortcut is either globally bound or explicitly and visibly scoped — it never silently means something different depending on what "emerged" onto the current screen. (This is the direct predictability guarantee the Boundary in §3 depends on.)
6. **A fixed AI-state indicator**: whether AI/agent subsystems are reachable at all (relevant given OPS-REQ-002's independent-disable requirement) is shown in one constant location, so a user is never surprised that an AI-dependent affordance silently does nothing.
7. **A fixed navigation/identity chrome**: primary navigation structure (however minimal, given Intent > Navigation deprioritizes it as the *primary* way to move around) does not itself reflow or reorder based on role or context — role/context changes what content appears in it (see §3), never its existence or position.

Each of these is falsifiable by a straightforward test: capture the same screen region across N different instances (different roles, different Node instances, AI on vs. off) and diff the structural layout, not the content. `[TBD] — needs prototype` to actually build and run this diff test; this document only specifies what the test would check.

---

## 3. What's allowed to be Emergent, and the boundary

Per Phase 8's Perspective Engine mention and UX-REQ-002/UX-REQ-004, the following legitimately vary by role/context/task:

- **Which content populates a fixed region** (§2 item 2): e.g., the "Linked Context" region on an Issue shows different linked Nodes depending on what's actually connected — that's the entire point of Context > Page.
- **Which findings/suggestions surface in the AI Review region**, and their ranking/prioritization — this can and should be context-sensitive (a security-relevant repo surfaces different review emphasis than a docs-only repo).
- **The default disclosure level shown** (see §5) — an expert user may see a denser default than a first-time user, provided the *mechanism* for expanding/collapsing is itself fixed (same interaction, same location, per §2 item 2).
- **Role-scoped visibility of regions' *contents*** — e.g., a Manager's rollup view shows cost/telemetry content a Developer's PR view doesn't, but this is a difference in *which page type* is being viewed (Manager rollup vs. Developer PR page), not a difference in how the *same* page type renders for different roles logged into it.
- **Ambient AI suggestions surfaced inline** — their presence/absence and content is emergent; the *slot* they render into (see §4) is fixed.

**The boundary, stated as the falsifiable rule this phase commits to:** emergent content may change **what** information or action is shown; it must never change **where** a Node type's core regions live, **what** a keyboard shortcut does, or **whether/where** the Human Authority control appears. `[PROPOSAL]` A region is permitted to be *empty* (no linked Requirement exists yet) but is never permitted to *not exist* for that Node type, and is never permitted to *relocate*. This is the concrete test that distinguishes "Emergent Context" (allowed) from "Emergent UI" in the sense Phase 11 Angle 9 was worried about (allowed to break stability) — this phase's answer to the spec's original question is: **no, provided the boundary above is enforced; yes, if it isn't** — and nothing in the current requirement text (UX-REQ-002 as literally written) enforces it. That enforcement gap is Finding F-1 below.

---

## 4. Ambient AI budget check (80/15/5 stress test)

The spec's own target (80% Ambient / 15% Suggested / 5% Chat) is explicitly a design aspiration, not a hard metric — treated here as a check, not a compliance gate. Classifying every AI touchpoint identified in §1's walkthrough:

| Loop step | AI touchpoint | Classification if built well | Classification if built naively | Risk |
|---|---|---|---|---|
| 1. Issue creation | Intent resolution (UX-REQ-001) | Ambient (resolves silently to the right View) | Suggested (shows candidate interpretations to pick from) | Low — either outcome is acceptable, neither is Chat |
| 2. Context retrieval | Bounded context assembly | Ambient (invisible unless inspected) | Ambient | None — this step has no natural pull toward Chat |
| 3. Agent Branch | Branch/workspace setup | Ambient (status indicator) | Ambient | None |
| 4. Code Change | AI-authored diff, in-line annotations | Ambient (diff renders with inline AI-origin markers) | **Chat** (a "chat with the agent about this diff" thread becomes the primary way to understand it) | **Real** — this is the step most likely to regress to Chat if a conversational interface is used as the default explanation mechanism instead of inline annotation |
| 5. CI | none (CI is not itself AI) | N/A | N/A | None |
| 6. AI Review | Findings surfaced | Ambient/Suggested (inline findings, optionally with a "why" expandable) | **Chat** (an "AI reviewer" persona posting comments in a thread, requiring back-and-forth to fully understand) | **Real, and the sharpest risk in the loop** — GitHub Copilot-style and similar tools commonly implement AI review as a comment-thread persona, which is a comfortable, familiar pattern to build but is definitionally Chat-shaped even when no literal chat window opens; a thread the human must scroll and interpret sequentially, one comment at a time, is Chat in substance regardless of UI chrome. |
| 7. Human Approval | Approval decision support (e.g., a summary of what changed and why) | Suggested (a summary panel, not a conversation) | **Chat** (an "ask the AI to explain this change" input the human must engage before they feel able to approve) | **Real** — if the platform's only way to get clarity before approving is to open a chat with the agent, the loop's one legitimate interruption point becomes doubly interruptive: block-and-converse instead of block-and-decide |
| 8. Merge | none | N/A | N/A | None |
| 9. Graph update | Query assistance for traversal (if UX-REQ-001's intent-first entry extends to "show me the chain") | Ambient/Suggested (renders directly) | Suggested (fine either way) | Low |

**Verdict:** `[INFERENCE]` As currently *specified* (not yet built), the loop does not inherently skew toward Chat — most steps have no natural pull toward it. But two steps (Code Change's AI annotations, and AI Review's findings) are genuine risk points where the easiest, most familiar implementation path (a comment-thread UI, borrowed directly from existing PR-review tooling) is Chat-shaped even without looking like a literal chat window, and nothing in UX-REQ-002/003/004 as written prevents that default. **Being honest per the task brief: the requirements corpus does not yet guarantee an 80/15/5 (or even a directionally similar) skew — it is achievable given the Stable Skeleton in §2 (which reserves a fixed *inline annotation* region rather than a thread), but is not guaranteed by anything currently written down.** This is Finding F-2 below.

---

## 5. Progressive Disclosure check

Testing UX-REQ-004 concretely against two UI surfaces, each with two disclosure levels, for a first-time vs. expert user doing the same MVP loop:

**Surface A — the "Why" trace (Evidence Edges backing a claim, e.g., "tests passed")**
- Level 1 (default, both users see this): a single-line summary badge — "✓ 3/3 checks passed" — with no Edge detail shown.
- Level 2 (expanded, one click/keypress from Level 1, same control for both users): the actual Evidence Edges, their source (which CI run, which Node), and timestamp.
- `[PROPOSAL]` Difference by expertise: identical mechanism for both users (§2's predictability rule requires this) — the *default expansion state* may differ (an expert user's preference could persist Level 2 as their default), but the interaction path to get there must be the same control in the same place for both. This satisfies Progressive Disclosure without violating the Stable Skeleton boundary in §3.

**Surface B — the AI Review findings panel**
- Level 1 (default): count and severity-bucketed summary ("2 High, 4 Medium findings") with the single highest-severity finding shown inline.
- Level 2 (expanded): full findings list, each with its own further disclosure (finding text at first, "why" — the reasoning/evidence — one more click in, per Why>What).
- `[PROPOSAL]` A third, genuinely different level for expert users specifically: a "diff-annotated" mode where findings render as inline diff comments rather than a separate panel at all — this is a real difference in information architecture, not just expansion state, and is the strongest concrete instance of Progressive Disclosure actually resolving something (see §8, Tension 1).

**Honest assessment:** Today, Progressive Disclosure is aspirational — UX-REQ-004's own acceptance criterion explicitly defers "the right default bound" to this phase, and this phase can only propose concrete levels (as above), not verify them, since no wireframe or prototype exists to test against real users. `[TBD] — needs wireframe/prototype/user testing` for both surfaces above.

---

## 6. Human Authority UX check

Phase 9 DoD step 7's requirement — "a Human with Policy-granted authority sees the pending approval as a visibly distinct, in-context, actionable control" — is **not concretely specified enough to build today.** It names three properties (visibly distinct, in-context, actionable) but specifies none of their visual/interaction realization. Tightening:

`[PROPOSAL]` Concrete UI pattern:
- **Visibly distinct:** a persistent, high-contrast badge/border treatment reserved exclusively for pending-human-approval state — never reused for any other status (not CI-pending, not AI-review-in-progress) so that its meaning is unambiguous at a glance, consistent with §2's predictability rule.
- **In-context:** rendered inside the fixed "Approval" region (§2 item 3) on the same page the human is already viewing the change from — never a redirect, never a separate "pending approvals" inbox as the *only* path (a secondary rollup inbox for scanning across many pending approvals is fine and useful, but must not be the sole path to acting on any individual one).
- **Actionable:** the control itself performs the approval/rejection directly (a real button invoking the Policy-gated Action), not a link that navigates elsewhere to perform it — and the negative case (unauthorized human) is proposed to render the same fixed region as *visibly present but disabled with an inline reason* ("requires X role"), rather than hidden — hiding it would fail discoverability and could read as the feature not existing at all, while disabled-with-reason preserves both Human Authority's transparency goal and the negative-access guarantee already required by Phase 9 DoD's negative test.
- **Deliberate friction, not zero-click:** per §1 Step 7 and §8 Tension 2 below, this control should require an explicit confirming action (not a single accidental click) and should keep the evidence panel (§5 Surface A) visible in the same view at the moment of decision, so approval is never possible without the supporting evidence being at least present on-screen, even if collapsed.

This remains `[TBD] — needs wireframe/prototype` for actual visual execution, but the interaction contract above is concrete enough to hand to a designer, which the original one-sentence requirement was not.

---

## 7. Accessibility and keyboard-first check

`[TBD] — needs full accessibility audit.` This phase cannot certify WCAG conformance without an actual implementation. What can be stated as a requirement now, based on the loop walkthrough in §1 and the Stable Skeleton in §2:

- **Statable now:** every action required to complete the MVP loop (create Issue, open PR, approve, merge, expand disclosure levels, invoke intent-entry) must have a keyboard-reachable path, because §2 item 5 already requires a fixed, global keyboard scheme — a loop with a mouse-only approval control would directly violate that Stable Skeleton item, not just accessibility best practice generally. This is a natural consequence of §2, not a new principle.
- **Statable now:** the Human Authority approval control (§6) must be operable via keyboard with a confirming step (not a single stray keypress triggering a Policy-gated merge-adjacent action) — same "deliberate friction" logic as §6.
- **`[TBD]`:** actual screen-reader semantics for a "context-assembled" page whose region contents vary (§3) — dynamic content regions are a known screen-reader challenge (live-region announcement timing, focus management on content change) and nothing in this phase's walkthrough can resolve that without a prototype.
- **`[TBD]`:** color-only distinction risk in §6's "visibly distinct" badge treatment — must not rely on color alone (a colorblind-safe secondary indicator, e.g. an icon or label, is needed) but which specific WCAG level (AA vs AAA) the platform targets is a product decision not made anywhere in the corpus to date.
- **`[TBD]`:** the diff/review-dense surfaces in Steps 4/6 (§1) are exactly the kind of information-dense UI most prone to failing keyboard/screen-reader navigation in practice across the industry; a specific navigation-order and landmark-region spec is needed before this can be called done.

---

## 8. Beginner vs Expert workflow tension

Two real, concrete tensions found in this pass:

**Tension 1 — Dense inline AI annotations (expert-optimized) vs. a first-time user's unfamiliarity with what "AI-authored" even means in this diff.** An expert benefits from inline, terse AI-origin markers directly in the diff (§4/§5 Surface B's proposed expert mode) — but a first-time user seeing an unfamiliar inline marker with no explanation may not know it's actionable or what it means, actively hurting their ability to complete the loop at all. **Resolution proposed:** Progressive Disclosure's Level 1 default for first-time users includes a one-time, dismissible inline explainer the first time an AI-origin marker is encountered (a "what's this" affordance attached to the marker itself, not a separate onboarding flow) — this uses the same disclosure mechanism (§5) rather than a bespoke onboarding system, keeping it consistent with the Stable Skeleton. `[PROPOSAL]`

**Tension 2 — Keyboard-first, single-action efficiency (expert-optimized) vs. Human Authority's requirement that approval carry deliberate friction (§6) so it isn't a rubber stamp.** These directly conflict: an expert wants "approve" reachable in one keystroke; the platform's own Human Authority principle argues against a single accidental keystroke being able to approve a Policy-gated Action. **This tension is NOT fully resolved by Progressive Disclosure** — disclosure level is about *how much information is shown*, not about *how much friction an action requires*, so it's the wrong tool for this particular tension. `[PROPOSAL]` honestly stated: the resolution proposed here is a deliberately un-collapsible confirmation step for approval specifically (a second keystroke/click that cannot be disclosure-collapsed away even for expert users), accepting a small efficiency cost at exactly one point in the loop as the intentional price of Human Authority remaining real rather than nominal. This is flagged as a tension Progressive Disclosure does not resolve, per the task brief's explicit instruction to admit when one doesn't.

A third, lower-severity tension worth naming: **discoverability of the intent-first entry point (§2 item 1) for first-time users vs. keyboard-shortcut power use for experts** — a persistent visible input satisfies discoverability but a keyboard shortcut to focus it satisfies expert efficiency; these are not in conflict (both can coexist on the same control), so this one is resolved by construction, not flagged as an open tension.

---

## 9. Direct response to Phase 11's Emergent-UX finding and RT-17

**On Angle 9 (Emergent UX / stability):** Phase 11 found the phrase "Stable Skeleton + Emergent Context" nowhere in the requirements text, and found UX-REQ-002 as literally written to be an unbounded, teeth-free promise. This phase's response is not a relabeling: §2 above enumerates seven falsifiable, testable Stable Skeleton items, and §3 states the boundary rule ("emergent content changes *what*, never *where*") as a literal constraint that a future wireframe or implementation can be checked against — this is exactly the "stability contract" Phase 11's own mitigation proposal (§110 of that document) asked for. It is a genuine tightening because it is falsifiable (§2's structural-diff test) where Phase 11 correctly found nothing falsifiable existed. What this phase does **not** do — and says so plainly — is verify the contract against a real implementation, because none exists yet; that verification remains `[TBD] — needs prototype`, which is the honest limit of a document-only review.

**On RT-17 (the 10x check):** Phase 11 found only 1 of 3 candidate 10x differentiators (cross-object traceback) ships its full experiential impact at MVP; the other two (unified review surface, provider-agnostic Gateway) ship backend mechanism only, with the UX moment deferred to V1 alongside UX-REQ-002. This UX pass was asked whether there's a UX-only way to make the other two visible at MVP without expanding MVP scope.

- **Unified review surface:** `[PROPOSAL]` — Yes, partially, and it is a genuine UX-only move. Phase 9's MVP already requires (DoD step 6/7) that AI Review findings and Human Approval both be retrievable/actionable off the same AgentRun/PR Node — the *data* for a unified surface is already MVP. What's V1-deferred per RT-17 is UX-REQ-002's full "context-assembled page" apparatus. But this pass's Stable Skeleton (§2 item 2 — fixed regions per Node type) is a much smaller UX commitment than full UX-REQ-002, and if the PR page's fixed regions (Diff, Linked Context, Evidence/CI, Approval — §2) are populated with the MVP-available data specifically (not the full emergent context-assembly engine, just the fixed skeleton with MVP data slotted in), a Reviewer already sees findings, evidence, and approval in one view without navigating to five tools — which is the actual experiential claim RT-17 says is missing. **This proposal does not require pulling UX-REQ-002 forward in full; it requires only the Stable Skeleton's fixed-region layout (§2), applied to MVP-available data.** That is a materially smaller ask than RT-17's own mitigation proposal (pulling a "minimal slice" of UX-REQ-002 forward), and this document independently arrives at essentially the same recommendation via the UX-review lens, which — per the same logic Phase 11 itself used when RT-01 and RT-17 converged — strengthens the case rather than weakening it.
- **Provider-agnostic Gateway:** No. `[PROPOSAL]` re-examined here honestly: there is no UX-only way to make "swap providers without touching config" visible if only one provider is actually wired up at MVP (per AI-REQ-003's demotion in Phase 9 §2 and RT-17's own note that MVP ships "effectively one hardcoded provider"). A UI element implying provider choice when only one provider exists would be actively misleading — closer to a dark pattern than a UX win. This document declines to propose a UX workaround here and says so plainly, matching the task brief's instruction to admit when the answer is no.

---

## 10. Findings

Numbered, severity-tagged, each citing the requirement/principle affected.

**F-1 (Critical).** UX-REQ-002 as currently written has no enforceable Stable Skeleton constraint — Phase 11 Angle 9 found this already; this phase confirms it's still true of the requirement text itself (only this document's §2/§3 proposal fixes it, and that proposal is not yet part of the master doc). Affects: UX-REQ-002, Product Principle 6 (Graph-Native) applied to UI, Product Principle 8 (Human Authority) indirectly (an unstable approval-region location would itself be a Human Authority failure). Recommend Phase 13 adopt §2/§3 of this document as UX-REQ-002's binding stability contract, not just note it as resolved.

**F-2 (High).** Nothing in the requirements corpus prevents the Code-Change-annotation and AI-Review-findings steps (§1 Steps 4/6) from regressing to a Chat-shaped comment-thread pattern, which is the single most common failure mode for this exact kind of feature industry-wide. Affects: Product Principle "Ambient AI > Chatbot" (§13, referenced but not directly quoted at §34), UX-REQ-002. The 80/15/5 aspiration is achievable but not guaranteed by current text (§4).

**F-3 (High).** Phase 9 DoD step 7's Human Authority language ("visibly distinct, in-context, actionable") has zero visual/interaction specification — not buildable as written. Affects: UX-REQ-003, Phase 9 DoD step 7. §6 of this document proposes a concrete pattern; not yet adopted into the master doc.

**F-4 (Medium).** The negative case for an unauthorized human's approval attempt (Phase 9 DoD's own negative test) has no specified UI treatment (hidden vs. disabled-with-reason vs. visible-then-erroring) — three materially different UX/trust outcomes, unresolved. Affects: UX-REQ-003, SEC-REQ-001.

**F-5 (Medium).** Discoverability of the Engineering Graph's full-chain traceback (Step 9, §1) inside the loop itself — as opposed to a separate graph-query tool the user must consciously open — is unresolved, and this is precisely the platform's strongest claimed differentiator per RT-17. If it's undiscoverable in-loop, it doesn't function as a 10x moment regardless of backend completeness. Affects: GRF-REQ-008, UX-REQ-001, RT-17 finding.

**F-6 (Medium).** Progressive Disclosure (UX-REQ-004) has two concrete levels proposed in this document (§5) but zero validated against real users — the requirement's own acceptance criterion already flagged "the right default bound" as undetermined, and it remains undetermined after this pass, only now with a concrete starting proposal instead of none. Affects: UX-REQ-004.

**F-7 (Medium).** Beginner/expert friction tension at the approval step (§8, Tension 2) is not resolved by Progressive Disclosure and has no other requirement addressing it; left as an accepted, deliberate tradeoff in this document's proposal, but not yet a recorded product decision. Affects: UX-REQ-003, UX-REQ-004, Product Principle 8.

**F-8 (Low).** No accessibility conformance target (WCAG level) is stated anywhere in the corpus; §7 identifies several concrete `[TBD]` items but the overarching target itself is an open product decision, not merely an execution detail. Affects: UX-REQ (general), no existing requirement ID covers accessibility explicitly — this is itself a gap worth naming (there is no `A11Y-REQ` prefix anywhere in the master doc).

**F-9 (Low).** The intent-first entry point's behavior when AI is disabled (OPS-REQ-002) is not specified — does it degrade to plain search, or does the affordance disappear? §2 item 1 proposes graceful degradation as a Stable Skeleton requirement, but this is not yet in the master doc. Affects: UX-REQ-001, OPS-REQ-002, GIT-REQ-010's "AI-independent operation" principle applied to the UI layer specifically (currently only specified at the protocol/data layer).

**Severity rollup:** Critical: 1 (F-1). High: 2 (F-2, F-3). Medium: 4 (F-4, F-5, F-6, F-7). Low: 2 (F-8, F-9). Total: 9.

---

## 11. Open Questions

**Product Decision**
- `[TBD]` Whether Phase 13 adopts this document's §2/§3 Stable Skeleton + Emergent Context boundary as UX-REQ-002's binding stability contract verbatim, or revises it — this document proposes it but cannot bind the master doc (F-1).
- `[TBD]` Whether the §9 proposal (apply the Stable Skeleton's fixed-region layout to MVP-available data as a UX-only way to realize RT-17's "unified review surface" differentiator at MVP) is accepted, and if so whether it changes Phase 9's MVP/V1 line for any part of UX-REQ-002 — this is a scope decision Phase 13 must make explicitly, not infer from this document.
- `[TBD]` Whether an explicit `A11Y-REQ` requirement domain and a stated WCAG conformance target should be added to the master doc — currently absent entirely (F-8).
- `[TBD]` Whether §8 Tension 2's "deliberate friction at approval, unresolved by Progressive Disclosure" is accepted as-is or needs a different resolution — flagged as a genuine unresolved tension, not a design already settled.

**Architecture/Design Decision**
- `[TBD]` — needs wireframe: the actual visual treatment for Human Authority's "visibly distinct" state (§6), the AI-origin markers in diffs (§4/§5 Surface B), and the negative-approval-attempt UI (F-4).
- `[TBD]` — needs prototype: whether §2's structural Stable Skeleton diff-test is practically implementable as an automated regression check (analogous to a visual-regression test suite) or requires manual design review only.
- `[TBD]` — needs prototype: dynamic-content-region screen-reader semantics for context-assembled pages (§7).

**Research Needed**
- `[TBD]` — needs user testing: whether the §5 disclosure levels proposed for the "Why" trace and AI Review findings panel actually match real first-time-vs-expert usage patterns; this document's levels are a starting proposal, not validated.
- `[TBD]` — needs user testing: whether Tension 1's proposed one-time dismissible explainer (§8) actually helps first-time users or is ignored/annoying in practice — a common failure mode for onboarding-affordance proposals generally.
- `[TBD]` — needs user testing: real-world validation of the §9 "fixed-region-with-MVP-data" proposal against actual Reviewer persona feedback, per Phase 11's own recommendation that Phase 12 validate RT-09/RT-17 "against real persona feedback, not just internal reasoning" — this document is internal reasoning only; it does not satisfy that recommendation, and says so.

---

## Next Phases

- **Phase 12 — UX Red-Team Review**: this document.
- **Phase 13 — Final Baseline**: must resolve this document's Product Decision open questions as binding calls, decide whether to adopt §2/§3 as UX-REQ-002's stability contract, and reconcile this document's findings against Phase 11's RT-01/RT-09/RT-17 (all three converge, from independent angles, on UX-REQ-002/its Stable-Skeleton realization as the single highest-leverage unresolved item carried into the final baseline).
