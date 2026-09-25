# AI-Native Engineering Platform — Requirements Research: Phase 6

**Status:** v1.0. **Scope:** Phase 6 only (Product Primitive Discovery). Builds directly on `docs/requirements/phase1-5-research.md` — read that first. This document does not re-litigate Phases 1–5's competitor facts; it cites them and does not upgrade their confidence tags.

**Tagging convention** (inherited from phase1-5-research.md, reused unchanged):
- `[FACT]` — verifiable, primary-sourced.
- `[UNVERIFIED-FACT]` — plausible, cited, but not primary-sourced or independently confirmed.
- `[INFERENCE]` — reasonable conclusion from facts, not independently verified.
- `[PROPOSAL]` — our own design idea for the Target Platform, not a claim about any competitor.
- `[TBD]` — unresolved; needs follow-up.

Where this document cites a phase1-5-research.md claim, it preserves that claim's original tag. New synthesis in this document is tagged `[PROPOSAL]` unless it is a plain restatement of a phase1-5 finding.

---

## 1. Method

This phase follows an emergent chain, deliberately in this order, so that primitives are *discovered* from evidence rather than *invented* and then rationalized:

**Observation → Pattern → Constraint → Primitive → Interaction → Workflow → Platform Capability**

- **Observation**: concrete, cited facts about how existing platforms model their objects (Phase 1–5 evidence).
- **Pattern**: recurring shapes across observations — e.g., "every competitor that has any graph coherence at all achieves it by narrowing scope, not by having more object types."
- **Constraint**: what any primitive set must satisfy to fix the patterns' failure modes — irreducibility, composability, auditability, extensibility without schema migration.
- **Primitive**: the smallest set of node/relationship/fact types that satisfies the constraints. A primitive is judged, not assumed — Section 5 subjects each candidate to a survival test.
- **Interaction**: how primitives combine pairwise (a Node with an Edge, an Event about a Node, Evidence attached to an Edge).
- **Workflow**: how interactions chain into a recognizable engineering activity (a review, a release).
- **Platform Capability**: the user/agent-facing feature (e.g., "queryable engineering graph," Phase 5's top differentiator) that the workflow layer enables.

This chain matters because Phase 5 identified the knowledge graph across the full engineering surface as the platform's clearest differentiator, and Phase 3 identified *why* competitors fail to have one: not lack of ambition, but architectures that grew business objects independently (GitHub's Issues/PR/Discussions `[FACT]`, Phase 3 §GitHub) or bolted a graph across separately-owned products (Bitbucket/Jira/Confluence `[INFERENCE]`, Phase 3 §Bitbucket). A primitive-first design is the direct architectural response to that observed failure mode — but Section 5 exists precisely so this response doesn't become an unfalsifiable justification for adding primitives nobody needs.

---

## 2. Observations (grounded in Phase 1–5)

