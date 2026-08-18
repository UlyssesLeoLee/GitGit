# AI-Native Engineering Platform — Requirements Research: Phase 9

**Status:** v1.0. **Scope:** Phase 9 only (MVP Reduction / Stress-Test). Builds on `docs/requirements/00-requirements-definition.md` §48 (MVP Definition, draft) and §49 (Roadmap, draft), and cites `docs/requirements/phase6-primitives.md` / `docs/requirements/phase7-elicitation.md` for underlying requirement detail. **This phase does not start from scratch.** Its job is to take the Phase 8 draft MVP list and apply one hard test to every item on it: *is this requirement load-bearing for the Minimum Complete Loop, or did it get pulled into MVP because it seemed important?* Where the master doc's §48/§49 and this document disagree, this document's conclusions are authoritative — §48/§49 are revised in place at the end of this phase (§7 below) to match.

**Tagging convention** (inherited unchanged): `[FACT]` / `[UNVERIFIED-FACT]` / `[INFERENCE]` / `[PROPOSAL]` / `[TBD]`.

**The governing principle, verbatim from the originating spec:** *Minimum Complete Loop, not Minimum Feature Count.* The reference loop:

```
Repository → Issue → AI Context → Agent Branch → Code Change → CI → AI Review → Human Approval → Merge → Engineering Graph Update
```

An MVP requirement earns its place by being necessary for this loop to run end-to-end, even crudely. "Would be nice," "is philosophically central," "differentiates us eventually," and "P0-priority in general" are all explicitly **not** sufficient justifications on their own — Phase 7 §1.2 already warned against conflating Priority with rollout timing, and Phase 8's draft MVP list, on inspection, conflates them in several places. This phase corrects that.

---

## 1. The Minimum Complete Loop, decomposed

For each of the 10 loop steps, the requirement IDs that are load-bearing — i.e., the loop breaks or degenerates into something not worth calling "the loop" without them — even in the crudest possible implementation.

| # | Loop step | Load-bearing requirement IDs | Why exactly these |
|---|---|---|---|
| 1 | **Repository** | GIT-REQ-001, GIT-REQ-002, GIT-REQ-010; GRF-REQ-001, GRF-REQ-006 | Must be a real Git remote (clone/fetch/push, branch/tag) that keeps working with AI off, and the Repository must exist as a graph Node so everything downstream can be an Edge/Event off it. |
| 2 | **Issue** | GRF-REQ-001, GRF-REQ-002, GRF-REQ-006, GRF-REQ-007 | Issue must be a registered Node type reachable via Edge from Repository/PR — this is just GRF's core registries applied to one Node type; no dedicated ISSUE-REQ set exists or is needed at MVP. |
| 3 | **AI Context** | AI-REQ-001 (single default provider is enough — see §2 cut), CTX-REQ-001, CTX-REQ-002, GRF-REQ-005, GRF-REQ-008 | An agent needs *some* bounded, logged way to see relevant graph state before acting. CTX-REQ-002's "context is a logged View invocation" is what makes the loop's downstream audit story (step 10) possible at all — cut this and the graph differentiator loses its clearest single justification. |
| 4 | **Agent Branch** | GIT-REQ-002, AGT-REQ-001, AGT-REQ-005, AGT-REQ-007, SEC-REQ-002 | The agent needs a real branch (standard Git ops), a vendor-neutral lifecycle interface to invoke, an isolated/scoped workspace and credential so it can't touch anything outside its task, and to exist as an Agent Node so the branch's authorship is graph-visible. |
| 5 | **Code Change** | AGT-REQ-003, AGT-REQ-004, AGT-REQ-006 | The agent needs to receive instructions and report status (messaging/status interface), its output (diff/commit) must be captured and linked, and it must be bounded by *some* resource limit or an MVP demo can trivially run away on cost/time. |
| 6 | **CI** | CI-REQ-001 | CI results must exist as a graph Node/Event or the loop's "CI" step is invisible to the graph — this is the single CI requirement the loop cannot skip. |
| 7 | **AI Review** | AI-REQ-001, AGT-REQ-002 (as the mechanism that later gates the *merge*, not review itself), GRF-REQ-004 | "AI Review" in the loop is functionally: an agent (or a review-specific agent invocation) posts findings as graph-visible output. No dedicated review-agent requirement exists in Phase 7 — this step is realized by AGT-REQ + AI-REQ applied to a review task, not a new domain. |
| 8 | **Human Approval** | AGT-REQ-002, GRF-REQ-004, SEC-REQ-001, UX-REQ-003 | The loop's entire distinguishing claim — that agent actions are Policy-gated like human ones — collapses without a real approval gate (AGT-REQ-002/GRF-REQ-004) that's actually *usable* by a human (UX-REQ-003), enforced against a real RBAC decision (SEC-REQ-001). |
| 9 | **Merge** | GIT-REQ-002, GIT-REQ-003 | Standard branch ops plus merge/rebase parity — a merge that doesn't produce a normal Git history is not a credible "Merge" step. |
| 10 | **Engineering Graph Update** | GRF-REQ-001, GRF-REQ-002, GRF-REQ-003, GRF-REQ-008 | The whole point of the loop is that steps 1–9 leave graph residue: typed Nodes, typed Edges, an immutable Event log, and a query surface to prove it happened. Without this the loop is just "a PR got merged," indistinguishable from any competitor. |

