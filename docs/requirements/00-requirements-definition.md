# AI-Native Engineering Platform — Requirements Definition

**Status:** Baseline draft — Phases 1–8 complete, Phases 9–13 pending.

**Scope of this document:** Phase 8 (Requirements Specification). This is the master requirements book — it assembles and cross-references Phases 1–7 into the target 55-section structure the original research program calls for. It does not re-derive prior findings; where a section is pure consolidation it summarizes and links back to the source phase document rather than repeating it. Sections requiring genuinely new synthesis (marked below) are written in full here for the first time.

**Source documents** (read for full detail; this document does not duplicate them):
- [`./phase1-5-research.md`](./phase1-5-research.md) — Research Plan, Evidence Collection, Competitor Analysis, Capability Matrix, Gap Analysis
- [`./phase6-primitives.md`](./phase6-primitives.md) — Product Primitive Discovery
- [`./phase7-elicitation.md`](./phase7-elicitation.md) — Requirements Elicitation (atomic, ID-tagged requirements)

**Tagging convention** (inherited unchanged from all prior phases):
- `[FACT]` — verifiable claim, cited to a primary source.
- `[UNVERIFIED-FACT]` — plausible, cited, but not primary-sourced or independently confirmed.
- `[INFERENCE]` — reasonable conclusion from facts, not independently verified.
- `[PROPOSAL]` — our own design idea for the Target Platform, not a claim about any competitor.
- `[TBD]` — unresolved; needs follow-up before a later phase relies on it.

This document additionally uses these section-level markers where useful: **[NEW SYNTHESIS]** (content genuinely produced in Phase 8, not present in Phases 1–7) and **[CROSS-REFERENCE]** (content that summarizes/points at a prior phase without materially adding to it).

---

## Table of Contents

