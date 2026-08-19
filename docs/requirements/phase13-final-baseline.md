# AI-Native Engineering Platform — Requirements Research: Phase 13 (Final Baseline)

**Status:** v1.0 — **this document closes the 13-phase program and declares Baseline v1.0.** **Scope:** Phase 13 only (Final Baseline, per original spec §48–50). Builds on and resolves every finding in `docs/requirements/phase11-red-team.md` (17 findings, RT-01 through RT-17) and `docs/requirements/phase12-ux-review.md` (9 findings, F-1 through F-9) — **26 findings total**. *(Corrected during the Phase 15 final audit: this line previously said "16 findings … 25 findings total", inheriting a miscount from Phase 11's own severity rollup, which had omitted RT-08 from its table. Every individual finding, RT-08 and F-5 included, was written up in full and dispositioned in §1 below — the error was confined to the headline counts, not the substance. See `phase15-final-audit.md` finding A-3.)* This document does not re-derive prior research; it disposes of open findings and makes binding calls on questions previous phases correctly left open.

**Tagging convention** (inherited unchanged): `[FACT]` / `[UNVERIFIED-FACT]` / `[INFERENCE]` / `[PROPOSAL]` / `[TBD]`.

**Read in full before writing this document:** `phase1-5-research.md`, `phase6-primitives.md`, `phase7-elicitation.md`, `00-requirements-definition.md` (master doc, all 55 sections), `phase9-mvp-reduction.md`, `phase10-architecture.md`, `phase11-red-team.md`, `phase12-ux-review.md`.

---

## 0. How to read this document

Section 1 dispositions every one of the 25 Phase 11/12 findings individually. Section 2 summarizes which master-doc edits were actually made (the edits themselves live in `00-requirements-definition.md` and `phase9-mvp-reduction.md`, not duplicated here). Section 3 is the Changes-After-Review log. Section 4 is the mandatory Three Moats analysis. Section 5 answers the governing question. Section 6 declares Baseline v1.0. Section 7 is the consolidated master Open Questions list.

**Disposition counts** (26 findings total — 17 from Phase 11, 9 from Phase 12):

| Disposition | Count | Finding IDs |
|---|---|---|
| **Accepted-and-fixed** | 19 | RT-01, RT-02, RT-03, RT-08, RT-09, RT-10, RT-11, RT-12, RT-13, RT-14, RT-17, F-1, F-2, F-3, F-4, F-5, F-7, F-8, F-9 |
| **Accepted-deferred** | 4 | RT-04, RT-05, RT-15, F-6 |
| **Rejected** | 2 | RT-06, RT-07 |
| **Needs-human-decision** | 1 | RT-16 |

All 3 Critical findings from Phase 11 (RT-10, RT-14, RT-17) and the 1 Critical finding from Phase 12 (F-1) are **Accepted-and-fixed**, per this phase's mandatory bar for Critical findings.

---

## 1. Disposition of every Phase 11 and Phase 12 finding

### 1.1 The four Criticals (mandatory Accepted-and-fixed or well-argued Rejected)

**RT-10 (Critical) — Platform-process compromise is an uncovered threat model.**
**Disposition: Accepted-and-fixed.**
This is the single most consequential finding of the entire program. Phase 10's monolith-first architecture — itself correctly justified against microservice over-engineering, and not reopened here — concentrates Policy evaluation, agent/CI credential issuance, sandbox spawning, and audit-Event writes in one process. Every AISEC-REQ item (§36) was written assuming that process is trustworthy; none addressed its compromise as a distinct scenario. **Fix applied:** new requirement **AISEC-REQ-009** (master doc §36) requires database-role-level append-only enforcement on the `events` table (an `INSERT`-only PostgreSQL grant, independent of application-layer code) plus an internal privilege-separation review for the credential-issuance/Policy-evaluation code paths. This is timed **MVP** for the database-role half — it is cheap (a grant, not new infrastructure) and closes the highest-severity finding of the whole program — with the review half allowed to land at V1 under capacity pressure. A new Risk Register entry, **R13** (§53), records the scenario explicitly, distinct from R3/R7/R8. This directly restores the credibility of Principle 5 (Agent-Native) and Principle 7 (Evidence-Native), both of which depend on the audit trail surviving compromise of the one component every other guarantee assumes is trustworthy.

**RT-14 (Critical) — MVP ships with zero AI cost visibility for a persona that pays the bill directly.**
**Disposition: Accepted-and-fixed.**
Phase 9 demoted AI-REQ-002 to V1 on a load-bearing-for-loop-*completion* test that is correct on its own terms but blind to a distinct risk: the self-hosted-adopter persona (OPS-REQ-001/002/003's exact target) runs real metered API spend against every loop iteration with zero requirement-guaranteed visibility until V1, which is worse than nearly every competitor's own baseline tooling (per Phase 1-5's competitive research) and inconsistent with Phase 9's own reasoning for keeping AGT-REQ-006 (identical risk class — "can trivially run away on cost/time") at MVP. **Fix applied:** master doc §27 amends AI-REQ-002 to pull a **minimal cumulative-spend-visibility slice** into MVP — a running total, not the full per-request dashboard — with an explicit disclosure-requirement fallback if even that proves infeasible at implementation time. MVP must not ship silent on this dimension either way.

**RT-17 (Critical) — The MVP may ship plumbing without the 10x moment.**
**Disposition: Accepted-and-fixed.**
Of three candidate 10x differentiators derivable from this program's own stated value proposition (one-query cross-object traceback; a unified, agent-and-human-equal review surface; a provider-agnostic AI Gateway), only the first ships its full experiential impact at MVP under Phase 9's cuts — the other two ship backend mechanism only. Phase 9's load-bearing-for-completion test never asked "which 3 things would make a user switch, and are they still here" — a distinct question RT-17 is right to say produces a different, non-overlapping cut set. Independently, RT-01 (Angle 1, "is this just a GitHub clone with AI bolted on?") converged on the same requirement (UX-REQ-002) as the fix, and Phase 12 §9 independently re-derived essentially the same proposal from a pure UX-review angle — three independent angles converging on one fix strengthens rather than weakens the case. **Fix applied:** see F-1's disposition below (UX-REQ-005) — the same fix resolves RT-01, RT-17, and F-1/F-5 simultaneously, which is recorded once here and cross-referenced rather than triplicated. The provider-agnostic-Gateway differentiator is *not* rescued by a UX-only move (Phase 12 §9 examined this honestly and found no defensible UX workaround when only one provider is wired up — a fake "choose your provider" UI would be a dark pattern); its architectural-discipline half is instead addressed via RT-02's fix (AI-REQ-005), which restores *credibility* of the claim without faking the experience.

**F-1 (Critical) — UX-REQ-002 as written has no enforceable Stable Skeleton constraint.**
**Disposition: Accepted-and-fixed.**
Phase 11 Angle 9 (RT-09) found the phrase "Stable Skeleton + Emergent Context" nowhere in the requirements text; Phase 12 confirmed the gap and then closed it with a concrete, falsifiable proposal (§2/§3 of that document). **Fix applied:** master doc §34 adopts Phase 12 §2/§3 **verbatim** as a new, independently-traceable requirement, **UX-REQ-005** — the seven-item Stable Skeleton list, the boundary rule ("emergent content changes *what*, never *where*"), and the falsifiable structural-diff test. Per Phase 12 §9's converging proposal (and RT-01/RT-17's independent convergence on the same point), a **minimal fixed-region slice** of UX-REQ-005 is pulled forward to **MVP**: the PR/Issue page reviewed at Phase 9 DoD step 7 must render its fixed regions populated with MVP-available data. This single fix disposes RT-01, RT-09, RT-17, F-1, and F-5 together — the master doc records this once at UX-REQ-005 and each finding below cross-references it rather than duplicating the fix language.

### 1.2 Remaining Phase 11 findings (RT-01 through RT-17, excluding the three Criticals above)

**RT-01 (Medium) — Angle 1: is this just a GitHub clone with AI bolted on?**
**Disposition: Accepted-and-fixed.** Resolved by UX-REQ-005's MVP minimal slice (see RT-17/F-1 above) — the specific mitigation RT-01 proposed (render at least one graph fact inline on the page a human reviews during the DoD's manual checkpoint) is exactly what UX-REQ-005's MVP slice now requires.

**RT-02 (High) — Angle 2: is this AI-washing?**
**Disposition: Accepted-and-fixed.** Master doc §27 amends AI-REQ-005: its architectural-discipline half (a fitness-function test proving a second no-op/mock provider can be swapped in without touching call sites) is pulled forward to MVP, mirroring the CLOUD-REQ-001 precedent Phase 9 itself established for a structurally identical problem. This makes Principle 4's "Gateway, not a single-vendor bolt-on" claim testable at MVP rather than merely asserted.

**RT-03 (Medium) — Angle 3: is this over-engineered? (AISEC-REQ-005 test-surface mismatch)**
**Disposition: Accepted-and-fixed.** Master doc §36 rescopes AISEC-REQ-005's MVP acceptance criterion: the Policy-scope-containment logic remains MVP (enforced in code), but the transitive multi-hop test is honestly deferred to V1 alongside its dependency AGT-REQ-009, rather than implying an MVP-era test that has nothing to attack yet.

**RT-04 (High) — Angle 4: is the Engineering Graph actually necessary?**
**Disposition: Accepted-deferred, to V1.** This is fundamentally a "prove it" gap no amount of further requirements analysis can close — RT-04's own text says so ("no full mitigation available at requirements-research level"). **Why not Rejected:** the finding is correct that no prior phase has shown a concrete task GitHub's weak linking cannot accomplish, only that it's less coherent architecturally. **Why not Accepted-and-fixed now:** the fix RT-04 itself proposes (a benchmarked comparison of the graph-native query against a scripted GitHub GraphQL equivalent) requires a working MVP to run against — it cannot be executed by more requirements synthesis. **Deferred action, recorded as binding:** this comparison benchmark is added to §44's Performance Requirements benchmark list (already `[TBD] – Benchmark Required` in that section) as an explicit named item, and to the V1 rollout as an acceptance test for the "necessary, not merely nicer" claim specifically — if the GitHub-API reconstruction proves comparably fast and correct at MVP-representative scale, this document commits in advance to downgrading the graph's positioning language from "necessary" to "more convenient and more consistent," a materially smaller but still real claim.

**RT-05 (Medium) — Angle 5: was Memgraph correctly rejected, or did Phase 10 under-scope future needs?**
**Disposition: Accepted-deferred, to V1/V2.** Phase 10's rejection of Memgraph for MVP stands — nothing in this finding disputes the MVP-scale reasoning. What RT-05 correctly identifies is that Phase 10's revisit trigger for introducing a dedicated graph engine was implicit (buried in a parenthetical about "a V2/Future capability... greenlit for build") rather than a named, first-class line item, and that the Incident Responder persona's core JTBD (§6, cross-repo traceback) is exactly the kind of capability that trigger should name explicitly. **Action, recorded as binding:** `phase10-architecture.md` §1.3's revisit trigger is amended (by pointer, since Phase 10 documents are not rewritten in place per this program's convention of dated phase documents) to name "the Incident Responder's cross-repo `caused_by` traceback capability (DTWIN-REQ-002) being greenlit for build" as an explicit, first-class revisit-trigger condition, not merely an example nested inside a general parenthetical — recorded in this document's §7 Open Questions (Architecture Decision) so it is not lost.

**RT-06 (Low) — Angle 6: is microservices-early avoided, or is MVP secretly fragmented?**
**Disposition: Rejected.** Phase 11 itself concluded "no action needed at the architecture-topology level" after genuine adversarial effort, correctly distinguishing a real (but narrower) risk — internal modularity discipline within one large process — from the finding's original framing (secret microservice fragmentation). That narrower risk is an implementation/code-review concern, not a requirements-phase finding; nothing in this program's method (requirements and architecture *decisions*, not code review of an implementation that does not yet exist) can meaningfully act on it further. **Reasoning for outright rejection rather than deferral:** there is no requirement or architecture-decision-shaped action to take — "write modular code" is not a testable acceptance criterion this document can add without becoming code-review guidance masquerading as a requirement, which this program's own discipline (Phase 7 §1.3's "testable" bar) argues against.

**RT-07 (Low) — Angle 7: does Intent Commit add user burden without proportional value?**
**Disposition: Rejected.** Phase 11 found ITC-REQ-001/002 already self-defended this exact risk in their own text (explicitly forbidding gating `git push` on intent annotation, and naming the "developers route around a perfunctory field" risk directly). ITC-REQ is Future-timed with zero MVP/V1 exposure. **Reasoning for rejection:** there is no near-term exposure to mitigate, and the one residual concern Phase 11 itself named (informal social pressure eroding "optional" in practice) is a UX-research question for whichever future phase actually builds Intent Commit, not something a Future-tier, unbuilt requirement's *text* can be usefully hardened against today — adding speculative acceptance criteria for a feature years from being built would violate this program's own discipline against inventing unbenchmarked detail (§44's convention, applied here by analogy).

**RT-08 (High) — Angle 8: is Semantic Diff trustworthy? (Evidence-Edge epistemic-status gap)**
**Disposition: Accepted-and-fixed.** Master doc §25 amends GRF-REQ-010 to require a mandatory, non-optional `epistemic_status` enum (`verified` / `asserted` / `attested`) on every Evidence-typed Edge, rendered distinctly in any UI surface showing Evidence. This closes the gap RT-08 identified precisely: DIFF-REQ-002's careful "advisory, not authoritative" language for the Experimental semantic-diff feature does not, by itself, stop the identical trust problem entering through AI Review — a mainstream MVP-adjacent feature — under a different requirement ID. Timed V1 (GRF-REQ-010 itself is V1), but now a binding, non-optional acceptance criterion rather than an implicit expectation.

**RT-09 (Medium) — Angle 9: does Emergent UX risk unpredictability?**
**Disposition: Accepted-and-fixed.** Resolved by UX-REQ-005 (see F-1 above) — the same fix.

**RT-11 (Medium) — Angle 11: is local deployment too heavy?**
**Disposition: Accepted-and-fixed.** Phase 10's per-component weight discipline was genuinely sound; what was missing was a *cumulative* footprint check across Platform process + PostgreSQL + concurrent agent/CI sandboxes, and OPS-REQ-005 (the requirement that would catch this) is V1-timed, leaving MVP permitted to ship with zero documented resource-footprint claim despite Principle 2's "local-first... checked from day one" language. **Fix applied:** rather than pulling all of OPS-REQ-005 forward (a larger ask than the finding requires), Phase 9's Definition of Done step 10 (the chaos test) gains an added, **informational, non-gating** obligation: record peak concurrent RAM/CPU during the run. This is cheap (an addition to an already-required test) and converts "local-first, lightweight" from an assumed brand claim into an actually-measured one before MVP's first real self-hosted users, without making OPS-REQ-005's full documented-and-bounded acceptance criterion an MVP gate it was never load-bearing enough to be.

**RT-12 (High) — Angle 12: is the MVP still too big? (independent re-derivation)**
**Disposition: Accepted-and-fixed**, item-by-item as RT-12 itself structured the finding:
1. AGT-REQ-006 — RT-12 defended this as correctly kept; no action needed (this sub-item was never in dispute).
2. SEC-REQ-004 — genuine gap (MVP-timed but untested by the MVP's own DoD, since the DoD's agent is pre-provisioned). **Fixed** by honest documentation rather than DoD expansion: master doc §48 now records SEC-REQ-004 as "enforced in code, untested by the MVP automated suite until V1's DoD revision adds a new-agent-onboarding step" — consistent with this program's discipline of not silently carrying an inconsistency forward (the same discipline Phase 9 itself applied to GRF-REQ-009/010).
3. CTX-REQ-002 — RT-12 flagged a justification-consistency issue (defended as load-bearing for the loop's *point*, not its mechanical *completion*) but did not ask for a scope change. **No requirement change**; the distinction is recorded here for future phases' benefit, consistent with RT-12's own recommendation.
4. API-REQ-001's "documented" half — RT-12 flagged this as a genuine candidate for further narrowing (parallel to the API-REQ-003 precedent) but did not assert it should be cut. **No change made** — this document declines to narrow API-REQ-001 further without stronger justification than "it's structurally similar to a prior cut"; a documented API surface at MVP is cheap and directly supports AISEC-REQ-006's anti-Policy-bypass concern, which weighs against narrowing it.
5. UX-REQ-003's minimal-satisfying-implementation gap — **Fixed** via UX-REQ-007 (see F-3 disposition below), which adds the exact minimum-legibility bar RT-12 asked for.

**RT-13 (High) — Angle 13: did Phase 10 underestimate Git infrastructure complexity?**
**Disposition: Accepted-and-fixed** (primary items), **Accepted-deferred** (the monorepo load-test sub-item). Master doc §17 adds two fixes: (a) a new requirement **GIT-REQ-011** (repository maintenance/GC scheduling) closing the zero-prior-coverage gap RT-13 correctly identified — no phase of this program had mentioned Git GC/repack at all; timed V1, since a fresh MVP demo instance never accumulates enough loose objects to need it; (b) an amendment to GIT-REQ-006 extending REL-REQ-002's existing fail-closed-on-timeout precedent explicitly to the synchronous Git-hook callback path, with a concurrent-push non-serialization requirement, closing the concurrency-analysis gap RT-13 identified; timed V1. The **monorepo per-push ingestion-volume load test** RT-13 also flagged is genuinely a benchmark, not a requirements decision — it is added to §7 (Open Questions, Research Needed) rather than invented here, consistent with §44's discipline against fabricating performance figures.

**RT-15 (High) — Angle 15: should mature open-source components have been reused instead of building custom?**
**Disposition: Accepted-deferred, to a required pre-implementation ADR.** RT-15's finding is well-argued and its recommended action (evaluate, don't assume) is correct, but its own text already concedes the likely eventual conclusion is defensible ("build against real `git` directly... that conclusion, if reached, should be an explicit, argued ADR decision, not an unexamined default"). This is not a requirements-document fix — it is a mandate that implementation begin with a documented evaluation rather than an implicit default. **Action, recorded as binding:** added to §7 (Open Questions, Architecture Decision) as a required pre-implementation ADR covering both the Gitea/Forgejo embed-vs-build question and Apache AGE as a middle option in the graph-storage revisit-trigger discussion (extending RT-05's fix above with RT-15's specific candidate). **Why deferred rather than fixed now:** this program's own discipline (Phase 10 §7's ADR-required-list convention) treats exactly this class of decision as belonging to a dedicated ADR at implementation time, not to a retroactive requirements-document edit asserting an answer this program has no new evidence to support.

**RT-16 (High) — Angle 16: why would a user choose this over GitHub/GitLab? (switching cost / cold-start graph)**
**Disposition: Needs-human-decision.** This is the one finding in the entire disposition log that genuinely cannot be resolved by further analysis. RT-16 correctly identifies that this program's 100+ requirements contain no symmetric import capability (GitHub/GitLab Issue/PR/CI history → this platform's graph) and that the graph differentiator is least valuable on day one for a switching team, precisely when the graph is empty. The unresolved question is not a requirements gap this document can fill by writing a new IMPORT-REQ domain speculatively — it is a **go-to-market positioning decision** (target new projects/teams first vs. build import tooling to court migrations) that determines whether such a domain should exist at all, and at what priority. Writing binding requirements for a go-to-market stance this program has no authority or evidence to decide would be scope creep beyond what a requirements-research program should resolve unilaterally. **Recorded, not silently dropped:** added to §7 (Open Questions, Product Decision) as a named, explicit decision for product/business leadership, with the two concrete options RT-16 itself named (build symmetric import as a V1/V2 requirement domain; or explicitly state in Non-Goals that MVP targets new projects/teams and cold-start-graph is an accepted, deliberate adoption-cost tradeoff) preserved verbatim so the decision-maker has the real options in front of them, not a re-summarized version.