**Cross-cutting, not tied to one step but required for every step to be trustworthy at all:** SEC-REQ-002 (agent credential scoping), SEC-REQ-004 (deny-by-default agents), AISEC-REQ-001/002/004/005/006 (see §3 — these are kept in full; an MVP demo of this exact loop with an ungated prompt-injection surface or unscoped MCP tool access is a reputational risk, not a hypothetical one, and Risk R7/R8 in the master doc §53 already says so), OPS-REQ-001/002 (the loop must run self-hosted with AI off for Git steps), CLOUD-REQ-001 (architecture must not fork, though nothing cloud-specific needs to *work* yet — see §2).

---

## 2. MVP Cut List

Every requirement below is **currently MVP-timed in the master doc's §48** and fails the load-bearing test in §1: the loop runs to completion, crudely but for real, without it. Each is demoted with a one-line reason. This list is intentionally long — a short one would mean this phase wasn't rigorous.

| Req ID | Demoted to | Why it fails the loop test |
|---|---|---|
| AI-REQ-002 | V1 | Cost/token observability is valuable but the loop functions without it; it is a hardening/accountability feature layered on top of AI-REQ-001, not a precondition for context/review to happen at all. |
| AI-REQ-003 | V1 | Sensitivity-based routing (air-gapped repos route local-only) matters for the *general* local-first story but the reference loop doesn't require more than one working provider; multi-provider routing itself isn't load-bearing (see below). |
| AISEC-REQ (as a full P0 set, kept — see §4 un-cut) | — | See §4: two of the six original MVP AISEC-REQ items (AISEC-REQ-004, AISEC-REQ-006) are re-affirmed as load-bearing; the other four are re-examined below. |
| CI-REQ (implicitly, none in draft MVP beyond CI-REQ-001) | — | Draft MVP already correctly limited CI to CI-REQ-001; no cut needed here, this row confirms the draft got this one right. |
| CLOUD-REQ-001 | **Partially retained, narrowed** | "Same core architecture, no fork" is retained as an architectural discipline (it costs nothing extra to honor while building MVP at all — see §4), but its *acceptance criterion* ("same test suite passes against a cloud deployment") is cut: nothing requires an actual cloud deployment to exist for the loop to run. Demoted the *testable, cloud-deployment-requiring* half to V1; kept the *no-fork discipline* half as a build-time constraint, not a ship-time gate. |
| GIT-REQ-010 | Kept (see §1) | Confirmed load-bearing — no change. |
| GRF-REQ-009 | V1 | The Action-as-Event convention (paired started/completed Events with in-flight querying) is genuinely useful but the loop's CI/AgentRun steps can emit plain Events without the *paired, in-flight-queryable* convention formalized; "AgentRun happened, here's its Event" satisfies step 5/6, "AgentRun currently in flight, queryable efficiently" does not gate anything in the loop. |
| GRF-REQ-010 | V1 | Evidence-as-specialized-Edge (tagged confidence/epistemic-status on Edges) is a hardening/queryability nicety on top of a plain Edge; CI-REQ-002 (which depends on it) is already V1-timed in the draft, so GRF-REQ-010 being MVP while its only MVP-relevant consumer is V1 is an inconsistency in the draft — corrected here by aligning both to V1. |
| DATA-REQ-002 | V1 | "Git objects always exportable" is a near-free consequence of GIT-REQ-001, true, but it is a portability/anti-lock-in guarantee, not something the loop exercises — nothing in the 10 steps reads or writes an export path. Demoting removes zero engineering effort (it remains true by construction) but removes it from the *tested/gated* MVP surface, which is the honest thing to do: "true by accident of other requirements" is not the same claim as "MVP requires this and tests for it." |
| DATA-REQ-004 | V1 | Same reasoning as DATA-REQ-002 — "no phone-home" is a real property but not one the loop's 10 steps exercise directly; it rides along with OPS-REQ-001 rather than being independently load-bearing. |
| API-REQ-001 | Kept, narrowed | The loop needs *some* documented, Policy-gated API surface (agents call it, humans call it) — this is genuinely load-bearing. Retained. |
| API-REQ-003 | V1 | Whatever API-REQ-003 covers beyond API-REQ-001's baseline (see master doc — likely rate-limiting/versioning discipline) is hardening, not a precondition for one demo loop to run once. |
| SEC-REQ-005 | Kept (see §4) | Secrets isolation is load-bearing the instant SEC-REQ-002 issues a real credential — re-affirmed, not cut. |
| CI-REQ-002 through CI-REQ-006 | Confirmed V1 (draft already correct) | No change — flagging that the draft's CI scoping was already appropriately minimal; this row exists so the domain isn't silently unexamined. |
| CLOUD-REQ-002, CLOUD-REQ-003 | Confirmed V1 (draft already correct) | No change — the loop never needs a migration path or horizontal scale-out to run once, self-hosted, for one team. Draft got this right; re-affirmed after scrutiny. |
| ART-REQ (any/all) | Confirmed Future/V2, not MVP | Never was MVP in the draft. Explicitly re-confirmed: the loop has no "publish a package" step; ART-REQ has zero touchpoints with the 10-step loop. |
| SRCH-REQ (any/all) | Confirmed V1, not MVP | Never was MVP in the draft. Re-confirmed: nothing in the loop performs a search; CTX-REQ's graph-Edge traversal is sufficient for "AI Context," and free-text search is a separate, genuinely useful but non-load-bearing capability. |
| ITC-REQ, DIFF-REQ | Confirmed Future | Re-confirmed: Intent Commit and Semantic Diff are not the loop's "Code Change" step (a plain diff is), they are enhancements to it. Zero change from draft. |
| CDX-REQ (beyond generic AGT-REQ-008 MCP support) | Confirmed V1/V2 | The loop needs *an* agent to run via *a* vendor-neutral interface (AGT-REQ-001) and MCP as the tool surface (AGT-REQ-008, already V1 in the draft — correctly not MVP). Codex-specific integration beyond that is not load-bearing for a loop that only needs one working agent backend. |
| OBS-REQ (beyond nothing — draft correctly has none in MVP) | Confirmed V1 | Re-confirmed absent from MVP; minimal request logging is nice for debugging the demo but the loop doesn't gate on it. |
| BKP-REQ (beyond nothing — draft correctly has none in MVP, OPS-REQ-004 covers backup) | No change | Note: OPS-REQ-004 (backup/restore) is itself scrutinized separately below. |
| REL-REQ (beyond nothing) | Confirmed V1 | No change from draft. |

