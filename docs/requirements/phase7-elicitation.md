# AI-Native Engineering Platform — Requirements Research: Phase 7

**Status:** v1.0 draft. **Scope:** Phase 7 only (Requirements Elicitation). Builds on `docs/requirements/phase1-5-research.md` and `docs/requirements/phase6-primitives.md` — read both first. This document does not re-litigate Phase 1-6 findings; it cites them. It is a comprehensive *first draft* of atomic, ID-tagged requirements for Phase 8 to assemble, cross-reference, and polish into the full 55-section specification. It is not itself that document — no formal front matter, no exhaustive traceability matrix, no rollout roadmap (that is Phase 8/9's job).

**Tagging convention** (inherited unchanged from phase1-5-research.md / phase6-primitives.md):
- `[FACT]` — verifiable, primary-sourced.
- `[UNVERIFIED-FACT]` — plausible, cited, but not primary-sourced or independently confirmed.
- `[INFERENCE]` — reasonable conclusion from facts, not independently verified.
- `[PROPOSAL]` — our own design idea for the Target Platform, not a claim about any competitor.
- `[TBD]` — unresolved; needs follow-up.

---

## 1. Method

### 1.1 ID scheme

Requirements are ID-tagged by domain prefix, sequentially numbered within each prefix:

| Prefix | Domain |
|---|---|
| `GIT-REQ` | Git-native compatibility |
| `GRF-REQ` | Engineering graph (Node/Edge/Event/Policy/View) |
| `AGT-REQ` | Agent runtime |
| `AI-REQ` | AI Gateway / model routing |
| `CTX-REQ` | Context engine |
| `SEC-REQ` | Security (RBAC/ABAC, agent privilege, audit) |
| `CI-REQ` | CI/CD |
| `UX-REQ` | High-level UX principles |
| `OPS-REQ` | Local-first deployment |
| `CLOUD-REQ` | Cloud-readiness |

IDs are stable once assigned — Phase 8 may renumber for the final document but should carry a mapping back to these Phase 7 IDs so traceability isn't lost.

### 1.2 Requirement schema

Every requirement below carries: **ID, Title, Description, Rationale, Priority, Source, Acceptance Criteria, Dependencies, Risks, Evidence tag**.

- **Priority** is P0 (product cannot exist without it) / P1 (high value) / P2 (enhancement) / P3 (experimental). This is a distinct axis from MVP/V1/V2/Future rollout timing (Phase 9's job) — a requirement can be P0-priority but V1-timed if it is not needed for a first usable release, and this draft notes likely timing in the Rationale/Risks prose where useful, without claiming it as an authoritative roadmap.
- **Source** cites a Phase 1-5 finding (`Oxx`/`§section`), a Phase 6 primitive/verdict (`§5`, `§6`), or `[PROPOSAL]` if this document originates the requirement without direct prior-phase grounding.
- **Evidence** tag follows the convention above and is applied to the requirement's *claim about the world/competitors*, not to the fact that "we propose this" (most requirements are legitimately `[PROPOSAL]` at the requirement level even when grounded in a `[FACT]`-tagged source finding — the requirement itself is a design decision, not a verified fact).

### 1.3 Quality bar

Requirements must be **Atomic** (one testable statement, not a bundle), **Testable** (acceptance criteria a QA engineer or auditing agent could check without further interpretation), **Traceable** (a Source), **Unambiguous** (no "excellent," "seamless," "robust" without a measurable referent), **Necessary** (traces to a real gap/finding, not padding), **Feasible** (no requirement here presumes an unresolved Phase 6 open question is resolved in a particular direction — where a requirement depends on such a decision, it is marked in Dependencies/Risks). Vague requirements ("system should provide excellent AI experience") are rejected outright; where the underlying need is real, it is decomposed into concrete pieces instead (e.g., latency bound, provider-count minimum, fallback behavior).

---

## 2. GIT-REQ — Git-Native Compatibility

Per the original spec's "must not break the standard Git ecosystem" principle, and Phase 5's identification of Git-native compatibility as table stakes, not a differentiator.

**GIT-REQ-001 — Standard Git protocol compliance**
Description: The platform MUST serve and accept standard Git wire protocol (v2 preferred, v1 fallback) over both SSH and HTTPS transports, indistinguishable from a standard Git server to any unmodified Git client.
Rationale: Any deviation forces users onto a forked client, which no competitor surveyed does and which would forfeit the entire existing Git tooling ecosystem.
Priority: P0. Source: [PROPOSAL], informed by phase1-5 baseline (all surveyed forges are Git-protocol-compliant). Acceptance Criteria: `git clone`, `git fetch`, `git push` over both `ssh://` and `https://` succeed against the platform using an unmodified, unconfigured stock Git client (no plugins) at the latest stable Git release and the oldest Git release still in common LTS distros (informational: confirm minimum version in Phase 8). Dependencies: none. Risks: protocol v2's capability negotiation must not silently regress to v1 in ways that hide file listing/ref-filtering performance gains. Evidence: `[PROPOSAL]`.

**GIT-REQ-002 — Standard branch/tag operations**
Description: Full support for branch create/delete/list, tag (lightweight and annotated) create/delete/list, and ref updates via the standard `receive-pack`/`upload-pack` mechanisms.
Rationale: Baseline Git functionality; any gap breaks CI systems and existing scripts.
Priority: P0. Source: [PROPOSAL]. Acceptance Criteria: standard `git branch`, `git tag`, `git push --tags`, `git push origin --delete <branch>` operate with no platform-specific flags. Dependencies: GIT-REQ-001. Risks: none material. Evidence: `[PROPOSAL]`.

**GIT-REQ-003 — Merge and rebase parity**
Description: The platform's server-side merge (for PR/MR merge buttons) MUST produce commits and history identical in shape to what a local `git merge`/`git rebase` would produce with the same strategy, and MUST expose the merge strategy (merge commit / squash / rebase) as an explicit, auditable choice.
Rationale: Server-side merges that silently rewrite history in non-standard ways break bisect, blame, and downstream tooling — an observed source of friction across forges.
Priority: P0. Source: [PROPOSAL]. Acceptance Criteria: for each of the three strategies, resulting commit graph structure matches local Git's output for an equivalent local operation on the same inputs, verified in an automated test. Dependencies: GIT-REQ-002, GRF-REQ (merge as Node+Event, §4). Risks: squash/rebase strategies interact with signature verification (SEC-REQ) and must preserve or clearly break provenance, not silently drop it. Evidence: `[PROPOSAL]`.

**GIT-REQ-004 — Git LFS support**
Description: The platform MUST support Git LFS (Large File Storage) pointer files and the LFS batch transfer API for both SSH and HTTPS remotes.
Rationale: Binary/large-asset workflows (game dev, ML model weights, design files) are common enough that lacking LFS is a disqualifying gap for a chunk of the addressable market; every major forge surveyed supports it.
Priority: P1. Source: [PROPOSAL] (LFS support is industry-standard, not independently re-verified per competitor in Phase 1-5). Acceptance Criteria: `git lfs push`/`pull`/`clone` succeed against the platform using the stock `git-lfs` client; LFS objects are content-addressed and deduplicated server-side. Dependencies: GIT-REQ-001. Risks: LFS storage backend choice affects OPS-REQ (self-hosted storage footprint) and CLOUD-REQ (object-store parity) — flagged as a Phase 10 architecture concern. Evidence: `[UNVERIFIED-FACT]` that all major competitors support LFS; not independently re-verified this phase — see Open Questions.

**GIT-REQ-005 — Submodule and worktree support**
Description: Standard Git submodule resolution and `git worktree` operations MUST function unmodified against platform-hosted repositories, including submodules pointing at other repositories on the same platform instance.
Rationale: Multi-repo engineering organizations rely on submodules; breaking them is a common migration blocker.
Priority: P1. Source: [PROPOSAL]. Acceptance Criteria: a repository with a submodule pointing to a second platform-hosted repo clones and updates correctly via `git submodule update --init --recursive`. Dependencies: GIT-REQ-001. Risks: cross-repo submodule permissions must respect SEC-REQ RBAC — a submodule reference should not leak access to a repo the user cannot otherwise read. Evidence: `[PROPOSAL]`.

**GIT-REQ-006 — Server-side hooks with graph-aware extensions**
Description: The platform MUST support standard Git server-side hooks (`pre-receive`, `update`, `post-receive`) *and* expose an extension point where a hook's outcome is recorded as a graph Event (§GRF), not merely a shell exit code.
Rationale: This is where Git-native and graph-native compatibility meet — Phase 6 requires provenance to be structural (Event), and hooks are the natural point of contact for that on the Git-write path.
Priority: P1. Source: phase6-primitives §3.3, §5 (Event as structural provenance). Acceptance Criteria: a `pre-receive` hook rejection is queryable afterward as an Event on the affected ref/commit Node, not only as a client-side error message. Dependencies: GIT-REQ-001, GRF-REQ-003 (Event primitive). Risks: hook execution must not become a synchronous bottleneck on push latency; needs a timeout/async-Event-emission design (Phase 10). Evidence: `[PROPOSAL]`.

**GIT-REQ-007 — Partial clone / shallow clone / sparse checkout support**
Description: Standard partial clone (`--filter=blob:none`), shallow clone (`--depth`), and sparse checkout MUST be supported server-side.
Rationale: Large monorepos are a primary target use case (per Phase 5's positioning against GitLab's self-hosting weight); without these, monorepo clone times are a disqualifying UX failure.
Priority: P1. Source: [PROPOSAL]. Acceptance Criteria: `git clone --filter=blob:none --depth=1` against a multi-GB test repository completes and results in a working, if partial, checkout with correct on-demand object fetch on file access. Dependencies: GIT-REQ-001. Risks: interacts with protocol v2 capability negotiation (GIT-REQ-001); a naive implementation can silently fall back to full clone. Evidence: `[PROPOSAL]`.

**GIT-REQ-008 — Signed commit/tag verification**
Description: The platform MUST verify and surface GPG/SSH commit and tag signatures, and MUST allow Policy (§GRF/§SEC) to require signed commits on protected branches.
Rationale: Signature verification is the cryptographic root of the provenance chain the platform is trying to make structural (Phase 6's Event/Policy answer to O2); without it, "who really made this commit" remains unverifiable.
Priority: P1. Source: phase6-primitives §3.6 (Policy), O2. Acceptance Criteria: an unsigned push to a branch with a "require signed commits" Policy is rejected with a Policy-violation Event recorded (GRF-REQ, SEC-REQ). Dependencies: GIT-REQ-002, SEC-REQ (Policy enforcement). Risks: key management/rotation UX is easy to get wrong and become a support burden; scoped to verification+enforcement here, key distribution is out of scope for this requirement (see §11 non-requirements). Evidence: `[PROPOSAL]`.

**GIT-REQ-009 — Mirror / migration import**
Description: The platform MUST support importing an existing Git repository (including full history, branches, tags, and — best-effort — LFS objects) from GitHub, GitLab, Bitbucket, or a bare Git URL, without data loss to the Git object graph itself.
Rationale: Zero-friction migration is a precondition for adoption against entrenched incumbents (Phase 3/5 competitive positioning).
Priority: P1. Source: [PROPOSAL], informed by phase1-5 competitive landscape. Acceptance Criteria: post-import, `git log --all` object count and tip SHAs match the source repository exactly; a spot-checked sample of LFS objects resolves correctly. Dependencies: GIT-REQ-001, GIT-REQ-004. Risks: platform-specific metadata (issues, PR comments) is NOT covered by this requirement — see §11 non-requirements; scope creep risk if conflated with full project migration. Evidence: `[PROPOSAL]`.

**GIT-REQ-010 — Git protocol availability independent of AI subsystem**
Description: Core Git read/write operations (clone/fetch/push) MUST remain fully functional when the AI Gateway, Agent runtime, and Context engine are degraded, unreachable, or disabled.
Rationale: Direct requirement from the original spec's "AI Down ≠ Git Down" principle; duplicated here from OPS-REQ-001 because it is *also* a Git-layer correctness requirement, not only a deployment-topology one — the two requirements test different failure surfaces (this one: Git server code paths have no hard dependency on AI services; OPS-REQ-001: the deployment topology allows running without them at all).
Priority: P0. Source: [PROPOSAL], original spec principle. Acceptance Criteria: with the AI Gateway process killed, `git push`/`git clone`/`git fetch` against the platform succeed with no elevated latency beyond normal retry/timeout budgets, verified in an integration test that kills the AI Gateway mid-suite. Dependencies: none (this is an architectural isolation requirement, informing but not depending on OPS-REQ-001). Risks: easy to violate accidentally if logging/eventing paths synchronously call an AI-adjacent service (e.g., auto-summarization on push) — any such feature must be async and non-blocking. Evidence: `[PROPOSAL]`.

---

## 3. GRF-REQ — Engineering Graph

Directly derived from Phase 6's surviving MVP primitive set: **Node, Edge, Event, Policy, View**, with Agent as a Node subtype and Action/Evidence deferred to V1 as conventions.

**GRF-REQ-001 — Typed Node registry**
Description: The platform MUST provide a Node primitive with a stable ID, a type tag drawn from an extensible registry, type-defined scalar properties, and created/updated timestamps, and MUST allow new Node types to be registered without a schema migration to the core graph engine.
Rationale: Direct realization of Phase 6 §3.1/§6 — the mechanism by which the spec's node vocabulary (Organization, Project, Repository, Requirement, Issue, ADR, Commit, Symbol, PR, Review, Test, Deployment, Human, Agent, AgentRun, Policy) is composed rather than hardcoded.
Priority: P0. Source: phase6-primitives §3.1, §5 (MVP verdict), §6. Acceptance Criteria: registering a new Node type (e.g., `vulnerability`) is achievable via configuration/API without downtime or a database migration script, verified by a test that registers a novel type and creates an instance in the same test run. Dependencies: none. Risks: an overly permissive registry (arbitrary schema-less properties) undermines queryability; needs a typed-property contract per registered type (Phase 10). Evidence: `[PROPOSAL]`.

**GRF-REQ-002 — Typed, directed Edge primitive**
Description: The platform MUST provide an Edge primitive connecting two Nodes with a type tag drawn from an extensible registry, directionality, optional scalar properties, creation timestamp, and creator (Human or Agent Node).
Rationale: Phase 6 §3.2 — replaces the informal, untyped cross-linking observed at GitHub (O1) and Bitbucket/Jira (O5), Phase 5's #1 differentiator.
Priority: P0. Source: phase6-primitives §3.2, O1, O5. Acceptance Criteria: creating an Edge of an unregistered type is rejected; querying "all Edges of type `depends_on` from Node X" returns typed, directional results in a single query, not a client-side join across separate object stores. Dependencies: GRF-REQ-001. Risks: none material beyond query performance at scale (Phase 10). Evidence: `[PROPOSAL]`.

**GRF-REQ-003 — Immutable Event log**
Description: The platform MUST provide an append-only, immutable Event primitive recording type, subject (Node/Edge ID), actor (Human or Agent Node), payload, timestamp, and optional causal-predecessor Event ID, and MUST NOT allow Events to be edited or deleted through normal application paths.
Rationale: Phase 6 §3.3 — the direct architectural answer to O2 (GitHub's audit log as a fragile, separately-architected bolt-on subsystem that suffered a documented outage).
Priority: P0. Source: phase6-primitives §3.3, O2. Acceptance Criteria: an attempt to mutate or hard-delete an existing Event via the standard API is rejected with an authorization/immutability error, not silently accepted; only an explicitly privileged, separately-audited administrative retention/erasure path (e.g., for legal compliance) may remove Events, and that path itself emits its own Event. Dependencies: GRF-REQ-001. Risks: retention/compliance erasure requirements (e.g., GDPR) are in tension with pure immutability — needs an explicit, narrow, audited exception path, not a general mutation API (see Open Questions). Evidence: `[PROPOSAL]`.

**GRF-REQ-004 — Policy evaluation primitive**
Description: The platform MUST provide a Policy primitive expressing a condition, a scope (Node/Edge types it applies to), an enforcement mode (advisory/blocking), and an owner, evaluated against Actions/Edge-creation/state-changes before they take effect in blocking mode.
Rationale: Phase 6 §3.6/§5 — generalizes CODEOWNERS/branch-protection/CI-gating patterns scattered across every competitor, and is the mechanism that lets Agent actions be gated identically to human actions (O8/O9), which no competitor surveyed does uniformly.
Priority: P0. Source: phase6-primitives §3.6, §5, O8, O9. Acceptance Criteria: a blocking Policy scoped to "PR merge requires 1 approval Edge from a Node of type `owner`" prevents merge via API/CLI when unmet, and the rejection is recorded as a `policy_violation` Event (GRF-REQ-003) with a reference to the specific Policy Node. Dependencies: GRF-REQ-001, GRF-REQ-002, GRF-REQ-003. Risks: synchronous blocking evaluation is a latency/availability dependency — must remain evaluable without a round-trip to a central service in self-hosted/air-gapped mode (see Open Questions, carried from Phase 6 §7). Evidence: `[PROPOSAL]`.

**GRF-REQ-005 — View primitive (queryable projections)**
Description: The platform MUST provide a named, parameterized View primitive defining a reusable query/projection over Nodes/Edges/Events, invokable by both humans (UI) and agents (API/MCP), with each invocation itself logged as an Event.
Rationale: Phase 6 §3.10/§5 — View absorbed Context's product value (§5 verdict): "how was this agent's context assembled and can we audit it later" is answered by logging View invocations rather than persisting a separate Context object.
Priority: P0. Source: phase6-primitives §3.10, §5, O6, O12. Acceptance Criteria: invoking a registered View with parameters returns a projection over live graph data, and a corresponding `view_invoked` Event is queryable afterward, referencing the invoking actor (Human or Agent Node) and the parameters used. Dependencies: GRF-REQ-001, GRF-REQ-002, GRF-REQ-003. Risks: this requirement is the load-bearing mechanism for CTX-REQ (§6) — if View-invocation logging proves too coarse-grained to reconstruct "exactly what an agent saw," Phase 6's Context-cut decision needs revisiting (flagged in Phase 6 §7 and carried into this document's Open Questions). Evidence: `[PROPOSAL]`.

**GRF-REQ-006 — Core Node type registry seeded from the spec's engineering vocabulary**
Description: At minimum, the following Node types MUST be pre-registered and documented: Organization, Project, Repository, Requirement, Issue, ADR, Commit, Symbol, PR, Review, Test, Deployment, Release, Human, Agent, AgentRun, Policy, Incident.
Rationale: Direct carry-forward of the original spec's §10 node vocabulary and Phase 6 §6's mapping table; pre-registering these avoids every deployment reinventing the same basic vocabulary ad hoc.
Priority: P0. Source: phase6-primitives §6, original spec §10. Acceptance Criteria: each listed type exists in the default type registry on a fresh install and has a documented property schema. Dependencies: GRF-REQ-001. Risks: over-specifying property schemas up front risks a de facto schema migration problem re-appearing one level up (in the type registry rather than the DB); property schemas should themselves be versionable per type. Evidence: `[PROPOSAL]`.

**GRF-REQ-007 — Core Edge type registry**
Description: At minimum, the following Edge types MUST be pre-registered: `implements`, `depends_on`, `modifies`, `derived_from`, `supersedes`, `reviewed_by`, `generated_by`, `assigned_to`, `blocks`, `caused`, `part_of`, `includes`.
Rationale: Direct carry-forward of the original spec's §10 edge vocabulary.
Priority: P0. Source: phase6-primitives §6, original spec §10. Acceptance Criteria: each type is present in a fresh install's registry with documented source/target Node-type constraints. Dependencies: GRF-REQ-002. Risks: none beyond GRF-REQ-006's. Evidence: `[PROPOSAL]`.

**GRF-REQ-008 — Graph query API (human and agent-facing)**
Description: The platform MUST expose a query interface over the Node/Edge/Event graph (e.g., a graph query language or equivalent structured query API) reachable both from the web UI and from the Agent runtime/MCP surface, with pagination and result-size limits.
Rationale: A graph with no query surface is not queryable in practice — this is what makes Phase 5's differentiator real rather than aspirational.
Priority: P0. Source: phase6-primitives §2 pattern extraction, Phase 5 differentiator. Acceptance Criteria: a query for "all PRs that `implement` a given Requirement Node, with their Review status" returns correct results in one API call, both via UI-triggered request and via an MCP tool call from an Agent session. Dependencies: GRF-REQ-001, GRF-REQ-002. Risks: unbounded queries against a large graph are a resource-exhaustion vector — needs pagination/limits from day one (ties to SEC-REQ resource limits). Evidence: `[PROPOSAL]`.

**GRF-REQ-009 — Action-as-Event-convention for in-flight state**
Description: Long-running units of work (CI runs, AgentRuns, deployments) MUST be representable as paired `*_started`/`*_completed` (or `*_failed`) Events sharing a correlation ID, queryable as "currently in flight" without a dedicated Action primitive.
Rationale: Phase 6 §5 explicitly deferred Action to V1 as a convention, not a structural primitive, pending real usage data. This requirement operationalizes that convention so MVP delivery isn't blocked on the Action-vs-Event debate.
Priority: P1. Source: phase6-primitives §5 (Action verdict). Acceptance Criteria: a query for "AgentRuns with a `_started` Event and no matching `_completed`/`_failed` Event" returns correct in-flight results without a full history scan (i.e., is index-backed, not O(n) over all Events). Dependencies: GRF-REQ-003. Risks: if in-flight queries prove to be a genuine bottleneck at scale, this is the trigger condition Phase 6 named for promoting Action to a first-class primitive — Phase 9/10 should watch for it. Evidence: `[PROPOSAL]`.

**GRF-REQ-010 — Evidence-as-specialized-Edge convention**
Description: Edges MAY carry an epistemic-status property set (tag drawn from `[FACT]`/`[INFERENCE]`/`[PROPOSAL]`/`[TBD]`-equivalent vocabulary, confidence, source reference) to realize "Evidence" per Phase 6 §5's demotion verdict, without requiring a separate Evidence primitive.
Rationale: Phase 6 §5 demoted Evidence to a specialized Edge type; this requirement makes that concrete and testable rather than leaving it implicit.
Priority: P1. Source: phase6-primitives §3.9, §5 (Evidence verdict). Acceptance Criteria: an Edge of type `justified_by` (or equivalent) with a `confidence`/`tag` property can be created between a Review Node and a Test Node, and is distinguishable in queries from a plain relationship Edge. Dependencies: GRF-REQ-002. Risks: if this tagging vocabulary is left to free text instead of an enumerated property, it loses the queryability that is the entire point (per Phase 6 §5's rationale for keeping it distinct at all). Evidence: `[PROPOSAL]`.

**GRF-REQ-011 — Requirement and ADR as Node types with traceability Edges**
Description: Requirement and ADR MUST be pre-registered Node types supporting `implements` (PR→Requirement), `motivated_by`, and `supersedes` (ADR→ADR) Edges, enabling exactly the kind of requirement-to-code traceability this document itself is trying to establish for the platform's own development.
Rationale: Direct realization of Phase 6's "Intent cut as a primitive, kept as a Node type" verdict (§5) and the original spec's traceability ambitions.
Priority: P1. Source: phase6-primitives §3.4, §5 (Intent verdict), §6. Acceptance Criteria: a PR Node can carry an `implements` Edge to a Requirement Node, and a View exists that renders "which PRs implement Requirement X" and "which Requirements are un-implemented." Dependencies: GRF-REQ-001, GRF-REQ-002, GRF-REQ-005. Risks: none material. Evidence: `[PROPOSAL]`.

---

## 4. AGT-REQ — Agent Runtime

Vendor-neutral agent execution surface, per the original spec's AgentRuntime interface list, with Agent modeled as a Node subtype per Phase 6 §3.8/§5.

**AGT-REQ-001 — Vendor-neutral agent lifecycle control**
Description: The platform MUST expose start/stop/pause/resume operations for an agent run through a single, vendor-neutral interface, regardless of which underlying model/agent framework executes the work.
Rationale: Direct requirement from the original spec's AgentRuntime interface; avoids locking the platform to one agent vendor, mirroring the multi-provider posture Phase 5 recommends for AI more broadly.
Priority: P0. Source: original spec (AgentRuntime interface), phase1-5 §Phase 5 multi-provider theme. Acceptance Criteria: an integration test starts, pauses, resumes, and stops an agent run using two different underlying agent backends through the identical API surface, with no backend-specific parameters required for the basic lifecycle calls. Dependencies: GRF-REQ-006 (Agent, AgentRun Node types). Risks: "vendor-neutral" can degrade into lowest-common-denominator if backend-specific capabilities (e.g., subagents, hooks) have no extension point — needs a capability-negotiation mechanism, not just a fixed interface (Phase 10). Evidence: `[PROPOSAL]`.

**AGT-REQ-002 — Human approve/reject gate on agent actions**
Description: The platform MUST support a configurable approval gate where a specified class of agent Action (e.g., merge, deploy, force-push, destructive file operations) requires explicit human approve/reject before execution, expressed as a Policy (§GRF).
Rationale: Direct requirement from the original spec's AgentRuntime interface (`approve`/`reject`) and the "Human Authority" principle referenced in the UX section; operationalizes Phase 6's requirement that Agent actions be Policy-gated the same as human actions (O8/O9).
Priority: P0. Source: original spec (Human Authority principle, AgentRuntime interface), phase6-primitives O8/O9, GRF-REQ-004. Acceptance Criteria: an agent attempting a Policy-gated Action without prior approval is blocked, the block is recorded as a `policy_violation` Event, and a human approval action is itself recorded as an Event with the approving Human Node as actor. Dependencies: GRF-REQ-004, AGT-REQ-001. Risks: over-gating erodes the productivity benefit of agents; the *set* of gated actions is a product/config decision (Phase 9), not fixed by this requirement. Evidence: `[PROPOSAL]`.

**AGT-REQ-003 — Agent messaging and status interface**
Description: The platform MUST expose message (send instruction/context to a running agent), status (current state: idle/running/waiting-for-approval/failed/completed), and events (stream of the agent's own tool-call/decision Events) as distinct API operations.
Rationale: Direct requirement from the original spec's AgentRuntime interface list.
Priority: P0. Source: original spec (AgentRuntime interface). Acceptance Criteria: a client can poll status and receive a materially different value across at least the four listed states within a single test run, and can subscribe to the Event stream and receive Events in causal order (using the causal-predecessor field per GRF-REQ-003). Dependencies: GRF-REQ-003, AGT-REQ-001. Risks: none material beyond general API design. Evidence: `[PROPOSAL]`.

**AGT-REQ-004 — Agent artifact capture**
Description: Files, diffs, generated documents, and other artifacts an agent run produces MUST be captured and linked to the AgentRun Node via `produced` Edges (GRF-REQ-007), retrievable after the run completes or fails.
Rationale: Direct requirement from the original spec's AgentRuntime interface (`artifacts`); prevents agent output from being ephemeral/unauditable, per Phase 6's O2/O9-driven provenance stance.
Priority: P0. Source: original spec (AgentRuntime interface), phase6-primitives §4.3. Acceptance Criteria: after an agent run that creates a PR, the AgentRun Node has a `produced` Edge to the PR Node, and any intermediate artifact (e.g., a generated test file later discarded) remains retrievable via the AgentRun's artifact list even if not part of the final PR. Dependencies: GRF-REQ-002, GRF-REQ-006. Risks: artifact storage volume at scale is an OPS-REQ/CLOUD-REQ storage-cost concern; needs a retention policy (Open Questions). Evidence: `[PROPOSAL]`.

**AGT-REQ-005 — Scoped, workspace-isolated agent execution**
Description: Each agent run MUST execute against an isolated workspace (filesystem/environment scope) and MUST NOT have implicit access to any resource — repository, credential, or graph subset — outside what its Policy-defined scope explicitly grants.
Rationale: Direct requirement from the original spec's AgentRuntime interface (`workspace`, `credentials`) and its "temporary scoped identity" example; the minimal-privilege posture is also SEC-REQ's foundation for agent trust.
Priority: P0. Source: original spec (AgentRuntime interface, scoped-identity example). Acceptance Criteria: an agent run scoped to Repository A cannot read or write Repository B's contents or credentials via any API path, verified by a negative-access integration test. Dependencies: GRF-REQ-004, SEC-REQ-002 (agent scoped credentials). Risks: workspace isolation mechanism (container/VM/sandbox) is a Phase 10 architecture decision; this requirement constrains behavior, not implementation. Evidence: `[PROPOSAL]`.

**AGT-REQ-006 — Resource limits on agent execution**
Description: The platform MUST allow configuring per-run resource limits (wall-clock time, token/cost budget, tool-call count, concurrent-run count) and MUST terminate a run that exceeds its limit, recording a `resource_limit_exceeded` Event.
Rationale: Direct requirement from the original spec's AgentRuntime interface (`resource limits`); prevents runaway agent cost/compute, a real operational risk with autonomous agents.
Priority: P0. Source: original spec (AgentRuntime interface). Acceptance Criteria: an agent run configured with a token budget of N is terminated at or before N+ε tokens consumed, with the terminating Event recorded and visible via AGT-REQ-003's status/events interface. Dependencies: AGT-REQ-001, GRF-REQ-003. Risks: limits that are too aggressive by default will frustrate legitimate long-running tasks; default values are a Phase 9 product decision, not fixed here. Evidence: `[PROPOSAL]`.

**AGT-REQ-007 — Agent as a Node subtype with full graph participation**
Description: Every Agent and every AgentRun MUST be represented as a Node in the engineering graph (not a separate, ungraphed subsystem), with the same Edge/Event/Policy mechanics available to Agent Nodes as to Human Nodes.
Rationale: Direct realization of Phase 6 §3.8/§5's verdict — Agent survives as a documented Node subtype specifically to prevent agent actions from becoming a second-class, ungraphed special case, the failure mode Phase 5 observed at every competitor (O8).
Priority: P0. Source: phase6-primitives §3.8, §5, O8. Acceptance Criteria: a query against the graph for "all Nodes that authored an Edge/Event of type X" returns both Human and Agent Nodes uniformly, with no separate query path required for agent-originated activity. Dependencies: GRF-REQ-001, GRF-REQ-002, GRF-REQ-003. Risks: none material — this is largely a discipline requirement on top of already-required primitives. Evidence: `[PROPOSAL]`.

**AGT-REQ-008 — Multi-agent-framework interoperability via MCP**
Description: The Agent runtime MUST support the Model Context Protocol (MCP) as a tool-invocation surface, for both platform-provided tools exposed to external agent clients and external MCP tools/servers an agent run may call.
Rationale: MCP is now the de facto cross-vendor tool-invocation standard (O12); supporting it is necessary but, per O12, not sufficient — it standardizes tool calls, not context/session/provenance modeling, which is what GRF-REQ/CTX-REQ separately address.
Priority: P1. Source: phase1-5 §Phase 5 "Emerging", O12. Acceptance Criteria: an external MCP-compliant client can list and invoke the platform's exposed tools (e.g., a graph-query View) using standard MCP handshake/tool-call messages, with no platform-specific protocol extension required for basic invocation. Dependencies: GRF-REQ-005, GRF-REQ-008. Risks: per-vendor tool-count caps observed elsewhere (Cursor's 40-tool cap, O12) suggest the platform's own tool surface should be curated/composable, not a flat dump of every View as a separate tool — a design concern for Phase 10, not this requirement. Evidence: `[FACT]`/`[INFERENCE]` per O12.

**AGT-REQ-009 — Subagent / delegated-scope execution**
Description: The platform SHOULD support an agent run spawning a scoped subagent run with a narrower Policy scope than its parent, with the parent-child relationship recorded as a `generated_by`/`part_of` Edge chain.
Rationale: Claude Code's hooks/skills/subagents separation was independently identified as a relatively principled context-engineering pattern worth generalizing (O10).
Priority: P2. Source: phase1-5 §Phase 3, O10. Acceptance Criteria: a subagent run's effective Policy scope is provably a subset of its parent's (no privilege escalation via delegation), verified by a negative test attempting scope widening. Dependencies: AGT-REQ-005, AGT-REQ-007. Risks: nested delegation chains complicate audit trail readability; needs a flattened "effective actor chain" View for humans (UX-REQ). Evidence: `[INFERENCE]` per O10.

---

## 5. AI-REQ — AI Gateway / Model Routing

**AI-REQ-001 — Multi-provider model access**
Description: The platform MUST route inference requests to at least two independent model providers (e.g., a cloud API provider and a locally-hosted model runtime) through a single internal interface, selectable per-request or per-policy.
Rationale: Avoids single-vendor lock-in and is a precondition for OPS-REQ's air-gapped/local operation, which by definition cannot depend on a single cloud-only provider.
Priority: P0. Source: [PROPOSAL], informed by phase1-5 multi-provider competitive theme and Phase 5's local/cloud parity differentiator. Acceptance Criteria: a routing configuration can direct the same logical request type to either a cloud provider or a local model without application-code changes, verified by an integration test that swaps providers via config only. Dependencies: none. Risks: providers differ in tool-calling/context-window semantics; the gateway's abstraction must not silently degrade to the lowest-common-denominator feature set (specific mitigations are Phase 10). Evidence: `[PROPOSAL]`.

**AI-REQ-002 — Cost and token observability per request**
Description: Every inference request routed through the AI Gateway MUST be logged with provider, model, token counts (input/output), and cost estimate, linked to the initiating Agent/View/Human via an Event.
Rationale: Cost runaway is a named operational risk (AGT-REQ-006); this requirement is the observability precondition for enforcing that budget and for CI-REQ/OPS-REQ cost accountability generally.
Priority: P0. Source: [PROPOSAL]. Acceptance Criteria: after any AI Gateway call, a queryable Event exists with provider/model/token/cost fields populated and a reference to the requesting actor. Dependencies: GRF-REQ-003, AGT-REQ-006. Risks: cost-estimate accuracy depends on provider pricing data staying current — a data-maintenance risk, not a design flaw. Evidence: `[PROPOSAL]`.

**AI-REQ-003 — Routing policy by data sensitivity**
Description: The platform MUST support a routing Policy that restricts which requests may be sent to external/cloud providers based on data classification (e.g., a repository flagged as air-gapped/sensitive routes only to local models).
Rationale: Direct requirement for credible self-hosted/air-gapped operation (Phase 5 differentiator) — without this, "local-first" is undermined by any AI feature that silently phones home.
Priority: P0. Source: [PROPOSAL], phase1-5 Phase 5 local/cloud parity theme. Acceptance Criteria: a repository tagged "no-external-AI" causes any AI Gateway request scoped to it to be routed only to configured local providers, and a routing attempt to an external provider is blocked and recorded as a Policy violation Event. Dependencies: AI-REQ-001, GRF-REQ-004. Risks: classification must be enforced at the Gateway layer, not left to each feature to self-police — a single choke point (Phase 10). Evidence: `[PROPOSAL]`.

**AI-REQ-004 — Provider fallback and degradation behavior**
Description: When a configured provider is unreachable or rate-limited, the Gateway MUST fail over to a configured fallback provider (if policy allows) or return a structured, distinguishable error — never silently hang or silently drop the request.
Rationale: Reliability requirement; ties into OPS-REQ-001's "AI Down ≠ Git Down" principle by ensuring AI failures are visible and bounded rather than cascading.
Priority: P1. Source: [PROPOSAL], original spec "AI Down ≠ Git Down" principle. Acceptance Criteria: killing the primary provider mid-test causes fallback to the secondary provider within a bounded time, or a structured `provider_unavailable` error if no fallback is configured/allowed. Dependencies: AI-REQ-001. Risks: none material. Evidence: `[PROPOSAL]`.

**AI-REQ-005 — Model/provider swap without application changes**
Description: Changing the model or provider backing a given routing policy MUST NOT require changes to calling application code (agents, Views, CI integrations) — only a Gateway configuration change.
Rationale: Direct consequence of AI-REQ-001's abstraction; stated separately because it's independently testable and is a common point of accidental coupling.
Priority: P1. Source: [PROPOSAL]. Acceptance Criteria: swapping the model backing a routing policy in config, with no code deploy, changes observed provider/model in AI-REQ-002's logs on the next call. Dependencies: AI-REQ-001, AI-REQ-002. Risks: none material. Evidence: `[PROPOSAL]`.

**AI-REQ-006 — Prompt/response caching for repeated context**
Description: The Gateway SHOULD support caching of provider-side context (e.g., prompt caching where the provider supports it) to reduce redundant token cost for repeated large-context calls (e.g., a Context View invoked repeatedly against a stable repo state).
Rationale: Directly reduces the cost risk AI-REQ-002 makes visible; complements CTX-REQ's minimal-context design goal from the other direction (reduce cost of necessary large contexts, not just avoid unnecessary ones).
Priority: P2. Source: [PROPOSAL]. Acceptance Criteria: a repeated call with an unchanged large context segment shows a measurable cost/latency reduction versus the first call, for providers that support caching. Dependencies: AI-REQ-001, AI-REQ-002. Risks: cache invalidation correctness (stale cached context served after repo change) is a correctness risk, not just a performance one — needs explicit invalidation tied to the relevant Node/Edge change Events. Evidence: `[PROPOSAL]`.

---

## 6. CTX-REQ — Context Engine ("Context-as-a-Service")

Informed by Sourcegraph's per-query RAG reconstruction (O6) and Phase 6's decision to fold Context into logged View invocations rather than a separate primitive.

**CTX-REQ-001 — Minimal-context retrieval by default**
Description: Context assembly for an agent task MUST default to a bounded, relevance-ranked subset of the graph (and, where applicable, code search results) rather than a full-repository scan, with the bound configurable.
Rationale: Directly targets the cost/latency failure mode implicit in O6 (Sourcegraph's RAG reconstruction) done naively — full-repo rescans are both slow and expensive, and Phase 5 positions this platform's graph as a faster substrate than RAG-from-scratch for durable relationships.
Priority: P0. Source: phase1-5 O6, phase6-primitives §3.5, §5 (Context verdict). Acceptance Criteria: for a repository above a configured size threshold, the default context-assembly View returns within a bounded latency (specific number is a Phase 9/10 SLA decision) without reading every file in the repository, verified by an instrumented test counting files touched. Dependencies: GRF-REQ-005, GRF-REQ-008. Risks: "relevance ranking" quality is itself a hard, ongoing research problem — this requirement bounds cost/latency, it does not guarantee retrieval quality (a separate, likely V2, concern). Evidence: `[PROPOSAL]`.

**CTX-REQ-002 — Context assembly is a logged View invocation**
Description: Every context assembled for an agent or human session MUST be the result of an explicitly registered View invocation (GRF-REQ-005), logged as an Event, not an ad hoc, unlogged retrieval path.
Rationale: Direct operationalization of Phase 6 §5's Context-cut verdict: "the genuinely new idea — persisting and auditing what an agent saw — is better captured by logging View invocations than by inventing storage for Context as its own thing."
Priority: P0. Source: phase6-primitives §3.5, §5, §7 Open Questions. Acceptance Criteria: for any agent session, the exact context it received is reconstructable after the fact from the View-invocation Event (parameters + View definition + graph state reference/timestamp), without needing a separately persisted Context blob. Dependencies: GRF-REQ-005. Risks: if graph/code state has mutated since the invocation, exact reconstruction of *content* (not just parameters) may require either a state snapshot or a reproducibility guarantee against the Event-timestamped graph state — this is the open architectural question Phase 6 flagged and this document does not resolve (see §12). Evidence: `[PROPOSAL]`.

**CTX-REQ-003 — Cross-repository context assembly**
Description: Context assembly MUST be able to span multiple repositories within a Human/Agent's authorized scope, resolving Edges (e.g., `depends_on`, `part_of`) that cross repository boundaries, not limited to a single repo's contents.
Rationale: This is the concrete product answer to O6's "cross-repo context reconstructed per-query" limitation — the platform's durable graph Edges should make cross-repo context a query, not a fresh RAG pass each time.
Priority: P1. Source: phase1-5 O6. Acceptance Criteria: a context View parameterized on Repository A returns related Nodes from Repository B when a `depends_on` Edge connects them, subject to the requester's access to Repository B. Dependencies: CTX-REQ-001, GRF-REQ-002, SEC-REQ-001. Risks: must not leak Repository B content to a requester unauthorized for B — this is a hard SEC-REQ dependency, not optional. Evidence: `[PROPOSAL]`.

**CTX-REQ-004 — Explicit context budget and truncation transparency**
Description: When assembled context exceeds a model's context window or a configured budget, the platform MUST apply a documented truncation/prioritization strategy and MUST record what was excluded, not silently drop content without a trace.
Rationale: Prevents a common, hard-to-debug agent failure mode (silently missing context) from being invisible; ties to the audit-first posture (SEC-REQ, Phase 6 O2 answer).
Priority: P1. Source: [PROPOSAL]. Acceptance Criteria: when truncation occurs, the View-invocation Event (CTX-REQ-002) records which candidate Nodes/Edges were excluded and why (e.g., rank cutoff), retrievable for debugging. Dependencies: CTX-REQ-001, CTX-REQ-002. Risks: none material beyond implementation complexity. Evidence: `[PROPOSAL]`.

**CTX-REQ-005 — Portable, vendor-neutral project instructions**
Description: The platform MUST recognize and incorporate a vendor-neutral project-instruction convention (e.g., an AGENTS.md-equivalent file) into default context assembly, without requiring a platform-proprietary format as the only option.
Rationale: O11 establishes that portable, structured "intent/context" artifacts (AGENTS.md, working across 8+ tools) are independently valued; not supporting the existing convention would force needless migration friction.
Priority: P1. Source: phase1-5 O11. Acceptance Criteria: a repository containing an AGENTS.md-equivalent file has its contents included in default agent context assembly with no additional configuration. Dependencies: CTX-REQ-001. Risks: none material. Evidence: `[FACT]` per O11 (convention's existence/adoption); requirement itself `[PROPOSAL]`.

---

## 7. SEC-REQ — Security

RBAC/ABAC, agent minimal-privilege model, and audit trail as a first-class structural property (per Phase 5's finding that GitHub's audit log is a fragile bolt-on).

**SEC-REQ-001 — Role- and attribute-based access control**
Description: The platform MUST support both role-based (RBAC: fixed roles like Owner/Maintainer/Reader) and attribute-based (ABAC: conditions over Node/Edge properties, e.g., "path starts with /infra") access control, evaluated as Policy (GRF-REQ-004).
Rationale: RBAC alone cannot express the fine-grained, path/type-scoped access patterns competitors approximate with CODEOWNERS-style mechanisms; ABAC-over-Policy generalizes that.
Priority: P0. Source: phase6-primitives §3.6 (Policy), [PROPOSAL]. Acceptance Criteria: an access decision can be expressed as either a role check or a property-condition check (or both combined) through the same Policy mechanism, verified by two test Policies of each kind. Dependencies: GRF-REQ-004. Risks: ABAC expressiveness vs. evaluation performance at scale is a Phase 10 concern. Evidence: `[PROPOSAL]`.

**SEC-REQ-002 — Temporary, scoped agent credentials**
Description: Every agent run MUST execute under a temporary, narrowly-scoped credential (not a long-lived human or service-account token) whose scope is derived from its Policy (AGT-REQ-005) and which expires automatically at run completion or timeout.
Rationale: Direct requirement from the original spec's "temporary scoped identity" example for agents; this is the concrete minimal-privilege mechanism, distinct from AGT-REQ-005's behavioral requirement (workspace isolation) — this requirement is specifically about the credential/token lifecycle.
Priority: P0. Source: original spec (temporary scoped identity example). Acceptance Criteria: an agent run's credential is provably invalid for use after the run ends or its timeout elapses, verified by attempting an API call with the expired credential post-run. Dependencies: AGT-REQ-005, AGT-REQ-006. Risks: credential-issuance latency must not become a per-run startup bottleneck; needs pre-warming or fast issuance path (Phase 10). Evidence: `[PROPOSAL]`.

**SEC-REQ-003 — Structural audit trail (not a bolt-on subsystem)**
Description: Every state-changing operation by a Human or Agent — access grant/revoke, Policy change, merge, deploy, credential issuance — MUST produce an Event in the same core Event log as all other graph activity (GRF-REQ-003), not a separately-architected audit subsystem with its own availability characteristics.
Rationale: Direct architectural response to O2 — GitHub's audit log is a structurally separate subsystem that suffered a documented 28-minute outage from a shared-dependency failure; this requirement exists specifically so the platform's audit trail cannot fail independently of (or dependently in an undocumented way on) the rest of the system.
Priority: P0. Source: phase1-5 O2, phase6-primitives §3.3, §5. Acceptance Criteria: security-relevant operations (a defined list, maintained as platform config) are verified by test to each produce a queryable Event via the exact same query path (GRF-REQ-008) used for ordinary graph Events — no separate "audit API." Dependencies: GRF-REQ-003, GRF-REQ-008. Risks: "same log as everything else" raises retention/compliance-export questions (e.g., exporting only security Events for a compliance audit) — needs a filtered View (GRF-REQ-005), not a separate store. Evidence: `[FACT]` per O2; requirement `[PROPOSAL]`.

**SEC-REQ-004 — Least-privilege default for new agents and integrations**
Description: A newly registered Agent or external integration MUST default to zero Policy grants (deny-by-default) until explicitly scoped, never to an implicit broad-access default.
Rationale: Complements SEC-REQ-002; addresses the common real-world failure mode of overly broad default service-account permissions.
Priority: P0. Source: [PROPOSAL]. Acceptance Criteria: a freshly registered Agent Node with no explicit Policy grants fails all non-trivial API calls with an authorization error, verified by a negative test immediately after registration. Dependencies: GRF-REQ-004, GRF-REQ-006. Risks: overly strict defaults could create onboarding friction; mitigated by tooling (a guided "grant this scope" UX flow), which is a UX-REQ/Phase 12 concern, not this requirement. Evidence: `[PROPOSAL]`.

**SEC-REQ-005 — Secrets and credential storage isolated from the graph**
Description: Credentials, tokens, and secrets referenced by Policy/Agent configuration MUST be stored in a dedicated secrets store with access mediated by Policy, and MUST NOT be retrievable as plain Node/Edge properties via the general graph query API (GRF-REQ-008).
Rationale: Prevents an entire class of accidental secret exposure via broad graph queries or View definitions; the graph's queryability (its core value) must not become a secrets-exfiltration surface.
Priority: P0. Source: [PROPOSAL]. Acceptance Criteria: a graph query attempting to select a Node property flagged as `secret` returns a redacted value or an authorization error, never the plaintext, verified by a test querying a Node with a secret-flagged property as an unauthorized actor. Dependencies: GRF-REQ-001, GRF-REQ-008. Risks: property-level flagging must be enforced at the query engine, not per-View — a single choke point (Phase 10). Evidence: `[PROPOSAL]`.

**SEC-REQ-006 — Signed, non-repudiable Event authorship**
Description: Where feasible, Events attributable to a specific Human MUST be cryptographically bound to that Human's verified identity (e.g., signed commits per GIT-REQ-008; signed approval actions), so authorship cannot be forged by another party with API access.
Rationale: Extends GIT-REQ-008's signature verification beyond commits to the broader Event stream, closing the gap where "the Event says Human X approved this" is trusted by database record alone.
Priority: P2. Source: [PROPOSAL], extends GIT-REQ-008. Acceptance Criteria: an approval Event created via an API call using Human X's session credential but claiming a different actor identity is rejected. Dependencies: GIT-REQ-008, GRF-REQ-003. Risks: full non-repudiation (cryptographic signing of every Event) is expensive; this requirement is scoped to security/approval-relevant Events, not the full Event stream, to keep it feasible — flagged in Risks rather than silently narrowed. Evidence: `[PROPOSAL]`.

**SEC-REQ-007 — Compliance-scoped Event export**
Description: The platform MUST support exporting a filtered, time-bounded slice of the Event log (via a View, GRF-REQ-005) in a format suitable for external compliance/SIEM tooling, without requiring direct database access.
Rationale: Practical consequence of SEC-REQ-003's "same log, not a separate subsystem" design — compliance teams still need a bounded export, which must be a View, not a special-cased audit-export subsystem (which would silently recreate the O2 failure mode one layer up).
Priority: P2. Source: [PROPOSAL], extends SEC-REQ-003. Acceptance Criteria: a time-bounded, actor-filtered Event export completes and validates against a standard log-ingestion schema (specific format TBD in Phase 8/10). Dependencies: SEC-REQ-003, GRF-REQ-005. Risks: none material. Evidence: `[PROPOSAL]`.

---

## 8. CI-REQ — CI/CD

Must integrate with the graph model (build/test/deploy as Node+Event instances), not be a bolted-on YAML silo.

**CI-REQ-001 — CI runs as first-class graph Nodes**
Description: Every CI run (build, test, lint, or custom job) MUST be represented as a Node (type `ci-run` or a registered subtype), linked to the triggering Commit/PR Node via an Edge, with its stage transitions recorded as Events.
Rationale: Direct realization of Phase 6 §4.4's Release worked example and §6's mapping — avoids CI being a bolted-on silo whose results are only visible in a separate UI, unlike the graph-integrated model this platform is differentiated on.
Priority: P0. Source: phase6-primitives §4.4, §6. Acceptance Criteria: a CI run triggered by a push produces a Node queryable via GRF-REQ-008 with `caused` or `triggered_by` Edges to the Commit, and `queued`/`started`/`completed`/`failed` Events in order. Dependencies: GRF-REQ-001, GRF-REQ-002, GRF-REQ-003. Risks: none material beyond general pipeline-engine implementation (Phase 10). Evidence: `[PROPOSAL]`.

**CI-REQ-002 — Test results as Evidence-typed Edges**
Description: Individual test results MUST be linked to the Review/PR/Release Node they support via Evidence-typed Edges (GRF-REQ-010), not only surfaced in a CI-run log blob.
Rationale: Operationalizes Phase 6 §4.2/§4.4's worked examples (approval/deployment Evidence Edges to Test Nodes), making "was this actually tested" a graph-queryable fact, not something a human has to click into a CI log to find.
Priority: P1. Source: phase6-primitives §4.2, §4.4, GRF-REQ-010. Acceptance Criteria: a query "does PR X have a passing-test Evidence Edge" returns a direct answer without parsing CI log text. Dependencies: CI-REQ-001, GRF-REQ-010. Risks: none material. Evidence: `[PROPOSAL]`.

**CI-REQ-003 — Pipeline definitions are graph-native, not a hidden YAML silo**
Description: CI pipeline definitions MUST themselves be addressable, versioned graph entities (e.g., a `pipeline-def` Node type with `part_of` Edge to Repository), queryable and diffable through the same graph/query surface as other Nodes, even though the underlying definition format may still be a YAML/config file in the repo.
Rationale: This is the concrete meaning of "not a bolted-on YAML silo" — the *artifact* may remain YAML for ecosystem-compatibility reasons (portability, existing tooling), but its relationship to the graph (which repo, which runs it triggered, its own version history) must be first-class, not invisible to the query layer.
Priority: P1. Source: [PROPOSAL], extends phase6-primitives §6. Acceptance Criteria: a query for "which pipeline definition version triggered CI-run X" returns a direct graph answer, not requiring separate lookup in CI-provider-specific storage. Dependencies: GRF-REQ-001, CI-REQ-001. Risks: risk of over-scoping this into "replace all CI config with a proprietary format" — explicitly not required here; see §11 non-requirements. Evidence: `[PROPOSAL]`.

**CI-REQ-004 — Existing CI ecosystem compatibility (webhook/exec compatibility)**
Description: The platform MUST support triggering external CI systems (e.g., via standard webhook payloads compatible with common CI runners) and ingesting their results back into the graph (CI-REQ-001), for organizations not ready to run pipelines natively on-platform.
Rationale: Mirrors GIT-REQ's "don't break the ecosystem" principle applied to CI — forcing an immediate CI migration would be a comparable adoption blocker to forcing a Git client change.
Priority: P1. Source: [PROPOSAL]. Acceptance Criteria: a push event triggers a configured external webhook with a payload an unmodified common CI runner can consume, and that runner's result, posted back via a documented status API, is ingested as a CI-REQ-001 Node/Event. Dependencies: GIT-REQ-001, CI-REQ-001. Risks: results ingested from an external system carry weaker provenance guarantees (no guarantee the external system itself is trustworthy) — should be flagged distinctly in the graph (e.g., `source: external`) rather than presented identically to native runs. Evidence: `[PROPOSAL]`.

**CI-REQ-005 — Agent-triggerable and agent-consumable CI**
Description: An Agent (AGT-REQ) MUST be able to trigger a CI run and consume its result (pass/fail, Evidence Edges) as part of an agent task, through the same API/MCP surface as a human-triggered run, subject to the Agent's Policy scope.
Rationale: Ties CI-REQ into the agent-native ambition directly — an agent verifying its own PR against tests before requesting human review is a core expected workflow, not a nice-to-have.
Priority: P1. Source: [PROPOSAL], ties AGT-REQ + CI-REQ. Acceptance Criteria: an agent run can trigger a CI-REQ-001 run for its own PR and read back pass/fail via AGT-REQ-003's status interface, without human-only API paths. Dependencies: CI-REQ-001, AGT-REQ-003, AGT-REQ-005. Risks: none material beyond general Policy scoping already covered by AGT-REQ-005. Evidence: `[PROPOSAL]`.

**CI-REQ-006 — Deployment as a Policy-gated Action-convention**
Description: A deployment/Release action MUST be gated by Policy (GRF-REQ-004) before execution (e.g., requiring passing CI-REQ-002 Evidence and/or human approval), and its full started/completed/rolled-back lifecycle recorded per GRF-REQ-009's Event-pairing convention.
Rationale: Direct realization of Phase 6 §4.4's Release worked example; on inference from the general CI/CD pattern, since dedicated Argo CD/Workflows research remains a flagged Phase 6 `[TBD]` (O13) not resolved in this pass.
Priority: P1. Source: phase6-primitives §4.4, O13 `[TBD]` carried forward. Acceptance Criteria: a deployment blocked by an unmet Policy (e.g., missing passing-test Evidence) is rejected with a recorded Policy-violation Event; a completed deployment's Events form a queryable started→completed chain. Dependencies: GRF-REQ-004, GRF-REQ-009, CI-REQ-002. Risks: rollback semantics specifically were flagged `[TBD]` in Phase 6 (O13) and are not resolved by this requirement — see Open Questions. Evidence: `[TBD]` carried forward from phase6-primitives O13.

---

## 9. UX-REQ — High-Level UX Principles

A deliberately small set — detailed UX is Phase 12's job. These are the highest-level, testable requirements directly implied by the original spec's Intent>Navigation, Context>Page, Human Authority, and Progressive Disclosure principles.

**UX-REQ-001 — Intent-first entry point**
Description: The primary UI entry point MUST accept a natural-language or structured statement of intent (e.g., "review this PR," "find what depends on this module") and resolve it to the relevant View/graph query, rather than requiring the user to first navigate a fixed menu/page hierarchy to find the right screen.
Rationale: Direct requirement from the original spec's "Intent > Navigation" principle.
Priority: P1. Source: original spec (Intent > Navigation principle). Acceptance Criteria: a usability test task phrased as an intent statement is completable without the user first consulting a sitemap/menu structure, for a defined set of common intents. Dependencies: GRF-REQ-005, GRF-REQ-008. Risks: this requirement is easy to satisfy shallowly (a search box) without satisfying it meaningfully (actual intent resolution to structured queries); Phase 12 should define the acceptance bar more rigorously with real usability testing. Evidence: `[PROPOSAL]`.

**UX-REQ-002 — Context-assembled pages, not fixed page templates**
Description: A given "page" (e.g., viewing a PR) MUST render its content by composing relevant Views/graph context dynamically (what's relevant to *this* PR: linked Requirement, failing tests, blocking Policy), not from a single fixed template that hides related graph context behind separate navigation.
Rationale: Direct requirement from the original spec's "Context > Page" principle; also the human-facing expression of CTX-REQ's context-assembly machinery.
Priority: P1. Source: original spec (Context > Page principle). Acceptance Criteria: for a defined set of Node-type detail pages, the rendered page includes at least the directly connected Edges/Evidence/Policy state without requiring additional navigation, verified against a content checklist per page type. Dependencies: GRF-REQ-005, GRF-REQ-008. Risks: over-loading a page with "everything connected" recreates information overload; needs Progressive Disclosure (UX-REQ-004) as a counterbalance. Evidence: `[PROPOSAL]`.

**UX-REQ-003 — Human authority is visible and actionable, not just logged**
Description: Anywhere an Agent action is pending human approval (AGT-REQ-002) or has occurred, the UI MUST make the human-approval state visibly distinct from agent-only activity, and MUST provide an in-context approve/reject control, not only a downstream audit-log entry.
Rationale: Direct requirement from the original spec's "Human Authority" principle; distinguishes this from SEC-REQ-003/AGT-REQ-002, which are the backend mechanics — this requirement is specifically that the mechanic is surfaced usably, not buried.
Priority: P1. Source: original spec (Human Authority principle), AGT-REQ-002. Acceptance Criteria: a pending-approval agent Action is visually distinguishable from a completed/human action in a usability test, and approval is completable from the same view without navigating to a separate audit screen. Dependencies: AGT-REQ-002, SEC-REQ-003. Risks: none material. Evidence: `[PROPOSAL]`.

**UX-REQ-004 — Progressive disclosure of graph complexity**
Description: Default views MUST show a bounded, prioritized subset of connected graph information with an explicit, discoverable mechanism to reveal more (not an unbounded dump of every Edge/Event by default).
Rationale: Direct requirement from the original spec's "Progressive Disclosure" principle; also the direct counterbalance to UX-REQ-002's risk of information overload.
Priority: P1. Source: original spec (Progressive Disclosure principle). Acceptance Criteria: a Node with more than a configured threshold of connected Edges renders a bounded default set plus an explicit "show more" control in a usability test, rather than all Edges at once. Dependencies: UX-REQ-002. Risks: the "right" default bound is a design/tuning question for Phase 12, not settled here. Evidence: `[PROPOSAL]`.

---

## 10. OPS-REQ — Local-First Deployment

Single-machine/air-gapped/LAN capable, offline Git operation independent of AI availability.

**OPS-REQ-001 — Single-machine, fully offline deployable**
Description: The platform MUST be deployable as a single-machine (or single small cluster) installation with no required outbound network dependency for core Git and graph functionality, suitable for air-gapped operation.
Rationale: Direct requirement from the original spec's local-first, self-hostable principle and Phase 5's positioning against GitLab's operationally heavy self-hosting story (O4).
Priority: P0. Source: [PROPOSAL], original spec local-first principle, O4 (contrast). Acceptance Criteria: a fresh install with all outbound network access blocked at the firewall completes setup and serves GIT-REQ-001/GRF-REQ-008 functionality successfully. Dependencies: GIT-REQ-010. Risks: this is exactly the axis Phase 6 flagged GitLab as struggling with (O4, Gitaly/Praefect/Sidekiq/Workhorse layering) — the platform must not recreate that operational weight; this is a Phase 10 architecture risk to actively design against, not merely test for after the fact. Evidence: `[INFERENCE]` per O4 (competitive contrast); requirement `[PROPOSAL]`.

**OPS-REQ-002 — AI subsystem independently disableable**
Description: The platform MUST support running with the AI Gateway and Agent runtime entirely disabled (not merely unreachable) as a supported deployment mode, with all Git/graph functionality (GIT-REQ, GRF-REQ) remaining fully available.
Rationale: The stronger, deployment-topology form of GIT-REQ-010 — "AI Down ≠ Git Down" as a supported *configuration*, not only a resilience property under failure.
Priority: P0. Source: original spec "AI Down ≠ Git Down" principle. Acceptance Criteria: a deployment configured with `ai.enabled=false` (or equivalent) starts successfully and passes the full GIT-REQ/GRF-REQ acceptance suite with zero AI-related processes running. Dependencies: GIT-REQ-010, AI-REQ-001 (as the thing being disabled). Risks: none material if GIT-REQ-010's isolation requirement is met at the code-path level first. Evidence: `[PROPOSAL]`.

**OPS-REQ-003 — LAN-scoped multi-user operation without cloud dependency**
Description: A single-machine/on-prem deployment MUST support multiple concurrent users over a local network (LAN) with full RBAC/ABAC (SEC-REQ-001) and no cloud service dependency for authentication or authorization.
Rationale: Distinguishes "local-first" from "single-user only" — a realistic self-hosted deployment is a team, not one person, per the original spec's self-hostable framing.
Priority: P0. Source: [PROPOSAL]. Acceptance Criteria: two distinct local user accounts, authenticated against a local identity store, can concurrently perform independent Policy-gated operations correctly attributed per-user. Dependencies: SEC-REQ-001, OPS-REQ-001. Risks: local identity/auth implementation (LDAP/local DB/etc.) is a Phase 10 concern; this requirement fixes the capability, not the mechanism. Evidence: `[PROPOSAL]`.

**OPS-REQ-004 — Backup and restore of the full graph and Git object store**
Description: The platform MUST provide a documented, tested backup procedure covering Git object data, the Node/Edge/Event/Policy graph store, and secrets store (SEC-REQ-005) as a consistent point-in-time set, and a restore procedure that verifiably reconstructs a working instance.
Rationale: Self-hosted operators bear their own disaster-recovery responsibility (unlike a SaaS-only competitor) — this is a precondition for anyone trusting the platform with production engineering data.
Priority: P0. Source: [PROPOSAL]. Acceptance Criteria: a backup taken from a running instance restores to a new instance that passes a smoke test confirming Git history, graph queries, and Policy state all match pre-backup state. Dependencies: OPS-REQ-001, GRF-REQ-003, SEC-REQ-005. Risks: consistency across the Git store and graph store during backup (they must not be backed up at different logical timestamps) is a real correctness risk — a Phase 10 architecture concern. Evidence: `[PROPOSAL]`.

**OPS-REQ-005 — Resource footprint documented and bounded for small deployments**
Description: The platform MUST document minimum viable hardware/resource requirements for a small-team (defined size, e.g., ≤20 users) single-machine deployment, and MUST NOT require a multi-service orchestration layer (e.g., Kubernetes) as a hard prerequisite for that scale.
Rationale: Direct contrast with O4's GitLab finding (self-hosting operational complexity from bolted-on services solving Git's own scaling weaknesses) — this requirement exists specifically so the platform doesn't repeat that pattern for small deployments.
Priority: P1. Source: phase6-primitives O4. Acceptance Criteria: a documented small-team deployment procedure completes on a single VM/machine without requiring a container orchestrator, verified by following the documented steps in a clean environment. Dependencies: OPS-REQ-001. Risks: this requirement doesn't forbid Kubernetes for *large* deployments (CLOUD-REQ handles that scale) — scope is explicitly small-deployment only. Evidence: `[INFERENCE]` per O4; requirement `[PROPOSAL]`.

---

## 11. CLOUD-REQ — Cloud-Readiness

Must be satisfiable by the same core architecture as local, no local/cloud fork; migration path stated as a requirement, not assumed.

**CLOUD-REQ-001 — Same core architecture across local and cloud deployment**
Description: Cloud/managed deployment MUST run the same core Git/graph/Policy engine codebase as the local/self-hosted deployment (differing in scale-out topology and managed-service integrations only), not a structurally forked "cloud edition."
Rationale: Direct requirement from the original spec's "local-first, cloud-ready" principle; prevents the platform from developing feature/behavior drift between editions, a common source of migration pain in dual-edition products.
Priority: P0. Source: [PROPOSAL], original spec local-first/cloud-ready principle. Acceptance Criteria: the same GIT-REQ/GRF-REQ acceptance test suite passes unmodified against both a local single-machine deployment and a cloud-hosted deployment of the platform. Dependencies: OPS-REQ-001. Risks: the temptation to build cloud-only convenience features that quietly become required is the main risk here — any cloud-only feature must be additive, not a dependency for core Git/graph function (ties to GIT-REQ-010/OPS-REQ-002). Evidence: `[PROPOSAL]`.

**CLOUD-REQ-002 — Documented, tested local-to-cloud migration path**
Description: The platform MUST provide a documented, tested procedure to migrate a local/self-hosted instance's full state (Git objects, graph, Policies, secrets references) to a cloud-hosted instance of the same platform, and vice versa.
Rationale: Direct requirement, stated explicitly rather than assumed, per this task's instruction — a "cloud-ready" claim without a real, tested migration path is aspirational, not a requirement; this is also the natural counterpart to OPS-REQ-004's backup/restore requirement, applied across deployment topology rather than across time.
Priority: P1. Source: [PROPOSAL]. Acceptance Criteria: a migration run against a representative test instance results in a cloud instance passing the same smoke test defined in OPS-REQ-004, with before/after state comparison confirming no data loss. Dependencies: OPS-REQ-004, CLOUD-REQ-001. Risks: secrets migration specifically needs a re-provisioning strategy rather than raw secret transfer, for security hygiene — a Phase 10 design detail, flagged here so it isn't missed. Evidence: `[PROPOSAL]`.

**CLOUD-REQ-003 — Horizontal scale-out without primitive-model changes**
Description: Cloud deployment MUST be able to scale Git-serving and graph-query capacity horizontally (multiple stateless service instances against a shared/replicated store) without requiring changes to the Node/Edge/Event/Policy/View primitive model or its APIs.
Rationale: Ensures Phase 6's primitive model is genuinely deployment-topology-agnostic, not implicitly single-machine-shaped; directly informed by Phase 6 §7's open architecture question about single-store vs. graph-plus-event-log-split storage, whichever way that's resolved.
Priority: P1. Source: phase6-primitives §7 (storage architecture open question). Acceptance Criteria: a load test against a multi-instance cloud deployment shows linear-ish throughput scaling for read-heavy graph queries (GRF-REQ-008) without API-level behavior differences from the single-machine deployment. Dependencies: CLOUD-REQ-001, GRF-REQ-008. Risks: this requirement is explicitly dependent on Phase 6's still-open storage-architecture decision (§7) — acceptance criteria may need revision once that decision lands in Phase 10. Evidence: `[TBD]` dependent on phase6-primitives §7 open question.

**CLOUD-REQ-004 — Multi-tenant isolation for managed/cloud offering**
Description: A cloud-hosted managed deployment MUST support strict tenant isolation (data, Policy, secrets, and agent execution) between organizations sharing the same platform instance, verified equivalent in strength to separate single-tenant deployments.
Rationale: Necessary for any realistic managed/SaaS-style offering; distinct from OPS-REQ-003's LAN multi-user requirement, which assumes a single trusted organization.
Priority: P2. Source: [PROPOSAL]. Acceptance Criteria: a cross-tenant access attempt (e.g., Tenant A's Agent attempting to query Tenant B's graph) fails identically to how it would against a wholly separate deployment, verified by a negative test. Dependencies: SEC-REQ-001, SEC-REQ-005, CLOUD-REQ-001. Risks: multi-tenancy is a significant architecture commitment (shared-schema-with-row-isolation vs. fully separate stores per tenant) best deferred to Phase 10/V2 unless a managed offering is confirmed in-scope for an earlier release; flagged as P2 for that reason, not because tenant isolation itself is low-value. Evidence: `[PROPOSAL]`.

---

## 12. Explicit Non-Requirements Check

Per domain, at least one thing deliberately NOT written as a requirement here, and why — early scope control, not deferral by oversight.

- **GIT**: No requirement for a proprietary alternative version-control backend (e.g., non-Git DVCS support). Rationale: contradicts the "Git-native" identity itself; out of scope by definition, not merely lower priority.
- **GRF**: No requirement specifying the graph storage engine (property-graph DB vendor, event-sourcing framework choice). Rationale: Phase 6 §7 explicitly deferred this to Phase 10 (Architecture Design) as a "solved-elsewhere engineering problem," not a product requirement; writing one here would be presuming an unresolved decision.
- **AGT**: No requirement mandating a specific agent orchestration framework (e.g., a particular multi-agent framework) as the reference implementation. Rationale: AGT-REQ-001's vendor-neutrality requirement would be undermined by simultaneously hard-requiring one framework; framework choice for the platform's own reference agent is a Phase 10 implementation decision.
- **AI**: No requirement for the platform to fine-tune or host its own foundation model. Rationale: out of scope — the AI Gateway's job is routing/observability across existing providers (AI-REQ-001), not model development; conflating the two would balloon scope far past what Phase 5's evidence supports as differentiating.
- **CTX**: No requirement specifying a particular embedding model or vector index technology for any semantic-search component of context assembly. Rationale: CTX-REQ-001 requires bounded, ranked retrieval but is deliberately silent on *how* ranking is computed — that's an implementation/Phase 10 concern, and over-specifying it here risks locking in a specific RAG approach that Phase 6 was cautious about generalizing from Sourcegraph's single data point (O6, `[INFERENCE]`).
- **SEC**: No requirement for a specific compliance certification (SOC 2, ISO 27001, FedRAMP) at this phase. Rationale: certification is a business/go-to-market decision with cost and timeline implications far outside requirements elicitation; the underlying technical controls this document does require (audit trail, RBAC/ABAC, secrets isolation) are necessary-but-not-sufficient inputs to any future certification effort, tracked separately.
- **CI**: No requirement for a proprietary CI configuration language replacing existing YAML-based pipeline formats. Rationale: explicitly rejected in CI-REQ-003's rationale — the goal is graph-native *linkage*, not format replacement, to avoid an unnecessary ecosystem-compatibility break mirroring the GIT-REQ non-goal above.
- **UX**: No requirement specifying visual design system, color palette, or component library. Rationale: explicitly out of scope for Phase 7 (and Phase 8) — that is Phase 12's (Stakeholder Validation / detailed UX) job; this document intentionally stops at high-level, testable interaction principles.
- **OPS**: No requirement for a specific containerization/orchestration technology (Docker, Kubernetes, etc.) as the mandated packaging format. Rationale: OPS-REQ-005 explicitly requires that small deployments NOT need an orchestrator, and CLOUD-REQ-003 requires scale-out capability, but the specific packaging technology satisfying both is a Phase 10 implementation choice, not a product requirement.
- **CLOUD**: No requirement for supporting more than one specific cloud provider (AWS/GCP/Azure) by name at this phase. Rationale: CLOUD-REQ-001's "same core architecture" requirement is provider-agnostic by construction; naming specific providers is a go-to-market/Phase 9 rollout decision, not a Phase 7 requirement.

---

## 13. Traceability Stub

Seeds Phase 8's full Traceability Matrix — a representative sample per domain, not exhaustive.

| Requirement ID | Primary Phase 1-5 Evidence / Phase 6 Primitive |
|---|---|
| GIT-REQ-010 | Original spec "AI Down ≠ Git Down" principle |
| GIT-REQ-004 | [UNVERIFIED-FACT] industry-standard LFS support (not independently re-verified this phase) |
| GRF-REQ-003 | phase1-5 O2 (GitHub audit log outage); phase6-primitives §3.3, §5 |
| GRF-REQ-004 | phase1-5 O8/O9; phase6-primitives §3.6, §5 |
| GRF-REQ-005 | phase6-primitives §3.10, §5 (Context-into-View verdict) |
| AGT-REQ-002 | Original spec Human Authority principle; phase6-primitives O8/O9 |
| AGT-REQ-007 | phase6-primitives §3.8, §5 (Agent as Node subtype verdict) |
| AI-REQ-003 | Original spec local-first principle; Phase 5 local/cloud parity differentiator |
| CTX-REQ-001 | phase1-5 O6 (Sourcegraph RAG reconstruction) |
| CTX-REQ-005 | phase1-5 O11 (AGENTS.md convention) |
| SEC-REQ-003 | phase1-5 O2 (audit log as bolt-on failure) |
| SEC-REQ-002 | Original spec temporary scoped identity example |
| CI-REQ-001 | phase6-primitives §4.4, §6 (Release worked example) |
| CI-REQ-006 | phase1-5 O13 (Argo CD/Workflows precedent, `[TBD]`) |
| UX-REQ-001 | Original spec Intent > Navigation principle |
| OPS-REQ-001 | phase6-primitives O4 (GitLab operational-weight contrast) |
| CLOUD-REQ-003 | phase6-primitives §7 (storage architecture open question) |

---

## 14. Open Questions

**Research Needed**
- `[TBD]` Git LFS support parity across all surveyed competitors was asserted `[UNVERIFIED-FACT]` (GIT-REQ-004) rather than independently re-verified per Phase 2's evidence-collection bar; a lightweight confirmation pass before Phase 8 locks acceptance criteria would strengthen this requirement's Source tag.
- `[TBD]` Argo CD/Workflows' rollback state machine remains unresearched (carried from Phase 6 O13) and directly affects CI-REQ-006's acceptance criteria for rollback behavior — flagged again here since it is now blocking a concrete requirement, not just a worked example.
- `[TBD]` What compliance/regulatory retention-erasure requirements (e.g., GDPR right-to-erasure) actually demand of an append-only Event log (GRF-REQ-003) needs dedicated legal/compliance research before Phase 8 can finalize the "narrow, audited exception path" mentioned in GRF-REQ-003's Risks — this document does not attempt that research.

**Product Decision**
- `[TBD]` Whether a managed/multi-tenant cloud offering (CLOUD-REQ-004) is actually in scope for an early release, or purely a later-phase possibility, determines whether CLOUD-REQ-004 should be P1 instead of P2 — not resolvable from Phase 1-6 evidence alone; needs an explicit product decision, ideally surfaced at Phase 9 (MVP Definition).
- `[TBD]` Default resource-limit values for agent runs (AGT-REQ-006: token budgets, wall-clock limits, concurrency caps) are deliberately left unspecified here — too strict frustrates legitimate use, too loose reintroduces the cost-runaway risk the requirement exists to prevent. This is a tuning decision needing either usage data or an explicit risk-tolerance call from product leadership.
- `[TBD]` Which specific Agent Actions require human approval by default (AGT-REQ-002's "specified class of agent Action") is left as configurable rather than fixed; an explicit default policy is a product decision with real adoption-friction-vs-safety tradeoffs, not resolvable by this phase's method.

**Architecture Decision**
- `[TBD]` CTX-REQ-002's exact-reconstruction guarantee for View invocations depends on how graph state versioning/snapshotting is architected (carried directly from Phase 6 §7's Node/Edge/Event storage-architecture question) — this document states the requirement's *intent* (auditable context) without presuming the storage answer, consistent with Phase 6's own agnosticism, but Phase 10 must resolve it before CTX-REQ-002's acceptance criteria can be made fully precise.
- `[TBD]` CLOUD-REQ-003's horizontal scale-out requirement is written assuming *some* resolution to Phase 6 §7's single-store-vs-split-store question exists that supports it — if Phase 10 instead concludes a single non-shardable store is the right MVP tradeoff (deliberately accepting a lower cloud-scale ceiling short-term), CLOUD-REQ-003's priority/timing should be revisited, not silently kept at P1.
- `[TBD]` GRF-REQ-004's requirement that Policy be evaluable in blocking mode without a round-trip to a central service (for air-gapped/self-hosted operation, per Phase 6 §7's carried-forward open question) has real implications for how Policy definitions are distributed/cached versus centrally stored — unresolved here, flagged for Phase 10.

---

## Next Phases

- **Phase 6 — Product Primitive Discovery**: complete.
- **Phase 7 — Requirements Elicitation**: this document.
- **Phase 8 — Requirements Specification (full 55-section document)**: pending — assembles/polishes/cross-references this draft, resolves ID renumbering if any, produces the full Traceability Matrix.
- **Phase 9 — MVP Definition**: pending — resolves Priority-vs-rollout-timing questions flagged above.
- **Phase 10 — Architecture Design**: pending — resolves the several architecture-decision `[TBD]`s this phase surfaced.
- **Phase 11 — Red-Team Review**: pending.
- **Phase 12 — Stakeholder Validation**: pending — detailed UX design building on §9's high-level principles.
- **Phase 13 — Final Baseline**: pending.