### 1.3 Remaining Phase 12 findings (F-2 through F-9, excluding F-1 above)

**F-2 (High) — nothing prevents Chat-shaped regression at Code-Change-annotation/AI-Review steps.**
**Disposition: Accepted-and-fixed.** Master doc §34 adds **UX-REQ-006**, requiring AI-authored diff annotations and AI Review findings to render as inline, ambient annotations in a fixed region (per UX-REQ-005) rather than a conversational comment-thread — a hard shape constraint, not a preference, since Phase 12 §4 found this is the single most common failure mode for exactly this kind of feature industry-wide. Timed V1 (alongside UX-REQ-002/005's full apparatus), since MVP's minimal fixed-region slice does not yet build the full AI Review findings panel this constrains.

**F-3 (High) — Human Authority language has zero visual/interaction specification.**
**Disposition: Accepted-and-fixed.** Master doc §34 adds **UX-REQ-007**, adopting Phase 12 §6's concrete pattern (visibly-distinct badge treatment reserved exclusively for pending-approval state; in-context rendering in the fixed Approval region; directly actionable, not a link; minimum legibility — shows the specific Action and actor) as UX-REQ-003's binding acceptance-criteria addendum. Timed MVP, since it amends an already-MVP requirement without changing its MVP status.

**F-4 (Medium) — unauthorized-approval-attempt UI treatment unspecified.**
**Disposition: Accepted-and-fixed.** Folded into UX-REQ-007 (see F-3 above): the negative case is specified as visibly-present-but-disabled-with-inline-reason, never silently hidden and never visible-then-erroring, resolving the three-way ambiguity F-4 identified. Timed MVP, same as UX-REQ-007.

**F-5 (Medium) — Engineering Graph traceback discoverability in-loop is unresolved.**
**Disposition: Accepted-and-fixed.** Resolved by UX-REQ-005 (see F-1/RT-17 above) — the same fix; F-5 is a specific instance of the same discoverability gap RT-17 named at the program level.

**F-6 (Medium) — Progressive Disclosure levels proposed but unvalidated.**
**Disposition: Accepted-deferred, to V1 with user testing.** Phase 12 §5 proposed concrete disclosure levels for two surfaces (the "Why" trace, the AI Review findings panel) but explicitly flagged them as a starting proposal, not validated against real users — this is honestly a research/testing gap, not a specification gap this document can close by asserting the levels are correct. **Action, recorded as binding:** Phase 12 §5's specific proposed levels are adopted as the **starting default** for UX-REQ-004's implementation (so V1 does not start from nothing), with mandatory user-testing validation recorded in §7 (UX Validation) before those defaults are treated as final.

**F-7 (Medium) — beginner/expert friction tension at approval, unresolved by Progressive Disclosure.**
**Disposition: Accepted-and-fixed.** Master doc §34 (via UX-REQ-007, see F-3 above) adopts Phase 12 §8 Tension 2's proposed resolution explicitly: a deliberately un-collapsible confirmation step for approval, accepting a small, permanent efficiency cost at exactly one point in the loop as the intentional price of Human Authority remaining real rather than nominal. **Why Accepted-and-fixed rather than Needs-human-decision:** Phase 12 already did the hard reasoning and proposed a specific, well-argued resolution; the only thing missing was this program formally recording acceptance of that resolution rather than leaving it "not yet a recorded product decision" as Phase 12 itself flagged. This document now records it as accepted.

**F-8 (Low) — no accessibility conformance target stated anywhere.**
**Disposition: Accepted-and-fixed.** Master doc §34 adds **A11Y-REQ-001**, stating a WCAG 2.1 Level AA conformance target — the first accessibility-domain requirement in the program — with the keyboard-reachability half (a direct, low-cost consequence of UX-REQ-005 item 5) pulled forward to MVP and the full conformance audit at V1. **Why WCAG 2.1 AA specifically, and why this document can make that call:** AA is the widely-adopted industry-standard target (used as the compliance bar in most jurisdictions' accessibility law, e.g. the EU's EN 301 549 and the U.S. ADA Title II regulations as commonly applied) and requires no new product research to select defensibly — this is a standards-adoption decision, not a novel product judgment call, so it does not need to be pushed to Needs-human-decision the way RT-16's go-to-market question does.

**F-9 (Low) — intent-first entry point's AI-disabled behavior unspecified.**
**Disposition: Accepted-and-fixed.** Folded into UX-REQ-005 item 1 (graceful degradation to plain search, never disappearing) — master doc §34 records this explicitly as closing F-9's gap.

---

## 2. Master document edits actually applied

Every "Accepted-and-fixed" disposition above corresponds to a real edit in `00-requirements-definition.md`, not merely a description in this document. Summary of what was touched (see the file itself for full text):

- **§25 (Engineering Graph Requirements):** GRF-REQ-010 amended with mandatory `epistemic_status` enum (RT-08).
- **§27 (AI Gateway Requirements):** AI-REQ-002 amended with minimal MVP cost-visibility slice (RT-14); AI-REQ-005 amended with MVP architectural-discipline gate (RT-02).
- **§17 (Git Requirements):** new GIT-REQ-011 (repository maintenance/GC, RT-13); GIT-REQ-006 amended with hook-latency/concurrency requirement (RT-13).
- **§34 (UX Requirements):** four new requirements — **UX-REQ-005** (Stable Skeleton + Emergent Context, RT-01/RT-09/RT-17/F-1/F-5), **UX-REQ-006** (Ambient-by-default AI rendering, F-2), **UX-REQ-007** (Human Authority concrete pattern, F-3/F-4/F-7), **A11Y-REQ-001** (WCAG 2.1 AA target, F-8) — plus a UX-REQ-001 amendment (F-9).
- **§36 (AI Security Requirements):** new **AISEC-REQ-009** (Platform process integrity, RT-10, the program's highest-priority fix); AISEC-REQ-005 acceptance criterion rescoped (RT-03).
- **§48 (MVP Definition):** MVP scope amended to add the minimal slices above; MVP requirement count revised 37 → 43; SEC-REQ-004's untested-at-MVP status honestly documented (RT-12).
- **§53 (Risk Register):** new **R13** (Platform-process compromise, RT-10).
- **§54 (Open Questions):** superseded in favor of this document's §7, per the task's required consolidation.
- **Front matter (Status line):** updated to declare Baseline v1.0 and point to this document.
- **`phase9-mvp-reduction.md`:** addendum appended recording the Phase 13 MVP-scope delta against Phase 9's own Cut/Keep Lists and DoD, without altering Phase 9's original reasoning.

---

## 3. Changes After Review

A scannable log of what changed as a direct result of Phases 11–12, organized by type. This is the single most important section for a reader who wants to see red-team input actually reshape the baseline, not just get summarized.

### Added (new requirement IDs that did not exist before Phase 13)
- **AISEC-REQ-009** — Platform process integrity (database-role-level append-only audit enforcement). *Reason: RT-10.* **MVP** (database-role half) / V1 (privilege-separation review half).
- **UX-REQ-005** — Stable Skeleton + Emergent Context, the binding stability contract Phase 11 found missing and Phase 12 supplied. *Reason: RT-01, RT-09, RT-17, F-1, F-5.* **MVP** (minimal fixed-region slice) / V1 (full apparatus).
- **UX-REQ-006** — Ambient-by-default rendering, forbidding Chat-shaped regression at the two highest-risk loop steps. *Reason: F-2.* V1.
- **UX-REQ-007** — Human Authority concrete visual/interaction pattern, amending UX-REQ-003's acceptance criterion. *Reason: F-3, F-4, F-7, RT-12 item 5.* **MVP.**
- **A11Y-REQ-001** — WCAG 2.1 AA conformance target, the program's first accessibility-domain requirement. *Reason: F-8.* MVP (keyboard-reachability half) / V1 (full audit).
- **GIT-REQ-011** — Repository maintenance (GC/repack) scheduling, a zero-prior-coverage gap. *Reason: RT-13.* V1.
- **R13** — Platform-process compromise, Risk Register. *Reason: RT-10.*

### Amended (existing requirement IDs whose acceptance criteria changed)
- **GRF-REQ-010** — mandatory `epistemic_status` enum added. *Reason: RT-08.*
- **AI-REQ-002** — minimal cumulative-spend-visibility slice pulled into MVP. *Reason: RT-14.*
- **AI-REQ-005** — architectural-discipline half (fitness-function test) pulled into MVP. *Reason: RT-02.*
- **AISEC-REQ-005** — MVP acceptance criterion rescoped to remove an untestable multi-hop claim. *Reason: RT-03.*
- **GIT-REQ-006** — hook-callback latency/concurrency requirement added. *Reason: RT-13.*
- **UX-REQ-001** — AI-disabled degradation behavior specified (via UX-REQ-005 item 1). *Reason: F-9.*
- **Phase 9 DoD step 10** — gains an informational, non-gating peak-resource-usage measurement obligation. *Reason: RT-11.*
- **SEC-REQ-004's MVP status** — no requirement-text change, but its untested-by-the-MVP-suite status is now explicitly documented rather than silently inconsistent. *Reason: RT-12 item 2.*

### Downgraded / Narrowed (nothing this phase found was strong enough to demote out of the baseline entirely — see the note below)
- **No requirement was downgraded in priority or removed from scope as a result of Phase 11/12.** This is itself worth stating plainly: this program's red-team and UX-review phases found gaps to *fill* (Critical/High findings pointing at missing coverage) far more than scope to *cut* — a different shape of finding than Phase 9's own MVP-reduction pass, which was explicitly a cutting exercise. The two phases were doing structurally different jobs, and their outputs look different as a result; this is not evidence Phase 11/12 were less rigorous, only that adversarial review of an already-cut MVP tends to surface omissions more than redundancies.

### Deferred (accepted as real, explicitly pushed to V1/V2/a required pre-implementation ADR, not silently dropped)
- Full RT-04 benchmark comparison (graph-native query vs. scripted GitHub GraphQL equivalent) — V1 acceptance test, added to §44's benchmark list.
- RT-05's revisit-trigger sharpening (naming the Incident Responder's cross-repo traceback capability as an explicit trigger condition) — recorded in §7 below, targets `phase10-architecture.md`'s trigger language at the next architecture-document revision.
- RT-13's monorepo per-push ingestion-volume load test — Research Needed, §7.
- RT-15's Gitea/Forgejo embed-vs-build and Apache AGE middle-option evaluations — required pre-implementation ADRs, §7.
- F-6's Progressive Disclosure default levels — adopted as V1 starting defaults, pending mandatory user-testing validation, §7.

### Rejected (with reasoning, not merely dropped)
- RT-06 (secret MVP fragmentation) — the real, narrower risk it surfaced (in-monolith modularity discipline) is a code-review concern with no testable requirements-level acceptance criterion to add; see §1.2.
- RT-07 (Intent Commit user burden) — zero near-term exposure (Future-timed, already self-defended in its own requirement text); adding speculative hardening to an unbuilt Future-tier requirement would violate this program's own anti-fabrication discipline; see §1.2.

### Needs-human-decision (genuinely not resolvable by more analysis)
- RT-16 — whether to build symmetric GitHub/GitLab history import (courting migrations) or explicitly position MVP for new-project adoption (accepting the cold-start-graph tradeoff). This is a go-to-market decision for product/business leadership, not a requirements gap; both concrete options are preserved verbatim in §7 below.

---

## 4. The Three Moats

Per original spec §49, this section identifies and rigorously argues exactly three core moats. Phase 5's Gap Analysis flagged three candidate differentiators: **(a)** the knowledge graph, **(b)** true self-hosted + cloud parity with agent-native execution, **(c)** unified audit trail across human and agent actions. Phase 9 §4 tested all three for survival against its own MVP cuts and found all three intact. This section re-tests them one more time — now against Phase 11/12's adversarial findings and this phase's fixes — and asks the harder question Phase 9 did not: are these genuinely *moats* (durable competitive protection), or merely *differentiators* (real but erodible advantages)? The distinction matters and this section does not collapse it.

**Method:** each candidate is scored against seven dimensions per the task brief — Competitor Difficulty, User Value, Technical Defensibility, Data Flywheel, Network Effect, Switching Cost (both directions), and AI Advancement Resistance — with an honest verdict on each, including admitting where a dimension is weak or absent.

### Moat 1 — The unified, queryable engineering graph (requirement→ADR→issue→PR→test→release→incident→agentrun)

- **Competitor Difficulty:** High, but not for the reason usually claimed. RT-04's own finding is instructive here: GitHub/GitLab could technically bolt a formal graph API onto their existing Issues/PR/Discussions data with moderate engineering effort — the *data* mostly already exists, cross-linked informally. What is genuinely hard to replicate is the *architectural discipline* of having built every subsystem against a shared Node/Edge/Event substrate from day one (Phase 6 §2's pattern extraction: coherence is a side effect of a single shared data model, not an add-on feature). GitHub's O1 finding (three weakly-linked systems built independently over 15+ years) is not a bug they could patch — it is 15+ years of independent product-team decisions that would require a genuine internal rearchitecture, not a new API surface, to undo. `[INFERENCE]`
- **User Value:** Real but currently *unproven at MVP scale* per RT-04's own honest framing — this document does not overstate it. The value is highest for the Incident Responder and Architect personas' JTBDs (§6) and compounds with graph size and history depth (RT-16's own point about cold-start graphs cuts against Moat 1's day-one value for a switching team, which is directly relevant here).
- **Technical Defensibility:** Medium. Phase 10's single-PostgreSQL decision is a good MVP-scale engineering choice, but nothing about the *graph model itself* (Node/Edge/Event/Policy/View) is patentable or otherwise legally exclusive — it is a well-understood combination of the labeled-property-graph and event-sourcing patterns (Phase 6 §5's own honest "prior-art check"). Defensibility comes from execution and accumulated data, not from the idea being novel.
- **Data Flywheel:** **Yes, and this is the strongest property of this moat.** Every loop iteration (Issue → ... → Merge → Graph Update) adds Nodes/Edges/Events that make every subsequent query, context assembly, and traceback more valuable — this is a genuine compounding effect, not merely more data of the same fixed value. A competitor starting today needs not just the architecture but years of accumulated graph history to match the value this moat delivers to an established deployment.
- **Network Effect:** **No, and this document says so plainly rather than inventing one.** The graph's value to one team does not increase because other teams (at other companies, on other instances) also use the platform — each self-hosted or cloud-tenant graph is siloed by design (a direct, deliberate consequence of the self-hosted-and-private-by-default positioning). There is no cross-tenant network effect here, unlike (say) GitHub's OSS-hosting network effect.
- **Switching Cost (user considering adoption):** **Negative in the near term, positive long-term** — this is RT-16's finding stated as a moat property rather than merely a risk: a team switching *to* this platform starts with an empty graph (this document does not paper over that), while a team switching *away* *after* years of accumulated graph history faces a real, compounding switching cost *against* leaving, precisely because of the Data Flywheel property above. The moat protects retention far more than it drives initial adoption — a real asymmetry worth stating honestly.
- **Switching Cost (platform vendor's own lock-in risk):** Low, by explicit design — DATA-REQ-001 (full graph export in an open format) exists specifically so the platform's own moat does not become a customer-hostile lock-in mechanism, unlike Linear's zero-export self-hosting gap (Phase 3 O7/Phase 5). This is a deliberate tradeoff: the moat is allowed to be weaker as a pure lock-in mechanism in exchange for the trust the anti-lock-in positioning buys (Risk R12).
- **AI Advancement Resistance:** **High.** A 10x-smarter frontier model does not, by itself, give a competitor a persistent, queryable, typed graph of an organization's own engineering history — it makes any individual *query* against such a graph better-answered if the graph exists, but does nothing to make the graph exist at a competitor lacking one. This moat is largely orthogonal to frontier-model capability, which is one of the few genuinely reassuring findings in this analysis.
- **Verdict:** **Survives as a real moat**, but weaker on Day 1 and stronger the longer a team stays — a retention moat more than an acquisition moat. This is a more honest characterization than Phase 5's original framing, which did not distinguish acquisition-time value from retention-time value.

### Moat 2 — True self-hosted + cloud parity with agent-native execution

- **Competitor Difficulty:** Medium-High, but asymmetric across competitors. For GitHub specifically: high, because Enterprise Server is documented as declining/niche (`[FACT]`, Phase 3) — reversing that trend is an organizational-priority decision, not a technical one, and Microsoft's own commercial incentives (Enterprise Cloud pricing, GitHub AI Credits) point away from re-investing in self-hosted parity. For GitLab specifically: **lower than this program's earlier phases implied.** GitLab already offers genuine self-hosted parity (CE/EE, Duo Self-Hosted air-gapped operation) — Phase 10's own finding is that GitLab's self-hosting is *operationally heavy* (Gitaly/Praefect/Sidekiq/Workhorse), not *absent*. A determined GitLab engineering investment in simplifying that operational weight is a real, plausible threat to this specific moat that this document does not want to understate.
- **User Value:** High for the exact persona this platform's positioning targets (§4: "organizations that need to run AI coding agents at scale without losing self-hosting control") — but this is a narrower value proposition than Moat 1's, since it matters most to security/compliance-sensitive or air-gapped-requirement organizations specifically, not universally.
- **Technical Defensibility:** Medium. Phase 10's single-process, single-PostgreSQL, no-fork architecture (§4/§5) is a genuinely disciplined engineering choice, but the *idea* of "same codebase, different deployment topology" is not defensible IP — GitLab already does a version of this (same CE/EE binary, feature-gated). What's defensible is *not recreating GitLab's own operational weight* while doing it — an execution bet, not a structural one.
- **Data Flywheel:** No direct flywheel — self-hosting capability does not compound with usage the way the graph does.
- **Network Effect:** No.
- **Switching Cost (user):** Low as a switching-*to* driver (a team already self-hosting GitLab CE could plausibly self-host this platform too, with comparable effort), but this moat's real value is *retention against forced-cloud-migration pressure* — the same pressure Phase 3's evidence shows pushed most large GitHub Enterprise Server customers to Cloud by 2023–2026. A team on this platform never faces that pressure, by construction (CLOUD-REQ-001's no-fork discipline, now MVP-gated via AI-REQ-005's fitness-function precedent extended architecture-wide).
- **Switching Cost (vendor lock-in risk):** Low — DATA-REQ-004 (no phone-home) makes self-hosted operation genuinely independent, consistent with the anti-lock-in stance across all three moats.
- **AI Advancement Resistance:** **Medium, and this is the moat most exposed to frontier-model advancement, stated honestly.** A 10x-smarter model narrows the *agent-execution-quality* half of this moat's value proposition somewhat — if every frontier model becomes dramatically better at coding regardless of which platform hosts it, "agent-native execution" as a *capability* differentiator (not the self-hosting-*parity* half, which is orthogonal to model quality) erodes toward commodity, consistent with Phase 5's own tiering of "basic AI coding assist" as Commodity, not Differentiator. What survives frontier-model advancement is specifically the *self-hosted-parity* half (running that better model on infrastructure you control, audited, at the same capability as cloud) — not the execution quality itself.
- **Verdict:** **Survives, but only the self-hosting-parity half is genuinely durable; the "agent-native execution" half is the most AI-advancement-exposed component of any of the three moats and should be understood as time-limited value, not permanent differentiation.**

### Moat 3 — Unified audit trail across human and agent actions

- **Competitor Difficulty:** Medium. GitHub's audit log is a genuinely separate subsystem today (`[FACT]`, O2, the April 2026 outage), and unifying it with a formal agent-action model would require real rearchitecture — but this is the *weakest* of the three moats against a determined competitor response, because unlike Moat 1 (15+ years of independently-built object types) this is a narrower, more addressable engineering gap: a competitor could plausibly ship "agent actions are Policy-gated and audited the same as human actions" as a discrete feature investment without needing the full graph-native rearchitecture Moat 1 would require. RT-10's own finding sharpens this further: this program's own architecture (Phase 10's monolith) had a real, previously-uncovered gap in this exact moat (Platform-process compromise) — now fixed via AISEC-REQ-009, but a reminder that "we audit agents like humans" is a claim that must be continuously re-earned against new threat models, not a one-time architectural decision.
- **User Value:** High specifically for compliance-sensitive organizations and the Reviewer/Manager personas' JTBDs (§6) — "show me the evidence in one place" and "one queryable source of truth" are both direct expressions of this moat's value.
- **Technical Defensibility:** Medium-Low, honestly. The mechanism (Event-sourced audit trail, Policy gates applied uniformly to Agent and Human Nodes) is, per Phase 6 §5's own prior-art check, a straightforward application of event-sourcing plus RBAC — nothing here is technically novel. What's defensible is the *product decision* to treat Agent as a first-class Node subtype from day one (§3.8/§5 of Phase 6) rather than bolting agent-audit on after the fact — a decision, not an invention.
- **Data Flywheel:** Weak-to-moderate. Longer audit history has compounding forensic/compliance value (more incidents traceable, more patterns detectable), but this is a smaller effect than Moat 1's graph flywheel, since audit-trail value per-Event is roughly constant rather than growing combinatorially with graph connections.
- **Network Effect:** No.
- **Switching Cost (user):** Real for compliance-locked organizations (an established, continuous audit history has genuine regulatory/legal value that resets if you switch platforms), but narrower in applicability than Moat 1's general retention effect — it matters most to a specific buyer profile, not universally.
- **Switching Cost (vendor lock-in risk):** Low — SEC-REQ-007 (compliance-scoped Event export) and DATA-REQ-005 (export includes historical Event data) exist specifically so this moat's own audit history remains portable, consistent with the program's anti-lock-in stance.
- **AI Advancement Resistance:** **High.** Nothing about a smarter frontier model reduces the value of, or makes trivially replicable, a structural commitment to auditing agent actions identically to human ones — if anything, smarter and more autonomous agents make this moat *more* valuable over time (more autonomous action needs more rigorous audit), not less. This is the moat with the clearest positive correlation to AI advancement rather than exposure to it.
- **Verdict:** **Survives, and is the moat most likely to *strengthen* as frontier models improve** — but it is also the weakest of the three on Technical Defensibility and Competitor Difficulty, meaning it is the one most likely to be directly copied by a determined competitor investment, even though the underlying value only grows.