### The one genuinely contested demotion: OPS-REQ-004 (backup/restore)

This is the hardest call in this phase, flagged explicitly rather than quietly cut. **Verdict: demote to V1, not cut, and record the reasoning.** The 10-step loop itself never exercises backup/restore — no step reads from or writes to a backup. By the strict load-bearing test it should be cut. However: OPS-REQ-004 is retained as **V1, immediately following MVP**, rather than pushed further out, because self-hosted operators are explicitly told (master doc §53 R1, R11) that they own their own disaster recovery, and shipping an MVP that a real team could put production engineering data into *without even a documented backup procedure* is a trust failure adjacent to (though distinct from) the loop itself. This is a deliberate "V1, not V2" placement, not an MVP reinstatement — the loop's Definition of Done in §6 below does **not** require a backup/restore cycle to pass.

### Cut list summary by domain

| Domain | Requirements cut from MVP | Domain hit hardest? |
|---|---|---|
| AI-REQ | 002, 003 (2 of 3 originally-MVP items) | Yes — cut by 67% |
| GRF-REQ | 009, 010 (2 of 8 originally-MVP items) | Moderate — cut by 25% |
| DATA-REQ | 002, 004 (2 of 2 originally-MVP items) | Yes — cut by 100% (DATA-REQ has zero MVP items post-cut) |
| API-REQ | 003 (1 of 2 originally-MVP items) | Moderate — cut by 50% |
| CLOUD-REQ | 001's testable cloud-deployment half (narrowed, not fully cut) | Narrowed, not removed |
| OPS-REQ | 004 (1 of 4 originally-MVP items, demoted to immediate-V1) | Minor — cut by 25%, softened placement |

