# Phase 14 — IPA (Japan) Standards Compliance Review

**Status:** Supplementary compliance pass on top of **Baseline v1.0** (Phases 1–13, see `phase13-final-baseline.md`). This is **not** a re-run of Phase 1–13 and does **not** re-litigate Phase 11 (red-team) or Phase 12 (UX) findings — it applies a distinct lens the original 13-phase program never applied: Japan's **IPA (独立行政法人情報処理推進機構 / Information-technology Promotion Agency, Japan)** published standards and guidance for requirements engineering.

## 0. Scope, Method, and an Honest Limitation

**Scope.** Three IPA-published (or IPA/SEC-published) frameworks, per the requester's explicit choice of "all of them":

1. **非機能要求グレード (Non-Functional Requirements Grade)** — IPA/SEC's six-category grading system for non-functional requirements.
2. **要求仕様定義ガイド / 上流工程共通フレーム (Upstream Process Common Frame)** — process guidance for the requirements-definition phase itself (traceability, stakeholder consensus, change management).
3. **情報セキュリティ関連ガイド (Information Security related guidance)** — IPA's security-requirement authoring norms.

**Method.** Live web search (WebSearch) against Japanese-language queries for each framework. **`www.ipa.go.jp` itself was unreachable from this research environment** — direct fetches to `ipa.go.jp` were blocked by the environment's network egress policy (`EGRESS_BLOCKED`). This means every claim below about what IPA's primary documents actually say is sourced from **secondary/aggregator material** (blog posts, consulting-firm summaries, Qiita/Zenn technical-blog write-ups, e-words.jp glossary entries) that themselves cite or paraphrase the IPA originals, not from this session directly reading the IPA PDF/HTML documents. Per this program's own established discipline (see `phase1-5-research.md`'s self-correction on treating aggregator claims as certain), **nothing below is tagged `[FACT]`.** Claims consistently corroborated across multiple independent secondary sources are tagged `[UNVERIFIED-FACT]`; single-source or inferred claims are tagged `[INFERENCE]`; anything not found at all is tagged `[TBD]`. A reader who needs primary-sourced certainty on exact IPA wording should independently retrieve `ipa.go.jp/archive/digital/iot-en-ci/jyouryuu/hikinou/ent03-b.html` ("非機能要求グレード") and IPA's 共通フレーム2013 (SEC BOOKS) PDF directly — both were located by URL in this research but could not be fetched from this environment.

**Tagging convention** (reused unchanged from Phases 1–13):
- `[FACT]` — verifiable claim, cited to a primary source, independently confirmed in this session.
- `[UNVERIFIED-FACT]` — plausible, cited, corroborated across secondary sources, but not primary-sourced by this session.
- `[INFERENCE]` — reasonable conclusion, not independently verified.
- `[PROPOSAL]` — this program's own design decision.
- `[TBD]` — unresolved; needs follow-up (here: needs someone with direct `ipa.go.jp` access, e.g. from Japan, or via a library/document-request channel, to confirm against the primary PDF).

**What this document reviews.** `00-requirements-definition.md` (all 55 sections, with emphasis on §35 SEC-REQ, §36 AISEC-REQ, §39–41 deployment, §42–45 Observability/Backup/Performance/Reliability, §46 Data Ownership, §52 Traceability, §53 Risk Register), `phase7-elicitation.md`'s SEC-REQ/OPS-REQ/CI-REQ items, and `phase13-final-baseline.md`'s amendments — as they stand at Baseline v1.0.

---

## 1. IPA Framework Summary

### 1.1 非機能要求グレード (Non-Functional Requirements Grade)

`[UNVERIFIED-FACT]` — corroborated across e-words.jp, Qbook, dddots.jp, BTNコンサルティング, and a Zenn technical write-up (dated 2026): IPA/SEC first published the 非機能要求グレード in 2010, with subsequent revisions (a widely-cited "2018" edition appears in secondary sources); it defines **six top-level categories (6大項目)**:

1. **可用性 (Availability)**
2. **性能・拡張性 (Performance / Scalability)**
3. **運用・保守性 (Operability / Maintainability)**
4. **移行性 (Migration / Portability)**
5. **セキュリティ (Security)**
6. **システム環境・エコロジー (System Environment / Ecology)**