1. [Executive Summary](#1-executive-summary)
2. [Research Methodology](#2-research-methodology)
3. [Product Vision](#3-product-vision)
4. [Product Positioning](#4-product-positioning)
5. [Personas](#5-personas)
6. [Jobs To Be Done](#6-jobs-to-be-done)
7. [Competitive Landscape](#7-competitive-landscape)
8. [GitHub Deep Dive](#8-github-deep-dive)
9. [GitLab Deep Dive](#9-gitlab-deep-dive)
10. [AI Coding Agent Landscape](#10-ai-coding-agent-landscape)
11. [Competitive Matrix](#11-competitive-matrix)
12. [Gap Analysis](#12-gap-analysis)
13. [Product Principles](#13-product-principles)
14. [Scope](#14-scope)
15. [Non-Goals](#15-non-goals)
16. [Core Domain Model](#16-core-domain-model)
17. [Git Requirements](#17-git-requirements)
18. [Repository Requirements](#18-repository-requirements)
19. [Collaboration Requirements](#19-collaboration-requirements)
20. [Issue Requirements](#20-issue-requirements)
21. [MR/PR Requirements](#21-mrpr-requirements)
22. [CI/CD Requirements](#22-cicd-requirements)
23. [Artifact Requirements](#23-artifact-requirements)
24. [Search Requirements](#24-search-requirements)
25. [Engineering Graph Requirements](#25-engineering-graph-requirements)
26. [Context Engine Requirements](#26-context-engine-requirements)
27. [AI Gateway Requirements](#27-ai-gateway-requirements)
28. [Agent Runtime Requirements](#28-agent-runtime-requirements)
29. [Codex Integration Requirements](#29-codex-integration-requirements)
30. [Intent Commit Requirements](#30-intent-commit-requirements)
31. [Semantic Diff Requirements](#31-semantic-diff-requirements)
32. [Repository Digital Twin Requirements](#32-repository-digital-twin-requirements)
33. [Emergent Workflow Requirements](#33-emergent-workflow-requirements)
34. [UX Requirements](#34-ux-requirements)
35. [Security Requirements](#35-security-requirements)
36. [AI Security Requirements](#36-ai-security-requirements)
37. [Permission Requirements](#37-permission-requirements)
38. [Audit Requirements](#38-audit-requirements)
39. [Local Deployment Requirements](#39-local-deployment-requirements)
40. [Cloud Requirements](#40-cloud-requirements)
41. [Offline/Air-Gapped Requirements](#41-offlineair-gapped-requirements)
42. [Observability Requirements](#42-observability-requirements)
43. [Backup/Recovery Requirements](#43-backuprecovery-requirements)
44. [Performance Requirements](#44-performance-requirements)
45. [Reliability Requirements](#45-reliability-requirements)
46. [Data Ownership / Portability](#46-data-ownership--portability)
47. [API / MCP / Webhook Requirements](#47-api--mcp--webhook-requirements)
48. [MVP Definition](#48-mvp-definition)
49. [V1/V2/Future Roadmap](#49-v1v2future-roadmap)
50. [User Journeys](#50-user-journeys)
51. [Acceptance Criteria](#51-acceptance-criteria)
52. [Traceability Matrix](#52-traceability-matrix)
53. [Risk Register](#53-risk-register)
54. [Open Questions](#54-open-questions)
55. [Research Sources](#55-research-sources)

[Appendix: ADR Required List](#appendix-adr-required-list)

---

## 1. Executive Summary

**[NEW SYNTHESIS]**

This program is defining the requirements for a local-first, cloud-ready, Git-native, AI-native, agent-native, graph-native, self-hostable software engineering platform — tentatively named the "AI-Native Engineering Platform." Seven phases of research and design precede this document; this document is the eighth, and its job is to consolidate that work into the formal specification the remaining five phases (MVP reduction, architecture, red-team review, stakeholder validation, final baseline) will build on.

**What the research found.** Phases 1–5 examined nine forges and AI-coding-agent products in depth (GitHub, GitLab, Gitea/Forgejo, Bitbucket, Sourcegraph, Linear, Codex, Claude Code, Cursor) and found a consistent structural pattern: every competitor either has no queryable cross-object engineering graph at all (GitHub's Issues/PR/Discussions are three weakly-linked systems bolted on over 15 years), or has graph coherence scoped narrowly to one product's data model (GitLab: code/CI-scoped; Linear: project-management-scoped, with no self-hosting at any tier; Sourcegraph: code-only, and reconstructed per-query via RAG rather than persisted). No competitor surveyed models requirement, ADR, incident, agent, agentrun, prompt, skill, model, context, or policy as first-class graph nodes. Nor does any competitor treat agent-originated actions as structurally equal, audited citizens alongside human actions — audit trails are consistently a separate, sometimes fragile subsystem (GitHub's own audit log suffered a documented 28-minute outage in April 2026 from a shared-dependency failure). These findings (Phase 5's Gap Analysis) point at one clear differentiator: a genuine, first-class engineering knowledge graph spanning the full requirement→ADR→issue→PR→test→release→incident→agentrun lifecycle, with human and agent activity structurally equal within it.

**What was designed in response.** Phase 6 resisted the temptation to over-engineer that differentiator into a large bespoke ontology. Applying an explicit anti-over-abstraction test against the originating spec's ten candidate primitives, it concluded the MVP substrate needs only **five**: Node, Edge, Event, Policy, View — with Agent modeled as a Node subtype (not a sixth primitive) and Action/Evidence deferred to conventions built on Event and Edge rather than new mechanics. This was a deliberate response to an observed pattern: GitLab and Linear's real-world graph coherence came from narrow, shared data models, not rich taxonomies, while GitLab's self-hosting operational weight (Gitaly/Praefect/Sidekiq/Workhorse) is itself a cautionary tale about solving a problem — Git's own scaling limits — by bolting on services rather than designing the substrate correctly from the start.

Phase 7 then elicited 67 atomic, ID-tagged, acceptance-criteria-bearing requirements across ten domains (Git-native compatibility, the engineering graph, agent runtime, AI Gateway, context engine, security, CI/CD, UX, local-first deployment, cloud-readiness), directly traceable to Phase 6's primitives and Phase 1–5's evidence.

**What this document adds.** Phase 8 assembles that work into the full 55-section structure the program's target document calls for. Roughly a third of the sections are new synthesis not attempted in Phases 1–7: the product vision/positioning/principles/non-goals framing (§§3–5, 13–15), several requirement domains genuinely not covered by Phase 7's ten prefixes — artifact/package registry, search, Codex/MCP interop, Intent Commit, semantic diff, digital twin, emergent workflows, AI-specific security, data portability (§§23–24, 29–33, 36, 46) — and the program-level synthesis work that ties everything together: an MVP/V1/V2/Future rollout assignment for every requirement produced so far (§§48–49), a risk register (§53), and a consolidated open-questions list spanning all eight phases (§54).

**What is deliberately not yet done.** This document does not resolve architecture (Phase 10), does not run a red-team pass against its own requirements (Phase 11), and does not validate personas/journeys against real stakeholders (Phase 12). Several sections here are honest stubs — most notably Performance Requirements (§44), where the program's own research discipline (never fabricate numbers) means most performance targets remain `[TBD] – Benchmark Required` rather than invented figures.

**Bottom line for a reader in a hurry:** the differentiator is the graph; the substrate is five primitives, not ten; self-hosted and cloud share one architecture, not two; agents are first-class, policy-gated, audited citizens of the graph, not a bolt-on. Everything else in this document is detail in service of that spine.

---

## 2. Research Methodology

**[CROSS-REFERENCE]** See [`./phase1-5-research.md` §Phase 1](./phase1-5-research.md#phase-1--research-plan) for the full methodology: competitor category selection, the primary-source-first citation discipline, the `[FACT]`/`[UNVERIFIED-FACT]`/`[INFERENCE]`/`[PROPOSAL]`/`[TBD]` tagging convention (reused unchanged through Phases 6–8), and the live-WebSearch/WebFetch evidence-gathering approach used across Phases 2–5. Phase 6 added an emergent-discovery method (Observation → Pattern → Constraint → Primitive → Interaction → Workflow → Platform Capability, see [`./phase6-primitives.md` §1](./phase6-primitives.md#1-method)); Phase 7 added an atomic-requirement schema and quality bar (Atomic/Testable/Traceable/Unambiguous/Necessary/Feasible, see [`./phase7-elicitation.md` §1](./phase7-elicitation.md#1-method)). This document's own method is assembly, not new evidence-gathering — new claims introduced in Phase 8 are tagged `[PROPOSAL]` per the inherited convention, and no Phase 8 section re-runs live research over a claim already tagged in an earlier phase.

---

## 3. Product Vision

**[NEW SYNTHESIS]**

The AI-Native Engineering Platform is a single, self-hostable system of record for the full software engineering lifecycle — not a Git host with AI features bolted on, and not an AI agent with a Git client bolted on, but a platform where the engineering graph itself (requirements, code, reviews, tests, releases, incidents, and the humans and agents that act on all of it) is the product, and Git hosting, CI/CD, and AI assistance are capabilities built directly against that graph rather than separate subsystems glued together after the fact.

Three years from a first release, success looks like: engineering organizations that today run GitHub/GitLab plus Jira/Linear plus Sourcegraph plus a fleet of loosely-integrated AI coding agents can instead run one self-hosted (or cloud) platform where every one of those capabilities queries and writes to the same graph, where an AI agent's actions carry exactly the same policy gates, audit trail, and accountability as a human's, and where "local-first" is not a downgraded mode but the same codebase running at a smaller scale — a team can start on a laptop and grow into a multi-region cloud deployment without a rewrite or a data-model fork.

The vision is deliberately narrower than "replace GitHub and Jira and Slack and Sourcegraph and Devin all at once." Per Phase 5's Gap Analysis, the platform commodifies (matches parity on, does not over-invest in) Git hosting, PR review, CI/CD, and basic AI coding assist — because every competitor already has these — and differentiates specifically on the graph, on true self-hosted/cloud parity, and on treating agents as first-class audited graph citizens, because Phase 1–5's evidence shows no competitor does all three.

---

## 4. Product Positioning

**[NEW SYNTHESIS]**

**Positioning statement:** For engineering organizations that need to run AI coding agents at scale without losing self-hosting control, audit rigor, or a coherent picture of how requirements, code, and incidents relate — the AI-Native Engineering Platform is a self-hostable, cloud-ready engineering system of record that models the entire engineering graph as queryable, policy-gated, agent-and-human-equal data, unlike GitHub/GitLab, whose graph coherence (where it exists at all) is confined to one product's data model, and unlike Claude Code/Codex/Cursor, which are powerful agents with no persistent graph of their own to act against.

**Positioning relative to the three competitor clusters Phase 1–5 examined:**
- **vs. forges (GitHub, GitLab, Bitbucket, Gitea/Forgejo):** the platform matches Git/PR/CI parity (commodity, per Phase 5) but adds a first-class graph and equal-citizenship agent model none of them has; unlike GitHub it treats self-hosting as a first-class, permanent deployment mode rather than a declining option, and unlike GitLab it aims to avoid recreating Git's own scaling-limit workarounds (Gitaly/Praefect/Sidekiq/Workhorse) as unavoidable self-hosting weight.
- **vs. code-intelligence layers (Sourcegraph):** the platform absorbs cross-repo context natively as durable graph Edges rather than leaving it to be a layer that reconstructs context per-query via RAG on top of someone else's system of record.
- **vs. terminal/IDE-first AI agents (Codex, Claude Code, Cursor):** the platform does not compete on agent quality/UX — it deliberately treats MCP as an interop surface (`CDX-REQ`, §29) so those tools can act *against* the platform's graph, rather than trying to out-build them as agent products in their own right.

**Anti-positioning (what it is not):** not a full project-management suite (Jira/Linear replacement) beyond the Issue node type it needs for the graph; not a chat/communication tool; not a hosted-only SaaS with self-hosting as an afterthought. See §15 Non-Goals for the explicit decision record.

---

## 5. Personas

**[NEW SYNTHESIS]** — compact; full journey mapping is Phase 12's job (see §50 for a stub pointing there).

| Persona | Role | Primary graph interaction | Core need |
|---|---|---|---|
| **Developer** | Writes code, opens PRs, reviews peers | Node types: Commit, PR, Review, Issue | Fast, unsurprising Git/PR workflow; AI assist that doesn't hide what changed |
| **AI Agent** | Autonomous/semi-autonomous task executor (coding agent, review agent, triage agent) | Agent/AgentRun Node subtype; Policy-gated Actions | Clear scope, minimal-necessary context, an approval path that doesn't block trivial work |
| **Reviewer** | Approves/blocks changes, may be human or agent | Review Node, Evidence-typed Edges to Test Nodes | Confidence that "tests passed" and "policy satisfied" are graph facts, not claims to verify by hand |
| **Architect** | Owns ADRs, cross-repo dependency structure, Policy definitions | ADR, Requirement Nodes; `depends_on`/`supersedes` Edges | Traceability from requirement to implementation to decision record, queryable, not tribal knowledge |
| **Manager** | Tracks delivery, risk, and agent cost/behavior across teams | Views over Issue/PR/AgentRun Nodes | A trustworthy rollup that doesn't require trusting five disconnected tools' exports |
| **Incident Responder** | Investigates production incidents, traces root cause | Incident Node; `caused_by` Edges to Commit/Deployment | Fast graph traversal from symptom to responsible change, including agent-authored changes |

---

## 6. Jobs To Be Done

**[NEW SYNTHESIS]** — compact.

- **Developer:** "When I open a PR, help me get it merged with confidence, without me having to manually chase down which tests ran, which policies apply, and who needs to approve it."
- **AI Agent:** "When I'm assigned a task, give me exactly the context I need, let me act within a clear, enforced scope, and make my provenance auditable without extra effort on my part."
- **Reviewer:** "When I'm asked to approve something, show me the evidence (tests, policy state, related requirements) in one place, not scattered across five tools."
- **Architect:** "When I make or revisit a design decision, let me see and query everything that decision affects, including agent-authored code, without reconstructing it by memory or grep."
- **Manager:** "When I need to know how delivery, risk, or agent cost is trending, give me one queryable source of truth instead of reconciling exports from separate tools."
- **Incident Responder:** "When something breaks in production, let me trace from the failure straight back to the responsible commit/deployment/agent-run, fast, including cross-repo causes."

---

## 7. Competitive Landscape

**[CROSS-REFERENCE]** Full evidence and per-product analysis: [`./phase1-5-research.md` Phase 2–3](./phase1-5-research.md#phase-2--evidence-collection). Products in scope: GitHub, GitLab, Bitbucket, Gitea/Forgejo, Sourcegraph/Cody, Linear (all deep-dived); Codex, Claude Code, Cursor, Devin, OpenHands, Gemini CLI (AI-agent cluster, §10 below); Gerrit, Jira, GitHub Issues/Projects standalone, Argo CD/Workflows, CodeQL, Sentry (flagged `[TBD]` — not reached with dedicated research in Phase 1–5, carried forward as an open item, §54).

---

## 8. GitHub Deep Dive

**[CROSS-REFERENCE]** Full entry: [`./phase1-5-research.md` §Phase 3 "GitHub (deep dive)"](./phase1-5-research.md#github-deep-dive). Key takeaways carried into this document's requirements: no native cross-object engineering graph despite Issues/PR/Discussions/Projects (`[FACT]`, source of GRF-REQ's motivating gap, O1); audit log is a structurally separate subsystem that suffered a documented outage (`[FACT]`, source of SEC-REQ-003, O2); self-hosting (Enterprise Server) is increasingly niche/declining (`[FACT]`, motivates OPS-REQ's "self-hosting is not second-class" stance); Advanced Security unbundled into paid add-ons as of April 2025 (`[UNVERIFIED-FACT]`, primary-domain source not directly WebFetched).

---

## 9. GitLab Deep Dive

**[CROSS-REFERENCE]** Full entry: [`./phase1-5-research.md` §Phase 3 "GitLab (deep dive)"](./phase1-5-research.md#gitlab-deep-dive). Key takeaways: single-Rails-monolith architecture gives genuinely stronger cross-object graph coherence than GitHub (`[INFERENCE]`, O3), but self-hosting operational complexity (Gitaly/Praefect/Sidekiq/Workhorse) is a direct product of working around Git's own scaling limits with bolted-on services (`[INFERENCE]`, O4) — this is the specific failure mode OPS-REQ-005 and Phase 6's storage-architecture open question exist to avoid repeating at the primitive-storage layer. GitLab Duo Self-Hosted demonstrates that credible air-gapped AI operation is achievable (`[FACT]`), informing AI-REQ-003's routing-by-sensitivity requirement.

---

## 10. AI Coding Agent Landscape

**[CROSS-REFERENCE]** Full entries: [`./phase1-5-research.md` §Phase 3](./phase1-5-research.md#openai-codex--codex-cli-deep-dive) — Codex/Codex CLI, Claude Code, Cursor deep dives; Devin/OpenHands/Gemini CLI evidence summary. Key structural finding carried forward `[INFERENCE]`: Copilot and Duo both bolt AI onto an existing forge's data model ("AI as forge feature"), while Claude Code/Codex/Cursor treat the forge as just one more MCP-reachable tool ("forge as agent tool") — the platform deliberately takes neither posture, aiming instead to *be* the graph agents operate against natively (motivates §29 Codex Integration Requirements and AGT-REQ-008's MCP support). AGENTS.md's cross-tool adoption (`[FACT]`, O11) directly motivates CTX-REQ-005 and CDX-REQ (§29). Claude Code's hooks/skills/subagents separation (`[UNVERIFIED-FACT]`, O10) is the pattern generalized into AGT-REQ-009's subagent/delegated-scope requirement.

---

## 11. Competitive Matrix

**[CROSS-REFERENCE]** Full matrix (10 competitors × 14 capability rows, including the proposed Target Platform column): [`./phase1-5-research.md` Phase 4](./phase1-5-research.md#phase-4--capability-matrix). Not reproduced here in full to avoid drift between two copies of the same table — treat phase1-5-research.md as the single source of truth for this matrix; if it is revised in a later pass, this section's cross-reference remains valid without edit. The single row most load-bearing for this document's requirements is "Knowledge graph": ❌/⚠️ across every competitor, ✅ `[PROPOSAL, core value prop]` for the Target Platform — the direct empirical basis for prioritizing GRF-REQ (§25) as P0.

---

## 12. Gap Analysis

**[CROSS-REFERENCE]** Full analysis: [`./phase1-5-research.md` Phase 5](./phase1-5-research.md#phase-5--gap-analysis). Summary of the four-tier classification, reused directly by §48 (MVP Definition):
- **Commodity** (build to parity, don't over-invest): Git hosting, PR/MR review, CI/CD, API/webhook extensibility, RBAC, basic AI coding assist.
- **Differentiator** (real competitive edge): knowledge graph across the full engineering surface; true self-hosted + cloud parity with agent-native execution; unified audit trail across human and agent actions.
- **Emerging** (market still forming standards): MCP-based agent context/tool access; agent execution as a forge-native feature.
- **Experimental** (interesting, not MVP-worthy): full semantic diff; forge federation; "engineering digital twin."

This four-tier classification is the primary input to every MVP/V1/V2/Future assignment in §48 — a requirement whose Phase 5 tier is "Experimental" is not assigned MVP timing in this document regardless of how compelling its individual rationale reads in isolation.

---

## 13. Product Principles

**[NEW SYNTHESIS]**

These eight principles, drawn from the original spec and validated against Phase 1–7's findings, govern every requirement decision in this document. Where a requirement conflicts with a principle, the principle wins unless explicitly flagged as an Open Question (§54).

1. **Git-Native.** The platform is indistinguishable from a standard Git server to any unmodified Git client (GIT-REQ-001). This is non-negotiable table stakes (Phase 5: "Commodity") — deviation here forfeits the entire existing Git tooling ecosystem, which no competitor surveyed does.
2. **Local-First.** A single-machine, fully offline install is a first-class, permanent deployment mode (OPS-REQ-001), not a shrinking legacy option the way GitHub's Enterprise Server has become (`[FACT]`, Phase 3). Local-first is a design constraint checked from day one, not a mode bolted onto a cloud-first architecture later.
3. **Cloud-Ready.** The same core codebase runs at cloud scale (CLOUD-REQ-001) — no structurally forked "cloud edition." This directly avoids the local/cloud feature-drift trap that dual-edition products commonly fall into.
4. **AI-Native.** AI assistance routes through one internal Gateway abstraction (AI-REQ-001) spanning multiple providers including local models, not a single-vendor bolt-on feature.
5. **Agent-Native.** Agents are Node subtypes with full graph participation (AGT-REQ-007) — Policy-gated, audited, and queryable identically to humans (O8/O9). This is the platform's single most consistently-cited differentiator across Phases 1–7.
6. **Graph-Native.** The engineering graph (Node/Edge/Event/Policy/View, §16) is the substrate every other capability is built against, not a reporting layer bolted on top of separately-owned data.
7. **Evidence-Native.** Claims about "why" — a decision, an approval, a passing test — are structurally distinguishable from bare assertions via the Evidence-as-specialized-Edge convention (GRF-REQ-010), not left as free-text comments a human must interpret.
8. **Human Authority.** Wherever an agent action requires approval, that approval is visible and actionable in-context (UX-REQ-003), not merely logged after the fact — the platform's audit-first posture must never be mistaken for a substitute for real-time human control.

---

## 14. Scope

**[NEW SYNTHESIS]**

In scope for this program (Phases 1–13) and the resulting platform:
- Git hosting (server-side, full protocol compatibility) — §17–18
- Code collaboration: issues, PRs/MRs, reviews — §19–21
- CI/CD execution and graph-linked results — §22
- The engineering graph itself: Node/Edge/Event/Policy/View primitives and the core Node/Edge type registry — §16, §25
- AI Gateway (multi-provider model routing) — §27
- Context engine (bounded, auditable context assembly for agents and humans) — §26
- Agent runtime (vendor-neutral agent lifecycle, policy gating, credentials) — §28
- Security: RBAC/ABAC, structural audit trail, AI-specific security controls — §35–38
- Local-first and cloud deployment, sharing one architecture — §39–41
- API/MCP/webhook surfaces for external tooling and agent interop — §47
- A small, explicitly-scoped set of experimental/future capabilities kept in the graph but not MVP-targeted: artifact registry (§23), semantic diff (§31), digital twin (§32), emergent workflow generation (§33), Intent Commit (§30).

Out of scope: see §15 Non-Goals for the explicit decision record on adjacent capabilities competitors offer that this platform deliberately does not build.

---

## 15. Non-Goals

**[NEW SYNTHESIS]** — each item below is a decision, not a placeholder; tagged `[PROPOSAL]` throughout as this document's own design call, cross-checked against Phase 7 §12's Explicit Non-Requirements Check where overlapping.

| Candidate non-goal | Decision | Rationale |
|---|---|---|
| **Wiki / freeform documentation product** | `[PROPOSAL]` Out of scope. | Knowledge belongs in the graph as typed Nodes (ADR, Requirement) with real Edges, not an unstructured wiki competing with the graph for the same "why did we decide this" job. A thin Markdown-rendering surface over ADR/Requirement Nodes is in scope; a general-purpose wiki product is not. |
| **Full project-management suite (Jira/Linear-class breadth: custom workflow states, Gantt-style multi-team roadmaps, OKRs)** | `[PROPOSAL]` Out of scope beyond the Issue Node type and its graph Edges (§20). | Phase 5 places PM breadth outside the platform's differentiator; Linear's deep dive (Phase 3) shows a narrowly-scoped, single-purpose PM tool already does this well — competing on PM breadth would dilute focus on the graph differentiator. Issue tracking as *graph-linked data* stays in scope; issue tracking as a *feature-competitive PM product* does not. |
| **Chat / real-time messaging product** | `[PROPOSAL]` Out of scope. | Comments/threads on graph Nodes (PR, Issue, Review) are in scope as structured collaboration; a general team-chat product (Slack-class) is not — no competitor evidence in Phase 1–5 suggests this is where differentiation lies. |
| **Codespaces-equivalent (full hosted cloud dev environment/IDE)** | `[PROPOSAL]` Out of scope for MVP/V1; revisit no earlier than V2. | Real product, but orthogonal to the graph differentiator and a large independent engineering investment (GitHub's own Codespaces has had documented availability incidents, Phase 2 evidence); the platform's agent workspace isolation (AGT-REQ-005) covers agent execution environments, which is the actually-necessary subset. |
| **Full Kubernetes cluster management (becoming a general K8s control plane)** | `[PROPOSAL]` Out of scope. | OPS-REQ-005 explicitly requires small deployments NOT need an orchestrator; CLOUD-REQ-003 requires scale-out capability but not that the platform itself become K8s tooling. Deployment *of* the platform may use K8s; the platform does not become a K8s management product. |
| **Built-in IDE / code editor** | `[PROPOSAL]` Out of scope. | Cursor's deep dive (Phase 3) shows IDE-first is a viable but different product category, contingent on being inside the editor; the platform's value is being the graph/forge multiple IDEs and agents connect *to* (via MCP/API), not a competing editor. |
| **Fine-tuning or hosting a proprietary foundation model** | `[PROPOSAL]` Out of scope, carried forward unchanged from Phase 7 §12. | The AI Gateway routes across existing providers (AI-REQ-001); model development is a different, vastly larger business than the platform's scope. |
| **Non-Git version-control backend support** | `[PROPOSAL]` Out of scope, carried forward unchanged from Phase 7 §12. | Contradicts the Git-Native principle by definition. |

---

## 16. Core Domain Model

**[CROSS-REFERENCE]** Full derivation and anti-over-abstraction analysis: [`./phase6-primitives.md` §3–6](./phase6-primitives.md#3-candidate-primitives). Summary: the domain model is five primitives — **Node, Edge, Event, Policy, View** — with **Agent** as a documented Node subtype (not a sixth primitive) and **Action**/**Evidence** built as conventions on top of Event-pairing and specialized Edge typing respectively, promotable to independent primitives later only if real usage data (not speculation) shows the convention insufficient (§7 Open Questions in phase6-primitives.md, carried into §54 below). The original spec's full node vocabulary (Organization, Project, Repository, Requirement, Issue, ADR, Commit, Symbol, PR, Review, Test, Deployment, Release, Human, Agent, AgentRun, Policy, Incident) and edge vocabulary (`implements`, `depends_on`, `modifies`, `derived_from`, `supersedes`, `reviewed_by`, `generated_by`, `assigned_to`, `blocks`, `caused`, `part_of`, `includes`) are realized as registered *types* over Node/Edge, not as separate structural mechanics — see [`./phase6-primitives.md` §6](./phase6-primitives.md#6-relationship-to-the-engineering-graph-requirements-original-spec-10) for the full mapping table, and GRF-REQ-006/007 (§25 below) for the requirement-level commitment to pre-registering that vocabulary.

---

## 17. Git Requirements

**[CROSS-REFERENCE]** Full requirement set: [`./phase7-elicitation.md` §2 GIT-REQ](./phase7-elicitation.md#2-git-req--git-native-compatibility) (GIT-REQ-001 through 010). Summary table:

| ID | Title | Priority |
|---|---|---|
| GIT-REQ-001 | Standard Git protocol compliance | P0 |
| GIT-REQ-002 | Standard branch/tag operations | P0 |
| GIT-REQ-003 | Merge and rebase parity | P0 |
| GIT-REQ-004 | Git LFS support | P1 |
| GIT-REQ-005 | Submodule and worktree support | P1 |
| GIT-REQ-006 | Server-side hooks with graph-aware extensions | P1 |
| GIT-REQ-007 | Partial clone / shallow clone / sparse checkout | P1 |
| GIT-REQ-008 | Signed commit/tag verification | P1 |
| GIT-REQ-009 | Mirror / migration import | P1 |
| GIT-REQ-010 | Git protocol availability independent of AI subsystem | P0 |

---

## 18. Repository Requirements

**[CROSS-REFERENCE]** Repository-level behavior (hosting, protocol compliance, LFS, submodules, migration) is fully covered by GIT-REQ-001 through 009 above — there is no separate repository requirement set distinct from Git-native compatibility, because in this platform a Repository is simply a Node type (per §16) that Git operations address; see GIT-REQ-006 for repository/graph-linkage-specific behavior. No gap identified requiring new requirements at this phase.

---

## 19. Collaboration Requirements

**[CROSS-REFERENCE]** PR/Review collaboration mechanics are covered by GRF-REQ-002/007 (typed Edges including `reviewed_by`), CI-REQ-002 (test-result Evidence Edges into Review), and the worked Review composition example at [`./phase6-primitives.md` §4.2](./phase6-primitives.md#42-review). No standalone `COLLAB-REQ` prefix is introduced: Phase 6 explicitly modeled Review as "not a hardcoded type — a Node of type `review` plus its Edge/Event pattern" rather than a bespoke collaboration subsystem, and Phase 7 did not identify a collaboration behavior (e.g., comment threading, mentions, notifications) that isn't already covered by Node/Edge/Event mechanics plus UX-REQ-002's context-assembled page requirement. `[TBD]` Notification/mention delivery mechanics specifically (who gets pinged, via what channel) were not elicited as atomic requirements in Phase 7 — flagged as a gap for Phase 9 to confirm is intentionally deferred rather than accidentally dropped (see §54).

---

## 20. Issue Requirements

**[CROSS-REFERENCE]** Issue is a pre-registered Node type (GRF-REQ-006) with the full worked composition at [`./phase6-primitives.md` §4.1](./phase6-primitives.md#41-issue) (Node + `motivated_by`/`blocks`/`implemented_by`/`assigned_to` Edges + status-history Events + closing Evidence + status-change Policy gate). GRF-REQ-011 (§25) additionally requires Requirement/ADR traceability Edges from Issue-adjacent Nodes. No separate `ISSUE-REQ` prefix needed — this is a direct case of the pattern demonstrated in §16: new domain concepts compose from existing primitives without new mechanics.

---

## 21. MR/PR Requirements

**[CROSS-REFERENCE]** PR is a pre-registered Node type (GRF-REQ-006); merge/rebase mechanics are GIT-REQ-003; CI-linked results are CI-REQ-001/002; review composition is §19 above. `[PROPOSAL]` One gap worth flagging explicitly rather than silently assuming covered: **draft/WIP PR state** and **stacked/dependent PR chains** were not elicited as atomic Phase 7 requirements. Both are realizable as Node properties/Edges under the existing model (a `depends_on` Edge between PR Nodes for stacking; a `draft` status property) with no new primitive required, but neither has a dedicated requirement ID yet. Recommend Phase 9 confirm whether this is MVP-relevant or safely deferred — tracked in §54 as a Product Decision open question.

---

## 22. CI/CD Requirements

**[CROSS-REFERENCE]** Full requirement set: [`./phase7-elicitation.md` §8 CI-REQ](./phase7-elicitation.md#8-ci-req--cicd) (CI-REQ-001 through 006). Summary table:

| ID | Title | Priority |
|---|---|---|
| CI-REQ-001 | CI runs as first-class graph Nodes | P0 |
| CI-REQ-002 | Test results as Evidence-typed Edges | P1 |
| CI-REQ-003 | Pipeline definitions are graph-native, not a hidden YAML silo | P1 |
| CI-REQ-004 | Existing CI ecosystem compatibility (webhook/exec compatibility) | P1 |
| CI-REQ-005 | Agent-triggerable and agent-consumable CI | P1 |
| CI-REQ-006 | Deployment as a Policy-gated Action-convention | P1 |

CI-REQ-006's rollback-semantics detail remains `[TBD]`, carried forward from Phase 6/7's Argo CD/Workflows research gap (O13) — see §54.

---

## 23. Artifact Requirements

**[NEW SYNTHESIS]** — `[PROPOSAL]`, tagged as a gap not covered by Phase 7. Phase 5's Capability Matrix scored package/artifact registry as `⚠️ [PROPOSAL, MVP-deferred]` for the Target Platform, consistent with GitHub/GitLab having one (✅) but it being outside this program's differentiator focus. A small requirement set, prefix `ART-REQ`, kept intentionally thin:

**ART-REQ-001 — Build artifacts linked to Release/CI-run Nodes**
Description: The platform SHOULD support attaching build artifacts (binaries, container images, generated packages) to a Release or CI-run Node via `produced` Edges, retrievable by reference, without requiring the artifact registry itself to be a bespoke package-manager reimplementation.
Priority: P2. Source: Phase 5 Gap Analysis (MVP-deferred). Dependencies: GRF-REQ-002, CI-REQ-001. Evidence: `[PROPOSAL]`.

**ART-REQ-002 — Content-addressed, deduplicated artifact storage**
Description: Stored artifacts MUST be content-addressed to avoid duplicate storage of identical build outputs across CI runs, mirroring GIT-REQ-004's LFS deduplication approach.
Priority: P2. Source: [PROPOSAL], consistent with GIT-REQ-004. Dependencies: ART-REQ-001. Evidence: `[PROPOSAL]`.

**ART-REQ-003 — Standard package-format compatibility (interop, not reimplementation)**
Description: Where the platform exposes a package registry, it SHOULD speak at least one existing standard package-client protocol (e.g., a container registry API, or a language-ecosystem package protocol) rather than inventing a proprietary client protocol, mirroring the Git/CI ecosystem-compatibility principle applied elsewhere (GIT-REQ non-goals, CI-REQ-003 rationale).
Priority: P3 (Future). Source: [PROPOSAL]. Evidence: `[PROPOSAL]`.

Per Phase 5's "MVP-deferred" characterization, this domain is explicitly V2/Future-timed in §49, not MVP.

---

## 24. Search Requirements

**[NEW SYNTHESIS]** — `[PROPOSAL]`, informed by Phase 5's Sourcegraph findings. Prefix `SRCH-REQ`, small set:

**SRCH-REQ-001 — Code search across repositories in scope**
Description: The platform MUST provide code search (text and, at minimum, symbol-aware) across all repositories a Human/Agent is authorized to access, without requiring a separate deployed product (avoiding the Sourcegraph pattern of code search as a bolt-on layer atop another system of record, per O6).
Priority: P1. Source: phase1-5 §Phase 3 Sourcegraph deep dive, O6. Dependencies: GRF-REQ-008 (graph query), SEC-REQ-001. Evidence: `[FACT]` per O6 (Sourcegraph's differentiator); requirement `[PROPOSAL]`.

**SRCH-REQ-002 — Graph-aware search (search results linked to graph Nodes)**
Description: Search results MUST resolve to addressable graph Nodes (Commit, Symbol, PR, Issue) where such a Node exists, not just plain-text file matches — allowing a search result to be a starting point for a graph traversal (e.g., "what depends on this symbol"), not a dead end.
Priority: P1. Source: [PROPOSAL], extends phase6-primitives §6 (Symbol Node type). Dependencies: SRCH-REQ-001, GRF-REQ-002. Evidence: `[PROPOSAL]`.

**SRCH-REQ-003 — Search index freshness bound**
Description: The search index MUST reflect pushed changes within a documented, bounded staleness window, and that window MUST be queryable/surfaced (not silently stale), addressing the "search staleness" risk this program's Risk Register (§53) flags explicitly.
Priority: P1. Source: [PROPOSAL]. Specific staleness bound: `[TBD] – Benchmark Required` (see §44). Dependencies: SRCH-REQ-001. Evidence: `[PROPOSAL]`.

**SRCH-REQ-004 — Cross-repo search respects access boundaries**
Description: Search MUST NOT surface content (including matches, snippets, or symbol names) from a repository the requesting Human/Agent is not authorized to read, even in aggregate/ranked results.
Priority: P0. Source: [PROPOSAL], extends SEC-REQ-001. Dependencies: SRCH-REQ-001, SEC-REQ-001. Evidence: `[PROPOSAL]`.

---

## 25. Engineering Graph Requirements

**[CROSS-REFERENCE]** Full requirement set: [`./phase7-elicitation.md` §3 GRF-REQ](./phase7-elicitation.md#3-grf-req--engineering-graph) (GRF-REQ-001 through 011). Summary table:

| ID | Title | Priority |
|---|---|---|
| GRF-REQ-001 | Typed Node registry | P0 |
| GRF-REQ-002 | Typed, directed Edge primitive | P0 |
| GRF-REQ-003 | Immutable Event log | P0 |
| GRF-REQ-004 | Policy evaluation primitive | P0 |
| GRF-REQ-005 | View primitive (queryable projections) | P0 |
| GRF-REQ-006 | Core Node type registry seeded from spec vocabulary | P0 |
| GRF-REQ-007 | Core Edge type registry | P0 |
| GRF-REQ-008 | Graph query API (human and agent-facing) | P0 |
| GRF-REQ-009 | Action-as-Event-convention for in-flight state | P1 |
| GRF-REQ-010 | Evidence-as-specialized-Edge convention | P1 |
| GRF-REQ-011 | Requirement and ADR as Node types with traceability Edges | P1 |

This is the single largest concentration of P0 requirements in the program, consistent with Phase 5's identification of the graph as the platform's primary differentiator.

---

## 26. Context Engine Requirements

**[CROSS-REFERENCE]** Full requirement set: [`./phase7-elicitation.md` §6 CTX-REQ](./phase7-elicitation.md#6-ctx-req--context-engine-context-as-a-service) (CTX-REQ-001 through 005). Summary table:

| ID | Title | Priority |
|---|---|---|
| CTX-REQ-001 | Minimal-context retrieval by default | P0 |
| CTX-REQ-002 | Context assembly is a logged View invocation | P0 |
| CTX-REQ-003 | Cross-repository context assembly | P1 |
| CTX-REQ-004 | Explicit context budget and truncation transparency | P1 |
| CTX-REQ-005 | Portable, vendor-neutral project instructions (AGENTS.md-equivalent) | P1 |

CTX-REQ-002's exact-reconstruction guarantee remains dependent on Phase 10's unresolved graph-storage-versioning decision (carried from Phase 6 §7 and Phase 7 §14) — see §54.

---

## 27. AI Gateway Requirements

**[CROSS-REFERENCE]** Full requirement set: [`./phase7-elicitation.md` §5 AI-REQ](./phase7-elicitation.md#5-ai-req--ai-gateway--model-routing) (AI-REQ-001 through 006). Summary table:

| ID | Title | Priority |
|---|---|---|
| AI-REQ-001 | Multi-provider model access | P0 |
| AI-REQ-002 | Cost and token observability per request | P0 |
| AI-REQ-003 | Routing policy by data sensitivity | P0 |
| AI-REQ-004 | Provider fallback and degradation behavior | P1 |
| AI-REQ-005 | Model/provider swap without application changes | P1 |
| AI-REQ-006 | Prompt/response caching for repeated context | P2 |

---

## 28. Agent Runtime Requirements

**[CROSS-REFERENCE]** Full requirement set: [`./phase7-elicitation.md` §4 AGT-REQ](./phase7-elicitation.md#4-agt-req--agent-runtime) (AGT-REQ-001 through 009). Summary table:

| ID | Title | Priority |
|---|---|---|
| AGT-REQ-001 | Vendor-neutral agent lifecycle control | P0 |
| AGT-REQ-002 | Human approve/reject gate on agent actions | P0 |
| AGT-REQ-003 | Agent messaging and status interface | P0 |
| AGT-REQ-004 | Agent artifact capture | P0 |
| AGT-REQ-005 | Scoped, workspace-isolated agent execution | P0 |
| AGT-REQ-006 | Resource limits on agent execution | P0 |
| AGT-REQ-007 | Agent as a Node subtype with full graph participation | P0 |
| AGT-REQ-008 | Multi-agent-framework interoperability via MCP | P1 |
| AGT-REQ-009 | Subagent / delegated-scope execution | P2 |

---

## 29. Codex Integration Requirements

**[NEW SYNTHESIS]** — `[PROPOSAL]`, prefix `CDX-REQ`. Specific to Codex/MCP interop per the original spec's dual-direction architecture ("Platform MCP" exposing the platform's own tools to external agents, and "Agent Runtime → Codex" consuming Codex/other external agents as executors). Distinct from AGT-REQ-008's general MCP support in that these requirements are about specific cross-tool compatibility conventions, not the MCP protocol mechanics themselves.

**CDX-REQ-001 — AGENTS.md compatibility as a first-class, not fallback, convention**
Description: The platform MUST treat an AGENTS.md-equivalent file's contents as directly consumable by its own Agent runtime (not merely by external agents connecting via MCP), so a repository's AGENTS.md works identically whether the acting agent is platform-native or an external tool like Codex.
Priority: P1. Source: phase1-5 O11 (AGENTS.md's 8+-tool cross-vendor adoption), extends CTX-REQ-005. Dependencies: CTX-REQ-005, AGT-REQ-001. Evidence: `[FACT]` per O11; requirement `[PROPOSAL]`.

**CDX-REQ-002 — Platform as an MCP server (Platform MCP)**
Description: The platform MUST expose its graph query (GRF-REQ-008), View (GRF-REQ-005), and Agent runtime (AGT-REQ-003) capabilities as standard MCP tools, callable by external agent clients (Codex, Claude Code, Cursor, etc.) without requiring those clients to be platform-aware beyond standard MCP handshake.
Priority: P1. Source: [PROPOSAL], extends AGT-REQ-008. Dependencies: AGT-REQ-008, GRF-REQ-005, GRF-REQ-008. Evidence: `[PROPOSAL]`.

**CDX-REQ-003 — Platform as an MCP client (Agent Runtime → Codex/external agents)**
Description: The Agent runtime MUST be able to invoke Codex (or another external agent CLI/service) as a scoped executor for a delegated task, via MCP or an equivalent documented interface, with the external agent's actions still subject to AGT-REQ-002 approval gating and AGT-REQ-005 scope isolation as if it were a platform-native agent.
Priority: P2. Source: [PROPOSAL], extends AGT-REQ-001's vendor-neutrality. Dependencies: AGT-REQ-001, AGT-REQ-002, AGT-REQ-005, AGT-REQ-008. Risks: an external agent's internal actions (its own tool calls) are not directly observable by the platform's Event log the way a native agent's are — only the task boundary is; this is a real provenance-completeness gap, not fully solved by this requirement. Evidence: `[PROPOSAL]`.

**CDX-REQ-004 — Approval-mode mapping across agent vendors**
Description: Where an external agent (e.g., Codex) has its own native approval-mode concept (suggest/auto-edit/full-auto per Codex's documented modes, `[FACT]`), the platform MUST map its own Policy-gated approval model (AGT-REQ-002) onto that agent's native modes rather than silently ignoring one or the other, so approval semantics don't diverge between "what Codex thinks it's allowed to do" and "what the platform's Policy actually permits."
Priority: P2. Source: phase1-5 §Phase 2 Codex CLI ("Agent approvals & security", official OpenAI docs, `[FACT]`). Dependencies: AGT-REQ-002, CDX-REQ-003. Evidence: `[FACT]` per Codex approval-mode documentation; requirement `[PROPOSAL]`.

---

## 30. Intent Commit Requirements

**[NEW SYNTHESIS]** — `[PROPOSAL]`, prefix `ITC-REQ`. Per original spec §11: IntentCommit is an engineering-semantic layer *over* the standard git commit, not a replacement for it — this is explicitly Experimental per Phase 5's Gap Analysis ("no competitor surveyed demonstrated this"), and is deferred to V2/Future in §49, not MVP or V1. Kept deliberately small.

**ITC-REQ-001 — IntentCommit as an optional Evidence-typed annotation on Commit Nodes**
Description: The platform MAY support attaching a structured "intent" annotation (why this commit exists, what Requirement/Issue it addresses, expected behavioral change) to a Commit Node via an Evidence-typed Edge (GRF-REQ-010), without altering the underlying Git commit object itself.
Priority: P3 (Experimental). Source: original spec §11; Phase 5 Gap Analysis (Experimental tier). Dependencies: GRF-REQ-010, GIT-REQ-002. Risks: risk of becoming a mandatory-feeling extra step that developers route around (writing a perfunctory intent annotation just to satisfy the field) — should remain optional and additive, never gating GIT-REQ merge operations. Evidence: `[PROPOSAL]`.

**ITC-REQ-002 — IntentCommit never gates or rewrites standard Git history**
Description: No IntentCommit feature may require rewriting commit history, altering SHAs, or blocking a standard `git push` for a commit lacking an intent annotation.
Priority: P3 (Experimental), but treat this specific constraint as non-negotiable if ITC-REQ-001 is ever built. Source: [PROPOSAL], direct consequence of GIT-REQ principle (§13). Dependencies: ITC-REQ-001, GIT-REQ-001. Evidence: `[PROPOSAL]`.

This entire domain is explicitly marked Future in §49 — it should not consume MVP/V1 engineering effort.

---

## 31. Semantic Diff Requirements

**[NEW SYNTHESIS]** — `[PROPOSAL]`, prefix `DIFF-REQ`. Explicitly Experimental per Phase 5's Gap Analysis ("No competitor surveyed demonstrated this as a shipped capability... this remains a research-grade capability"). Deferred to Future in §49.

**DIFF-REQ-001 — Semantic (behavior-aware) diff as an optional View, not a default**
Description: The platform MAY offer a semantic-diff View (highlighting behavioral changes, not just line-level text changes) as an opt-in capability for supported languages, layered on top of the standard text diff (which remains the default and MUST always be available, per GIT-REQ compatibility).
Priority: P3 (Experimental/Future). Source: Phase 5 Gap Analysis (Experimental tier — "Unknown" across the entire competitive matrix for this capability). Dependencies: GRF-REQ-005 (View primitive). Evidence: `[PROPOSAL]`.

**DIFF-REQ-002 — Semantic diff correctness is advisory, never authoritative for merge decisions**
Description: A semantic-diff View's output MUST NOT be used as the sole basis for a Policy gate (GRF-REQ-004) blocking a merge — it is a human/agent aid, not a verifier, given the research-grade maturity of the underlying technique.
Priority: P3 (Experimental/Future). Source: [PROPOSAL], risk mitigation given Phase 5's "research-grade, not proven" characterization. Dependencies: DIFF-REQ-001. Evidence: `[PROPOSAL]`.

---

## 32. Repository Digital Twin Requirements

**[NEW SYNTHESIS]** — `[PROPOSAL]`, tied to CTX-REQ, mostly Future. Phase 5 characterizes this as "a natural extension of the knowledge-graph differentiator... but represents a multi-year research investment, not something to attempt in an MVP."

**DTWIN-REQ-001 — Historical graph state is replayable via Event log, not a separate simulation engine**
Description: Because Events are immutable and timestamped (GRF-REQ-003), the platform MUST support reconstructing the graph's state as of any past point in time by replaying/filtering the Event log — this is the necessary substrate for a future "digital twin" capability, without committing to building the twin's simulation/what-if layer itself.
Priority: P2 (substrate requirement, MVP-adjacent even though the "digital twin" product concept itself is Future). Source: [PROPOSAL], direct consequence of GRF-REQ-003's immutability property. Dependencies: GRF-REQ-003. Evidence: `[PROPOSAL]`.

**DTWIN-REQ-002 — "What changed and why" queries across a time range**
Description: A View SHOULD exist answering "what Nodes/Edges changed between time T1 and T2, and what Events/actors caused each change" as a bounded, performant query — the practical, near-term expression of digital-twin value before any full simulation capability is attempted.
Priority: P2. Source: [PROPOSAL], extends DTWIN-REQ-001. Dependencies: DTWIN-REQ-001, GRF-REQ-005. Evidence: `[PROPOSAL]`.

**DTWIN-REQ-003 (Future) — Full simulation/what-if replay**
Description: `[TBD]` — a full "digital twin" capability (simulating the effect of a hypothetical change against historical graph state) is explicitly out of scope for MVP/V1/V2 and is not specified further here; Phase 5 characterizes it as a multi-year research investment. Revisit only after DTWIN-REQ-001/002 are in production and real usage patterns justify the investment.
Priority: P3 (Future, research-stage). Source: Phase 5 Gap Analysis (Experimental tier). Evidence: `[TBD]`.

---

## 33. Emergent Workflow Requirements

**[NEW SYNTHESIS]** — `[PROPOSAL]`, tied to original spec §18 (AI Suggest → Human Confirm workflow generation). Mostly V2/Future.

**WKFL-REQ-001 — Workflow as a Policy-gated chain of Action-convention types**
Description: A "Workflow" (a recognized, repeatable chain of engineering activity, e.g., "issue triage → assignment → PR → review → merge → release") MUST be representable as a named sequence of Action-convention (GRF-REQ-009) steps connected by Policy-gated Edges, per the composition pattern already sketched in [`./phase6-primitives.md` §4.4](./phase6-primitives.md#44-release) ("Workflow as a Policy-gated chain of Action types with Edges enforcing ordering").
Priority: P2. Source: phase6-primitives §4.4 (parenthetical worked-example note). Dependencies: GRF-REQ-004, GRF-REQ-009. Evidence: `[PROPOSAL]`.

**WKFL-REQ-002 (V2/Future) — AI-suggested workflow generation, human-confirmed before activation**
Description: The platform MAY suggest a new Workflow definition (WKFL-REQ-001) based on observed repeated patterns in the Event log (e.g., "this five-step sequence has occurred manually 40 times this quarter"), but any suggested Workflow MUST be explicitly reviewed and confirmed by a human before it can gate real Actions — this is a direct application of the Human Authority principle (§13) to workflow automation itself, not just individual agent actions.
Priority: P3 (V2/Future). Source: original spec §18 ("AI Suggest → Human Confirm"), Human Authority principle (§13). Dependencies: WKFL-REQ-001, AGT-REQ-002 (pattern reused), DTWIN-REQ-002 (pattern detection over historical Events). Risks: pattern-detection quality is unproven and easy to overfit to noisy historical data; must never auto-activate a suggested workflow without the explicit confirmation step. Evidence: `[PROPOSAL]`.

Both are explicitly V2/Future-timed in §49; WKFL-REQ-001's substrate (Action-convention chains) already exists at MVP via GRF-REQ-009, so building a *named* Workflow on top of it is a comparatively low-risk V2 addition, but the AI-suggestion piece (WKFL-REQ-002) is not.

---

## 34. UX Requirements

**[CROSS-REFERENCE]** Full requirement set: [`./phase7-elicitation.md` §9 UX-REQ](./phase7-elicitation.md#9-ux-req--high-level-ux-principles) (UX-REQ-001 through 004) — deliberately small; full UX journeys, visual design, and usability validation are Phase 12's job, not duplicated here. Summary table:

| ID | Title | Priority |
|---|---|---|
| UX-REQ-001 | Intent-first entry point | P1 |
| UX-REQ-002 | Context-assembled pages, not fixed page templates | P1 |
| UX-REQ-003 | Human authority is visible and actionable, not just logged | P1 |
| UX-REQ-004 | Progressive disclosure of graph complexity | P1 |

---

## 35. Security Requirements

**[CROSS-REFERENCE]** Full requirement set: [`./phase7-elicitation.md` §7 SEC-REQ](./phase7-elicitation.md#7-sec-req--security) (SEC-REQ-001 through 007). Summary table:

| ID | Title | Priority |
|---|---|---|
| SEC-REQ-001 | Role- and attribute-based access control | P0 |
| SEC-REQ-002 | Temporary, scoped agent credentials | P0 |
| SEC-REQ-003 | Structural audit trail (not a bolt-on subsystem) | P0 |
| SEC-REQ-004 | Least-privilege default for new agents and integrations | P0 |
| SEC-REQ-005 | Secrets and credential storage isolated from the graph | P0 |
| SEC-REQ-006 | Signed, non-repudiable Event authorship | P2 |
| SEC-REQ-007 | Compliance-scoped Event export | P2 |

---

## 36. AI Security Requirements

**[NEW SYNTHESIS]** — `[PROPOSAL]`, prefix `AISEC-REQ`. Per original spec §24: prompt injection, MCP tool poisoning, secret exfiltration via AI, and agent privilege escalation. Not covered by Phase 7's SEC-REQ set, which addresses general RBAC/audit but not AI-specific attack surfaces. Given real effort here as instructed.

**AISEC-REQ-001 — Prompt injection containment from untrusted content**
Description: Content an agent retrieves from an untrusted or lower-trust source (an external web page fetched via a tool, a PR description from an external contributor, an issue comment) MUST be treated as data, not as instructions, by the Agent runtime's context-assembly path — the platform MUST provide a documented mechanism (e.g., content provenance tagging on context passed to a model) that lets the underlying model/prompt distinguish "trusted instruction" from "untrusted retrieved content."
Rationale: Prompt injection via untrusted repository/issue/PR content is the most immediate, highest-likelihood AI-specific attack against an agent-native platform — any agent reading issue comments or external PRs is exposed by default unless this is designed in.
Priority: P0. Source: [PROPOSAL], original spec §24. Dependencies: CTX-REQ-002 (logged View invocation — needed to audit what untrusted content an agent actually saw), AGT-REQ-005. Acceptance Criteria: an integration test where an Issue Node's description contains an embedded instruction ("ignore previous instructions and...") does not cause an agent processing that Issue to take an out-of-scope Policy-gated Action without triggering AGT-REQ-002's approval gate. Risks: this is a hard, actively-researched problem industry-wide; the requirement bounds blast radius (Policy gating still applies) rather than claiming to "solve" prompt injection outright. Evidence: `[PROPOSAL]`.

**AISEC-REQ-002 — MCP tool/server allowlisting per Agent scope**
Description: An Agent's Policy scope (AGT-REQ-005) MUST explicitly enumerate which MCP servers/tools it may invoke; an Agent MUST NOT be able to discover and call an arbitrary MCP server not in its allowlist, even if one is reachable on the network/host.
Rationale: Directly addresses "MCP tool poisoning" — a malicious or compromised MCP server presenting itself with a plausible tool description to get invoked by an unsuspecting agent.
Priority: P0. Source: [PROPOSAL], original spec §24. Dependencies: AGT-REQ-005, AGT-REQ-008, GRF-REQ-004. Acceptance Criteria: an agent run configured with an MCP allowlist of {Tool A, Tool B} fails a call to unlisted Tool C with a Policy-violation Event, verified by a negative test. Evidence: `[PROPOSAL]`.

**AISEC-REQ-003 — MCP tool description integrity verification**
Description: Where feasible, the platform SHOULD verify that an MCP tool's advertised description/schema has not changed unexpectedly between agent runs (a "rug pull" attack where a tool behaves differently than what was reviewed/approved), flagging a detected change as a Policy-relevant Event requiring re-approval before continued use.
Rationale: Extends AISEC-REQ-002 — allowlisting a tool by name is insufficient if the tool's actual behavior/schema can silently change after approval.
Priority: P1. Source: [PROPOSAL], original spec §24. Dependencies: AISEC-REQ-002, GRF-REQ-003. Evidence: `[PROPOSAL]`.

**AISEC-REQ-004 — Secret-value exclusion from AI Gateway payloads**
Description: The AI Gateway (AI-REQ-001) MUST NOT include values from SEC-REQ-005-flagged secret properties in any request payload sent to a model provider, including indirectly via assembled Context (CTX-REQ-001/002) — this is the AI-specific extension of SEC-REQ-005's graph-query-level secret redaction to the model-inference path specifically.
Rationale: A graph query can already redact secrets (SEC-REQ-005), but an AI Gateway request is a distinct data path that could bypass that redaction if not independently enforced — e.g., an agent reading a config file containing an inline credential and including it verbatim in a prompt.
Priority: P0. Source: [PROPOSAL], extends SEC-REQ-005, original spec §24. Dependencies: SEC-REQ-005, AI-REQ-001, CTX-REQ-001. Acceptance Criteria: a test context containing a known secret pattern (e.g., a flagged API-key-shaped string) is scrubbed or blocked before an outbound Gateway request, verified by inspecting the actual request payload sent to a mock provider. Evidence: `[PROPOSAL]`.

**AISEC-REQ-005 — Agent privilege escalation prevention via delegation**
Description: A subagent or delegated agent run (AGT-REQ-009) MUST NOT be able to acquire a Policy scope broader than its parent's, including indirectly via a chain of delegations (transitive scope containment), regardless of how many delegation hops occur.
Rationale: This restates and strengthens AGT-REQ-009's existing "no privilege escalation via delegation" acceptance criterion, extending it explicitly to *transitive* chains (A delegates to B delegates to C), which the original AGT-REQ-009 wording does not explicitly cover.
Priority: P0. Source: [PROPOSAL], extends AGT-REQ-009, original spec §24. Dependencies: AGT-REQ-009, SEC-REQ-001. Acceptance Criteria: a three-hop delegation chain attempting to widen scope at any hop is rejected, verified by a negative test covering at least two hops (not just direct parent-child). Evidence: `[PROPOSAL]`.

**AISEC-REQ-006 — Agent output does not bypass Policy via generated code/config**
Description: Code or configuration generated/modified by an agent MUST still pass through the same Policy gates (GRF-REQ-004) as human-authored changes before merge/deploy — an agent MUST NOT be able to self-certify its own output as policy-compliant by virtue of having generated it.
Rationale: Addresses a subtle escalation path: an agent modifying CI config or Policy definitions themselves as part of its task, potentially weakening the very gates meant to constrain it.
Priority: P0. Source: [PROPOSAL], original spec §24. Dependencies: GRF-REQ-004, AGT-REQ-002. Acceptance Criteria: an agent-authored change to a Policy definition itself requires the same (or a stricter) human-approval gate as any other Policy-gated Action, verified by a test where an agent attempts to modify a branch-protection Policy. Evidence: `[PROPOSAL]`.

**AISEC-REQ-007 — Rate-limited and anomaly-flagged agent behavior**
Description: The platform SHOULD detect and flag anomalous agent behavior patterns (e.g., a sudden spike in tool calls, repeated failed Policy checks, unusual data-access patterns) as a distinct Event type, surfaced to the responsible human/team, independent of the hard resource limits already required by AGT-REQ-006.
Rationale: AGT-REQ-006 bounds cost/runtime; this requirement addresses the different problem of a compromised or malfunctioning agent operating *within* its resource limits but behaving suspiciously (e.g., systematically probing repository access it doesn't normally use).
Priority: P2. Source: [PROPOSAL], original spec §24. Dependencies: AGT-REQ-006, GRF-REQ-003. Evidence: `[PROPOSAL]`.

**AISEC-REQ-008 — External MCP server sandboxing**
Description: Where an Agent invokes an external (non-platform-provided) MCP server, that invocation MUST occur from within the same workspace-isolated execution boundary required by AGT-REQ-005, not from a more privileged context, even though the MCP server itself is a separate process/service outside the platform's direct control.
Rationale: An external MCP server is inherently less trusted than a platform-native tool; its blast radius if compromised or malicious must be bounded by the same isolation guarantee as the rest of the agent's execution.
Priority: P1. Source: [PROPOSAL], extends AGT-REQ-005 and AISEC-REQ-002. Dependencies: AGT-REQ-005, AISEC-REQ-002. Evidence: `[PROPOSAL]`.

---

## 37. Permission Requirements

**[CROSS-REFERENCE]** RBAC/ABAC mechanics are fully specified at SEC-REQ-001 (§35); least-privilege defaults at SEC-REQ-004; agent credential scoping at SEC-REQ-002/AGT-REQ-005. Delta beyond SEC-REQ: none identified as a gap requiring a new requirement — this section exists in the 55-section outline primarily to make explicit that "permissions" is not a separate subsystem from the general Policy-based access-control model (§16, §35), consistent with this document's recurring theme (§13 Principle 6: graph-native) that access control is one more thing Policy (GRF-REQ-004) evaluates, not a bespoke permissions engine.

---

## 38. Audit Requirements

**[CROSS-REFERENCE]** The audit trail is fully specified at SEC-REQ-003 (structural audit trail, not a bolt-on subsystem) and SEC-REQ-007 (compliance-scoped Event export), both in §35, with the underlying mechanism being GRF-REQ-003's immutable Event log (§25/§16). Delta beyond SEC-REQ: none identified — this is the direct, load-bearing architectural response to Phase 1–5's O2 finding (GitHub's audit log outage) and is deliberately *not* split into a separate audit subsystem requirement set, because doing so would risk recreating the exact failure mode (audit as a separately-architected, independently-failable subsystem) this program's evidence identifies as the problem.

---

## 39. Local Deployment Requirements

**[CROSS-REFERENCE]** Full requirement set: [`./phase7-elicitation.md` §10 OPS-REQ](./phase7-elicitation.md#10-ops-req--local-first-deployment) (OPS-REQ-001 through 005). Summary table:

| ID | Title | Priority |
|---|---|---|
| OPS-REQ-001 | Single-machine, fully offline deployable | P0 |
| OPS-REQ-002 | AI subsystem independently disableable | P0 |
| OPS-REQ-003 | LAN-scoped multi-user operation without cloud dependency | P0 |
| OPS-REQ-004 | Backup and restore of the full graph and Git object store | P0 |
| OPS-REQ-005 | Resource footprint documented and bounded for small deployments | P1 |

---

## 40. Cloud Requirements

**[CROSS-REFERENCE]** Full requirement set: [`./phase7-elicitation.md` §11 CLOUD-REQ](./phase7-elicitation.md#11-cloud-req--cloud-readiness) (CLOUD-REQ-001 through 004). Summary table:

| ID | Title | Priority |
|---|---|---|
| CLOUD-REQ-001 | Same core architecture across local and cloud deployment | P0 |
| CLOUD-REQ-002 | Documented, tested local-to-cloud migration path | P1 |
| CLOUD-REQ-003 | Horizontal scale-out without primitive-model changes | P1 |
| CLOUD-REQ-004 | Multi-tenant isolation for managed/cloud offering | P2 |

CLOUD-REQ-003 remains dependent on Phase 10's unresolved storage-architecture decision (Phase 6 §7) — see §54.

---

## 41. Offline/Air-Gapped Requirements

**[CROSS-REFERENCE]** Fully covered by OPS-REQ-001 (single-machine, fully offline deployable) and AI-REQ-003 (routing policy by data sensitivity — the mechanism preventing any AI feature from silently phoning home when a repository is flagged air-gapped/sensitive), both above. Delta beyond OPS-REQ/AI-REQ: `[TBD]` whether Policy evaluation itself can run fully locally without a round-trip to a central service in air-gapped mode is an explicitly unresolved architecture question carried from Phase 6 §7 and Phase 7 §14 (GRF-REQ-004's Risks) — see §54; no new requirement is written here pending that resolution, consistent with this document's instruction not to presume an unresolved architecture decision.

---

## 42. Observability Requirements

**[NEW SYNTHESIS]** — `[PROPOSAL]`, prefix `OBS-REQ`, small set, per original spec §34–35 (AgentRun telemetry).

**OBS-REQ-001 — Per-AgentRun telemetry stream**
Description: Every AgentRun Node MUST expose a structured telemetry stream (tool calls, latency per step, token/cost consumption, Policy evaluations) queryable in real time during execution and retrievable after completion, building directly on AGT-REQ-003's status/events interface and AI-REQ-002's per-request cost logging.
Priority: P1. Source: original spec §34–35. Dependencies: AGT-REQ-003, AI-REQ-002, GRF-REQ-003. Evidence: `[PROPOSAL]`.

**OBS-REQ-002 — Platform-level health metrics independent of AI subsystem**
Description: Core platform health (Git-serving latency/error rate, graph-query latency/error rate) MUST be observable and MUST remain reportable even when the AI Gateway/Agent runtime is degraded or disabled, mirroring GIT-REQ-010/OPS-REQ-002's isolation requirement extended to the observability surface itself.
Priority: P1. Source: [PROPOSAL], extends GIT-REQ-010. Dependencies: GIT-REQ-010, OPS-REQ-002. Evidence: `[PROPOSAL]`.

**OBS-REQ-003 — Standard metrics/tracing export format**
Description: The platform SHOULD export metrics and traces in a standard, widely-supported format (e.g., an OpenTelemetry-compatible interface) rather than a proprietary-only format, so self-hosted operators can plug into existing observability tooling.
Priority: P2. Source: [PROPOSAL]. Dependencies: OBS-REQ-001, OBS-REQ-002. Evidence: `[PROPOSAL]`.

---

## 43. Backup/Recovery Requirements

**[NEW SYNTHESIS]** — `[PROPOSAL]`, prefix `BKP-REQ`. OPS-REQ-004 (§39) already establishes the core backup/restore requirement; this section adds detail, but most specifics genuinely need Phase 10 architecture decisions first (storage engine, single-store-vs-split-store, per Phase 6 §7) and are tagged `[TBD]`/Research Needed accordingly rather than invented.

**BKP-REQ-001 — Point-in-time consistency across Git store and graph store**
Description: A backup MUST capture the Git object store and the Node/Edge/Event/Policy graph store at a mutually consistent logical point in time — this directly restates OPS-REQ-004's flagged Risk as an explicit requirement rather than leaving it implicit.
Priority: P0. Source: OPS-REQ-004 (Risks). Dependencies: OPS-REQ-004. `[TBD]` — the specific mechanism (snapshot coordination, transactional boundary) depends entirely on Phase 10's storage-architecture decision (Phase 6 §7); cannot be made more concrete here. Evidence: `[PROPOSAL]`.

**BKP-REQ-002 — Documented recovery time/point objectives**
Description: The platform's documentation MUST state its supported Recovery Time Objective (RTO) and Recovery Point Objective (RPO) for both local/self-hosted and cloud deployment modes.
Priority: P1. Source: [PROPOSAL]. Specific RTO/RPO figures: `[TBD] – Benchmark Required`, per this program's discipline against inventing numbers (see §44). Dependencies: OPS-REQ-004, BKP-REQ-001. Evidence: `[TBD]`.

**BKP-REQ-003 (Research Needed) — Incremental/continuous backup vs. periodic snapshot**
Description: `[TBD]` — whether the platform supports continuous/incremental backup (lower RPO, higher operational complexity) or only periodic full snapshots for self-hosted deployments is unresolved; depends on Phase 10's storage engine choice and is not a product-research question this phase can settle.
Priority: P2. Source: [PROPOSAL]. Evidence: `[TBD]`.

Most of this domain is correctly left `[TBD]`/Research Needed per the task's own instruction — see §54.

---

## 44. Performance Requirements

**[NEW SYNTHESIS]** — mostly `TBD – Benchmark Required`, per the original spec's explicit instruction not to invent numbers. This document states the *categories* needing benchmarks rather than fabricating figures.

| Category | Why it matters | Status |
|---|---|---|
| Git operation latency (clone/fetch/push, incl. partial clone per GIT-REQ-007) at monorepo scale | Positioning against GitLab's self-hosting weight (§13 Principle 2) depends on this being competitive | `[TBD] – Benchmark Required` |
| Graph query latency (GRF-REQ-008) at representative Node/Edge counts | Direct precondition for CTX-REQ-001's "bounded latency" acceptance criterion | `[TBD] – Benchmark Required` |
| Context assembly latency for a typical agent task (CTX-REQ-001) | User-facing agent responsiveness | `[TBD] – Benchmark Required` |
| Policy evaluation latency in blocking mode (GRF-REQ-004) | Directly affects push/merge latency if evaluated synchronously | `[TBD] – Benchmark Required` |
| Search index freshness window (SRCH-REQ-003) | Staleness risk explicitly flagged in Risk Register (§53) | `[TBD] – Benchmark Required` |
| Concurrent-user ceiling for a single-machine deployment (OPS-REQ-003/005) | Defines the practical "small team" boundary before cloud/scale-out is needed | `[TBD] – Benchmark Required` |
| Horizontal scale-out throughput characteristics (CLOUD-REQ-003) | Depends on Phase 10's storage-architecture decision; cannot be benchmarked before that decision | `[TBD] – Benchmark Required, blocked on Phase 10` |

No specific millisecond/throughput figures are stated anywhere in this document. Any number appearing to be a performance target elsewhere in this document (there should be none) should be treated as an error and reported.

---

## 45. Reliability Requirements

**[NEW SYNTHESIS]** — `[PROPOSAL]`, prefix `REL-REQ`, small set.

**REL-REQ-001 — Git-serving availability independent of graph/AI subsystem health**
Description: Core Git read/write operations MUST remain available even if the graph query engine or AI subsystem is degraded, directly extending GIT-REQ-010's correctness requirement into an availability/SLA-shaped requirement.
Priority: P0. Source: [PROPOSAL], extends GIT-REQ-010. Dependencies: GIT-REQ-010. Specific availability target: `[TBD] – Benchmark/SLA Required`. Evidence: `[PROPOSAL]`.

**REL-REQ-002 — Graceful degradation, not cascading failure, on Policy-evaluation timeout**
Description: If a blocking Policy evaluation (GRF-REQ-004) cannot complete within a bounded timeout, the platform MUST fail closed (reject the gated Action) with a clear, distinguishable error, rather than hanging indefinitely or silently allowing the Action through.
Priority: P0. Source: [PROPOSAL], extends GRF-REQ-004. Dependencies: GRF-REQ-004. Evidence: `[PROPOSAL]`.

**REL-REQ-003 — No single-region/single-node dependency for cloud deployment**
Description: A cloud deployment configured for high availability MUST NOT have a single point of failure in the Git-serving or graph-query path, consistent with CLOUD-REQ-003's horizontal scale-out requirement.
Priority: P1. Source: [PROPOSAL], extends CLOUD-REQ-003. Dependencies: CLOUD-REQ-003. Evidence: `[PROPOSAL]`.

---

## 46. Data Ownership / Portability

**[NEW SYNTHESIS]** — `[PROPOSAL]`, prefix `DATA-REQ`, per original spec §26 (no vendor lock-in, exportability). Given real effort as instructed — this is a genuine differentiator opportunity given that Linear (Phase 3) offers zero export path competitive to self-hosting, and GitHub/GitLab's migration tooling (GIT-REQ-009) only covers Git objects, not the full engineering graph.

**DATA-REQ-001 — Full graph export in an open, documented format**
Description: The platform MUST support exporting the complete Node/Edge/Event/Policy graph (not just Git objects) in a documented, versioned, open format (not a proprietary binary blob), sufficient to reconstruct the graph's structure and history in another system.
Rationale: Direct response to the "no vendor lock-in" principle — a platform whose core differentiator is the graph must not make that same graph the thing that traps a customer if they want to leave.
Priority: P0. Source: [PROPOSAL], original spec §26. Dependencies: GRF-REQ-001, GRF-REQ-002, GRF-REQ-003. Acceptance Criteria: a full export of a test instance's graph, re-imported into a fresh instance, reproduces an equivalent graph (same Nodes/Edges/Event history) verified by a structural diff. Evidence: `[PROPOSAL]`.

**DATA-REQ-002 — Git objects always exportable via standard Git operations**
Description: Regardless of any platform-specific export tooling, the underlying Git object data MUST always remain retrievable via plain `git clone`/`git fetch` against every repository a user has access to — data ownership of the Git layer specifically must never depend on a platform-specific export feature at all, since GIT-REQ-001 already guarantees standard protocol access.
Rationale: This is the strongest, cheapest form of the no-lock-in principle — because GIT-REQ-001 is already required, Git data can never be "trapped" even if every other export mechanism failed.
Priority: P0. Source: [PROPOSAL], direct consequence of GIT-REQ-001. Dependencies: GIT-REQ-001. Evidence: `[PROPOSAL]`.

**DATA-REQ-003 — No proprietary-only storage of graph-adjacent artifacts**
Description: Artifacts referenced by the graph (AgentRun outputs per AGT-REQ-004, ART-REQ-001 build artifacts) MUST be retrievable via a documented API using standard content-addressing/reference schemes, not locked behind a platform-proprietary retrieval mechanism with no export path.
Priority: P1. Source: [PROPOSAL], extends AGT-REQ-004, ART-REQ-001. Dependencies: AGT-REQ-004. Evidence: `[PROPOSAL]`.

**DATA-REQ-004 — Self-hosted deployment requires no phone-home to operate**
Description: A self-hosted deployment (OPS-REQ-001) MUST be able to operate indefinitely without any network call to a vendor-controlled service for licensing/telemetry/feature-gating — self-hosting must be genuinely independent operation, not "self-hosted but licensed via a cloud check-in," per Phase 3's finding that GitLab's genuine self-hosting parity (vs. GitHub's declining Enterprise Server) is itself a competitive strength worth matching, not merely approximating.
Priority: P0. Source: [PROPOSAL], phase1-5 §Phase 3 GitLab deep dive (self-hosting parity strength), extends OPS-REQ-001. Dependencies: OPS-REQ-001. Evidence: `[INFERENCE]` per GitLab comparison; requirement `[PROPOSAL]`.

**DATA-REQ-005 — Export path covers deleted/superseded data per retention policy, not just current state**
Description: Where an organization's retention policy requires it, the export mechanism (DATA-REQ-001) MUST be able to include historical Event data (not only current Node/Edge state), consistent with GRF-REQ-003's immutable Event log — portability should not silently mean "only today's snapshot."
Priority: P2. Source: [PROPOSAL], extends DATA-REQ-001 and GRF-REQ-003. Dependencies: DATA-REQ-001, GRF-REQ-003, SEC-REQ-007 (compliance-export precedent). Evidence: `[PROPOSAL]`.

---

## 47. API / MCP / Webhook Requirements

**[NEW SYNTHESIS]** — `[PROPOSAL]`, prefix `API-REQ`, consolidating MCP mentions already specified elsewhere (AGT-REQ-008, CDX-REQ-002/003) plus genuinely new API/webhook requirements not covered by Phase 7.

**API-REQ-001 — Single documented API surface covering graph, Git, agent, and CI operations**
Description: The platform MUST expose one documented, versioned API (REST and/or GraphQL) covering graph query (GRF-REQ-008), Agent lifecycle (AGT-REQ-001/003), and CI/CD status (CI-REQ-001), rather than separate, inconsistent API surfaces per subsystem — this is the API-level expression of the graph-native principle (§13).
Priority: P0. Source: [PROPOSAL], extends GRF-REQ-008, AGT-REQ-003, CI-REQ-001. Dependencies: GRF-REQ-008, AGT-REQ-003. Evidence: `[PROPOSAL]`.

**API-REQ-002 — Webhook delivery for graph-relevant Events**
Description: The platform MUST support registering webhooks that fire on a configurable subset of Event types (GRF-REQ-003), with delivery retry and a documented at-least-once delivery guarantee, for integration with external systems that predate the platform's own MCP/API surface.
Priority: P1. Source: [PROPOSAL], extends GRF-REQ-003. Dependencies: GRF-REQ-003. Evidence: `[PROPOSAL]`.

**API-REQ-003 — API access itself is Policy-gated and audited identically to UI access**
Description: Every API/MCP call MUST pass through the same Policy evaluation (GRF-REQ-004) and produce the same Event-log entries (GRF-REQ-003) as an equivalent UI-driven action — there must be no "back door" API path that bypasses graph-native access control.
Priority: P0. Source: [PROPOSAL], direct consequence of §13 Principle 6 (graph-native). Dependencies: GRF-REQ-003, GRF-REQ-004. Evidence: `[PROPOSAL]`.

**API-REQ-004 — API rate limiting and abuse protection**
Description: The API/MCP surface MUST support configurable per-actor (Human or Agent) rate limits, distinct from AGT-REQ-006's agent-specific resource limits, applying uniformly to any API caller including non-agent integrations.
Priority: P1. Source: [PROPOSAL], extends AGT-REQ-006 to the general API surface. Dependencies: API-REQ-001. Evidence: `[PROPOSAL]`.

**API-REQ-005 — MCP tool surface is curated and composable, not a flat dump**
Description: The platform's exposed MCP tool set (CDX-REQ-002) MUST be composable/filterable per Agent scope, avoiding the flat, uncurated tool-count problem observed at competitors (Cursor's 40-tool cap, O12).
Priority: P1. Source: phase1-5 O12, phase7-elicitation AGT-REQ-008 (Risks). Dependencies: CDX-REQ-002, AGT-REQ-005. Evidence: `[FACT]`/`[INFERENCE]` per O12; requirement `[PROPOSAL]`.

---

## 48. MVP Definition

**[NEW SYNTHESIS, revised by Phase 9]** — this section was rigorously stress-tested in Phase 9 (MVP Reduction) against the "Minimum Complete Loop, not Minimum Feature Count" test. **See [`./phase9-mvp-reduction.md`](./phase9-mvp-reduction.md) for the full reasoning, the requirement-by-requirement Cut List, the Keep List justifications, and the concrete Definition of Done for the MVP loop.** The lists below reflect Phase 9's conclusions, not the original Phase 8 draft — 8 requirement IDs (AI-REQ-002/003, DATA-REQ-002/004, GRF-REQ-009/010, API-REQ-003, plus OPS-REQ-004 moved to an immediate-V1 special case) were demoted out of MVP for failing the load-bearing-for-the-loop test, and CLOUD-REQ-001 was narrowed to a build-time architectural-discipline gate rather than a ship-time cloud-deployment test, in order to preserve differentiator (b) (self-hosted + cloud parity) without reinstating non-load-bearing scope.

This is a careful, requirement-by-requirement assignment of **rollout timing** (MVP / V1 / V2 / Future), a distinct axis from the **Priority** (P0–P3) already assigned to each requirement in Phase 7 and this document. Per Phase 7 §1.2's own warning, these two axes must not be conflated: a requirement can be P0-priority but not MVP-timed if it is not needed for the first usable release (e.g., DATA-REQ-001 is P0-priority — data portability matters enormously — but is V1-timed, since an MVP's first customers are unlikely to need an export path on day one). Conversely, nothing here is MVP-timed with a Priority below P1: MVP scope is a strict subset of "important," never a place for a low-priority requirement to sneak in early.

**Method:** every requirement domain produced across Phases 7–8 is walked below. Assignment logic: (1) Phase 5's Commodity/Differentiator/Emerging/Experimental tiering is the primary input — Commodity and Differentiator requirements are MVP/V1 candidates, Emerging is V1/V2, Experimental is V2/Future by default; (2) a requirement's own Priority (P0/P1/P2/P3) further orders within a tier — P0 items in an MVP-eligible tier are MVP, P1 items are more often V1; (3) explicit dependency chains are respected — a requirement cannot be timed earlier than what it structurally depends on.

### MVP (first usable, self-hostable release — the graph substrate, basic Git/collaboration, minimal agent runtime, minimal security)

- **GIT-REQ:** 001, 002, 003, 010 (P0 core protocol + AI-independence). *(004, 005, 006, 007, 008, 009 → V1, all P1.)*
- **GRF-REQ:** 001, 002, 003, 004, 005, 006, 007, 008 (the entire P0 core — Node/Edge/Event/Policy/View plus the type registries and query API; this is the differentiator, it must exist from day one per Phase 5). *(009, 010, 011 → V1, all P1 — Phase 9 confirmed the Action/Evidence conventions are hardening/queryability affordances on top of the graph, not load-bearing for the graph itself, and resolved an ambiguity in this section's original wording that could have been read as pulling them into MVP.)*
- **AGT-REQ:** 001, 002, 003, 004, 005, 006, 007 (all P0 — a minimal but real agent runtime with approval gating, artifact capture, workspace isolation, and full graph participation is core to the agent-native positioning, not deferrable). *(008, 009 → V1.)*
- **AI-REQ:** 001 only (P0 — one working provider is the minimum "AI Context"/"AI Review" needs to exist at all; Phase 9 demoted 002/003 — see below). *(002 cost/token observability, 003 sensitivity-based routing → V1, both real but hardening/accountability layered on top of a working single-provider Gateway, not load-bearing for the reference loop to run once; 004, 005 → V1; 006 → V2.)*
- **CTX-REQ:** 001, 002 (P0 — minimal-context retrieval and logged View invocation are required for CTX-REQ-002's provenance guarantee to exist at all before agents are used in anger). *(003, 004, 005 → V1.)*
- **SEC-REQ:** 001, 002, 003, 004, 005 (all P0 — RBAC/ABAC, agent credential scoping, structural audit, least-privilege defaults, and secrets isolation are foundational; none are safely deferrable in a platform whose differentiator is trustworthy agent-inclusive audit). *(006, 007 → V1/V2, both P2.)*
- **AISEC-REQ:** 001, 002, 004, 005, 006 (P0 — prompt injection containment, MCP allowlisting, secret exclusion from AI payloads, anti-escalation, and Policy-bypass prevention are minimum viable AI-specific security; shipping an agent-native MVP without these is the platform's single biggest reputational risk, see Risk R7/R8 in §53; Phase 9 re-affirmed this entire set in full — no cuts). *(003, 008 → V1, both P1; 007 → V2, P2.)*
- **CI-REQ:** 001 (P0 — CI-as-graph-Nodes is required for the graph differentiator to cover the CI domain at all). *(002, 003, 004, 005, 006 → V1, all P1.)*
- **OPS-REQ:** 001, 002, 003 (all P0 — local-first is a core identity principle, not an add-on; a platform without a working offline single-machine deployment is not yet the product this program is defining). **004 (backup/restore) moved to an explicit "immediate V1" placement by Phase 9** — not load-bearing for the reference loop itself (the loop's Definition of Done does not exercise backup/restore), but placed as the very next item after MVP rather than ordinary V1, given self-hosted operators' disaster-recovery exposure; see `phase9-mvp-reduction.md` §2 for the full reasoning on this specific, deliberately contested call. *(005 → V1, P1.)*
- **CLOUD-REQ:** 001, **narrowed by Phase 9** to its build-time architectural-discipline half only — "same core codebase, no cloud-only hard dependency introduced in MVP-era code," enforced by review/tooling, not by standing up an actual cloud deployment. The ship-time claim ("same test suite passes against a cloud deployment") is not load-bearing for the reference loop and is deferred to V1 alongside CLOUD-REQ-002/003. This narrowing was deliberately un-cut back to an MVP gate (rather than dropped to V1 entirely) specifically to preserve differentiator (b), self-hosted + cloud parity — see `phase9-mvp-reduction.md` §4. *(002, 003 → V1; 004 → V2, P2.)*
- **UX-REQ:** 003 (P1, but treated as MVP given how directly it operationalizes the Human Authority principle — an MVP with agent approval gates that aren't visibly actionable in the UI fails the platform's own core promise). *(001, 002, 004 → V1, all P1 — real but not launch-blocking.)*
- **API-REQ:** 001 only (P0 — one documented, Policy-gated/audited API surface is required from day one so no "back door" pattern ever gets a chance to establish itself; Phase 9 demoted 003 — see below). *(002, 003, 004, 005 → V1 — API-REQ-003's additional discipline beyond the baseline documented surface is hardening, not load-bearing for one demo loop to run once.)*
- **DATA-REQ:** none (Phase 9 demoted both 002 and 004 out of MVP — see below). *(001, 002, 003, 004, 005 → V1 — both 002 "Git objects always exportable" and 004 "no phone-home" remain true by construction the moment GIT-REQ-001/OPS-REQ-001 exist, but Phase 9 concluded that being true-by-accident is not the same claim as being MVP-tested/gated; the reference loop's 10 steps never exercise an export path, so both are moved to the *tested* V1 surface where DATA-REQ-001/003/005 already lived, rather than left as an untested MVP claim.)*

**Revised MVP requirement count:** 45 → 37 across all domains (an 18% reduction, concentrated in AI-REQ [−2 of 3] and DATA-REQ [−2 of 2, now zero MVP items]). Full before/after table by domain: `phase9-mvp-reduction.md` §5.

### V1 (hardening and completeness — most P1s not already pulled into MVP, plus the domains explicitly deferred as conventions)

GIT-REQ-004/005/006/007/008/009; GRF-REQ-009/010/011; AGT-REQ-008/009; AI-REQ-002/003/004/005; CTX-REQ-003/004/005; SEC-REQ-006/007; CI-REQ-002/003/004/005/006; OPS-REQ-004 (immediate-V1, see MVP section above)/005; CLOUD-REQ-001's ship-time cloud-deployment testing (narrowed out of MVP, see above)/002/003; UX-REQ-001/002/004; API-REQ-002/003/004/005; DATA-REQ-001/002/003/004/005; AISEC-REQ-003/008; OBS-REQ-001/002; REL-REQ-001/002; BKP-REQ-001/002; CDX-REQ-001/002; SRCH-REQ-001/002/003/004 (Search is genuinely useful early but, per Phase 5, is not itself the differentiator — the graph is — so it is timed V1, right after the MVP graph substrate exists to search over).

### V2 (Emerging-tier and genuinely deferrable enhancements)

AI-REQ-006; CLOUD-REQ-004; SEC-REQ (none remaining — fully placed above); CDX-REQ-003/004; ART-REQ-001/002; WKFL-REQ-001; AISEC-REQ-007; OBS-REQ-003; BKP-REQ-003; DTWIN-REQ-001/002 (the *substrate* pieces, not the full twin).

### Future (Experimental tier per Phase 5, explicitly not near-term)

ITC-REQ-001/002 (Intent Commit); DIFF-REQ-001/002 (Semantic Diff); DTWIN-REQ-003 (full simulation); WKFL-REQ-002 (AI-suggested workflow generation); ART-REQ-003 (standard package-protocol interop, nice-to-have, no urgency); CLOUD-REQ-004 could plausibly move here instead of V2 if no managed/multi-tenant offering is confirmed by Phase 9 — flagged explicitly as a Product Decision in §54, not silently decided here.

**A note on what this MVP is not:** it is not a "small GitHub clone with a chatbot bolted on." Reading the MVP list above, roughly half of it (GRF-REQ's full P0 set, AGT-REQ's full P0 set, AISEC-REQ's five P0 items) is graph and agent-security infrastructure with no directly visible UI surface on day one. That is a deliberate reflection of Phase 5's finding: the differentiator is structural, not a feature checkbox, and an MVP that shortcuts the graph/agent-security substrate to ship visible features faster would be optimizing for the wrong thing per this program's own evidence.

---

## 49. V1/V2/Future Roadmap

**[NEW SYNTHESIS]** — expands §48 into a narrative roadmap.

**MVP → V1 transition trigger [revised by Phase 9]:** MVP is "done" when the concrete, testable Minimum Complete Loop Definition of Done in [`./phase9-mvp-reduction.md`](./phase9-mvp-reduction.md) §6 passes end-to-end: Repository → Issue → AI Context → Agent Branch → Code Change → CI → AI Review → Human Approval → Merge → Engineering Graph Update, including a chaos-style test confirming Git read/write remains unaffected by an AI Gateway/Agent runtime outage mid-loop. Note this is now explicitly narrower than the original draft trigger: it does **not** require a documented backup/restore cycle (OPS-REQ-004 is immediate-V1, not MVP, per Phase 9's load-bearing-for-the-loop test) — backup/restore remains a near-term priority but is no longer a literal MVP gate. This is deliberately a narrower bar than "feature complete" — it is the smallest slice that proves every one of the platform's eight principles (§13) simultaneously, rather than proving them one at a time in isolation.

**V1 focus:** hardening and completeness of what MVP proved — Git ecosystem compatibility gaps closed (LFS, submodules, signed commits, migration import), Evidence/Action conventions formalized (GRF-REQ-009/010), search made available, cloud migration path made real and tested (not just architecturally possible), and the UX principles beyond the bare Human Authority minimum (Intent-first entry, Context-assembled pages, Progressive Disclosure) built out. V1 is also where the AI Security domain's remaining P1 items (tool-integrity verification, external-MCP sandboxing) land, closing the gap between "MVP is safe enough to ship" and "V1 is safe enough to trust with less-supervised agent autonomy."

**V2 focus:** the Emerging-tier capabilities Phase 5 flagged as real but immature — a native artifact registry, the first Workflow-as-data capability (without AI-suggestion yet), digital-twin substrate (historical replay queries), and — contingent on an explicit Phase 9 product decision, not assumed here — a managed multi-tenant cloud offering.

**Future focus:** the Experimental tier Phase 5 explicitly named as multi-year research investments, not near-term roadmap items: Intent Commit, semantic diff, full digital-twin simulation, and AI-suggested workflow generation. None of these should consume MVP/V1/V2 engineering capacity; this document exists partly to make that boundary explicit so scope creep into "interesting but not proven" territory has a documented decision to point back to.

**Explicit non-linearity note:** this roadmap is not a strict waterfall — several V1 items (e.g., CDX-REQ-001 AGENTS.md compatibility) are low-effort and could plausibly ship alongside MVP if capacity allows; conversely, some MVP items (the full AISEC-REQ P0 set) are non-negotiable even if they threaten the MVP timeline, per the Risk Register's treatment of AI security risk (§53) as high-impact enough to justify slipping a date rather than shipping without it.

---

## 50. User Journeys

**[NEW SYNTHESIS]** — compact stubs only; full journey mapping with real usability validation is Phase 12's job. Each persona (§5) gets one paragraph referencing the primitive/requirement IDs it touches.

- **Developer:** Clones a repo (GIT-REQ-001), opens a PR (PR Node, §21), and sees a context-assembled page (UX-REQ-002) showing linked Requirement, CI status (CI-REQ-001), and Policy state (GRF-REQ-004) without navigating away. Requests review; a Reviewer approves once Evidence Edges (GRF-REQ-010) confirm tests passed (CI-REQ-002). Merges via GIT-REQ-003's parity-guaranteed merge. *Full detail: Phase 12.*
- **AI Agent:** Assigned a task (AGT-REQ-003), receives a minimal, logged context (CTX-REQ-001/002) with prompt-injection containment applied to any untrusted content in scope (AISEC-REQ-001), operates within a scoped, isolated workspace (AGT-REQ-005) under temporary credentials (SEC-REQ-002), attempts a Policy-gated merge, and is blocked pending human approval (AGT-REQ-002) before completing. *Full detail: Phase 12.*
- **Reviewer:** Opens an assigned Review (Review Node, §19), sees Evidence Edges to passing tests and satisfied Policy rather than needing to manually verify CI status, approves or requests changes, all recorded as Events (GRF-REQ-003). *Full detail: Phase 12.*
- **Architect:** Authors an ADR (ADR Node, GRF-REQ-011), links it via `supersedes`/`motivated_by` Edges to prior decisions and Requirement Nodes, later queries "which PRs implement Requirement X" via a View (GRF-REQ-005) to confirm coverage. *Full detail: Phase 12.*
- **Manager:** Runs a rollup View (GRF-REQ-005) over Issue/PR/AgentRun Nodes spanning multiple repos, including AI Gateway cost telemetry (AI-REQ-002, OBS-REQ-001), without reconciling exports from separate tools. *Full detail: Phase 12.*
- **Incident Responder:** Opens an Incident Node, traverses `caused_by` Edges back through Deployment → Release → Commit (§32's digital-twin substrate, DTWIN-REQ-002's time-range query) to identify the responsible change, including distinguishing agent-authored from human-authored commits via AGT-REQ-007's uniform Node querying. *Full detail: Phase 12.*

---

## 51. Acceptance Criteria

**[CROSS-REFERENCE]** Acceptance criteria live inline with each requirement throughout this document and Phase 7's source elicitation — this section confirms that convention rather than restating every criterion in a separate location, per this document's own "cross-reference, don't duplicate" discipline. Three representative examples, chosen to span domains:
- **GRF-REQ-003** (Immutable Event log): "an attempt to mutate or hard-delete an existing Event via the standard API is rejected with an authorization/immutability error... only an explicitly privileged, separately-audited administrative retention/erasure path... may remove Events, and that path itself emits its own Event."
- **AGT-REQ-005** (Scoped, workspace-isolated agent execution): "an agent run scoped to Repository A cannot read or write Repository B's contents or credentials via any API path, verified by a negative-access integration test."
- **AISEC-REQ-002** (MCP tool/server allowlisting, §36): "an agent run configured with an MCP allowlist of {Tool A, Tool B} fails a call to unlisted Tool C with a Policy-violation Event, verified by a negative test."

Every requirement in this document and its Phase 7 source follows this same shape: a concrete, verifiable condition an automated test or auditing agent could check without further interpretation, per Phase 7 §1.3's quality bar ("Testable").

---

## 52. Traceability Matrix

**[NEW SYNTHESIS]** — expands Phase 7 §13's traceability stub. Representative sample across domains (Competitor Evidence → Market Requirement → Product Requirement → Architecture Decision-pending), not exhaustive; full traceability is maintained incrementally as requirements evolve through Phase 9+.

| Competitor Evidence | Market Requirement (Gap) | Product Requirement(s) | Architecture Decision (pending) |
|---|---|---|---|
| GitHub Issues/PR/Discussions are three weakly-linked systems (`[FACT]`, O1) | A queryable, typed cross-object graph | GRF-REQ-001/002/007 | Node/Edge storage engine choice (Phase 6 §7, unresolved) |
| GitHub audit log outage, April 2026 (`[FACT]`, O2) | Structural, non-bolt-on audit trail | GRF-REQ-003, SEC-REQ-003 | Event log storage/replay design (Phase 6 §7) |
| GitLab Gitaly/Praefect/Sidekiq/Workhorse operational weight (`[INFERENCE]`, O4) | Self-hosting without multi-service orchestration burden | OPS-REQ-001/005 | Storage/execution substrate choice avoiding Git-scaling-workaround layering (Phase 10) |
| Sourcegraph's per-query RAG reconstruction, non-persistent (`[INFERENCE]`, O6) | Durable, queryable cross-repo context | CTX-REQ-001/002/003 | Graph-state versioning for exact context reconstruction (Phase 6 §7 / CTX-REQ-002 dependency) |
| No competitor treats agent actions as Policy-gated identically to human actions (`[FACT]`/`[UNVERIFIED-FACT]`, O8/O9) | Agent-inclusive, uniform Policy enforcement | GRF-REQ-004, AGT-REQ-002/007, AISEC-REQ-005/006 | Local (offline) Policy evaluation without central round-trip (Phase 6 §7, unresolved) |
| AGENTS.md cross-tool adoption (`[FACT]`, O11) | Vendor-neutral project-instruction convention | CTX-REQ-005, CDX-REQ-001 | None pending — convention adoption is a product/config decision, not architecture |
| Linear has no self-hosted tier at any price point (`[FACT]`, Phase 3) | Genuine self-hosting, not SaaS-only with a self-hosted-labeled tier | OPS-REQ-001, DATA-REQ-004 | None pending |
| MCP's 40-tool cap at Cursor and lack of a cross-vendor context/provenance data standard (`[FACT]`/`[INFERENCE]`, O12) | Curated, composable MCP tool surface with platform-native provenance | AGT-REQ-008, API-REQ-005, CDX-REQ-002 | MCP tool-scoping mechanism design (Phase 10) |
| Codex's documented approval-mode policy (suggest/auto-edit/full-auto) (`[FACT]`, Phase 2) | Cross-vendor approval-mode interoperability | AGT-REQ-002, CDX-REQ-004 | None pending |
| No competitor demonstrated semantic diff as shipped capability (`Unknown` across Phase 4 matrix) | N/A — confirms Experimental tier, not a gap to close near-term | DIFF-REQ-001/002 | Deferred entirely to Future; no architecture work warranted yet |

---

## 53. Risk Register

**[NEW SYNTHESIS]** — 10 risks per original spec §39, each with Risk ID, Description, Probability, Impact, Detection, Mitigation, Fallback, Owner Type.

**R1 — Git data corruption**
Description: Repository object-store corruption (disk failure, bug in server-side merge/rebase logic) causes loss or divergence of Git history.
Probability: Low. Impact: Critical. Detection: Periodic integrity verification (`git fsck`-equivalent) run against the object store; BKP-REQ-001 consistency checks. Mitigation: Content-addressed, checksummed storage; OPS-REQ-004 tested backup/restore. Fallback: Restore from last verified backup; GIT-REQ-003's merge-parity requirement limits divergence risk from server-side operations specifically. Owner Type: Platform Engineering.

**R2 — AI hallucination in agent-authored changes**
Description: An agent produces plausible-looking but incorrect code, tests, or documentation, and it is merged without adequate human scrutiny.
Probability: High. Impact: Medium (bounded by review gates, but real). Detection: CI-REQ-002 Evidence Edges show test coverage state; AGT-REQ-002 approval gate is the primary control point. Mitigation: Human Authority principle (§13) — no agent-authored merge bypasses review by default; UX-REQ-003 makes agent-origin visible so reviewers apply appropriate scrutiny. Fallback: Revert via standard Git history (GIT-REQ-002); Incident Node traceback (DTWIN-REQ-002) to identify affected downstream changes. Owner Type: Product / Agent Runtime.

**R3 — Agent destructive operations**
Description: An agent performs an irreversible or highly disruptive action (force-push, mass delete, credential misuse) beyond its intended scope.
Probability: Low-Medium. Impact: Critical. Detection: AISEC-REQ-005/006 transitive-scope and Policy-bypass tests; AGT-REQ-006 resource-limit breach Events. Mitigation: AGT-REQ-005 workspace isolation, SEC-REQ-002 temporary scoped credentials, AGT-REQ-002 approval gating on destructive-action classes by default. Fallback: GIT-REQ-002/003 standard Git recovery mechanisms; OPS-REQ-004 backup/restore for non-Git-recoverable damage. Owner Type: Security / Agent Runtime.

**R4 — Graph staleness**
Description: The graph's Node/Edge state diverges from actual repository/CI reality due to a missed Event, ingestion bug, or race condition, leading to Policy decisions or context assembly made against stale facts.
Probability: Medium. Impact: High. Detection: Reconciliation checks comparing graph state to Git/CI ground truth; CTX-REQ-004's truncation-transparency logging surfaces some staleness indirectly. Mitigation: GRF-REQ-003's Event-sourced design (state derived from an authoritative append-only log, not independently mutable caches). Fallback: Full graph rebuild/replay from the Event log (a direct benefit of Event immutability, per DTWIN-REQ-001). Owner Type: Platform Engineering.

**R5 — Search staleness**
Description: Search index lags behind pushed changes, causing agents/humans to act on outdated code.
Probability: Medium. Impact: Medium. Detection: SRCH-REQ-003's staleness-window requirement, once benchmarked (§44), makes this measurable rather than merely suspected. Mitigation: documented, bounded index-freshness window; surfacing staleness rather than hiding it. Fallback: Direct Git-layer fallback search (grep-equivalent) when index freshness cannot be guaranteed. Owner Type: Platform Engineering.

**R6 — CI isolation failure**
Description: A CI run (potentially agent-triggered per CI-REQ-005) escapes its intended sandbox, accessing resources or credentials outside its scope.
Probability: Low-Medium. Impact: High. Detection: Anomalous resource-access patterns (AISEC-REQ-007-equivalent monitoring applied to CI runs). Mitigation: CI-REQ-001's Node-scoped run model plus the same workspace-isolation discipline as AGT-REQ-005, applied to CI execution specifically (architecture detail for Phase 10). Fallback: Immediate run termination, credential rotation, Incident Node creation for post-mortem via DTWIN-REQ-002 traceback. Owner Type: Security / Platform Engineering.

**R7 — Secret leakage via AI Gateway**
Description: A secret value is inadvertently included in a prompt/context sent to an external model provider.
Probability: Medium (without controls); the whole point of AISEC-REQ-004 is to drive this toward Low. Impact: Critical. Detection: AISEC-REQ-004's payload-scrubbing verification tests; AI-REQ-002's per-request logging (audited for known secret patterns). Mitigation: AISEC-REQ-004 (secret-value exclusion from Gateway payloads), SEC-REQ-005 (secrets isolated from graph queries generally). Fallback: Immediate credential rotation upon detected leakage; SEC-REQ-003's structural audit trail makes "what was sent, when, to which provider" reconstructable for incident response. Owner Type: Security.

**R8 — MCP attacks (tool poisoning, rug-pull, malicious external servers)**
Description: A malicious or compromised MCP server manipulates an agent into taking unintended actions or exfiltrating data.
Probability: Medium (given MCP's stated immaturity, O12). Impact: High. Detection: AISEC-REQ-003's tool-description integrity verification. Mitigation: AISEC-REQ-002 (allowlisting), AISEC-REQ-008 (external MCP sandboxing within the same isolation boundary as native execution). Fallback: Immediate allowlist revocation, AgentRun termination, Policy-violation Event review. Owner Type: Security / Agent Runtime.

**R9 — Scaling failure under real load**
Description: Graph query or Git-serving performance degrades unacceptably as repository/graph size grows, particularly for cloud multi-tenant deployments.
Probability: Medium. Impact: High. Detection: Performance benchmarking program (§44) once Phase 10 architecture is fixed; CLOUD-REQ-003 load-test acceptance criteria. Mitigation: CLOUD-REQ-003's horizontal scale-out requirement, GRF-REQ-008's pagination/limits discipline from day one. Fallback: Query-complexity limits and circuit-breaking on the graph query API (API-REQ-004-equivalent applied defensively) rather than uncontrolled degradation. Owner Type: Platform Engineering.

**R10 — Migration failure (import/export data loss)**
Description: A GIT-REQ-009 import or DATA-REQ-001 export loses or corrupts data, undermining both the adoption story (import) and the anti-lock-in promise (export).
Probability: Low-Medium. Impact: High (reputational, especially for export — this is the platform's own stated differentiator against Linear's no-export lock-in). Detection: GIT-REQ-009 and DATA-REQ-001's own acceptance criteria (SHA/object-count matching; structural-diff re-import verification). Mitigation: automated verification built into the import/export tooling itself, not left to manual spot-checking. Fallback: retain the source system read-only until verified migration completeness is independently confirmed. Owner Type: Product / Platform Engineering.

**R11 — License / open-source governance risk**
Description: `[TBD]` — the platform's own licensing model (fully open source like Gitea, GPL-non-profit-governed like Forgejo, open-core like GitLab CE/EE, or fully proprietary) is not yet decided anywhere in Phases 1–8, and Phase 3's evidence shows this materially affects adoption trust (Forgejo's non-profit governance explicitly removes "rug-pull risk" per that deep dive).
Probability: N/A (not yet a risk realized, but an unresolved decision carrying real downstream risk either way). Impact: High (affects adoption, community trust, and the "no vendor lock-in" positioning claim's credibility). Detection: N/A — this is a Product/Legal decision gap, not something detectable via testing. Mitigation: Explicit licensing decision needed no later than Phase 9. Fallback: N/A. Owner Type: Product / Legal.

**R12 — Vendor lock-in perception despite DATA-REQ**
Description: Even with DATA-REQ-001/002 built, customers may distrust the export path's completeness/reliability without independent verification, undermining the "no lock-in" positioning claim.
Probability: Medium. Impact: Medium. Detection: Customer/market feedback (Phase 12+). Mitigation: DATA-REQ-001's acceptance criteria require structural-diff-verified round-trip export/import, not merely a claimed capability; publishing that verification process builds trust beyond the feature existing. Fallback: Third-party audit of the export/import path if trust remains a barrier post-launch. Owner Type: Product.

---

## 54. Open Questions

**[NEW SYNTHESIS]** — consolidates every `[TBD]` surfaced across Phases 1–8 into one master list, categorized per original spec §40.

### Research Needed
- Gerrit's review-centric model, Argo CD/Workflows' deployment/rollback state machine, and CodeQL/Sentry's policy/incident precedents remain unresearched (Phase 3 O13, carried through Phase 6 and Phase 7); directly affects CI-REQ-006's rollback acceptance criteria and should be resolved before Phase 8's schemas are treated as final at the field level. *(phase1-5-research.md, phase6-primitives.md §7, phase7-elicitation.md §14)*
- Git LFS support parity across all surveyed competitors was asserted `[UNVERIFIED-FACT]` (GIT-REQ-004) rather than independently re-verified; a lightweight confirmation pass would strengthen this requirement's evidence tag. *(phase7-elicitation.md §14)*
- No dedicated research was done on graph database/event-sourcing implementation options (Neo4j vs. alternatives vs. custom store; specific event-sourcing frameworks) — explicitly deferred to Phase 10, not a product-research gap. *(phase6-primitives.md §7)*
- Gerrit, Jira, GitHub Issues/Projects standalone, Argo CD/Workflows, CodeQL, Sentry were never reached with dedicated live research in Phase 1–5 given time budget; flagged explicitly so later phases know these are outstanding, not silently dropped. *(phase1-5-research.md §Phase 3)*
- What compliance/regulatory retention-erasure requirements (e.g., GDPR right-to-erasure) actually demand of an append-only Event log needs dedicated legal/compliance research before GRF-REQ-003's "narrow, audited exception path" can be finalized. *(phase7-elicitation.md §14)*

### Benchmark Needed
- Every category in §44 Performance Requirements: Git operation latency at monorepo scale, graph query latency, context-assembly latency, Policy-evaluation latency, search freshness window, concurrent-user ceiling for single-machine deployments, and horizontal scale-out throughput (this last one additionally blocked on the Phase 10 storage-architecture decision below).
- BKP-REQ-002's RTO/RPO figures.
- REL-REQ-001's specific availability target.

### Product Decision
- Whether a managed/multi-tenant cloud offering (CLOUD-REQ-004) is actually in scope for an early release or purely a later-phase possibility — determines whether it should be P1 instead of P2, and whether it belongs in V2 or Future in §49's roadmap. *(phase7-elicitation.md §14, this document §49)*
- Default resource-limit values for agent runs (AGT-REQ-006: token budgets, wall-clock limits, concurrency caps) — deliberately left unspecified; needs either usage data or an explicit risk-tolerance call from product leadership. *(phase7-elicitation.md §14)*
- Which specific Agent Actions require human approval by default (AGT-REQ-002's "specified class of agent Action") — left configurable rather than fixed. *(phase7-elicitation.md §14)*
- Whether **Action** and **Evidence** should be promoted from conventions (GRF-REQ-009/010) to independent first-class primitives — depends on real usage patterns not yet available; recommend revisiting at or after Phase 9 based on telemetry, not speculation. *(phase6-primitives.md §7)*
- Whether Context should ever be materialized as a persisted object vs. purely a logged View invocation — a storage/retention-cost tradeoff, not resolvable by research alone. *(phase6-primitives.md §7)*
- Whether Policy evaluation is synchronous/blocking by default vs. advisory-first, tightening later — no competitor evidence settles this either way. *(phase6-primitives.md §7)*
- Draft/WIP PR state and stacked/dependent PR chains (§21) — realizable under the existing primitive model but not yet elicited as dedicated requirements; confirm MVP-relevance vs. safe deferral. *(this document §21)*
- Notification/mention delivery mechanics (§19) — not elicited as atomic Phase 7 requirements; confirm intentional deferral. *(this document §19)*
- The platform's own licensing/governance model (R11, §53) — unresolved anywhere in Phases 1–8, materially affects adoption trust per Phase 3's Forgejo evidence. *(this document §53)*

### Architecture Decision
- Whether Node/Edge/Event live in one storage engine (favoring GitLab/Linear-style coherence) or are split across a graph store plus an append-only event log with the graph as a derived/materialized view — a core Phase 10 decision with major implications for consistency, replay cost, and self-hosting operational weight; getting this wrong risks recreating GitLab's Gitaly/Praefect/Sidekiq/Workhorse layering at the primitive-storage level. *(phase6-primitives.md §7)*
- How Policy evaluation composes with distributed/offline (self-hosted, air-gapped) operation — can Policy be evaluated locally without a round-trip to a central service? Raised directly by the local-first/cloud-parity differentiator but unresolved. *(phase6-primitives.md §7, phase7-elicitation.md §14, this document §41)*
- CTX-REQ-002's exact-reconstruction guarantee for View invocations depends on how graph state versioning/snapshotting is architected. *(phase7-elicitation.md §14, this document §26)*
- CLOUD-REQ-003's horizontal scale-out requirement assumes some resolution to the single-store-vs-split-store question exists that supports it; if Phase 10 instead concludes a single non-shardable store is the right MVP tradeoff, CLOUD-REQ-003's priority/timing should be revisited, not silently kept. *(phase7-elicitation.md §14, this document §40)*
- BKP-REQ-001's point-in-time consistency mechanism across the Git store and graph store depends entirely on the storage-architecture decision above. *(this document §43)*

### Legal Review
- GDPR/right-to-erasure implications for the immutable Event log (cross-listed under Research Needed above, but the resolution itself is a legal, not technical, determination).
- Licensing/governance model choice (R11, §53) has a legal dimension (open-core structuring, trademark, contributor agreements) beyond the pure product-positioning question listed under Product Decision above.

### Security Review
- The full AISEC-REQ set (§36) should receive a dedicated security/red-team pass before MVP ships — this document specifies the requirements but Phase 11 (Red-Team Review) is where they get adversarially tested, not here.
- CDX-REQ-003's provenance-completeness gap for external-agent-delegated tasks (an external agent's internal tool calls are not directly observable by the platform's Event log) is a known, unresolved security-relevant limitation flagged in that requirement's own Risks.

### UX Validation
- UX-REQ-001's "intent resolution" acceptance bar is easy to satisfy shallowly (a search box) without satisfying it meaningfully; Phase 12 needs to define this more rigorously with real usability testing.
- UX-REQ-004's "right" default disclosure threshold is a tuning question for Phase 12, not settled by this document.
- All six persona journeys in §50 are stubs pending Phase 12's full validation.

---

## 55. Research Sources

**[CROSS-REFERENCE]** The full, deduplicated bibliography of every URL cited across Phases 1–5 (40+ sources, with access dates and primary-vs-third-party status noted per entry) lives in [`./phase1-5-research.md` §Sources](./phase1-5-research.md#sources) — not duplicated here to avoid two copies drifting out of sync. Phases 6, 7, and this document (Phase 8) did not conduct new live web research; they cite Phase 1–5 findings by observation ID (`Oxx`) or section reference and introduce no new external sources. Any future update to this document that does introduce new research citations should either extend the Phase 1–5 bibliography directly (if the claim genuinely belongs to that evidence-collection scope) or add a clearly-separated "Phase 8+ Sources" subsection here — this document intentionally does not pre-create an empty placeholder for that to avoid implying research occurred that did not.

---

## Appendix: ADR Required List

Architecture decisions this program has already flagged as needing a formal ADR, pulled from every `[TBD]`-tagged "Architecture Decision" item across Phases 6–8 (see §54 for full context on each):

1. **Graph storage architecture** — single storage engine (Node/Edge/Event co-located) vs. split graph-store-plus-event-log-with-derived-view. *(phase6-primitives.md §7)*
2. **Policy evaluation distribution model** — centralized vs. locally-evaluable-without-round-trip, for air-gapped/self-hosted operation. *(phase6-primitives.md §7, phase7-elicitation.md §14)*
3. **Graph state versioning/snapshotting strategy** — required to make CTX-REQ-002's exact-context-reconstruction guarantee precise. *(phase7-elicitation.md §14)*
4. **Horizontal scale-out mechanism for cloud deployment** — contingent on Decision 1 above; CLOUD-REQ-003's priority/timing depends on this. *(phase7-elicitation.md §14)*
5. **Backup/restore point-in-time consistency mechanism** across the Git object store and graph store — contingent on Decision 1. *(this document, BKP-REQ-001)*
6. **Workspace isolation mechanism for agent/CI execution** (container/VM/sandbox choice) — referenced by AGT-REQ-005 and CI-REQ's isolation requirements as "constrains behavior, not implementation," explicitly deferred. *(phase7-elicitation.md AGT-REQ-005)*
7. **Secrets store technology and Policy-mediated access design** — SEC-REQ-005 fixes the behavior (not retrievable via graph query API); the store technology itself is undecided. *(phase7-elicitation.md SEC-REQ-005)*
8. **MCP tool-scoping/curation mechanism** — how the platform avoids the flat, uncurated tool-count problem (API-REQ-005) at an implementation level. *(this document §47)*
9. **GDPR/retention-erasure exception path for the immutable Event log** — technical design pending the Legal Review item in §54. *(phase7-elicitation.md GRF-REQ-003)*
10. **CI/CD execution isolation architecture** — how CI-REQ's Node-scoped run model achieves the same isolation guarantee as AGT-REQ-005 for agent-triggered runs specifically (R6, §53). *(this document §53)*

---

*End of Phase 8 baseline. Phases 9 (MVP Definition refinement against real constraints), 10 (Architecture Design — resolves the ADR list above), 11 (Red-Team Review), 12 (Stakeholder Validation — full UX journeys, persona validation), and 13 (Final Baseline) remain pending.*