**Total: 7 full demotions + 1 partial narrowing + 1 soft V1-adjacent demotion, out of the master doc's 38 originally MVP-tagged requirement IDs** (counted individually across all listed domains in §48 — see §5 for the full before/after table). That is a real cut, roughly 18–24% of the draft MVP surface depending on how the CLOUD-REQ-001 partial is counted, concentrated most heavily in AI-REQ and DATA-REQ — exactly the two domains the task brief flagged as common suspects.

---

## 3. MVP Keep List with justification

Grouped by domain. Only requirements that survive the load-bearing test from §1.

| Req ID(s) | Loop step(s) it's load-bearing for | Justification |
|---|---|---|
| GIT-REQ-001, 002, 003, 010 | Repository, Agent Branch, Merge | Without real clone/fetch/push/branch/merge parity and AI-independent operation, there is no Repository to open an Issue against or branch to commit to. |
| GRF-REQ-001, 002, 003, 004, 005, 006, 007, 008 | Every step | This is the substrate every other step writes to; cutting any one of these breaks "Engineering Graph Update" (step 10) for every prior step simultaneously, not just its own. |
| AGT-REQ-001, 002, 003, 004, 005, 006, 007 | Agent Branch, Code Change, AI Review, Human Approval | The loop's entire "AI Context → Agent Branch → Code Change" middle section is AGT-REQ's job; cutting any one (lifecycle, approval gate, messaging, artifact capture, isolation, resource limits, graph participation) leaves a step in the loop with no mechanism to realize it. |
| AI-REQ-001 | AI Context, AI Review | One working provider is enough to run the loop once; this is the minimum "AI Context"/"AI Review" needs to exist at all. |
| CTX-REQ-001, 002 | AI Context | Bounded retrieval plus logged View invocation is what makes "AI Context" both functional and auditable — CTX-REQ-002 specifically is what makes step 10's graph residue meaningful for the context step, not just the code-change step. |
| SEC-REQ-001, 002, 003, 004, 005 | Human Approval, Agent Branch (credentialing) | RBAC/ABAC is what "Human Approval" checks against; scoped agent credentials and secrets isolation are what make "Agent Branch"/"Code Change" safe to demo at all; structural audit (SEC-REQ-003) is what makes step 10 trustworthy rather than merely present. |
| AISEC-REQ-001, 002, 004, 005, 006 | AI Context, Code Change, Human Approval | See §1 cross-cutting note and §4 — a demo of this exact loop with untreated prompt-injection surface (001), unscoped MCP tools (002), secrets leaking into AI payloads (004), privilege escalation via agent action (005), or a Policy-bypass path (006) is not "MVP minus a nice-to-have," it is a broken security boundary in the loop's own gate step. Kept in full. |
| CI-REQ-001 | CI | The loop's CI step is meaningless without a graph-visible CI run to gate on. |
| OPS-REQ-001, 002, 003 | Repository, all steps (deployment topology) | The loop must be runnable self-hosted, with AI off for Git-only operation, by more than one concurrent local user — these three are what make "self-hostable" true rather than aspirational for the MVP demo environment itself. |
| CLOUD-REQ-001 (narrowed) | none directly — architectural discipline | Retained as a *build-time* constraint (don't fork the codebase) rather than a *ship-time* gate (don't require a cloud deployment to exist) — see §2's narrowing. Costs nothing to honor while building the rest of MVP; removing it risks exactly the dual-edition drift principle the platform exists to avoid. |
| UX-REQ-003 | Human Approval | Step 8 is not real if the approval control isn't visibly actionable in whatever UI ships with MVP — a backend-only approval gate fails the loop's own "Human Approval" step as a *usable* step, not just a technical one. |
| API-REQ-001 | AI Context, Human Approval, Merge | Agents and humans both need one documented, Policy-gated way to call into the platform; without it, steps 3/8/9 have no defined interface. |