| # | Observation | Source | Tag |
|---|---|---|---|
| O1 | GitHub's Issues, Pull Requests, and Discussions are three separate, weakly-linked object types bolted onto a Git host over 15+ years, with no unified graph schema. | phase1-5 §Phase 3, GitHub "Structural Limitations" | `[FACT]` |
| O2 | GitHub's audit log is a structurally separate subsystem from the rest of the product, and suffered a documented 28-minute outage from a shared-dependency failure (April 2026). | phase1-5 §Phase 2, GitHub / Phase 5 "Unified audit trail" | `[FACT]` |
| O3 | GitLab achieves stronger cross-object graph coherence than GitHub because it is architected as a single Rails monolith + Postgres, not because it has a formal graph API — relations between issue/MR/epic/pipeline are native to one data model. | phase1-5 §Phase 3, GitLab "Graph" | `[INFERENCE]` |
| O4 | GitLab's self-hosting operational complexity (Gitaly, Praefect, Sidekiq, Workhorse, Puma) is a product of solving Git's own scaling weaknesses with bolted-on services, not a product of the domain itself wanting five moving parts. | phase1-5 §Phase 3, GitLab "Structural Limitations" | `[INFERENCE]` |
| O5 | Bitbucket's "engineering graph" is really three separate products (Bitbucket/Jira/Confluence) glued by cross-linking; the graph lives in Jira/Confluence, not in Bitbucket itself. | phase1-5 §Phase 3, Bitbucket "Structural Limitations" | `[INFERENCE]` |
| O6 | Sourcegraph/Cody reconstructs cross-repo context per-query via RAG (vector embeddings + code search), not as a persistent structured graph of durable relationships. | phase1-5 §Phase 3, Sourcegraph "Structural Limitations" | `[INFERENCE]` |
| O7 | Linear is reportedly the most graph-coherent PM tool surveyed — a single data model spanning teams/issues/cycles/projects behind one GraphQL API — but that coherence is scoped to project-management objects only; it has zero code/CI/release/incident/agent objects and no self-hosted deployment option at any tier. | phase1-5 §Phase 3, Linear deep dive | `[UNVERIFIED-FACT]` (coherence claim) / `[FACT]` (no self-hosting) |
| O8 | Every competitor surveyed treats "agent" as either absent (Gitea/Forgejo, `[TBD]`/`[FACT]` no first-class AI agent platform identified), a bolt-on feature of an existing product (Copilot Coding Agent, Duo Agent Platform, Rovo Dev), or a first-class but ungraphed workspace member (Linear, agents assignable/@mentionable but not modeled as a graph node type with edges to its own actions). | phase1-5 §Phase 3, Phase 4 matrix "Agent execution" row | `[FACT]`/`[UNVERIFIED-FACT]` (mixed per-row) |
| O9 | No competitor surveyed models requirement, ADR, incident, agentrun, prompt, skill, model, context, or policy as first-class graph nodes at all (Phase 4 matrix, "Knowledge graph" row: ❌/⚠️ across the board). | phase1-5 §Phase 4 matrix | `[INFERENCE]` (matrix synthesis) |
| O10 | Claude Code's own hooks/skills/subagents/MCP separation (enforcement / knowledge / delegation / external tools) is a relatively principled context-engineering model compared to competitors' flatter designs. | phase1-5 §Phase 3, Claude Code "Strengths" | `[INFERENCE]`, built on `[UNVERIFIED-FACT]` sub-claims |
| O11 | AGENTS.md is a cross-tool, vendor-neutral convention for project instructions, working across 8+ agent tools — evidence that portable, structured "intent/context" artifacts are valued independent of any one vendor's schema. | phase1-5 §Phase 2/3, Codex | `[FACT]` |
| O12 | MCP adoption is now widespread across every AI agent surveyed, but each implementation has its own limits (Cursor's 40-tool cap) and there is no cross-vendor standard for how an agent's context/session/provenance should be modeled as *data*, only for tool invocation. | phase1-5 §Phase 5, "Emerging" | `[FACT]`/`[INFERENCE]` |
| O13 | Gerrit's review-centric model, Argo CD/Workflows' deployment model, and CodeQL/Sentry's policy/incident precedents were flagged as outstanding `[TBD]` in Phase 3 and not independently researched. This document treats "Review," "Deployment," and "Policy violation" as primitive-composed objects on inference from the general PR/CI/security patterns already evidenced, not from dedicated Gerrit/Argo/CodeQL evidence. | phase1-5 §Phase 3, "Gerrit, Jira, GitHub Issues/Projects (standalone), Argo CD/Workflows, CodeQL, Sentry" | `[TBD]` carried forward |

**Pattern extracted from O1–O13** `[PROPOSAL]`: graph coherence in the wild is not achieved by having a formal "graph primitive" layer — it is achieved (GitLab, Linear) as a side effect of a single shared data model, and lost (GitHub, Bitbucket) when object types are added independently over time or split across products. This is the core justification for primitives: not "graphs are good," but "every observed coherence failure traces to object types being modeled independently rather than composed from a shared substrate."

**Constraint extracted** `[PROPOSAL]`: a primitive set must (a) let new domain object types be *added by composition*, without a schema migration or a new bolted-on subsystem (fixes O1/O5); (b) make provenance/audit a structural property of the substrate itself, not a separate subsystem (fixes O2); (c) treat agents and their actions as data in the same substrate as human actions, not a special case (fixes O8/O9); (d) be small enough to actually hold a "single shared data model" property in practice — Linear and GitLab's coherence came from *narrow* scope and *few* moving parts, not from an elaborate primitive taxonomy (this constrains Section 5's cut line directly).