`[UNVERIFIED-FACT]` (single Zenn source, not cross-corroborated at this level of detail): the full grade methodology decomposes these six categories into **118 中項目 (sub-items / requirement-consideration items)** and roughly **238 指標 (230 after deduplication) grading indicators**, each of which the methodology expects a project to explicitly select a **level/grade (0–5, or 0–6 depending on edition)** for, rather than leave implicit.

`[INFERENCE]` from the training-era memory of this category structure, corroborated loosely by secondary sources but **not independently re-verified against the primary PDF in this session** — treat as directional, not authoritative: within **可用性 (Availability)**, sub-items commonly cited include 継続性 (continuity — planned/unplanned downtime tolerance), 耐障害性 (fault tolerance — redundancy design), 災害対策 (disaster recovery — RTO/RPO, backup site). Within **セキュリティ (Security)**, commonly cited sub-item groupings include 前提条件・制約条件 (preconditions/constraints), アクセス・利用制限 (access/usage restriction), データの秘匿 (data confidentiality), 不正・破壊からの回避 (avoidance of fraud/destruction), ネットワーク対策 (network countermeasures), and セキュリティリスク管理 (security risk management — ongoing vulnerability/incident management, not just point-in-time controls). **This decomposition is `[TBD]` for primary-source confirmation** — this session could not fetch the IPA document that would confirm the exact Japanese sub-item labels and their assignment to categories; the labels above should be treated as plausible-but-unconfirmed until checked against the primary PDF.

**The methodology's central discipline** (the point most load-bearing for this gap analysis, `[UNVERIFIED-FACT]` across all secondary sources reviewed): for each sub-item, the Grade format forces the requirements author to record an **explicit chosen level**, not merely "yes we have some requirement touching this area." A non-functional requirements document that only qualitatively gestures at an area ("the system should be highly available") without committing to a level (e.g., "Availability Level 2 of 5: planned maintenance windows permitted, no automatic failover required") does not satisfy the Grade methodology's intent, even if a requirement with that ID technically exists.

### 1.2 要求仕様定義ガイド / 上流工程共通フレーム (共通フレーム / Common Frame)

`[UNVERIFIED-FACT]` — corroborated across the IPA SEC BOOKS publish page, Wikipedia (ja), and a CIT-consulting summary: IPA/SEC published **共通フレーム2013** (Common Frame 2013, a revision of earlier 共通フレーム editions going back to 1994/98/2007), a comprehensive process framework covering the full system/software lifecycle from conception through disposal, with the explicit stated goal of giving all lifecycle participants (acquirer, supplier, operator) "共通の言葉" (a common vocabulary) for describing process activities and roles.