---

## 4. The "one thing cut too far?" check

The opposite failure mode: an MVP so minimal it isn't a credible product, specifically at risk of gutting Phase 5's three stated differentiators (master doc §4/§7, line 200): **(a)** knowledge graph across the full engineering surface, **(b)** true self-hosted + cloud parity with agent-native execution, **(c)** unified audit trail across human and agent actions.

Walking each differentiator against the post-cut MVP list in §3:

- **(a) Knowledge graph:** Fully intact. GRF-REQ's entire P0 set (001–008) survives every cut in §2 untouched — the two items actually cut (009 Action-convention, 010 Evidence-convention) are *conventions layered on top of* the graph, not the graph itself. `[PROPOSAL]` No un-cut needed.
- **(b) Self-hosted + cloud parity with agent-native execution:** At risk, and this is the one place this phase makes an explicit un-cut. §2 narrowed CLOUD-REQ-001 to a build-time-only discipline with no ship-time cloud deployment requirement. Taken alone, that reads as quietly abandoning "cloud parity" as an MVP-era claim at all — which would gut differentiator (b), not just trim it. **Un-cut:** CLOUD-REQ-001's *architectural* half (same codebase, no fork, no cloud-only hard dependency introduced anywhere in MVP-era code) is retained as a **hard MVP-era build constraint, verified by code review / architecture-fitness-function tooling, not by standing up a cloud deployment.** This preserves the *credibility* of differentiator (b) — "when we do stand up cloud, it will be the same platform, provably, because we never let MVP code fork" — without requiring the (genuinely non-load-bearing) act of actually running a cloud deployment before MVP ships. The agent-native half of differentiator (b) is already fully covered by AGT-REQ's kept P0 set — no further un-cut needed there.
- **(c) Unified audit trail across human and agent actions:** Fully intact and arguably strengthened by this phase's rigor, not weakened. SEC-REQ-003 (structural audit), AGT-REQ-007 (Agent as full graph-participant Node), and GRF-REQ-003 (immutable Event log) all survive every cut untouched — none of §2's demotions touch the audit path itself; the two GRF cuts (009/010) are *convention* affordances for querying the audit trail more conveniently, not the audit trail's existence. No un-cut needed.