---

## 3. Candidate Primitives

All ten candidates named in the originating spec are evaluated. Each entry gives: definition, carried data, irreducibility argument, and composition examples. Section 5 then applies the survival test — read this section as *candidates*, not as a decided architecture.

### 3.1 Node `[PROPOSAL]`
**Definition**: A typed, identity-bearing thing in the engineering graph — the vertex onto which everything else attaches.
**Carries**: stable ID, type tag (e.g. `work-item`, `commit`, `agent`), a small set of type-defined scalar properties, created/updated timestamps.
**Irreducibility**: Node is the substrate every other primitive references (Edges connect Nodes, Events describe changes to a Node, Evidence attaches to a Node). It cannot be built from the others without circularity.
**Composes**: Issue, Repository, Commit, Human, Agent are each a Node of a different type.

### 3.2 Edge `[PROPOSAL]`
**Definition**: A typed, directed relationship between two Nodes (or between a Node and an Event/Evidence, if those are modeled as addressable entities — see 5.2).
**Carries**: type tag (`implements`, `depends_on`, `reviewed_by`, `caused`, …), source/target Node IDs, optional properties (e.g. confidence, weight), created timestamp, creator (Human or Agent Node).
**Irreducibility**: relationships between typed things cannot be represented by Nodes alone without either duplicating data into both endpoints or losing directionality/type — a dedicated relationship primitive is the minimal way to express "graph" at all.
**Composes**: Issue —`blocks`→ Issue; PR —`implements`→ Requirement; AgentRun —`generated_by`→ Agent; ADR —`supersedes`→ ADR.

### 3.3 Event `[PROPOSAL]`
**Definition**: An immutable, timestamped record that something happened to a Node or Edge — the append-only unit of history.
**Carries**: type tag (`created`, `state_changed`, `commented`, `merged`), subject Node/Edge ID, actor (Human or Agent Node), payload (type-specific), timestamp, causal predecessor Event ID (optional, for chains).
**Irreducibility**: Nodes and Edges represent current (or point-in-time) state; without a distinct immutable-history primitive, "how did we get here" collapses into mutable Node properties, which is exactly the audit-log-as-bolted-on-subsystem failure observed in O2. Event is the direct architectural answer to O2 — provenance becomes structural, not a separate service.
**Composes**: an Issue's status history is an Event stream against its Node; a Review's approve/request-changes sequence is Events against the Review composite; an AgentRun's tool-call trace is an Event stream.