`[UNVERIFIED-FACT]` (from the same search, quoting a fragment of the primary PDF's table of contents visible in the search snippet): the framework's process structure includes **合意プロセス (Agreement Processes)** — with **取得プロセス (Acquisition Process)** and **供給プロセス (Supply Process)** sub-processes — governing how a requirements definition is formally agreed between acquiring and supplying parties.

`[UNVERIFIED-FACT]`: the framework's guidance on **変更管理 (change management)** during a project explicitly calls for pre-established rules — an approval route (承認ルート), cost-responsibility assignment, and impact-analysis method — decided *before* changes occur, specifically to prevent disputes; and, per the "共通フレーム2013とユーザのための要件定義ガイド" companion material referenced in search results, requirements definition explicitly carries **説明責任 (accountability)** for the acquiring organization, framed as a deliverable the business side, not just IT, is answerable for.

`[TBD]`: the precise wording of traceability requirements (要求のトレーサビリティ) within 共通フレーム2013's requirements-definition process activity, and the precise definition of what constitutes adequate stakeholder sign-off/合意 (consensus) evidence, could not be retrieved from a primary source in this session — the PDF (`ipa.go.jp/publish/qv6pgp000000107j-att/000062659.pdf`) was located by URL but not fetchable.

**What can be said with reasonable confidence** (`[INFERENCE]`, converging across the corroborated fragments above): 共通フレーム's process view of "upstream process" (上流工程) treats requirements definition as a **process with an explicit accountable owner, a formal agreement step between stakeholders, and a change-control discipline defined in advance** — not merely as a document-writing exercise. This is the dimension most relevant to Step 2's process gap-map below, regardless of exact wording.

### 1.3 情報セキュリティ関連ガイド (Information Security related guidance)

`[TBD]` — this sub-lens proved the hardest to pin to one named, citable IPA artifact in this session's search results, and no dedicated search pass distinct from the general 非機能要求グレード queries above was run given time constraints (a genuine research gap, flagged rather than papered over). What is used for the §3.3 gap-map below is the セキュリティ sub-item content already surfaced under §1.1 above (ネットワーク対策, セキュリティリスク管理, データの秘匿, 不正・破壊からの回避), since the 非機能要求グレード's own セキュリティ category **is** IPA's most concrete, checklist-form security-requirement authoring guidance according to the secondary sources reviewed — rather than treating "IPA security guidance" as a wholly separate, independently-sourced third body of material this session could not actually locate. Readers requiring the narrower IPA security-benchmark/secure-by-design publications (情報セキュリティ対策ベンチマーク, secure-by-design principle documents) specifically should treat that material as **entirely unresearched (`[TBD]`)** by this phase — a distinct follow-up task, not something folded into the findings below by assumption.

---

## 2. Gap Map 1 — 非機能要求グレード (Six Categories)

Legend: ✅ covered with explicit level/tier; ⚠️ requirement ID(s) exist but no explicit level is committed (the Grade methodology's central discipline, §1.1, is not satisfied); ❌ no requirement ID currently addresses this sub-area at all.

| Category | Existing requirement IDs | Sub-items per §1.1 (unverified decomposition) | Assessment |
|---|---|---|---|
| **① 可用性 (Availability)** | REL-REQ-001/002/003, OPS-REQ-004, BKP-REQ-001/002 | 継続性 (planned/unplanned downtime tolerance); 耐障害性 (redundancy); 災害対策 (RTO/RPO, DR site) | ⚠️ REL-REQ-001 requires availability but its "specific availability target" is explicitly `[TBD] – Benchmark/SLA Required` (§45); BKP-REQ-002 requires RTO/RPO be *documented* but the figures themselves are `[TBD]`. No requirement anywhere commits to an explicit **level** (e.g., "Level 2 of 5") the way the Grade methodology expects — every relevant ID defers to a future benchmark rather than stating a provisional target now. |
| **② 性能・拡張性 (Performance/Scalability)** | §44 Performance Requirements table (all rows), CLOUD-REQ-003 | Response time, throughput, concurrent-user ceiling, scale-out headroom | ⚠️ §44 is an intentionally honest all-`[TBD]` table by design (this program's own no-fabricated-numbers discipline). Correct research discipline, but it means the Grade's "commit to a level" expectation is currently **fully unmet** — there is no MVP-scoped provisional tier stated anywhere, only a promise to benchmark later. |
| **③ 運用・保守性 (Operability/Maintainability)** | OBS-REQ-001/002/003, OPS-REQ-005, GIT-REQ-011 | Monitoring, log retention, patch/maintenance windows, operational documentation | ⚠️ OBS-REQ set covers monitoring existence but not an explicit *maintainability level* (e.g., planned-maintenance-window frequency/duration, mean-time-to-patch commitment). GIT-REQ-011 (repository GC/maintenance, V1) exists but has no explicit operating-window commitment. |
| **④ 移行性 (Migration/Portability)** | DATA-REQ-001–005, GIT-REQ-009 | Data export completeness, format openness, migration-tooling verification | ✅ This is the program's **strongest** category against the Grade — DATA-REQ-001 has a concrete acceptance criterion (structural-diff-verified round-trip export/import) and GIT-REQ-009 similarly. No new requirement needed here; this program already independently arrived at Grade-comparable concreteness for §14 reasons unrelated to IPA (Phase 5's "no vendor lock-in" differentiator). |
| **⑤ セキュリティ (Security)** | SEC-REQ-001–007, AISEC-REQ-001–009 | 前提条件・制約条件, アクセス・利用制限, データの秘匿, 不正・破壊からの回避, ネットワーク対策, セキュリティリスク管理 | ⚠️/❌ See §4 (dedicated security gap-map) below — アクセス・利用制限/データの秘匿/不正・破壊からの回避 are well covered (SEC-REQ-001/005, AISEC-REQ-004/006); **ネットワーク対策 (network-layer controls: transport encryption, network segmentation) and セキュリティリスク管理 (ongoing vulnerability/patch management as a recurring process, not a point-in-time control) have no dedicated requirement ID anywhere in the master document.** |
| **⑥ システム環境・エコロジー (System Environment/Ecology)** | OPS-REQ-001/003/005 (partial) | Supported OS/platform matrix, physical/environmental constraints, power/resource consumption, disposal/decommissioning | ❌ This is the **weakest** category. OPS-REQ-005 documents "resource footprint... for small deployments" but nothing states a supported-platform matrix (which OS/architectures/container runtimes are supported), nothing addresses decommissioning/data-disposal requirements, and nothing addresses resource-consumption/ecology framing at all. This is the one category with essentially zero prior coverage even at the "requirement ID exists but vague" level. |

### Findings — 非機能要求グレード

**F14-1 [High] — No MVP-scoped provisional Availability level is stated (REL-REQ-001).**
REL-REQ-001 requires Git-serving availability be maintained but its target is wholly `[TBD]`. The Grade methodology's discipline is to state a *provisional* level even before full benchmarking (levels are meant to be negotiated early, refined later) — leaving it fully open through MVP is a process gap, not merely a data gap. Cites: REL-REQ-001, §44, §45.

**F14-2 [Medium] — No explicit Recovery Time/Point tier is stated (BKP-REQ-002).**
BKP-REQ-002 requires RTO/RPO be documented but states no provisional figures, even a conservative MVP-appropriate one. Cites: BKP-REQ-002, OPS-REQ-004.

**F14-3 [Medium] — セキュリティ category: ネットワーク対策 (network-layer security controls) has no dedicated requirement.**
Nothing in SEC-REQ or AISEC-REQ names transport-layer encryption (TLS everywhere, including internal service-to-service traffic), network segmentation between the Platform process, agent/CI sandboxes, and any externally-reachable surface, or ingress/egress filtering as an explicit requirement. AGT-REQ-005/AISEC-REQ-008 establish workspace *process* isolation but not *network* isolation specifically. Cites: SEC-REQ-001–007, AISEC-REQ-001–009 (absence).

**F14-4 [Medium] — セキュリティ category: セキュリティリスク管理 (ongoing vulnerability/patch management) has no dedicated requirement.**
The existing security requirements are all point-in-time behavioral controls (access control, audit, secret handling). None require an ongoing process for dependency-vulnerability scanning, patch-cadence commitments, or periodic security review — which is exactly the recurring-process framing IPA's セキュリティリスク管理 sub-item targets, and which AISEC-REQ-009(b)'s "internal privilege-separation review" gestures at only for one narrow slice (Platform-process privilege separation), not as a general ongoing-risk-management commitment. Cites: SEC-REQ-001–007, AISEC-REQ-009.

**F14-5 [Low] — システム環境・エコロジー category has essentially no coverage.**
No requirement states a supported OS/platform/architecture matrix, decommissioning/data-disposal handling, or resource-consumption/ecology framing. Given this program's MVP is a small-team self-hosted product (not an enterprise-scale/regulated-environment target where ecology/disposal compliance is typically load-bearing), this is assessed Low severity rather than High — but it is a genuine, total gap. Cites: OPS-REQ-001/003/005 (nearest-adjacent, none actually cover this).

**F14-6 [Low] — 運用・保守性 category: no explicit maintainability level (planned-maintenance-window commitment).**
OBS-REQ and GIT-REQ-011 address monitoring and GC mechanics but not an explicit maintenance-window or patch-latency commitment. Cites: OBS-REQ-001–003, GIT-REQ-011.

---

## 3. Gap Map 2 — 要求仕様定義ガイド / 上流工程共通フレーム (Process)

This program's own documented process — 13 phases, a `[FACT]`/`[UNVERIFIED-FACT]`/`[PROPOSAL]`/`[TBD]` tagging discipline throughout, a traceability stub at §52, and a Phase 11/12/13 review-and-disposition cycle — is checked against 共通フレーム's process expectations (§1.2 above): an accountable owner for the requirements-definition process, a formal agreement/consensus step between stakeholders, and a pre-established change-control discipline.

**Traceability (§52).** §52 is explicitly a "representative sample... not exhaustive," acknowledged as incomplete by the document itself. This partially satisfies 共通フレーム's traceability expectation in spirit (the mechanism and intent exist) but not in coverage (it is not exhaustive across all 150+ requirement IDs in the master document).

**Change management.** The Phase 9→10→11→12→13 progression, and specifically Phase 13's disposition log (Accepted-and-fixed / Accepted-deferred / Rejected / Needs-human-decision), **is** a genuine, documented change-control discipline with a pre-established rule format — this is a real strength of the program relative to 共通フレーム's expectations, arguably comparable in spirit even though it was not designed with 共通フレーム in mind.

**Stakeholder agreement/consensus (合意プロセス).** This is the program's clearest process gap. Every phase from 1 through 13, including the red-team (11) and UX (12) reviews, is conducted by the same research/authoring process (Claude-driven phases) with no point at which a **human stakeholder** (product owner, acquiring-organization representative, or the equivalent of 共通フレーム's 取得プロセス party) formally reviews and signs off on the requirements baseline. Phase 12 is titled "Stakeholder Validation" in the roadmap note at the end of §54's historical text, but its actual content (confirmed by reading §50 and the Phase 12 document) validates personas/journeys **as design artifacts**, not via an actual human stakeholder's sign-off. 共通フレーム's 合意プロセス framing — an explicit agreement step between the party defining requirements and the party who will be accountable for/bound by them — has no equivalent anywhere in this program.

### Findings — Process

**F14-7 [High] — No human stakeholder sign-off step exists anywhere in the 13-phase program.**
This is a real gap against 共通フレーム's 合意プロセス (Agreement Process) expectation. Every one of Phases 1–13 is an AI-research-and-authoring process; "Baseline v1.0" status was reached without an explicit point where a human with product/business accountability reviewed and formally accepted the requirements set (as distinct from a human merely commissioning/reading the work). Cites: front matter of `00-requirements-definition.md`, `phase12-ux-review.md`, `phase13-final-baseline.md` (no sign-off step present in any).

**F14-8 [Medium] — §52 Traceability Matrix is explicitly non-exhaustive.**
By the document's own admission ("representative sample... not exhaustive"), full requirement-by-requirement traceability to source evidence does not yet exist, which falls short of 共通フレーム's traceability expectation even though the mechanism/intent is present. Cites: §52.

---

## 4. Gap Map 3 — セキュリティ (AISEC-REQ / SEC-REQ vs. IPA Security Sub-Items)

Cross-checking §35 SEC-REQ and §36 AISEC-REQ against the セキュリティ category sub-items surfaced in §1.1/§1.3 (前提条件・制約条件, アクセス・利用制限, データの秘匿, 不正・破壊からの回避, ネットワーク対策, セキュリティリスク管理):

| IPA sub-item (unverified decomposition) | Existing coverage | Gap |
|---|---|---|
| 前提条件・制約条件 (preconditions/constraints) | Implicit throughout (deployment-mode framing, OPS-REQ) | None significant — this sub-item is typically a documentation/scoping exercise this program already does structurally via its Scope/Non-Goals sections (§14–15). |
| アクセス・利用制限 (access/usage restriction) | SEC-REQ-001, SEC-REQ-004, SEC-REQ-002 | Well covered. |
| データの秘匿 (data confidentiality) | SEC-REQ-005, AISEC-REQ-004 | Well covered for secrets specifically; general at-rest/in-transit encryption of the graph/Git stores themselves is not explicitly named anywhere — see F14-9. |
| 不正・破壊からの回避 (avoidance of fraud/destruction) | SEC-REQ-003, AISEC-REQ-006, AISEC-REQ-009 | Well covered, this is a program strength (structural audit trail, tamper-evident event log). |
| ネットワーク対策 (network countermeasures) | None | Same as F14-3 above. |
| セキュリティリスク管理 (security risk management) | AISEC-REQ-009(b) partial | Same as F14-4 above. |

**F14-9 [Medium] — No explicit at-rest/in-transit encryption requirement for the graph and Git object stores.**
SEC-REQ-005/AISEC-REQ-004 cover *secret values* specifically, but no requirement states that the graph database and Git object store themselves must support encryption at rest, or that all network transport (including internal Platform-process-to-database, and any LAN-scoped multi-user traffic per OPS-REQ-003) must be encrypted in transit. This is adjacent to but distinct from F14-3's broader network-controls gap. Cites: SEC-REQ-005, OPS-REQ-003 (absence).

---

## 5. Dispositions

| Finding | Severity | Disposition | Reasoning |
|---|---|---|---|
| F14-1 (no Availability level) | High | **Accepted-and-fixed** | Cheap to add a provisional MVP-scoped level now; §6 adds NFR-REQ-001. |
| F14-2 (no RTO/RPO tier) | Medium | **Accepted-and-fixed** | Folded into NFR-REQ-001 as a companion tier statement, amending BKP-REQ-002. |
| F14-3 (no network-controls requirement) | Medium | **Accepted-and-fixed** | Adds SEC-REQ-008. |
| F14-4 (no security-risk-management process requirement) | Medium | **Accepted-and-fixed** | Adds SEC-REQ-009. |
| F14-5 (システム環境・エコロジー gap) | Low | **Accepted-deferred (V1)** | Genuinely low urgency for an MVP small-team self-hosted product; adds NFR-REQ-003 timed V1 rather than MVP, to avoid scope creep into the MVP definition Phase 9 already carefully bounded. |
| F14-6 (no maintainability window commitment) | Low | **Accepted-deferred (V1)** | Folded into NFR-REQ-002, timed V1 alongside the rest of §44's benchmark-dependent items — inventing a number now would violate this program's own no-fabrication discipline; a *level choice* (not a number) is added at MVP instead (see NFR-REQ-002), with the numeric commitment itself V1. |
| F14-7 (no stakeholder sign-off step) | High | **Needs-human-decision** | This is a process gap this document cannot fix by editing a requirements doc — it requires an actual human stakeholder (the user, or whoever they designate) to perform a sign-off step. Recommendation: add a "Baseline v1.0 — Human Sign-Off" checkpoint before any future v1.1/v2.0 baseline is declared complete; recorded as a new item under §54/Phase 13's Open Questions rather than invented as a fake completed step. |
| F14-8 (traceability non-exhaustive) | Medium | **Accepted-deferred (V1)** | Genuine gap but expanding §52 to full exhaustive coverage of 150+ requirement IDs is a substantial mechanical exercise better done once, deliberately, alongside a V1 requirements-set stabilization pass rather than piecemeal here. |
| F14-9 (no at-rest/in-transit encryption requirement) | Medium | **Accepted-and-fixed** | Adds SEC-REQ-010. |

---

## 6. Severity Rollup

| Severity | Count | Findings |
|---|---|---|
| Critical | 0 | — |
| High | 2 | F14-1, F14-7 |
| Medium | 5 | F14-2, F14-3, F14-4, F14-8, F14-9 |
| Low | 2 | F14-5, F14-6 |
| **Total** | **9** | |

**Disposition summary:** Accepted-and-fixed: 5 (F14-1, F14-2, F14-3, F14-4, F14-9). Accepted-deferred (V1): 3 (F14-5, F14-6, F14-8). Needs-human-decision: 1 (F14-7). Rejected: 0.

---

## 7. New/Amended Requirements Added to `00-requirements-definition.md`

New prefix introduced: **`NFR-REQ`** (non-functional-requirement-grade-tier requirements — genuinely new because no existing prefix cleanly covers "an explicit Grade-methodology level choice spanning Availability/Performance/Operability," which cuts across REL-REQ/BKP-REQ/OBS-REQ/OPS-REQ rather than belonging to one). Existing prefixes `SEC-REQ` extended (008/009/010) since these are squarely security-behavioral requirements matching that prefix's existing scope.

- **NFR-REQ-001** — Availability Grade tier (§45, amending REL-REQ-001/BKP-REQ-002)
- **NFR-REQ-002** — Performance/Scalability and Operability/Maintainability Grade tier commitment (§44/§42)
- **NFR-REQ-003** — System Environment/Ecology minimum disclosure (§39, V1-timed)
- **SEC-REQ-008** — Network-layer security controls (§35)
- **SEC-REQ-009** — Ongoing security risk management process (§35)
- **SEC-REQ-010** — At-rest and in-transit encryption of graph and Git object stores (§35)

Full requirement text is in `00-requirements-definition.md` §35/§39/§44/§45 (see edits applied alongside this document). F14-7 (stakeholder sign-off) and F14-8 (traceability exhaustiveness) are recorded as open items, not new requirement IDs, per their Needs-human-decision/Accepted-deferred dispositions above — see the Phase 14 pointer added to §54.

---

*End of Phase 14. This document supplements, and does not supersede, Baseline v1.0. `00-requirements-definition.md`'s front matter and §54 now point here.*