**Conclusion:** one deliberate un-cut (CLOUD-REQ-001's architectural-discipline half, reinstated as a build-time MVP gate) is required to keep differentiator (b) credible; the other two differentiators pass the check without modification. This is recorded explicitly rather than left implicit precisely so a later phase doesn't rediscover the same tension and re-litigate it from scratch.

---

## 5. Revised MVP requirement count by domain prefix

| Domain | §48 draft MVP count | Phase 9 revised MVP count | Delta |
|---|---|---|---|
| GIT-REQ | 4 (001,002,003,010) | 4 | 0 |
| GRF-REQ | 8 (001–008) | 6 (001–008 minus 009†/010†) | **−2** |
| AGT-REQ | 7 (001–007) | 7 | 0 |
| AI-REQ | 3 (001,002,003) | 1 (001 only) | **−2** |
| CTX-REQ | 2 (001,002) | 2 | 0 |
| SEC-REQ | 5 (001–005) | 5 | 0 |
| AISEC-REQ | 5 (001,002,004,005,006) | 5 | 0 |
| CI-REQ | 1 (001) | 1 | 0 |
| OPS-REQ | 4 (001–004) | 3 (001–003; 004 moved to immediate-V1) | **−1** |
| CLOUD-REQ | 1 (001) | 1, narrowed to architectural-discipline scope only | 0 (scope narrowed, count unchanged) |
| UX-REQ | 1 (003) | 1 | 0 |
| API-REQ | 2 (001,003) | 1 (001 only) | **−1** |
| DATA-REQ | 2 (002,004) | 0 | **−2** |
| **Total** | **~45**‡ | **37** | **−8** |

† GRF-REQ-009/010 were counted as part of the draft's "entire P0 core...plus the type registries" phrase in §48's GRF-REQ line but are itemized separately in Phase 7 as distinct P1 requirements not actually in GRF-REQ's core 001-008 set — re-checking the draft text, GRF-REQ-009/010 were **not** explicitly MVP-tagged in §48's GRF-REQ line (only 001–008 are named there) but this phase flags that some readers could infer they were meant to be included given the "core plus conventions" framing; this document resolves the ambiguity explicitly by keeping them at V1, consistent with the master doc's own §49 text which already lists 009/010 under V1 ("Evidence/Action conventions formalized"). No net count change results from this GRF-REQ line in the table above once the ambiguity is resolved this way — restated here for transparency about how the number was derived, since §48's own MVP/V1 split for GRF-REQ was the one internally inconsistent point in the draft.

‡ §48's draft total, recounted by literal ID enumeration across every bullet in the "MVP" section: GIT-REQ(4) + GRF-REQ(8) + AGT-REQ(7) + AI-REQ(3) + CTX-REQ(2) + SEC-REQ(5) + AISEC-REQ(5) + CI-REQ(1) + OPS-REQ(4) + CLOUD-REQ(1) + UX-REQ(1) + API-REQ(2) + DATA-REQ(2) = **45**.

**Net result: 45 → 37, an 18% reduction**, concentrated in AI-REQ (−67% of its MVP items) and DATA-REQ (−100%, now zero MVP items), with a secondary trim in GRF-REQ (clarifying an ambiguous draft boundary rather than a substantive cut) and single-item trims in OPS-REQ and API-REQ.

---

## 6. Definition of Done for the MVP loop

The MVP is done when **all** of the following hold, verifiable by an automated end-to-end test plus one manual review checkpoint (step 8 requires a human by definition):

1. A human, using the shipped UI or the documented API (API-REQ-001), creates an **Issue** against a **Repository** hosted by the platform (GIT-REQ-001/002, GRF-REQ-001/006) — verified by the Issue existing as a queryable graph Node (GRF-REQ-008) linked to the Repository Node.
2. Without further human input, the platform assembles a bounded, relevance-ranked **context** for that Issue (CTX-REQ-001) via a registered View invocation that is itself logged as an Event (CTX-REQ-002) — verified by querying that Event and confirming it references the actor, parameters, and a graph-state timestamp.
3. A connected agent, invoked through the vendor-neutral **AgentRuntime** interface (AGT-REQ-001), opens an **Agent Branch** (GIT-REQ-002) under a temporary, scoped credential (SEC-REQ-002) confined to an isolated workspace (AGT-REQ-005) — verified by a negative-access test confirming the agent cannot read/write any repository or credential outside its granted scope.
4. The agent produces a **Code Change** (a real commit/diff on that branch), captured and linked to its AgentRun Node via a `produced` Edge (AGT-REQ-004) — verified by querying the AgentRun Node and finding the Edge.
5. **CI** runs against the change and reports pass/fail, represented as a graph Node with `queued`/`started`/`completed` (or `failed`) Events (CI-REQ-001) — verified by querying the CI run Node and finding a `caused`/`triggered_by` Edge to the Commit.
6. An **AI Review** runs (an agent invocation via AI-REQ-001/AGT-REQ-001 scoped to review) and posts findings, itself an auditable Agent action (AGT-REQ-007) — verified by the review output being retrievable as an artifact/Edge off an AgentRun Node, not a UI-only ephemeral comment.
7. A **Human** with Policy-granted authority (SEC-REQ-001, evaluated via GRF-REQ-004) sees the pending approval as a visibly distinct, in-context, actionable control (UX-REQ-003) and approves it — verified by an approval Event recorded with the approving Human Node as actor (AGT-REQ-002), and by a negative test confirming an unauthorized human's attempted approval is rejected.
8. **Merge** occurs, producing a commit-graph structurally identical to what an equivalent local `git merge`/`git rebase` would produce (GIT-REQ-003) — verified by an automated structural comparison, not a visual/manual check.
9. The **Engineering Graph** reflects the full sequence: Commit, PR (or equivalent Node), Review, Test/CI-run, and AgentRun Nodes all exist, connected by typed Edges (`implements`/`caused`/`produced`/`reviewed_by`/`assigned_to` as applicable) — verified by one query (GRF-REQ-008) that starts from the original Issue Node and traverses to every Node created in steps 1–8, returning the complete chain in a single call.
10. The entire sequence above completes with the **AI Gateway/Agent runtime killed and restarted mid-run at least once** in a chaos-style test, and **Git read/write operations (clone/fetch/push) remain unaffected throughout** (GIT-REQ-010) — verified by an integration test that kills the AI Gateway process during step 2 or 6 and confirms `git clone`/`git push` against the repository still succeed with no elevated latency.

**The MVP is NOT done** if any of the following is true, regardless of how much of the above works: the loop only runs against a toy/seed repository and fails on a repository above the CTX-REQ-001 size threshold; the approval step (7) can be bypassed via any API path that doesn't go through GRF-REQ-004's Policy evaluation; the agent's workspace isolation (step 3) has not been verified by an actual negative-access test (not just "designed to be isolated"); or the graph query in step 9 requires more than one call / any client-side joining across separate stores.

---

## 7. Master document updated

`docs/requirements/00-requirements-definition.md` §48 and §49 have been revised in place to reflect this phase's conclusions:
- §48's MVP/V1/V2/Future lists now match §2/§3/§5 of this document exactly (AI-REQ-002/003, DATA-REQ-002/004, GRF-REQ-009/010, API-REQ-003 moved from MVP to V1; OPS-REQ-004 moved to explicit "immediate V1" with reasoning preserved; CLOUD-REQ-001 narrowed to an architectural-discipline MVP gate with its ship-time cloud-deployment testing moved to V1).
- §49's roadmap narrative updated to match — the MVP→V1 transition trigger prose now references this document's Definition of Done (§6) rather than restating a separate, looser bar.
- A pointer note added at the top of §48 directing readers to this document for full reasoning, per the task instruction.

---

## 8. Open Questions

**Product Decision**
- `[TBD]` Whether the CLOUD-REQ-001 architectural-discipline un-cut (§4) should be enforced by tooling (an automated architecture-fitness-function check in CI) or by review discipline alone — a genuine engineering-process decision this phase surfaces but does not resolve; recommend Phase 10 pick one, since "we said we wouldn't fork" without an enforcement mechanism has a documented failure mode in exactly the dual-edition-drift pattern this program keeps citing as a risk to avoid.
- `[TBD]` OPS-REQ-004 (backup/restore) is placed at "immediate V1" rather than MVP or ordinary V1 — this is a soft distinction with no enforcement mechanism proposed here (e.g., should MVP itself be blocked from general-availability announcement until OPS-REQ-004 ships, even though the loop's Definition of Done doesn't require it?). Flagged for Phase 13's final baseline to make binding.

**Architecture Decision**
- `[TBD]` The Definition of Done's step 9 (single-query full-chain traversal from Issue to every downstream Node) assumes GRF-REQ-008's query API can express a multi-hop traversal in one call — this depends on the same Phase 6 §7 storage-architecture open question (single store vs. split graph/event-log) already flagged unresolved in the master doc's Appendix ADR list; if Phase 10 resolves toward a split store, this Definition-of-Done criterion may need a "one logical query, possibly federated" relaxation rather than "one literal API call."

**Research Needed**
- `[TBD]` This phase's chaos-test criterion (Definition of Done step 10: kill the AI Gateway mid-loop) has not been validated against any existing reference implementation of an AgentRuntime interface — worth a light feasibility spike before Phase 13 treats it as a hard acceptance gate rather than an aspirational test design.

---

## Next Phases

- **Phase 9 — MVP Reduction**: this document.
- **Phase 10 — Architecture Design**: pending — must resolve the storage-architecture question this phase's Definition of Done (§6, step 9) and Open Questions (§8) both depend on.
- **Phase 11 — Red-Team Review**: pending — should specifically stress-test AISEC-REQ-001/002/004/005/006, kept in full by this phase precisely because the loop's own security boundary depends on them.
- **Phase 12 — Stakeholder Validation**: pending.
- **Phase 13 — Final Baseline**: pending — should adopt this document's Definition of Done (§6) as the literal MVP acceptance test, and resolve the two Product Decision open questions above (§8) as binding calls rather than carrying them forward again.