### 3.4 Intent `[PROPOSAL]`
**Definition**: A structured statement of *why* — a goal, requirement, or desired outcome that other Nodes exist to satisfy.
**Carries**: description (structured/NL), origin (Human or Agent), priority/status, link target (what it's an Intent *for*).
**Irreducibility test (deferred to §5)**: candidate for demotion — an Intent looks a great deal like "a Node of type `requirement` with an Edge of type `motivates` to whatever it justifies." Kept as a candidate here because the originating spec calls it out explicitly and because O11 (AGENTS.md's popularity) suggests intent-capture is a valued *product* concept even if it isn't a structurally distinct *data* primitive.
**Composes**: Requirement, ADR's "context/decision" section, an Agent Task's original prompt.

### 3.5 Context `[PROPOSAL]`
**Definition**: The bounded slice of the graph (Nodes, Edges, Events, Evidence) assembled and handed to a Human or Agent to act on at a point in time.
**Carries**: a set of Node/Edge/Evidence references, an assembly method (explicit selection vs. retrieval), a timestamp, the consumer (which Agent/Human session it was assembled for).
**Irreducibility test (deferred to §5)**: Context is arguably not a stored primitive at all but a *query result* over Node/Edge/Evidence — i.e., a View (3.10) parameterized for agent consumption. Kept as a distinct candidate because O6 (Sourcegraph's non-persistent RAG reconstruction) and O12 (MCP's context ambiguity) both point at "how was this agent's context assembled and can we audit it later" as a real, currently-unsolved product need — but whether it needs its own storage primitive vs. being a logged View invocation is exactly the kind of question §5 must resolve.
**Composes**: what Copilot/Duo assemble ad hoc today (Repo + Issues + PR threads, per O-table in Phase 3 GitHub "Context" row) would, if persisted, be a Context instance.

### 3.6 Policy `[PROPOSAL]`
**Definition**: A rule, evaluated against Nodes/Edges/Events, that constrains or gates an Action.
**Carries**: condition (structured predicate or code reference), scope (which Node types/Edges it applies to), enforcement mode (advisory/blocking), owner.
**Irreducibility**: distinct from Evidence and Event because a Policy is *prospective* (evaluated before/during an Action) rather than a record of what already happened; distinct from Action because the same Policy is evaluated against many Actions. Directly serves Phase 5's "unified audit trail across human AND agent actions" differentiator and answers O8/O9 (agent actions need the same gate human actions get).
**Composes**: CODEOWNERS-equivalent (Policy: "PR touching path X needs review Edge from Node of type `owner`"), branch protection, an agent's tool-permission scope (Claude Code's per-subagent tool permissions, O10, generalized as Policy Nodes).

### 3.7 Action `[PROPOSAL]`
**Definition**: A requested or executed unit of work by a Human or Agent that, on completion, produces Events (and typically new/changed Nodes and Edges).
**Carries**: type (`comment`, `merge`, `run-tests`, `deploy`), actor, target Node(s), status (requested/in-progress/completed/failed), the Policy evaluations it passed/failed, timing.
**Irreducibility test (deferred to §5)**: Action looks like it might collapse into "an Event with a pre-state (requested) and post-state (completed)," i.e., Action = Event pair, not a fifth wheel. Kept as a candidate because Actions have a *duration* and an *in-flight* state (running CI, an agent mid-task) that a single immutable Event does not naturally represent — but §5 asks whether that's enough to justify a separate primitive versus an Event-type convention (`action_started`/`action_completed` sharing a correlation ID).
**Composes**: a CI run, an AgentRun, a merge operation.

### 3.8 Agent `[PROPOSAL]`
**Definition**: A Node subtype representing a non-human actor capable of performing Actions, distinguished from Human primarily by provenance/accountability requirements, not by graph mechanics.
**Carries**: model/version, owning Human or Policy, capability/tool scope (itself expressible as Policy Edges), session/run history (Edges to its AgentRuns).
**Irreducibility test (deferred to §5)**: this is the strongest candidate for "not actually a primitive, just a Node type" — nothing about Agent requires new graph mechanics beyond Node + Edge("performed_by")+ Policy(scope). Kept as an explicit named type (rather than silently folded into generic Node) because Phase 5 names "agent-native" as core to the platform's identity and O8 shows every competitor treats agents as second-class relative to humans in some way; making Agent a first-class *documented* Node type (not a first-class *primitive*) is itself a product decision worth stating plainly rather than leaving implicit.
**Composes**: Copilot Coding Agent / Duo Agent Platform / Claude Code subagent, each as an Agent Node with Policy-scoped tool access and an Edge history of AgentRuns.

### 3.9 Evidence `[PROPOSAL]`
**Definition**: A typed link from a claim/decision Node to the material that justifies it — the structural realization of "why," as opposed to Intent's structural realization of "what for."
**Carries**: type (`test-result`, `citation`, `benchmark`, `human-approval`), source (Node, external URL, or Event), confidence/tag (reusing this very document's `[FACT]`/`[INFERENCE]`/`[TBD]` scheme is a candidate default), target claim.
**Irreducibility**: distinct from a generic Edge because Evidence carries an epistemic status (how sure are we, and why) that a typed relationship alone doesn't express — and distinct from Event because Evidence is about *justification*, evaluated and re-evaluated, not a one-time occurrence record.
**Composes**: an ADR's "why we chose X" backed by Evidence Edges to benchmarks/incidents; a Review's approval backed by Evidence of passing tests; this very document's citation apparatus is an Evidence-primitive instance in miniature.

### 3.10 View `[PROPOSAL]`
**Definition**: A named, reusable, parameterized projection over Nodes/Edges/Events/Evidence — how the graph is rendered for a purpose (a kanban board, a dependency graph, an agent's context window, a release changelog).
**Carries**: query/projection definition, parameters, target audience/format.
**Irreducibility**: distinct from Context (3.5) in that a View is the *reusable definition*, while a Context (if it survives as separate at all) would be one *instantiation* of a View for one agent session. This is exactly the kind of overlap §5 must resolve — one of View/Context is likely redundant.
**Composes**: GitHub Projects' kanban board (a View over Issue Nodes filtered/grouped by status Edge/property), a release changelog (a View over Commit/Issue Nodes connected by `closes`/`part_of` Edges since the last Release Node).

---

## 4. Composition Worked Examples

Four domain objects, assembled explicitly from the Section 3 primitive set (using only the primitives that survive Section 5's cut is deferred — this section shows the full candidate set in use so the cut in §5 can be evaluated against real composition, not the abstract definitions alone).

### 4.1 Issue
- **Node**: type `work-item`, properties `{title, description, status}`.
- **Edges**: `motivated_by` → Intent/Requirement Node; `blocks`/`blocked_by` → other Issue Nodes; `implemented_by` → PR Node (once one exists); `assigned_to` → Human or Agent Node.
- **Events**: `created`, `status_changed` (×N), `commented` (×N), `assigned`, `closed` — the full history, replacing GitHub's implicit, UI-only status history (O1).
- **Evidence**: `closed` Event may carry Evidence Edges to the PR/test-run that justified closing it.
- **Policy**: gates who may change `status` to `closed` (a generalized CODEOWNERS/workflow-state rule).

### 4.2 Review
- **Node**: type `review`, properties `{verdict}`, target Edge → the PR/Commit Node under review.
- **Edges**: `reviewed_by` → Human/Agent Node; `evaluates` → PR Node; `references` → specific Commit/line locations (as sub-Nodes or structured Edge properties).
- **Events**: `requested`, `comment_added` (×N), `changes_requested`, `approved`/`rejected` — this is the direct generalization of Gerrit's review-centric model (O13, `[TBD]` precedent, used here on inference from the general PR-review pattern already evidenced across every forge in Phase 3, not from dedicated Gerrit research).
- **Evidence**: approval Events typically carry Evidence Edges to CI Test Nodes (`passed`) and possibly to a Policy evaluation record.
- **Action**: the review itself is an Action instance (`review`, actor = Human or Agent, target = PR Node) whose Events are the comment/verdict trail above.

### 4.3 Agent Task (AgentRun)
- **Node**: type `agentrun`, properties `{status, started_at}`.
- **Edges**: `generated_by` → Agent Node; `performed_for` → Issue/PR Node (the task assigned); `produced` → Commit/PR Nodes it created.
- **Events**: every tool call, file edit, and decision point as an Event with the AgentRun as subject and the Agent as actor — this is the structural answer to O2/O9: agent provenance is Events on a Node, not a bolted-on log.
- **Policy**: the Agent's tool/scope permissions (generalizing Claude Code's per-subagent tool permissions, O10) are evaluated per-Action within the run; any denied Action produces a `policy_violation` Event rather than silently failing.
- **Context**: the slice of the graph (repo state, relevant Issues, prior Events) assembled at task start — this is exactly the `[TBD]`-flagged Context-vs-View overlap from 3.5/3.10 made concrete: is this a persisted Context object, or a logged invocation of a "context-for-agent-task" View? This document does not resolve it (see Open Questions, §7).
- **Evidence**: the AgentRun's final PR carries Evidence Edges back to whichever tests/Policy checks it passed.

### 4.4 Release
- **Node**: type `release`, properties `{version, status}`.
- **Edges**: `includes` → Commit/PR Nodes since the prior Release; `deployed_by` → Action/AgentRun Node; `depends_on` → other Release/artifact Nodes.
- **Events**: `tagged`, `build_started`, `build_completed`, `deployed`, `rolled_back` — this generalizes the Argo CD/Workflows precedent flagged `[TBD]` in Phase 3 (O13), again on inference from the general CI/CD pattern rather than dedicated Argo research.
- **Evidence**: `deployed` Event carries Evidence Edges to the passing CI Test Nodes and any manual-approval Action.
- **View**: the changelog itself is a View — a projection over the `includes` Edges since the previous Release Node, resolving into a rendered document.

*(Incident and Workflow were left out of the four worked in depth here for space; both compose analogously — Incident as a Node with `caused_by`→Commit/Deployment Edges and an Event timeline of detection/mitigation/resolution; Workflow as a Policy-gated chain of Action types with Edges enforcing ordering.)*

---

## 5. Anti-Over-Abstraction Check

Every candidate is interrogated with: (a) could a competitor's existing simpler mechanism already do this? and (b) does it earn its keep, or is it abstraction for its own sake? This section is not a rubric applied for form — three candidates are cut or demoted below.

| Primitive | Simpler existing mechanism that already covers this? | Earns its keep? | Verdict |
|---|---|---|---|
| **Node** | No — every competitor has *some* notion of a typed entity (a row, an object), so this isn't novel, but it's also not skippable: there is no simpler substrate to build the rest on. | Yes — required as the base case. | **MVP** |
| **Edge** | GitHub's "references" and Bitbucket's Jira cross-links (O1, O5) are informal, untyped, unqueryable versions of this — exactly the gap the platform exists to close. | Yes — directly targets Phase 5's #1 differentiator. | **MVP** |
| **Event** | GitHub's audit log and activity feed are ad hoc, separately-architected versions of this (O2) — the whole point is to make it structural instead. | Yes — directly fixes the O2 failure mode and is the substrate for Evidence/provenance. | **MVP** |
| **Policy** | CODEOWNERS, branch protection rules, and CI gating configs are all narrower, type-specific versions of this scattered across every competitor. Generalizing them is real, not decorative — it's what lets Agent actions be gated the same way human actions are (O8/O9), which nothing surveyed does uniformly. | Yes. | **MVP** |
| **Agent** | A Node of type `agent` plus Policy-scoped Edges genuinely covers the mechanics — Agent does **not** need new graph machinery. But leaving it un-named risks agent-actions silently being modeled as a special case later (the exact failure Phase 5 warns against), so it survives as a *documented Node subtype*, not a sixth structural primitive. | Partially — earns its keep as a naming/governance convention, not as new mechanics. | **MVP, but demoted to "Node subtype," not a structural primitive** |
| **Action** | An Event with a `_started`/`_completed` pair sharing a correlation ID covers most of what Action wants, *except* querying "what's currently in flight" without scanning Event history for unmatched starts. That's a real, if narrow, gap for long-running Agent tasks and CI runs. | Marginal — genuinely useful for in-flight-state queries, but could plausibly be deferred and simulated with Event conventions early on. | **V1** (build as an Event convention for MVP; promote to a first-class primitive only if in-flight-state queries prove to be a real bottleneck) |
| **Evidence** | A typed Edge with a `confidence`/`tag` property *is* most of Evidence's mechanics — there's a real argument this collapses entirely into Edge. What's distinct is the epistemic-status vocabulary (this very document's FACT/INFERENCE/PROPOSAL/TBD tagging) as a first-class, queryable property rather than free text. Given the platform's stated ambition (a knowledge graph, not just a link graph) and how load-bearing that tagging discipline has already proven across Phases 1-6, this is worth keeping distinct — but it is thin enough to be a **specialized Edge type**, not an independent structural primitive. | Marginal — real product value, weak claim to structural independence. | **V1, demoted to specialized Edge type** |
| **Intent** | This is close to indistinguishable from "a Node of type `requirement`/`goal` plus a `motivates` Edge" (conceded explicitly in 3.4). No competitor mechanism does more than this either (Linear's issues, GitHub's issue body). | No — fails the abstraction-for-abstraction's-sake test as a *separate primitive*; survives only as a Node type. | **Cut as a primitive; kept as a Node type (`requirement`/`intent`)** |
| **Context** | Sourcegraph's RAG reconstruction (O6) and Copilot's ad hoc assembly both do this today *without* a persisted primitive, and do it adequately for their scope. The genuinely new idea — persisting and auditing what an agent saw — is better captured by *logging View invocations* (see View below) than by inventing storage for "Context" as its own thing. | No — redundant with View + Event once View exists. | **Cut as a separate primitive; folded into "logged View invocation."** |
| **View** | Every competitor's UI (kanban boards, dashboards, changelogs) is an ad hoc, non-reusable version of this. Making views first-class, parameterized, and re-usable — including for agent context assembly — is a genuine, non-redundant idea once Context is folded into it. | Yes, once carrying Context's job too. | **MVP** |

**Net result** `[PROPOSAL]`: the MVP primitive set is **five**, not ten: **Node, Edge, Event, Policy, View** — with **Agent** as a documented Node subtype (not a sixth structural primitive) and **Action** and **Evidence** deferred to V1 as *conventions* (Event-pairing, specialized Edge typing) rather than new mechanics, promoted to independent primitives only if real usage proves the convention insufficient. **Intent** and **Context** are cut outright as separate primitives; their product value survives as a Node type and a View-invocation-logging pattern, respectively.

This matches the pattern extracted in Section 2: GitLab and Linear's real-world graph coherence came from *narrow, shared* data models, not rich taxonomies. A 10-primitive MVP would repeat GitLab's Gitaly/Praefect/Sidekiq/Workhorse mistake (O4) at the data-model layer instead of the deployment layer — solving a problem competitors don't actually have (too few concept types) instead of the one they do have (concepts modeled independently of each other).

**Prior-art check** `[PROPOSAL]` `[INFERENCE]`: the surviving MVP set (Node/Edge/Event + Policy-as-gate) is a recognizable combination of two well-established patterns — the labeled-property-graph model (as used by Neo4j and similar systems) for Node/Edge, and event sourcing / append-only audit log design for Event. Both are decades-old, well-documented architecture patterns with known pitfalls (property-graph query performance at scale; event-sourcing's replay/snapshot complexity and eventual-consistency trade-offs). This document did not run dedicated searches to re-verify current 2026 tooling/vendor specifics for either pattern — treat "these are established patterns" as `[INFERENCE]` from general training knowledge, and treat any specific vendor/version claim about them as `[TBD]` pending Phase 7+ research if the architecture phase needs it. The practical implication: Phase 10 (Architecture Design) should treat "which graph storage engine" and "how snapshots/compaction work for the Event log" as solved-elsewhere engineering problems, not novel research.

---

## 6. Relationship to the Engineering Graph Requirements (original spec §10)

The original spec's node types (Organization, Project, Repository, Requirement, Issue, ADR, Commit, Symbol, PR, Review, Test, Deployment, Human, Agent, AgentRun, Policy) and edge types (implements, depends_on, modifies, generated_by, reviewed_by, caused, derived_from, supersedes) map onto the primitive model as **instances**, not as separate hardcoded concepts `[PROPOSAL]`:

| Spec concept | Primitive realization |
|---|---|
| Organization, Project, Repository | Node types, no special mechanics — plain containment via `part_of` Edges. |
| Requirement, ADR | Node types; ADR additionally carries Evidence-typed Edges (§5) to what justified the decision, and `supersedes` Edges to prior ADRs. |
| Issue | Node type, per §4.1 worked example. |
| Commit, Symbol | Node types; Symbol Nodes connect to Commit Nodes via `modifies`/`defined_in` Edges — this is where a future code-intelligence layer (Sourcegraph-equivalent, O6) would attach, as durable Edges rather than RAG reconstruction. |
| PR | Node type; composes with Review via `reviewed_by` Edges per §4.2. |
| Review | Not a hardcoded type — a Node of type `review` plus its Edge/Event pattern, per §4.2. |
| Test | Node type (a test result), linked to Review/Release via Evidence-typed Edges. |
| Deployment, Release | Node types per §4.4; Deployment is one `caused` Edge hop from Release. |
| Human, Agent | Node subtypes, per §3.8/§5's Agent verdict — same graph mechanics, distinguished for accountability/Policy-scope purposes only. |
| AgentRun | Node type per §4.3; its provenance is entirely Event-sourced rather than a separate audited-log subsystem (the direct fix for O2). |
| Policy | The primitive itself (§3.6/§5), not merely a node type — it is also the *mechanism* by which every other Edge/Action is gated. |
| `implements`, `depends_on`, `modifies`, `derived_from`, `supersedes`, `reviewed_by`, `generated_by`, `caused` | All Edge types (typed relationships between Nodes) — the spec's edge vocabulary is exactly what Edge (§3.2) exists to make queryable and typed, replacing the informal `references`-style linking observed at GitHub/Bitbucket (O1, O5). |

The takeaway `[PROPOSAL]`: the spec's §10 list is not in tension with a 5-primitive MVP — it is the *type registry* that Node and Edge are parameterized over. Adding a new spec concept later (e.g., a "Vulnerability" node type) should never require a new structural primitive or a schema migration to the graph engine itself, only a new registered Node/Edge type — this is the concrete, testable meaning of "composable" for this platform.

---

## 7. Open Questions

**Research Needed**
- `[TBD]` Phase 3 flagged Gerrit, Argo CD/Workflows, and CodeQL/Sentry as unresearched precedents for Review, Deployment, and Policy/Incident modeling respectively (O13). This document composed Review/Release/Policy on inference from general patterns already evidenced elsewhere in Phase 1-5, not from dedicated research into those three products. A follow-up pass specifically against Gerrit's review model and Argo's deployment/rollback state machine could sharpen §4.2/§4.4 and should happen before Phase 8's full specification locks in field-level schemas.
- `[TBD]` No dedicated research was done in this pass on graph database / event-sourcing implementation options (Neo4j vs. other property-graph engines vs. a custom store; specific event-sourcing frameworks). Flagged for Phase 10 (Architecture Design), not resolvable by more *product* research.

**Product Decision**
- `[TBD]` Whether **Action** and **Evidence** should be promoted from "convention" (Event-pairing / specialized Edge) to independent first-class primitives is not resolvable by research — it depends on real usage patterns (how often does "what's in flight right now" get queried; how much does epistemic-tagging get relied upon by users/agents) that don't exist yet. Recommend: build the MVP with the conventions from §5, instrument usage, and revisit at Phase 9 (MVP Definition) or later based on real telemetry, not speculation.
- `[TBD]` Whether Context should ever be materialized as a persisted object (vs. purely a logged View invocation, §5) is a product decision about how much agent-session auditability the platform commits to providing out of the box vs. leaving to Evidence/Event replay. This has cost implications (storage, retention policy) that are a product tradeoff, not a research question.
- `[TBD]` Whether Policy evaluation is synchronous/blocking by default (vs. advisory-first, tightening later) is a product posture decision affecting adoption friction vs. safety guarantees — no competitor evidence in Phase 1-5 settles this either way.

**Architecture Decision**
- `[TBD]` Whether Node/Edge/Event live in one storage engine (favoring GitLab/Linear-style coherence, O3/O7) or are split across a graph store plus an append-only event log with the graph as a derived/materialized view is a core Phase 10 decision. This document's primitive definitions are deliberately agnostic to that choice — both are legitimate realizations of the same Node/Edge/Event model — but the choice has major implications for consistency, replay cost, and self-hosting operational weight, which is exactly the axis Phase 5 flagged GitLab as struggling with (O4). Getting this wrong risks recreating GitLab's Gitaly/Praefect/Sidekiq/Workhorse layering at the primitive-storage level.
- `[TBD]` How Policy evaluation composes with distributed/offline (self-hosted, air-gapped) operation — e.g., can Policy be evaluated locally without a round-trip to a central service — is an architecture question raised directly by Phase 5's "true self-hosted + cloud parity" differentiator but not answered here.

---

## Next Phases

- **Phase 6 — Product Primitive Discovery**: this document.
- **Phase 7 — Requirements Elicitation**: pending.
- **Phase 8 — Requirements Specification (full 55-section document)**: pending.
- **Phase 9 — MVP Definition**: pending.
- **Phase 10 — Architecture Design**: pending.
- **Phase 11 — Red-Team Review**: pending.
- **Phase 12 — Stakeholder Validation**: pending.
- **Phase 13 — Final Baseline**: pending.