### Cross-moat honesty check

None of the three moats has a genuine network effect — this is the most important honest admission in this section. This platform's value does not increase because more organizations adopt it (unlike GitHub's OSS-hosting gravity or Linear's workspace-collaboration effects); each deployment is siloed by design. All three moats' Switching-Cost value is asymmetric — weak or negative at initial adoption (RT-16's cold-start problem applies to all three, most severely to Moat 1), strong at retention. This is a **retention-moat portfolio, not an acquisition-moat portfolio** — a materially different, more honest characterization than Phase 5's original "differentiator" framing implied, and it has direct go-to-market consequences (reinforcing RT-16's Needs-human-decision status: a retention-moat portfolio argues for a new-project-first go-to-market, since acquisition value against an established GitHub/GitLab deployment is comparatively weak on all three moats simultaneously — but this document does not make that call unilaterally, consistent with §1.2's RT-16 disposition).

---

## 5. The governing question

**"If GitHub and GitLab shipped 10x better AI tomorrow, why would this product still exist?"**

The honest answer, after 13 phases of work: **it would still exist, but only for the subset of organizations for whom self-hosted control, agent-inclusive audit rigor, and an accumulated engineering graph already matter more than raw AI capability — and it would exist as a smaller, more durable niche than "AI-native platform" positioning implies, not as a mass-market GitHub/GitLab replacement.** A 10x-better frontier model shipped by GitHub or GitLab tomorrow would immediately erode the weakest part of Moat 2 (agent-execution *quality* as a differentiator, per §4's analysis) — that half of the value proposition was never going to be durable, because it depends on this platform's own AI Gateway routing to the same frontier models everyone else can also route to; owning the Gateway abstraction does not make the underlying model smarter. But it would not touch Moat 1's core property (an organization's own accumulated, queryable engineering graph does not appear on GitHub/GitLab merely because their AI got better — the data model gap Phase 1-5's O1/O5 evidence documents is structural, not capability-limited) or Moat 3 (a better model does not make GitHub's audit log architecturally unified with agent actions — if anything, per §4, better/more autonomous agents make *unified, structural* audit *more* valuable, not less, strengthening rather than weakening this specific moat). The platform's honest value proposition, stated without marketing softening, is therefore: **not "better AI than GitHub/GitLab," which this program's own evidence (Phase 5's Commodity-tier classification of basic AI coding assist) already conceded was never the plan — but "the same frontier-model quality everyone else has, running against a graph and audit substrate GitHub/GitLab's own product history (15+ years of independently-built Issues/PR/Discussions, per O1) makes structurally expensive for them to retrofit."** If that structural gap is what a buyer actually values, a 10x-better competitor AI does not close it. If a buyer's real priority turns out to be raw agent capability above all else — which RT-17's own finding (only 1 of 3 candidate 10x moments ships its full experience at MVP) suggests is a real risk this program has not yet proven wrong — then this product's case weakens considerably, and this document says so plainly rather than asserting confidence this program's own evidence does not support. **The unresolved condition, stated as "it might not, unless X" per this section's own instruction:** it might not persist as a differentiated product, unless the graph and audit substrate actually deliver the compounding, felt value §4's Data Flywheel analysis argues for — and that has not yet been demonstrated with a working MVP, only argued for on paper. RT-04's own finding (no prior phase has shown a concrete task the graph does something GitHub's weak linking genuinely cannot) is the sharpest form of this caveat, and this document does not resolve it — it commits, per §1.2's RT-04 disposition, to a concrete V1 benchmark that could prove this analysis wrong.

---

## 6. Baseline v1.0 declaration

This document, together with the edits it makes to `00-requirements-definition.md` and `phase9-mvp-reduction.md`, marks **Baseline v1.0** of the AI-Native Engineering Platform requirements-definition program.

**What this baseline covers:** Phases 1 through 13, complete — competitor research and gap analysis (Phases 1–5), product primitive discovery (Phase 6), requirements elicitation (Phase 7), the full 55-section requirements specification (Phase 8, as amended by this document), MVP reduction against the Minimum Complete Loop test (Phase 9, as amended by this document's addendum), architecture design (Phase 10), red-team review (Phase 11) and UX red-team review (Phase 12) with every finding from both dispositioned in §1 above, and this final synthesis, moat analysis, and consolidated open-questions list (Phase 13).

**What explicitly remains outside this baseline's scope**, not silently deferred but named here as a deliberate boundary:
- **Implementation.** No code exists or is implied to exist by this baseline; every acceptance criterion in this document and the master doc is a specification for work not yet begun.
- **Detailed UI wireframes and prototypes.** Every `[TBD] — needs wireframe`/`needs prototype` item Phase 12 flagged (concrete visual treatment for Human Authority's badge state, AI-origin diff markers, the Stable Skeleton's actual pixel layout, screen-reader semantics for dynamic regions) remains genuinely open — this program specifies *constraints* a design must satisfy, not the design itself.
- **Legal/licensing review.** R11 (the platform's own open-source licensing/governance model) and the GDPR/right-to-erasure determination for the immutable Event log (§54's Legal Review category, carried into §7 below) are both explicitly unresolved — this program has no authority to make legal determinations and does not attempt to here.
- **Benchmark execution.** Every `[TBD] – Benchmark Required` item across §44 (Performance Requirements) and this document's own additions (RT-04's graph-necessity comparison, RT-13's monorepo ingestion-volume test) remains genuinely unmeasured — this program's discipline against inventing performance figures (stated explicitly since Phase 8 §44) holds through to this final document; no number is fabricated here either.
- **The RT-16 go-to-market decision.** Explicitly Needs-human-decision, per §1.2 — this baseline does not resolve it, and should not be read as implying either answer.
- **A formal ADR document set.** The Appendix ADR Required List (master doc, end of file) names 14 architecture decisions; several are RESOLVED by Phase 10, but the *formal ADR artifacts themselves* (as distinct from this research program's reasoning) are not produced by this program — Phase 10 §7 says so explicitly, and this document does not change that.

**Baseline maintenance going forward:** this document's §1 disposition log is the authoritative record of how every Phase 11/12 finding was resolved; the master doc's §54 pointer (amended by this document) directs future readers to this document's §7 rather than a stale, un-consolidated list. Future changes to the requirements baseline should be tracked as dated amendments referencing this document, not as silent edits that lose the disposition history this phase worked to establish.

---

## 7. Master Open Questions Consolidation

Merges Phase 8 §54, Phase 9/10/11/12's individual Open Questions lists, and every new item surfaced by this phase's dispositions into one deduplicated master list, categorized per the established convention. This supersedes the master doc's own §54 (which now points here).

### Research Needed
- Gerrit's review-centric model, Argo CD/Workflows' deployment/rollback state machine, and CodeQL/Sentry's policy/incident precedents remain unresearched (Phase 3 O13, carried through Phases 6–8); affects CI-REQ-006's rollback acceptance criteria. *(carried from §54)*
- Git LFS support parity across competitors was asserted `[UNVERIFIED-FACT]` (GIT-REQ-004), not independently re-verified. *(carried from §54)*
- Graph database/event-sourcing implementation options — resolved by Phase 10 for MVP (single PostgreSQL); the deferred question is now narrower: whether `gix` (gitoxide)'s production maturity relative to `libgit2` should be re-checked at implementation time (`phase10-architecture.md` §8), since Rust-Git-ecosystem maturity changes quickly. *(carried from phase10-architecture.md §8, supersedes the original phase6 item)*
- MinIO's and Grafana OSS's current license terms — flagged `[UNVERIFIED-FACT]` in Phase 10, should be re-verified immediately before V1+ adoption. *(carried from phase10-architecture.md §8)*
- PostgreSQL single-primary write-throughput ceiling for this platform's specific Node/Edge/Event write-amplification pattern at cloud multi-tenant scale — genuinely needs a benchmark. *(carried from phase10-architecture.md §8)*
- **[NEW, Phase 13]** Whether GitHub's GraphQL API plus existing weak-linking features can reconstruct Phase 9 DoD step 9's traversal with comparable correctness and acceptable latency — a concrete comparative benchmark that materially strengthens or weakens the "graph is necessary, not merely nicer" claim (RT-04, dispositioned §1.2 as Accepted-deferred to V1).
- **[NEW, Phase 13]** Per-push graph-ingestion volume bound for large monorepos — needs a load test against a real large repository (RT-13).
- What compliance/regulatory retention-erasure requirements (e.g., GDPR right-to-erasure) actually demand of an append-only Event log — needed before GRF-REQ-003's exception path can be finalized. *(carried from §54, cross-listed under Legal Review)*

### Benchmark Needed
- Every category in master doc §44: Git operation latency at monorepo scale, graph query latency, context-assembly latency, Policy-evaluation latency, search freshness window, concurrent-user ceiling, horizontal scale-out throughput. *(carried from §54; Phase 10 unblocked the architecture to benchmark against but did not run the benchmarks)*
- BKP-REQ-002's RTO/RPO figures. *(carried from §54)*
- REL-REQ-001's specific availability target. *(carried from §54)*
- Recursive-CTE traversal default depth/row-count limits for GRF-REQ-008 (`phase10-architecture.md` §7 ADR item 14). *(carried from phase10-architecture.md §8)*
- **[NEW, Phase 13]** RT-04's graph-necessity comparison benchmark (graph-native traversal vs. scripted GitHub GraphQL equivalent) — see Research Needed above; listed here too since it is simultaneously a benchmark and a research question.
- **[NEW, Phase 13]** Peak concurrent RAM/CPU during the Phase 9 DoD chaos test (step 10) — now an informational measurement obligation (RT-11), not gating, but the actual numbers are unmeasured until implementation.

### Product Decision
- Whether a managed/multi-tenant cloud offering (CLOUD-REQ-004) is in scope for an early release. *(carried from §54)*
- Default resource-limit values for agent runs (AGT-REQ-006). *(carried from §54)*
- Which specific Agent Actions require human approval by default (AGT-REQ-002). *(carried from §54)*
- Whether Action and Evidence (GRF-REQ-009/010) should be promoted from conventions to independent first-class primitives — depends on usage telemetry not yet available. *(carried from §54/phase6)*
- Whether Context should ever be materialized as a persisted object vs. purely a logged View invocation. *(carried from §54/phase6)*
- Whether Policy evaluation is synchronous/blocking by default vs. advisory-first. *(carried from §54/phase6)*
- Draft/WIP PR state and stacked/dependent PR chains — confirm MVP-relevance vs. safe deferral. *(carried from §54)*
- Notification/mention delivery mechanics — confirm intentional deferral. *(carried from §54)*
- The platform's own licensing/governance model (R11). *(carried from §54, cross-listed under Legal Review)*
- Whether the CLOUD-REQ-001/AI-REQ-005-style architectural-fitness-function-check pattern should be formalized as a standing CI-enforced discipline across every "un-cut, build-time-only" gate this baseline now has (CLOUD-REQ-001, AI-REQ-005, and by extension AISEC-REQ-009's database-role check) — Phase 9 and Phase 10 both flagged versions of this enforcement-mechanism question separately; this document notes they should likely be solved once, uniformly, rather than per-gate. *(consolidates phase9-mvp-reduction.md §8 and phase10-architecture.md §8's Product Decision items)*
- OPS-REQ-004 (backup/restore)'s "immediate V1" placement has no enforcement mechanism (should MVP general-availability announcement be blocked until it ships?) — still open, Phase 13 did not resolve this Phase 9-flagged question. *(carried from phase9-mvp-reduction.md §8)*
- **[NEW, Phase 13 — Needs-human-decision]** Whether to build symmetric GitHub/GitLab Issue/PR/CI-history import as a V1/V2 requirement domain, or explicitly position MVP go-to-market toward new projects/teams and accept the cold-start-graph tradeoff in Non-Goals (RT-16). **This is the one finding in the entire program's Phase 11/12 review that this document could not resolve by analysis** — both options are preserved here for product/business leadership.

### Architecture Decision
- Cloud-tier Policy evaluation model (in-process-per-instance vs. centralized) — still open for the multi-node cloud case specifically (self-hosted case resolved). *(carried from phase10-architecture.md §7 item 2/§8)*
- Sandbox technology for agent/CI execution (container/OCI vs. microVM vs. other). *(carried from phase10-architecture.md §7 item 13/§8)*
- Exact `git`-CLI/`libgit2`/`gix` operation boundary. *(carried from phase10-architecture.md §7 item 12/§8)*
- Secrets store technology and Policy-mediated access design. *(carried from phase10-architecture.md §7 item 7)*
- MCP tool-scoping/curation mechanism implementation. *(carried from phase10-architecture.md §7 item 8)*
- GDPR/retention-erasure exception path technical design — pending the Legal Review item below. *(carried from phase10-architecture.md §7 item 9)*
- Graph state versioning/snapshotting retention/compaction policy. *(carried from phase10-architecture.md §7 item 3)*
- Implementation language/framework choice (Rust vs. alternatives) — explicitly not resolved by requirements research; needs an engineering-leadership-owned ADR. *(carried from phase10-architecture.md §7 item 11)*
- **[NEW, Phase 13]** `phase10-architecture.md` §1.3's graph-database revisit trigger should name "the Incident Responder's cross-repo `caused_by` traceback capability (DTWIN-REQ-002) being greenlit for build" as an explicit, first-class trigger condition, not merely an example nested in a parenthetical (RT-05, dispositioned §1.2 as Accepted-deferred).
- **[NEW, Phase 13]** A required pre-implementation ADR evaluating (a) embedding/extending Gitea or Forgejo for the Git forge/web layer instead of building against `git` directly, and (b) Apache AGE as a middle option in the graph-storage revisit-trigger discussion, before either is dismissed by default (RT-15, dispositioned §1.2 as Accepted-deferred).
- **[NEW, Phase 13]** Whether the architecture-fitness-function enforcement pattern (see Product Decision above) should be a single, standing CI discipline applied uniformly across CLOUD-REQ-001/AI-REQ-005/AISEC-REQ-009, or decided per-gate.

### Legal Review
- GDPR/right-to-erasure implications for the immutable Event log. *(carried from §54)*
- Licensing/governance model choice (R11) — has a legal dimension (open-core structuring, trademark, contributor agreements) beyond the product-positioning question. *(carried from §54)*

### Security Review
- Whether "Platform process compromise" (RT-10) is now adequately addressed by AISEC-REQ-009's database-role-level fix, or whether the internal-privilege-separation-review half (deferred to V1 under capacity pressure) needs to be treated as equally non-negotiable — **recommend a dedicated security-architecture review before V1 treats AISEC-REQ-009 as closed**, since this document's own Accepted-and-fixed disposition covers the cheaper, higher-leverage half first and should not be read as claiming full resolution. *(supersedes the phase11 item on this topic)*
- CDX-REQ-003's provenance-completeness gap for external-agent-delegated tasks (an external agent's internal tool calls are not directly observable by the platform's Event log). *(carried from §54)*
- The full AISEC-REQ set (now including AISEC-REQ-009) should receive an implementation-time security/red-team pass distinct from this document's own requirements-level review — Phase 11 was a requirements-and-architecture-level red-team, not a penetration test of running code, which does not yet exist. *(carried from §54, sharpened)*

### UX Validation
- **[NEW, Phase 13]** Phase 12 §5's proposed Progressive Disclosure default levels (the "Why" trace, the AI Review findings panel) are adopted as V1 starting defaults but require mandatory user-testing validation before being treated as final (F-6, dispositioned §1.3 as Accepted-deferred).
- Whether §2's structural Stable Skeleton diff-test (UX-REQ-005) is practically implementable as an automated regression check or requires manual design review only — needs a prototype. *(carried from phase12-ux-review.md §11)*
- Dynamic-content-region screen-reader semantics for context-assembled pages — needs a prototype. *(carried from phase12-ux-review.md §11, A11Y-REQ-001)*
- Real-world validation of the fixed-region-with-MVP-data proposal (now UX-REQ-005's MVP slice) against actual Reviewer persona feedback — Phase 12 itself flagged that its own analysis was internal reasoning only, not real persona validation. *(carried from phase12-ux-review.md §11)*
- All six persona journeys in master doc §50 remain stubs pending full validation. *(carried from §54)*

---

## Next Phases

- **Phase 13 — Final Baseline**: this document. **This is the final phase of the 13-phase program.**
- There is no Phase 14. Future work against this baseline is implementation, benchmarking, legal review, and UX prototyping/validation — all explicitly named as out-of-scope for this research program in §6 above — not further requirements-research phases. Any future requirements changes should be tracked as dated amendments to this baseline, referencing the specific finding, benchmark result, or product decision that motivated the change, preserving the disposition-and-reasoning discipline this 13-phase program was run to establish.
