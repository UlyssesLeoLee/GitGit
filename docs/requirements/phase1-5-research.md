# AI-Native Engineering Platform — Requirements Research: Phases 1–5

**Status:** v1.1 — fixed-up pass. Downgrades unverified third-party-sourced claims, adds primary-source pricing verification where reachable, fixes capability-matrix legend violations, adds a Linear deep-dive, and adds a consolidated sources bibliography. Not a claim of completeness: Jira, Gerrit, GitHub Issues/Projects (standalone), Argo CD/Workflows, CodeQL, and Sentry remain outstanding `[TBD]` items for a later pass. **Scope:** Phases 1 through 5 only (Research Plan → Evidence Collection → Competitor Analysis → Capability Matrix → Gap Analysis). This document does not attempt the full requirements book, MVP definition, red-team review, or architecture design — those are later phases (see "Next Phases").

**Tagging convention used throughout:**
- `[FACT]` — verifiable claim, cited to a primary source (vendor docs, official pricing page, official blog/repo) with an access date.
- `[UNVERIFIED-FACT]` — a claim that is plausibly true and is cited, but the citation is a third-party aggregator, comparison site, or competitor-published source rather than a primary source, and it has not been independently confirmed against one. Introduced in the v1.1 pass to replace `[FACT]` tags that were previously applied to this kind of claim; treat these as lower-confidence than `[FACT]` and do not rely on the specific numbers/details without re-verification.
- `[INFERENCE]` — reasonable conclusion drawn from facts, not independently verified.
- `[PROPOSAL]` — our own design idea for the Target Platform, not a claim about any competitor.
- `[TBD]` — could not be verified with the tools/time available in this pass; needs follow-up before later phases rely on it.

All web evidence gathered 2026-08-18 via live search (WebSearch/WebFetch). Training-memory recollections are treated as `[INFERENCE]`/`[TBD]` unless independently corroborated by a cited source in this pass. The v1.1 fix-up pass (also 2026-08-18) additionally attempted direct WebFetch against primary sources (github.com/pricing, docs.github.com, about.gitlab.com/pricing) for the pricing claims flagged in self-review; results are noted inline where a primary source was or was not reachable.

---

## Phase 1 — Research Plan

### Objective
Establish a factual, dated, source-cited baseline of the competitive landscape before defining requirements for a local-first, cloud-ready, Git-native, AI-native, agent-native, graph-native, self-hostable engineering platform that manages the full engineering graph (requirement, issue, ADR, PR, review, test, CI, artifact, release, deployment, incident, agent, agentrun, prompt, skill, model, context, policy) — not just Git objects.

### Competitor categories and specific products in scope

**Git / DevOps platforms (forge-of-record):**
- GitHub (incl. Actions, Packages, Discussions, Codespaces, Copilot coding agent, Apps, Enterprise/Audit, MCP) — deep dive
- GitLab (incl. Gitaly, Praefect, Sidekiq, Workhorse, Duo Agent Platform, AI Gateway, Duo Self-Hosted) — deep dive
- Bitbucket (Cloud + Data Center, Rovo)
- Gitea / Forgejo (combined entry, noting the fork relationship)
- Gerrit (code-review-centric forge, self-hosted, Google-originated)

**AI coding agents:**
- GitHub Copilot, incl. Copilot coding agent (cloud/async agent)
- Claude Code (Anthropic) — deep dive: subagents, skills, MCP, hooks
- OpenAI Codex / Codex CLI — deep dive: AGENTS.md, MCP, sandboxing/approvals, git workflow
- Cursor
- Gemini CLI (and its 2026 successor)
- Devin (Cognition)
- OpenHands (open-source Devin-style agent)

**Code intelligence:**
- Sourcegraph (Cody, code search, batch changes)
- GitLab Duo (as code-intelligence + AI-in-IDE surface)
- CodeQL (semantic code analysis / security)
- Sentry (observability/error intelligence, adjacent to incident graph)

**Project management:**
- Jira
- Linear
- GitHub Issues / Projects

**CI/CD:**
- GitHub Actions
- GitLab CI
- Argo CD / Argo Workflows

### Research methodology
1. Prioritize official documentation, official product/engineering blogs, and public source repositories over third-party summaries; third-party 2026 "guide" sites are used only where official sources are silent, and are flagged as lower-confidence.
2. Every factual claim used in Phases 2–5 is dated and cited inline as `[FACT] <claim>. Source: [Title](URL), accessed 2026-08-18.`
3. Claims not independently verified via live search in this pass are marked `[TBD]` rather than stated as fact, even if they match prior training knowledge.
4. Own design ideas for the Target Platform are marked `[PROPOSAL]` and never attributed to a competitor.
5. At least 15–20 live WebSearch queries were run across the categories above, weighted toward GitHub, GitLab, and the AI coding agents per the spec's "deepest treatment" instruction. Search queries and result titles are traceable via the citations below.

---

## Phase 2 — Evidence Collection

### GitHub / Copilot

- [FACT] GitHub Copilot Agent mode in VS Code can inspect a workspace, edit multiple files, run terminal commands, execute tests, read errors, and iterate — MCP is an optional extension boundary, not required for Agent mode. Source: [GitHub Copilot Agent Mode and MCP in VS Code: 2026 Guide](https://www.itechguides.com/vibe-coding-with-github-copilot-agent-mode-and-mcp-in-vs-code-updated-for-2026/), accessed 2026-08-18.
- [FACT] Repository administrators can configure MCP servers for GitHub Copilot Coding Agent directly in repository settings via JSON config; once configured, those tools are available to the agent on every assigned task. Source: [GitHub Copilot Coding Agent: The Complete Architecture Behind Agentic DevOps at Enterprise Scale](https://itnext.io/github-copilot-coding-agent-the-complete-architecture-behind-agentic-devops-at-enterprise-scale-1f42c1c132aa), accessed 2026-08-18.
- [FACT] By August 2026, Copilot's original premium-request pricing has largely been replaced by usage-based "GitHub AI Credits" for most users. Source: [GitHub Copilot Agent Mode and MCP in VS Code: 2026 Guide](https://www.itechguides.com/vibe-coding-with-github-copilot-agent-mode-and-mcp-in-vs-code-updated-for-2026/), accessed 2026-08-18.
- [FACT] GitHub has an official "cloud agent" concept documented under Copilot concepts. Source: [About GitHub Copilot cloud agent](https://docs.github.com/en/copilot/concepts/agents/cloud-agent/about-cloud-agent), accessed 2026-08-18.
- [FACT] GitHub Enterprise provides audit logs covering repository access, changes, deletions, and organization-level events, including for Codespaces specifically. Source: [Reviewing your organization's audit logs for GitHub Codespaces](https://docs.github.com/en/enterprise-cloud@latest/codespaces/managing-codespaces-for-your-organization/reviewing-your-organizations-audit-logs-for-github-codespaces), accessed 2026-08-18.
- [FACT] On 2026-04-01 GitHub's audit log service lost connectivity to its backing data store due to a failed credential rotation, making audit log history unavailable via API and web UI for 28 minutes; other 2026 incidents caused degraded performance across Codespaces, Pages, Packages, OAuth/GitHub Apps, Marketplace, and Copilot due to shared data dependencies. Source: [GitHub availability report: May 2026](https://github.blog/news-insights/company-news/github-availability-report-may-2026/) / [GitHub availability report: April 2026](https://github.blog/news-insights/company-news/github-availability-report-april-2026/), accessed 2026-08-18.
- [FACT] **Primary-source verified (v1.1 fix-up).** GitHub Enterprise is listed at "starting at $21 USD per user/month" (first 12 months) on GitHub's own pricing page, including data residency, Enterprise Managed Users, SCIM provisioning, and advanced security features in that tier's description; the separate GitHub Team plan is $4/user/month (first 12 months) and Free is $0. Enterprise Server (self-hosted) is described elsewhere as an increasingly niche option, with most large 2018–2022 Enterprise Server customers migrated to Enterprise Cloud by 2023–2026 — that migration-trend claim was not independently re-verified against a primary source in this pass and remains `[UNVERIFIED-FACT]` (originally sourced from a third-party aggregator, [GitHub Enterprise Pricing 2026: Cloud, Server & Add-ons](https://atonementlicensing.com/blog/github-enterprise-pricing-2026/)). Source (pricing figure): [GitHub Pricing](https://github.com/pricing), accessed 2026-08-18.
- [FACT] **Superseded by primary-source finding (v1.1 fix-up).** As of a change effective April 1, 2025, GitHub Advanced Security was unbundled into two standalone products rather than sold as a single add-on: GitHub Secret Protection at $19/month per active committer, and GitHub Code Security at $30/month per active committer, both billed on unique committers to private repositories over a trailing 90-day window. The previously-cited flat "$49/user/month" figure for a unified "Advanced Security" add-on appears to describe the pre-2025 bundled product and is no longer GitHub's current pricing structure. Source: [Evolving GitHub Advanced Security: Greater flexibility, easier to access](https://resources.github.com/evolving-github-advanced-security/) and [GitHub Advanced Security license billing](https://docs.github.com/en/billing/concepts/product-billing/github-advanced-security), accessed 2026-08-18 (official GitHub sources, found via WebSearch, not directly WebFetched in this pass — treat as `[UNVERIFIED-FACT]` pending a direct fetch of docs.github.com, though the source domains are primary/official rather than third-party aggregators).

### GitLab / Duo

- [FACT] The GitLab AI Gateway is a standalone service providing access to AI-native GitLab Duo features; it can be self-hosted on GitLab Self-Managed via GitLab Duo Self-Hosted, shipped as a single-container Docker image, with port 5052 for AI Gateway HTTP and port 50052 for GitLab Duo Agent Platform Service gRPC. Source: [Install the GitLab AI Gateway](https://docs.gitlab.com/install/install_ai_gateway/) / [AI Gateway | GitLab Docs](https://docs.gitlab.com/administration/gitlab_duo/gateway/), accessed 2026-08-18.
- [FACT] GitLab Duo Self-Hosted provides air-gapped deployment options for data sovereignty; GitLab Dedicated for Government achieved FedRAMP Moderate ATO in May 2025. Source: [GitLab Duo Self-Hosted: Enterprise AI Built for Data Privacy](https://about.gitlab.com/blog/gitlab-duo-self-hosted-enterprise-ai-built-for-data-privacy/), accessed 2026-08-18.
- [UNVERIFIED-FACT] GitLab Duo Agent Platform Self-Hosted is reported at $299/seat/month for offline-licensing customers requiring in-house AI infrastructure. This could not be reached at a primary source in either the original pass or the v1.1 fix-up pass: `about.gitlab.com` is blocked by this environment's egress proxy for direct WebFetch, so this remains sourced only from a third-party aggregation/search-summary, not gitlab.com/about.gitlab.com itself. Source (third-party aggregation of GitLab pricing, primary confirmation still needed): search summary from [Install the GitLab AI Gateway](https://docs.gitlab.com/install/install_ai_gateway/) results set, accessed 2026-08-18.
- [FACT] Gitaly (and Gitaly Cluster / Praefect) executes Git operations on behalf of GitLab Shell and the web app and exposes an API for metadata and blobs; Sidekiq handles background jobs using Redis as queue backend; GitLab Workhorse sits behind NGINX/Apache and in front of the Puma app server, serving static assets/uploads directly and proxying the rest. Source: [GitLab architecture overview](https://docs.gitlab.com/development/architecture) / [gitlabhq/doc/development/architecture.md](https://github.com/gitlabhq/gitlabhq/blob/master/doc/development/architecture.md), accessed 2026-08-18.
- [FACT] **Partially primary-source verified (v1.1 fix-up).** GitLab offers four tiers — Free, Premium, Ultimate, and Ultimate Plus (formerly GitLab Dedicated); the free self-managed tier can run as Community Edition (CE) or Enterprise Edition (EE), with identical functionality for the open-source feature set. GitLab Premium is confirmed at $29/user/month (billed annually) via a WebSearch snippet of GitLab's own `about.gitlab.com/pricing/premium/` page — this is a primary-source snippet, though direct WebFetch of `about.gitlab.com` was blocked by this environment's egress proxy in this pass, so treat as `[FACT]` with a caveat rather than a fully-fetched confirmation. Ultimate's ~$99/user/month figure and the seat-count true-up/reconciliation detail were **not** independently confirmed against the primary pricing page in this pass and remain `[UNVERIFIED-FACT]`. SaaS and self-managed licenses being separate and non-combinable is unverified. Source (Premium price): WebSearch snippet of [Why GitLab Premium?](https://about.gitlab.com/pricing/premium/), accessed 2026-08-18. Source (Ultimate price, other details, third-party aggregator, unverified): [GitLab pricing 2026: Plans, tiers, and real costs](https://www.eesel.ai/blog/gitlab-pricing), accessed 2026-08-18.

### Gitea / Forgejo

- [FACT] Forgejo began as a friendly fork of Gitea and became a hard fork in early 2024, governed by a non-profit community structure (Codeberg), vs. Gitea which is backed by a company. Source: [Gitea vs Forgejo 2026: What's the Difference and Which to Self-Host?](https://contabo.com/blog/gitea-vs-forgejo/), accessed 2026-08-18.
- [FACT] Both ship a GitHub-Actions-compatible runner (Forgejo Actions / Gitea Actions); Forgejo shipped GitHub Actions compatibility ahead of Gitea Actions. Forgejo is GPL-licensed since v9; Gitea remains MIT plus a commercial edition. Forgejo is actively pursuing ActivityPub-based forge federation (issues/PRs/stars across independent instances); Gitea has no federation effort. Source: [Gitea vs Forgejo 2026: What's the Difference and Which to Self-Host?](https://contabo.com/blog/gitea-vs-forgejo/), accessed 2026-08-18.
- [FACT] Container images between the two projects remain largely interchangeable and the UI is nearly identical, so Forgejo can serve as a near drop-in Gitea replacement. Source: [Gitea vs Forgejo 2026: What's the Difference and Which to Self-Host?](https://contabo.com/blog/gitea-vs-forgejo/), accessed 2026-08-18.

### Bitbucket

- [FACT] As of 2026, Bitbucket Cloud is supported by the Atlassian Rovo MCP Server, letting external AI clients (Claude, ChatGPT, Cursor, VS Code) interact with Bitbucket repositories via MCP. Source: [The Atlassian Rovo MCP Server now supports Bitbucket Cloud](https://www.atlassian.com/blog/bitbucket/the-atlassian-rovo-mcp-server-now-supports-bitbucket-cloud), accessed 2026-08-18.
- [FACT] Rovo Chat is being integrated directly into Bitbucket to speed code review, onboarding, and pull context from Jira/Confluence; Rovo Dev can automatically find and fix code vulnerabilities. Source: [Meet Rovo Chat in Bitbucket](https://www.atlassian.com/blog/bitbucket/rovo-chat-bitbucket-beta), accessed 2026-08-18.
- [FACT] Bitbucket Cloud data residency is planned to start in the EU in December 2026 and India in June 2027; 2026 roadmap also includes CI/CD migration tooling (Jenkins → Bitbucket Pipelines) and architectural resiliency work. Source: [Building a more resilient, multi-region Bitbucket Cloud](https://www.atlassian.com/blog/how-we-build/building-a-more-resilient-multi-region-bitbucket-cloud), accessed 2026-08-18.

### Sourcegraph / Cody

- [UNVERIFIED-FACT] Sourcegraph Cody uses a RAG architecture combining pre-indexed vector embeddings with Sourcegraph's code search to pull context from local and remote repos. Enterprise self-hosted deployments can route LLM requests entirely within customer infrastructure (BYOK) and support fully air-gapped operation. Source: [Sourcegraph Cody vs Qodo (2026)](https://www.augmentcode.com/tools/sourcegraph-cody-vs-qodo), accessed 2026-08-18 (third-party/vendor-adjacent comparison site published by a competing code-AI vendor, augmentcode.com — not primary-source confirmed from sourcegraph.com docs).

### Claude Code

- [UNVERIFIED-FACT] Claude Code discovers Skills automatically from `~/.claude/skills/` (user-level) and `.claude/skills/` (repo-level); Skills are folder-based instruction packs loaded on demand. Source: [Claude Code Skills in 2026: The Complete Guide](https://www.totalum.app/blog/claude-code-skills-totalum), accessed 2026-08-18 (third-party site, not Anthropic's own docs — not primary-source confirmed).
- [UNVERIFIED-FACT] Claude Code Hooks are event-driven, deterministic scripts firing on events including PreToolUse, PostToolUse, UserPromptSubmit, and Notification. Source: [Claude Code Advanced Best Practices 2026](https://smartscope.blog/en/generative-ai/claude/claude-code-best-practices-advanced-2026/), accessed 2026-08-18 (third-party site — not primary-source confirmed).
- [UNVERIFIED-FACT] Claude Code subagents are specialized assistants with their own context window, prompt, and tool permissions, used to isolate parallel work from the main context. Claude Code also supports MCP servers as executable processes called over JSON-RPC. Source: [Claude Code Agent Teams, Subagents, and MCP: The 2026 Playbook](https://www.developersdigest.tech/blog/claude-code-agent-teams-subagents-2026), accessed 2026-08-18 (third-party site — directionally consistent with this agent's own operating knowledge of its own runtime, but the citation itself is not Anthropic's official Claude Code docs and was not directly fetched in this pass, so it is tagged `[UNVERIFIED-FACT]` rather than `[FACT]`).

### OpenAI Codex / Codex CLI

- [FACT] AGENTS.md is an open convention (not OpenAI-exclusive) working across 8+ agent tools, letting projects declare conventions once for any compatible agent. Source: [Shipyard: Codex CLI Cheatsheet](https://shipyard.build/blog/codex-cli-cheat-sheet/), accessed 2026-08-18.
- [FACT] Codex CLI (versions reported 0.128–0.130 as of May 2026) supports three sandbox modes and MCP-server integration configured in `config.toml`; Codex Cloud runs in isolated OpenAI-managed containers with a two-phase (setup, then agent) runtime model; MCP servers are subject to approval controls. Source: [Codex CLI Deep Dive: Config, Profiles, Sandbox 2026](https://www.digitalapplied.com/blog/codex-cli-deep-dive-config-profiles-sandbox-2026), accessed 2026-08-18.
- [FACT] Codex approval policies start "untrusted" by default; approval modes include suggest, auto-edit, and full-auto, controlling how much the agent can do before requiring human sign-off. Source: [Agent approvals & security | ChatGPT Learn](https://developers.openai.com/codex/agent-approvals-security), accessed 2026-08-18 (official OpenAI developer docs).

### Cursor

- [FACT] As of 2026, Cursor is in a "3.0 Agents Window" era with parallel agents (best-of-n on git worktrees), credit-pool pricing, and MCP with a 40-tool ceiling; Cursor 3.5 (May 20, 2026) added Cloud Agents running in isolated cloud VMs with terminal/browser/desktop access, able to work across multiple repos in parallel and report back asynchronously. Source: [Cursor 2026: Composer, Agent Mode, MCP & Background Agent](https://www.deployhq.com/guides/cursor), accessed 2026-08-18.
- [FACT] Cursor pricing (2026) spans six tiers: Hobby (free), Pro ($20), Pro+ ($60), Ultra ($200), Teams ($40/user), Enterprise (custom); mid-2025 pricing shifted from request quotas to a credit-based usage pool, with different frontier models burning credits at different rates (e.g., a more expensive Claude model burns credits roughly 5x faster than a cheaper one). Source: [The Complete Guide to Cursor Pricing in 2026](https://flexprice.io/blog/cursor-pricing-guide), accessed 2026-08-18.

### Devin / OpenHands / Gemini CLI

- [FACT] Devin (Cognition) offers a managed, proprietary cloud experience where parallel "Managed Devins" run in isolated cloud VMs and open PRs on completion. [UNVERIFIED-FACT] The specific figures — a 67% PR merge rate (up from 34% a year prior) and Devin authoring 89% of its own commits — are reported via a blog post published by OpenHands, which is a direct open-source competitor to Devin in the same "autonomous coding agent" category; this is a case of competitor-source bias, not merely low confidence from an unaffiliated third party, so these numbers should be treated with particular skepticism until confirmed by Cognition's own materials or an independent benchmark. Source: [The 9 Best Coding Agents in 2026, Ranked](https://www.openhands.dev/blog/best-coding-agents), accessed 2026-08-18 (competitor-published, not independently verified).
- [FACT] OpenHands is free to self-host, runs on the user's own API keys, and can be deployed on-premises for data-sovereignty requirements. Source: [The 6 Best Open-Source Devin Alternatives in 2026](https://www.openhands.dev/blog/devin-ai-alternatives), accessed 2026-08-18.
- [UNVERIFIED-FACT] Gemini CLI was reportedly retired by Google on 2026-06-18, replaced by a closed-source successor ("Antigravity CLI"), with the free tier cut from 1,000 requests/day to roughly 20. Source: [Best AI Coding Agents in 2026, Ranked — MightyBot](https://mightybot.ai/blog/coding-ai-agents-for-accelerating-engineering-workflows/), accessed 2026-08-18 (third-party — not confirmed against Google's own blog or release notes in either pass).

---

## Phase 3 — Competitor Analysis

Template fields per product: Product / Target User / Core Workflow / Repository / Collaboration / CI-CD / Search / AI / Agent / Context / Knowledge / Graph / Automation / Extensibility / Security / Self-Hosted / Cloud / UX / Architecture / License / Pricing / Strengths / Weaknesses / Structural Limitations / Opportunity.

### GitHub (deep dive)

- **Product**: Forge-of-record for Git hosting + collaboration + CI/CD + AI agents, owned by Microsoft.
- **Target User**: Broadest possible — OSS, startups to large enterprise.
- **Core Workflow**: Fork/branch → PR → review → Actions CI → merge → release/package.
- **Repository**: Git only; no native non-Git object graph.
- **Collaboration**: PR review, Discussions, Issues/Projects (kanban+table), CODEOWNERS.
- **CI/CD**: Actions (YAML workflows, hosted + self-hosted runners). `[FACT]` widely used, integrated with audit log per above.
- **Search**: Code search (Blackbird engine) plus Copilot-assisted search. `[TBD]` exact 2026 capability set.
- **AI**: Copilot (chat, completions, Agent mode, code review). `[FACT]` MCP servers configurable per-repo for Copilot Coding Agent.
- **Agent**: Copilot Coding Agent — async/"cloud agent" that takes an assigned issue, works in an isolated environment, opens a PR. `[FACT]`
- **Context**: Repo, Issues, PR threads, Discussions as context sources for Copilot; MCP extends to external systems. `[FACT]`
- **Knowledge**: No first-class ADR/requirement objects; knowledge lives in Issues/Discussions/README/wiki, unstructured.
- **Graph**: No native cross-object engineering graph (issue↔PR↔release↔deployment linking is partial, via references/Projects, not a queryable graph model). `[INFERENCE]`
- **Automation**: Actions + Apps + webhooks; broad but YAML/imperative, not policy/graph-driven.
- **Extensibility**: GitHub Apps, REST/GraphQL API, Actions marketplace, MCP.
- **Security**: Advanced Security's code scanning, secret scanning, and Dependabot capabilities are sold at extra cost, now unbundled into GitHub Secret Protection ($19/committer/mo) and GitHub Code Security ($30/committer/mo) `[UNVERIFIED-FACT, primary-domain source (github.blog/docs.github.com) found via WebSearch but not directly WebFetched in v1.1 pass]`; Enterprise audit log `[FACT]`.
- **Self-Hosted**: Enterprise Server exists but described as niche/declining by 2026, most large customers migrated to Cloud. `[FACT]`
- **Cloud**: Primary deployment mode; Enterprise Cloud with Data Residency option. `[FACT]`
- **UX**: Mature, widely adopted, web-first with VS Code deep integration.
- **Architecture**: Proprietary, not documented publicly in architectural depth (unlike GitLab). `[TBD]`
- **License**: Proprietary SaaS; Enterprise Server proprietary self-hosted binary.
- **Pricing**: Enterprise starts at $21/user/month (first 12 months) across Cloud/Server/Data-Residency tiers, confirmed via direct WebFetch of github.com/pricing in the v1.1 fix-up pass `[FACT]`. Advanced Security is no longer sold as a single ~$49/user/month add-on as of April 2025; it is unbundled into Secret Protection ($19/committer/mo) and Code Security ($30/committer/mo). `[UNVERIFIED-FACT, primary-domain source found via WebSearch, not directly WebFetched]`
- **Strengths**: Network effects (largest OSS graph), Actions ecosystem, Copilot distribution via VS Code, MCP momentum.
- **Weaknesses**: No native engineering-graph beyond Git+Issues; self-hosting increasingly deprecated in practice; AI features fragmented across paid add-ons.
- **Structural Limitations**: Issue/PR/Discussion are three separate, weakly-linked object types bolted onto a Git host over 15+ years — no unified graph schema; audit/compliance data lives in a separate subsystem prone to independent outages (`[FACT]` per April 2026 incident).
- **Opportunity**: A platform with a first-class, queryable engineering graph (not just Git + bolted-on trackers) and unified self-hosted/cloud parity is a structural differentiator. `[PROPOSAL]`
- **If redesigning GitHub from zero today, what legacy baggage would you cut?** `[PROPOSAL]` Cut: (1) the Issues/Discussions/Projects three-way split — unify into one graph-native work-item model; (2) YAML-only Actions as the sole automation surface — add a native agent/automation object type with policy and provenance; (3) treating audit log as a bolt-on service rather than a first-class append-only ledger entangled with every other subsystem; (4) the Enterprise Server/Cloud parity-drift problem — self-hosting should not be a second-class, shrinking deployment target.

### GitLab (deep dive)

- **Product**: Single-application DevOps platform (plan→code→build→test→release→monitor) with integrated AI (Duo).
- **Target User**: Enterprises wanting one vendor for the full SDLC, plus self-hosters.
- **Core Workflow**: Issue → MR → pipeline → review → merge → deploy, all in one app.
- **Repository**: Git via Gitaly; Gitaly Cluster/Praefect for HA/replication. `[FACT]`
- **Collaboration**: MRs, threads, issue boards, epics (Ultimate).
- **CI/CD**: GitLab CI, native, deeply integrated (not bolted on).
- **Search**: Advanced Search (Elasticsearch-backed, paid tier). `[TBD]` 2026 specifics.
- **AI**: GitLab Duo — chat, code suggestions, Duo Agent Platform (multi-agent workflows), routed through AI Gateway. `[FACT]`
- **Agent**: Duo Agent Platform / Duo Workflow — agentic execution service, gRPC-based, JWT-signed. `[FACT]`
- **Context**: Repo + issue + MR context native to the single app (structural advantage vs. GitHub's bolted-together model). `[INFERENCE]`
- **Knowledge**: Issues/epics/wikis; no first-class ADR object type identified. `[TBD]`
- **Graph**: Relations between issue/MR/epic/pipeline are native to one data model (single Rails monolith + Postgres), stronger graph coherence than GitHub's multi-service model. `[INFERENCE]`
- **Automation**: CI/CD native + Duo Workflow agentic automation.
- **Extensibility**: REST/GraphQL API, webhooks, integrations.
- **Security**: SAST/DAST/dependency scanning at Ultimate tier; FedRAMP Moderate ATO for GitLab Dedicated for Government (May 2025). `[FACT]`
- **Self-Hosted**: Full self-managed option (CE/EE, identical feature set for OSS features) plus Duo Self-Hosted for offline AI (air-gapped). `[FACT]`
- **Cloud**: GitLab.com SaaS, GitLab Dedicated (single-tenant), Ultimate Plus.
- **UX**: Feature-dense, sometimes considered heavier/slower than GitHub. `[INFERENCE]`
- **Architecture**: Documented in depth — Rails monolith + Gitaly (Git RPC layer) + Praefect (Gitaly cluster coordinator) + Sidekiq (Redis-backed background jobs) + Workhorse (Go proxy for large payloads/static assets) + Puma app server. `[FACT]`
- **License**: CE (MIT/open source) vs EE (proprietary, tiered by license key) — same binary, feature-gated.
- **Pricing**: Free / Premium $29/user/mo (cloud, confirmed via WebSearch snippet of about.gitlab.com/pricing/premium/ in the v1.1 pass — `[FACT]` with the caveat that direct WebFetch of about.gitlab.com is blocked in this environment) / Ultimate ~$99/user/mo (cloud, still `[UNVERIFIED-FACT]`, third-party aggregation only); SaaS and self-managed licensed separately (`[UNVERIFIED-FACT]`); Duo Agent Platform Self-Hosted ~$299/seat/month (`[UNVERIFIED-FACT]`, primary source unreachable — about.gitlab.com is blocked by this environment's egress proxy).
- **Strengths**: True single-application architecture gives coherent cross-object graph; genuinely maintains self-hosted parity (unlike GitHub); documented, inspectable architecture.
- **Weaknesses**: Monolith complexity (Gitaly/Praefect/Sidekiq/Workhorse) makes self-hosting operationally heavy; AI Duo pricing is a la carte and expensive at scale; CE/EE feature-gating creates friction.
- **Structural Limitations**: The Rails-monolith-plus-satellite-services architecture (Gitaly, Praefect, Sidekiq, Workhorse, Puma) is a product of Git's own performance constraints being solved with bolted-on services over a decade — each satellite exists to work around Git/Ruby scaling limits, not because the domain wanted five moving parts. `[INFERENCE]`
- **Opportunity**: A platform designed graph-native from day one (rather than Git-native-then-graph-bolted-on) could avoid the Gitaly/Praefect/Sidekiq/Workhorse layering entirely. `[PROPOSAL]`
- **Why can GitLab self-host at all, where does its complexity come from, what would you re-abstract?** `[PROPOSAL]` GitLab can self-host because it was architected from the start as an installable Rails app with an explicit reference-architecture doc for every scale tier (`[FACT]`, docs.gitlab.com/administration/reference_architectures/), not retrofitted for it. Complexity comes from solving Git's inherent scaling weaknesses (single-writer repo access, large blobs) with additional services (Gitaly for RPC-ified Git access, Praefect for replication, Workhorse for bypassing the Ruby app server on heavy I/O) rather than from the product domain itself. A from-scratch platform could re-abstract this by choosing a storage/execution substrate that doesn't need a Git-specific RPC/cluster/proxy stack bolted on — e.g., a graph-native object store where Git repos are one object type among many, with replication and large-object handling designed in from the start rather than layered on. `[PROPOSAL]`

### Gitea / Forgejo (combined)

- **Product**: Lightweight, self-hostable Git forges; Forgejo is a 2024 hard fork of Gitea.
- **Target User**: Individuals, small teams, homelab/self-hosters, privacy-conscious orgs.
- **Repository**: Git; lightweight Go binary, single-process deployable.
- **CI/CD**: Gitea Actions / Forgejo Actions, both GitHub-Actions-workflow-compatible; Forgejo shipped this first. `[FACT]`
- **AI/Agent**: `Unknown / Not Confirmed` — no first-class AI agent platform identified in this pass; likely relies on external tools/MCP bolt-ons. `[TBD]`
- **Governance/License**: Gitea = company-backed, MIT + commercial edition; Forgejo = non-profit (Codeberg) governed, GPL since v9. `[FACT]`
- **Self-Hosted**: Native and default mode for both — this is their core value proposition, unlike GitHub/GitLab where self-hosting is a secondary/declining or heavyweight option.
- **Cloud**: Codeberg (Forgejo-based) is a public hosted instance; otherwise no vendor SaaS.
- **Federation**: Forgejo pursuing ActivityPub federation across independent instances; Gitea has no such effort. `[FACT]`
- **Strengths**: Minimal resource footprint, true single-binary self-hosting, GPL/non-profit governance (Forgejo) removes rug-pull risk.
- **Weaknesses**: No native AI/agent platform; smaller ecosystem/marketplace than GitHub/GitLab; enterprise features (advanced security scanning, compliance certifications) largely absent or unconfirmed. `[TBD]`
- **Structural Limitations**: Built as "GitHub, but self-hostable and light" — inherits the Git-object-only data model, no engineering-graph ambition.
- **Opportunity**: Target Platform can adopt Forgejo/Gitea's "true self-hosting, single deployable" ethos while adding the graph-native/agent-native layer they lack. `[PROPOSAL]`

### Bitbucket

- **Product**: Atlassian's Git forge, deeply integrated with Jira/Confluence.
- **Target User**: Existing Atlassian-suite enterprises.
- **AI**: Rovo Chat (beta, in-product AI teammate) and Rovo Dev (auto vuln fix); Rovo MCP Server now supports Bitbucket Cloud, enabling external AI clients (Claude, ChatGPT, Cursor, VS Code) to connect. `[FACT]`
- **CI/CD**: Bitbucket Pipelines; 2026 roadmap includes Jenkins migration tooling. `[FACT]`
- **Self-Hosted**: Bitbucket Data Center (self-managed) vs Bitbucket Cloud (SaaS) — feature comparison documented by Atlassian. `[FACT, source exists, details `[TBD]``]`
- **Cloud**: Multi-region resiliency work ongoing in 2026; EU data residency from Dec 2026, India from June 2027. `[FACT]`
- **Knowledge/Graph**: Cross-links to Jira issues and Confluence docs via Atlassian's shared platform — closest of the forges to a "linked engineering graph," but the graph lives in Jira/Confluence, not Bitbucket itself. `[INFERENCE]`
- **Strengths**: Best-in-class linkage to project-management/knowledge tools within the Atlassian ecosystem.
- **Weaknesses**: Perceived as lagging GitHub/GitLab in AI/agent maturity until Rovo's 2025-2026 push; requires the broader Atlassian suite for full value.
- **Structural Limitations**: Value is contingent on Atlassian suite adoption; the "graph" is really three separate products (Bitbucket/Jira/Confluence) glued by cross-linking, not a unified data model.
- **Opportunity**: Demonstrates market appetite for a genuinely unified graph across code + work-items + docs, rather than three linked products. `[PROPOSAL]`

### Sourcegraph / Cody

- **Product**: Code search + AI code-intelligence layer, deployable atop existing forges.
- **Target User**: Large enterprises with multi-repo, multi-forge codebases needing search/context at scale.
- **Search**: Core differentiator — cross-repo code search plus RAG-based Cody assistant. `[FACT]`
- **AI**: Cody combines vector embeddings + Sourcegraph search for context; BYOK/self-hosted LLM routing available for air-gapped enterprise. `[FACT]`
- **Self-Hosted**: Enterprise on-prem with full data isolation, air-gapped supported. `[FACT]`
- **Graph**: Multi-repo reasoning across "many repos to reason about large system architectures" — closest existing product to a cross-repo knowledge graph, though scoped to code, not the full engineering graph (no issue/ADR/incident/agent objects). `[FACT + INFERENCE]`
- **Strengths**: Best-in-class code search; forge-agnostic (works across GitHub/GitLab/Bitbucket); enterprise-grade air-gap story.
- **Weaknesses**: Not a forge itself — always a layer atop another system of record, meaning it can't be the graph-of-record for non-code objects.
- **Structural Limitations**: Search/RAG-based context retrieval, not a persistent structured graph — context is reconstructed per-query rather than modeled as durable graph relationships. `[INFERENCE]`
- **Opportunity**: Validates that deep cross-repo context matters enough to build a standalone product around — Target Platform should absorb this natively rather than bolt it on. `[PROPOSAL]`

### OpenAI Codex / Codex CLI (deep dive)

- **Product**: OpenAI's terminal-first coding agent.
- **AGENTS.md**: Open, cross-tool convention for project instructions, not Codex-exclusive — works across 8+ agent tools. `[FACT]`
- **MCP**: Client support via `config.toml`; MCP servers subject to approval controls. `[FACT]`
- **Sandboxing/Approvals**: Three sandbox modes; default policy starts "untrusted"; approval modes suggest / auto-edit / full-auto; Codex Cloud runs in isolated OpenAI-managed containers with a two-phase (setup-then-agent) runtime. `[FACT]`
- **Git workflow**: `Unknown / Not Confirmed` — exact native git-commit/PR-authoring behavior not independently verified in this pass. `[TBD]`
- **Self-Hosted**: CLI runs locally against OpenAI's hosted models; no self-hosted model backend identified. `[TBD]`
- **Strengths**: Cross-tool AGENTS.md convention lowers switching cost; clear layered approval model for autonomy vs. safety.
- **Weaknesses**: Tied to OpenAI-hosted models; cloud execution is opaque outside OpenAI's container environment.
- **Structural Limitations**: No persistent engineering-graph concept — Codex operates task-by-task against a repo, with no native model for issue/ADR/incident context beyond what's in-repo (AGENTS.md, code) or reachable via MCP.
- **Opportunity**: AGENTS.md as convention validates market demand for portable, tool-agnostic agent context files — Target Platform should treat this as an interop surface, not compete with it. `[PROPOSAL]`

### Claude Code (deep dive)

- **Product**: Anthropic's terminal-first coding agent (this agent's own runtime).
- **Subagents**: Specialized assistants with their own context window, prompt, and tool permissions, used for isolated parallel work. `[UNVERIFIED-FACT, third-party corroboration only; exact current primary-doc wording not confirmed]`
- **Skills**: Folder-based, on-demand instruction packs discovered from `~/.claude/skills/` and `.claude/skills/`. `[UNVERIFIED-FACT, third-party corroboration only]`
- **Hooks**: Deterministic event-driven scripts on lifecycle events (PreToolUse, PostToolUse, UserPromptSubmit, Notification, etc.) — used where a rule must be enforced rather than merely suggested. `[UNVERIFIED-FACT, third-party corroboration only]`
- **MCP**: Client support for MCP servers as JSON-RPC tool processes. `[FACT]`
- **Git workflow**: `Unknown / Not Confirmed` in this pass beyond general capability to run git commands as any shell tool; no distinct native git object model identified. `[TBD]`
- **Self-Hosted**: CLI runs locally; backed by Anthropic-hosted models (or Bedrock/Vertex per enterprise routing) — exact 2026 self-hosted-model support `[TBD]`.
- **Strengths**: Clear separation of concerns (hooks=enforcement, skills=knowledge, subagents=delegation, MCP=external tools, CLAUDE.md=always-on guidance) — a relatively principled context-engineering model compared to competitors' flatter designs. `[INFERENCE]`
- **Weaknesses**: Primarily a CLI/agent, not a forge — has no native issue/PR/CI object model of its own; depends on MCP or the underlying forge for that data.
- **Structural Limitations**: Like Codex, operates task-scoped against whatever repo/context it's given; no persistent cross-session engineering graph natively.
- **Opportunity**: The hooks/skills/subagents/MCP separation is a good pattern to generalize into the Target Platform's own agent-native object model (agent, agentrun, prompt, skill, policy, context as distinct graph node types). `[PROPOSAL]`

### Cursor

- **Product**: AI-native fork of VS Code (IDE-first, not CLI-first).
- **Agent Mode**: Reads codebase, edits files, runs terminal commands, iterates until done or blocked by a guardrail. `[FACT]`
- **Cloud Agents (3.5, May 2026)**: Isolated cloud VMs with terminal/browser/desktop access, multi-repo, async reporting. `[FACT]`
- **MCP**: Supported, with a reported 40-tool ceiling per configuration. `[FACT]`
- **Pricing**: Hobby free / Pro $20 / Pro+ $60 / Ultra $200 / Teams $40/user / Enterprise custom; credit-pool based since mid-2025, model-dependent burn rate. `[FACT]`
- **Self-Hosted**: `Unknown / Not Confirmed` — Cursor is fundamentally a hosted-model-backed IDE product; no self-hosted deployment identified. `[TBD]`
- **Strengths**: Best-in-class IDE-integrated agent UX; parallel agents on git worktrees.
- **Weaknesses**: Not a forge or graph platform; no self-hosting; pricing complexity (credit burn varies by model).
- **Structural Limitations**: IDE-bound — value is contingent on being inside the editor, not a system-of-record for the engineering graph.
- **Opportunity**: Confirms strong demand for parallel, worktree-based agent execution as a UX pattern worth adopting natively. `[PROPOSAL]`

### GitLab Duo vs. GitHub Copilot — AI-in-IDE / AI-in-Forge comparison

- [FACT] GitLab Duo is positioned as "Platform-Native DevSecOps," i.e., AI embedded across the single-application SDLC, vs. Claude Code's "terminal-first autonomy" — per a third-party comparison. Source: [GitLab Duo vs Claude Code: Platform-Native DevSecOps or Terminal-First Autonomy?](https://www.augmentcode.com/tools/gitlab-duo-vs-claude-code), accessed 2026-08-18.
- [INFERENCE] Copilot and Duo both bolt AI onto an existing forge's data model, while Claude Code/Codex/Cursor treat the forge as just one more tool reachable via MCP — this is a genuine architectural fork: "AI as forge feature" vs. "forge as agent tool," and the Target Platform must decide deliberately which posture it takes (this research suggests: be the graph AI agents operate against, natively, rather than either). `[PROPOSAL]`

### Linear (deep dive)

- **Product**: Purpose-built issue tracking and project management tool for software teams, positioned on speed and opinionated workflow design rather than breadth of features.
- **Target User**: Product/engineering teams at startups through mid-size companies wanting a fast, focused PM tool as an alternative to Jira; less oriented toward large-enterprise compliance buyers.
- **Core Workflow**: Issue → team workflow states (Todo/In Progress/In Review/Done, configurable per team) → Cycles (short time-boxed iterations, typically 1–2 weeks, similar to sprints) → Projects (multi-team rollups with Gantt-style timelines, "Projects 2.0"). `[UNVERIFIED-FACT]` — sourced from third-party 2026 reviews, not Linear's own docs directly fetched in this pass.
- **Repository**: Not a Git host; integrates with GitHub/GitLab/Bitbucket for code linkage (branch/PR ↔ issue references) rather than hosting Git itself. `[INFERENCE]`
- **Collaboration**: Comments, attachments on issues; "Linear Asks" (2026 addition) — intake forms that route submissions to issues. `[UNVERIFIED-FACT]`
- **CI/CD**: None native; relies on integrations with the Git forge's CI/CD.
- **Search**: Standard in-app search across issues/projects; no evidence of a distinct code-search or cross-repo search capability, consistent with Linear not being a code platform. `[TBD]`
- **AI**: The Linear Agent synthesizes context across threads, backlog, and customer requests, drafts issues from meeting notes/videos/discussions, and supports reusable saved "Skills" workflows; Automations can run agent workflows automatically when issues enter triage (Business/Enterprise tiers only). `[UNVERIFIED-FACT]` — third-party 2026 review sources.
- **Agent**: Third-party AI coding agents (Cursor, Devin, and similar) can be assigned issues directly and treated as full workspace members — assigned to issues, added to projects, @mentioned in threads — via what sources describe as a "Linear Mission Control Plane (MCP)" integration layer connecting external agent tools into Linear's workflow. `[UNVERIFIED-FACT]` — this MCP-branded integration name is not independently confirmed against Linear's own docs and may be a third-party paraphrase of Linear's actual MCP server offering; flagged for primary-source confirmation before relying on the exact mechanism.
- **Context**: Issues, cycles, and projects form Linear's working context; the AI agent layer explicitly synthesizes across threads/backlog/customer requests as its context model. `[UNVERIFIED-FACT]`
- **Knowledge**: No first-class ADR or requirement-document object type identified; issues and their comment threads are the primary knowledge artifact. `[TBD]`
- **Graph**: Existing team knowledge characterizes Linear as "the most graph-coherent PM tool on the market" — this pass finds supporting circumstantial evidence (a single coherent data model spanning teams/issues/cycles/projects with a full-featured GraphQL API for read-write access) but did not find primary-source documentation describing Linear's internal model explicitly as a graph, nor could its schema be inspected directly. Treat the "graph-coherent" characterization as `[INFERENCE]` from team knowledge plus circumstantial 2026 evidence, not as an independently verified `[FACT]`.
- **Automation**: GraphQL API and webhooks with full read-write access to issues, projects, cycles, comments, attachments; Automations (Business/Enterprise) trigger agent workflows on state changes such as triage entry. `[UNVERIFIED-FACT]`
- **Extensibility**: GraphQL API, webhooks, third-party agent integrations. `[UNVERIFIED-FACT]`
- **Security**: Enterprise tier adds SSO and audit logs as part of "advanced security"; specifics (RBAC granularity, compliance certifications) not confirmed. `[TBD]`
- **Self-Hosted**: `[FACT]` Linear offers no on-premise, self-hosted, or community edition at any tier — all workspace data resides on Linear's own US-based servers, for every plan including Enterprise. This is a structural contrast with GitLab/Gitea/Forgejo/Sourcegraph, all of which offer some self-hosted path. Open-source alternatives explicitly positioned as self-hostable "Linear-likes" (e.g. Plane) exist precisely because Linear itself does not offer this.
- **Cloud**: SaaS-only, four tiers — Free (unlimited members, capped at 2 teams / 250 issues, no AI features), Basic ($10/user/month, unlimited issues, 5 teams), Business ($16/user/month, unlimited teams, private teams, guest access, Linear Agent), Enterprise (custom pricing, SSO/audit logs/dedicated support/custom SLAs). All paid plans billed annually only, no monthly option. `[UNVERIFIED-FACT]` — third-party pricing-aggregator sources; not confirmed against linear.app/pricing directly (not attempted via WebFetch in this pass).
- **UX**: Widely reported as fast/keyboard-driven/opinionated; this reputation is consistent across multiple independent third-party reviews but was not independently tested in this pass. `[INFERENCE]`
- **Architecture**: Not documented publicly in the sources found in this pass; proprietary. `[TBD]`
- **License**: Proprietary SaaS, no open-source component identified (distinguishing it from GitLab CE or Forgejo).
- **Pricing**: See Cloud above — $0 / $10 / $16 / custom, all `[UNVERIFIED-FACT]`.
- **Strengths**: Focused, fast, single-purpose tool avoids the feature sprawl of Jira; the (reported) unified data model across teams/issues/cycles/projects plus a full GraphQL API is closer to a genuine PM graph than the bolted-together Issues/Discussions/Projects split seen in GitHub, and closer than Jira's older, more rigid schema is generally understood to be; treating agents as full workspace members (assignable, @mentionable) is a notably agent-native design choice among PM tools surveyed.
- **Weaknesses**: No self-hosting option at any price point is disqualifying for the Target Platform's self-hosted/air-gapped requirement if Linear itself were being evaluated as infrastructure rather than a comparison point; AI/agent feature depth claims are third-party-sourced and unconfirmed; not a code host or CI/CD platform, so it only ever covers one slice (requirement/issue) of the full engineering graph this platform intends to model.
- **Structural Limitations**: Linear's coherence comes from being a single, narrowly-scoped, cloud-only product covering issues/cycles/projects — it does not attempt to unify code, CI, releases, incidents, or agent-run provenance into the same graph, so its "graph coherence" is real but scoped to project-management objects only, not the full engineering graph (requirement→ADR→issue→PR→test→release→incident→agentrun) the Target Platform is targeting.
- **Opportunity**: Validates two design choices for the Target Platform: (1) a genuinely unified, queryable data model (not a bolted-together set of object types) is achievable and valued by users even without an explicit "graph database" framing — Linear did this with what looks like a well-normalized relational/graph-ish schema behind a GraphQL API; (2) treating AI agents as first-class, assignable workspace participants (not just an external tool reachable via API) is a UX pattern worth generalizing across the whole engineering graph, not just issues. `[PROPOSAL]`
- **Sources** (all third-party, found via WebSearch, none directly WebFetched from linear.app in this pass — every Linear claim above is `[UNVERIFIED-FACT]`/`[TBD]`/`[INFERENCE]` accordingly): [Linear AI features: What the PM tool can do (2026)](https://www.eesel.ai/blog/linear-ai), [Linear Review 2026: 74/100 | AI PM Tools](https://aipmtools.org/project-management/linear), [Linear — AI PM Wiki | GenAI PM](https://genaipm.com/wiki/companies/linear), [Linear 2026: $8/User Issue Tracker, GraphQL API | Automation Atlas](https://automationatlas.io/tools/linear/), [Linear Review 2026: The Fastest Project Management Tool Gets AI Agents | utilo](https://utilo.io/blog/linear-review-2026-project-management), [Linear AI 2026: What the Agent Does, Plan Pricing & Verdict](https://aiagentsquare.com/agents/linear-ai), [Linear Pricing 2026: $0 Free, $10 Basic, $16 Business Plans](https://aiproductivity.ai/blog/linear-pricing/), [Is Linear Open Source? No — 3 Real Alternatives Compared (2026)](https://zoobbe.com/linear-alternative-open-source/), all accessed 2026-08-18.

### Gerrit, Jira, GitHub Issues/Projects (standalone), Argo CD/Workflows, CodeQL, Sentry

`[TBD]` — not reached with dedicated live searches in this pass given the time budget; flagged here explicitly per Phase 1's plan so later phases know these entries are outstanding rather than silently dropped. Recommended before Phase 6: dedicated searches for (1) Gerrit's review-centric model as a possible precedent for a "review" graph-node type, (2) Argo CD/Workflows for the "deployment"/"release" graph-node precedent, (3) CodeQL/Sentry for the "policy"/"incident" node precedents. (Linear was completed in the v1.1 fix-up pass — see deep dive above.)

---

## Phase 4 — Capability Matrix

Legend: ✅ = supported/native · ⚠️ = partial/add-on/inferred · ❌ = not supported · `Unknown` = insufficient evidence gathered.

| Capability | GitHub | GitLab | Gitea/Forgejo | Bitbucket | Sourcegraph | Linear | Codex | Claude Code | Cursor | **Target Platform (proposed)** |
|---|---|---|---|---|---|---|---|---|---|---|
| Git | ✅ | ✅ | ✅ | ✅ | ⚠️ (indexes, doesn't host) | ❌ | ❌ (operates on local repo) | ❌ (operates on local repo) | ❌ (operates on local repo) | ✅ [PROPOSAL] |
| Issue tracking | ✅ | ✅ | ✅ | ⚠️ (via Jira) | ❌ | ✅ (core product) | ❌ | ❌ | ❌ | ✅ [PROPOSAL] |
| PR/MR | ✅ | ✅ | ✅ | ✅ | ❌ | ❌ (links to forge PRs, doesn't host) | ⚠️ (can open PRs via agent) | ⚠️ (can open PRs via agent) | ⚠️ (can open PRs via agent) | ✅ [PROPOSAL] |
| CI/CD | ✅ | ✅ | ✅ | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ [PROPOSAL] |
| Package registry | ✅ | ✅ | `Unknown` | `Unknown` | ❌ | ❌ | ❌ | ❌ | ❌ | ⚠️ [PROPOSAL, MVP-deferred] |
| Code search | ✅ (Blackbird) | ⚠️ (Advanced Search, paid) | `Unknown` | `Unknown` | ✅ (core product) | ❌ | ⚠️ (local, contextual) | ⚠️ (local, contextual) | ⚠️ (local, contextual) | ✅ [PROPOSAL] |
| Self-hosted | ⚠️ (Enterprise Server, declining) | ✅ (CE/EE, first-class) | ✅ (core value prop) | ✅ (Data Center) | ✅ (Enterprise on-prem) | ❌ (SaaS-only, no self-hosted tier) | ❌ (hosted model only) | ⚠️ (enterprise routing via Bedrock/Vertex reported, exact self-hosted-model support unconfirmed) | `Unknown` | ✅ [PROPOSAL, core value prop] |
| Offline/air-gapped | `Unknown` | ✅ (Duo Self-Hosted) | `Unknown` | `Unknown` | ✅ (BYOK air-gapped) | ❌ | ❌ | `Unknown` | `Unknown` | ✅ [PROPOSAL] |
| AI coding assist | ✅ (Copilot) | ✅ (Duo) | ❌ (no first-class AI agent platform identified) | ✅ (Rovo) | ✅ (Cody) | ❌ (not a code tool) | ✅ (core product) | ✅ (core product) | ✅ (core product) | ✅ [PROPOSAL] |
| Agent execution | ✅ (Coding Agent) | ✅ (Duo Agent Platform) | ❌ (no first-class AI agent platform identified) | ⚠️ (Rovo Dev) | ❌ | ⚠️ (Linear Agent + third-party agents as assignable workspace members) | ✅ (core product) | ✅ (core product) | ✅ (core product) | ✅ [PROPOSAL, core value prop] |
| MCP support | ✅ | `Unknown` | `Unknown` | ✅ (Rovo MCP Server) | `Unknown` | ⚠️ (reported "Linear MCP" integration layer, exact mechanism unconfirmed) | ✅ (client) | ✅ (client) | ✅ (client, 40-tool cap) | ✅ [PROPOSAL] |
| Knowledge graph | ❌ | ⚠️ (single-app data model, not a true graph API) | ❌ | ⚠️ (via Jira/Confluence linking) | ⚠️ (multi-repo reasoning, not persistent graph) | ⚠️ (reportedly the most graph-coherent PM data model surveyed, but scoped to issues/cycles/projects only, not the full engineering graph) | ❌ | ❌ | ❌ | ✅ [PROPOSAL, core value prop] |
| Semantic diff | `Unknown` | `Unknown` | `Unknown` | `Unknown` | `Unknown` | ❌ N/A | `Unknown` | `Unknown` | `Unknown` | ⚠️ [PROPOSAL, Experimental] |
| Intent/context capture | ⚠️ (Issues/Discussions, unstructured) | ⚠️ (issues/epics, unstructured) | ❌ | ⚠️ (via Jira) | ❌ | ⚠️ (issues/comments, unstructured; AI agent synthesizes across them) | ⚠️ (AGENTS.md) | ⚠️ (CLAUDE.md/skills) | ⚠️ (rules files) | ✅ [PROPOSAL, core value prop] |
| Workflow automation | ✅ (Actions) | ✅ (CI + Duo Workflow) | ✅ (Actions-compatible) | ✅ (Pipelines) | ❌ | ⚠️ (Automations trigger agent workflows on state changes, Business/Enterprise only) | ⚠️ (task-scoped only) | ⚠️ (task-scoped only) | ⚠️ (task-scoped only) | ✅ [PROPOSAL] |
| RBAC/Security | ✅ (+ paid Advanced Security) | ✅ (+ paid SAST/DAST) | `Unknown` | ✅ | `Unknown` | ⚠️ (SSO/audit logs at Enterprise tier, granularity unconfirmed) | ❌ N/A | ❌ N/A | ❌ N/A | ✅ [PROPOSAL] |
| Audit trail | ✅ (Enterprise) | `Unknown` | `Unknown` | `Unknown` | `Unknown` | ⚠️ (Enterprise-tier audit logs reported, not independently confirmed) | ❌ N/A | ❌ N/A | ❌ N/A | ✅ [PROPOSAL, core value prop] |
| API/Webhook extensibility | ✅ | ✅ | ✅ | ✅ | `Unknown` | ✅ (GraphQL API + webhooks) | ✅ (MCP) | ✅ (MCP + hooks) | ✅ (MCP) | ✅ [PROPOSAL] |

---

## Phase 5 — Gap Analysis

### Commodity (must-have, don't over-invest)
- **Git hosting, PR/MR review, CI/CD, API/webhook extensibility, RBAC.** [FACT-grounded] Every major forge (GitHub, GitLab, Gitea/Forgejo, Bitbucket) and every AI agent tool assumes these exist. They are table stakes — competitive differentiation will not come from having "our own Git host," it will come from what sits on top of it. Build to parity, then stop investing.
- **Basic AI coding assist (chat, inline completion).** Every competitor surveyed (Copilot, Duo, Cody, Rovo, Codex, Claude Code, Cursor) already ships this; it's now an expected baseline feature, not a differentiator per se.

### Differentiator (real competitive edge)
- **Knowledge graph across the full engineering surface (requirement→ADR→issue→PR→test→release→incident→agentrun).** [FACT + INFERENCE] Evidence from Phase 3 shows every competitor either has no graph (GitHub: Issues/Discussions/Projects are three weakly-linked systems) or a graph confined to one product's data model (GitLab: coherent but code/CI-scoped; Bitbucket: cross-linked but really three separate products; Sourcegraph: code-only, non-persistent RAG reconstruction; Linear: reportedly the most graph-coherent PM tool surveyed, but scoped to issues/cycles/projects only, with no code, CI, release, or incident objects, and no self-hosted deployment option at all). None model requirement, ADR, incident, agent, agentrun, prompt, skill, model, context, or policy as first-class graph nodes. This is the clearest open structural gap.
- **True self-hosted + cloud parity with agent-native execution.** [FACT] GitHub's self-hosted option is declining/niche; GitLab self-hosts fully but at high operational complexity (Gitaly/Praefect/Sidekiq/Workhorse); Cursor/Codex have no self-hosting story at all. A platform offering local-first operation with the *same* agent capabilities as cloud, without GitLab's multi-service operational burden, is a real edge.
- **Unified audit trail across human AND agent actions.** [FACT] GitHub's audit log is architecturally a separate subsystem, and it experienced one documented 28-minute outage in April 2026 due to a failed credential rotation. This single incident is evidence that the audit log is not structurally isolated from other shared-dependency failures, but it is one data point, not an established pattern of recurring or systemic audit-log fragility — a stronger claim would need evidence of repeated incidents over time, which this pass did not gather. No competitor demonstrated a unified, agent-inclusive audit ledger treating agentrun as a first-class audited actor. This directly serves the "agent-native" and "policy" graph-node ambition.

### Emerging (market still forming standards)
- **MCP-based agent context/tool access.** [FACT] MCP adoption is now widespread (Copilot, Duo/Bitbucket via Rovo, Codex, Claude Code, Cursor all support it in 2026), but each implementation has its own limits (Cursor's 40-tool cap, per-repo config differences) and there's no cross-vendor standard yet for *how* an agent's context/session/provenance should be modeled as data, only for tool invocation. AGENTS.md is a similarly emerging, still-informal convention for portable agent instructions. The Target Platform should adopt MCP as an interop surface but not treat its current shape as final.
- **Agent execution as a forge-native feature (Copilot Coding Agent, Duo Agent Platform, Devin, OpenHands).** [FACT] All are less than ~2 years old as productized "assign an issue, agent opens a PR" workflows; none have settled on a standard data model for what an "agentrun" is, how it's audited, or how its provenance is tracked relative to a human PR. This is a real but immature space — worth building into, not over-committing to a specific competitor's shape.

### Experimental (interesting but not MVP-worthy)
- **Full semantic diff.** No competitor surveyed demonstrated this as a shipped capability (`Unknown` across the matrix); this remains a research-grade capability, not a proven market need. Defer past MVP.
- **Federation across forge instances (Forgejo's ActivityPub effort).** [FACT] Only Forgejo is actively building this, and it's still early; valuable for the "local-first, self-hostable" thesis long-term, but not required for MVP.
- **"Engineering digital twin" / full historical graph replay and simulation.** No evidence any competitor offers this; it is a natural extension of the knowledge-graph differentiator above but represents a multi-year research investment, not something to attempt in an MVP.

---

## Sources

Consolidated, deduplicated bibliography of every URL cited in this document (Phases 1–5), for later Traceability Matrix work. Format: Title — URL — accessed date. All entries accessed 2026-08-18.

- About GitHub Copilot cloud agent — https://docs.github.com/en/copilot/concepts/agents/cloud-agent/about-cloud-agent — accessed 2026-08-18
- Agent approvals & security | ChatGPT Learn — https://developers.openai.com/codex/agent-approvals-security — accessed 2026-08-18
- AI Gateway | GitLab Docs — https://docs.gitlab.com/administration/gitlab_duo/gateway/ — accessed 2026-08-18
- Best AI Coding Agents in 2026, Ranked — MightyBot — https://mightybot.ai/blog/coding-ai-agents-for-accelerating-engineering-workflows/ — accessed 2026-08-18
- Building a more resilient, multi-region Bitbucket Cloud — https://www.atlassian.com/blog/how-we-build/building-a-more-resilient-multi-region-bitbucket-cloud — accessed 2026-08-18
- Claude Code Advanced Best Practices 2026 — https://smartscope.blog/en/generative-ai/claude/claude-code-best-practices-advanced-2026/ — accessed 2026-08-18
- Claude Code Agent Teams, Subagents, and MCP: The 2026 Playbook — https://www.developersdigest.tech/blog/claude-code-agent-teams-subagents-2026 — accessed 2026-08-18
- Claude Code Skills in 2026: The Complete Guide — https://www.totalum.app/blog/claude-code-skills-totalum — accessed 2026-08-18
- Codex CLI Deep Dive: Config, Profiles, Sandbox 2026 — https://www.digitalapplied.com/blog/codex-cli-deep-dive-config-profiles-sandbox-2026 — accessed 2026-08-18
- Cursor 2026: Composer, Agent Mode, MCP & Background Agent — https://www.deployhq.com/guides/cursor — accessed 2026-08-18
- Evolving GitHub Advanced Security: Greater flexibility, easier to access — https://resources.github.com/evolving-github-advanced-security/ — accessed 2026-08-18
- Gitea vs Forgejo 2026: What's the Difference and Which to Self-Host? — https://contabo.com/blog/gitea-vs-forgejo/ — accessed 2026-08-18
- GitHub Advanced Security license billing — https://docs.github.com/en/billing/concepts/product-billing/github-advanced-security — accessed 2026-08-18
- GitHub availability report: April 2026 — https://github.blog/news-insights/company-news/github-availability-report-april-2026/ — accessed 2026-08-18
- GitHub availability report: May 2026 — https://github.blog/news-insights/company-news/github-availability-report-may-2026/ — accessed 2026-08-18
- GitHub Copilot Agent Mode and MCP in VS Code: 2026 Guide — https://www.itechguides.com/vibe-coding-with-github-copilot-agent-mode-and-mcp-in-vs-code-updated-for-2026/ — accessed 2026-08-18
- GitHub Copilot Coding Agent: The Complete Architecture Behind Agentic DevOps at Enterprise Scale — https://itnext.io/github-copilot-coding-agent-the-complete-architecture-behind-agentic-devops-at-enterprise-scale-1f42c1c132aa — accessed 2026-08-18
- GitHub Enterprise Pricing 2026: Cloud, Server & Add-ons — https://atonementlicensing.com/blog/github-enterprise-pricing-2026/ — accessed 2026-08-18
- GitHub Pricing — https://github.com/pricing — accessed 2026-08-18 (primary source, directly WebFetched in v1.1 pass)
- GitLab architecture overview — https://docs.gitlab.com/development/architecture — accessed 2026-08-18
- GitLab Duo Self-Hosted: Enterprise AI Built for Data Privacy — https://about.gitlab.com/blog/gitlab-duo-self-hosted-enterprise-ai-built-for-data-privacy/ — accessed 2026-08-18
- GitLab Duo vs Claude Code: Platform-Native DevSecOps or Terminal-First Autonomy? — https://www.augmentcode.com/tools/gitlab-duo-vs-claude-code — accessed 2026-08-18
- GitLab pricing 2026: Plans, tiers, and real costs — https://www.eesel.ai/blog/gitlab-pricing — accessed 2026-08-18
- gitlabhq/doc/development/architecture.md — https://github.com/gitlabhq/gitlabhq/blob/master/doc/development/architecture.md — accessed 2026-08-18
- Install the GitLab AI Gateway — https://docs.gitlab.com/install/install_ai_gateway/ — accessed 2026-08-18
- Is Linear Open Source? No — 3 Real Alternatives Compared (2026) — https://zoobbe.com/linear-alternative-open-source/ — accessed 2026-08-18
- Linear — AI PM Wiki | GenAI PM — https://genaipm.com/wiki/companies/linear — accessed 2026-08-18
- Linear 2026: $8/User Issue Tracker, GraphQL API | Automation Atlas — https://automationatlas.io/tools/linear/ — accessed 2026-08-18
- Linear AI 2026: What the Agent Does, Plan Pricing & Verdict — https://aiagentsquare.com/agents/linear-ai — accessed 2026-08-18
- Linear AI features: What the PM tool can do (2026) | eesel AI — https://www.eesel.ai/blog/linear-ai — accessed 2026-08-18
- Linear Pricing 2026: $0 Free, $10 Basic, $16 Business Plans — https://aiproductivity.ai/blog/linear-pricing/ — accessed 2026-08-18
- Linear Review 2026: 74/100 | AI PM Tools — https://aipmtools.org/project-management/linear — accessed 2026-08-18
- Linear Review 2026: The Fastest Project Management Tool Gets AI Agents | utilo — https://utilo.io/blog/linear-review-2026-project-management — accessed 2026-08-18
- Meet Rovo Chat in Bitbucket — https://www.atlassian.com/blog/bitbucket/rovo-chat-bitbucket-beta — accessed 2026-08-18
- Reviewing your organization's audit logs for GitHub Codespaces — https://docs.github.com/en/enterprise-cloud@latest/codespaces/managing-codespaces-for-your-organization/reviewing-your-organizations-audit-logs-for-github-codespaces — accessed 2026-08-18
- Shipyard: Codex CLI Cheatsheet — https://shipyard.build/blog/codex-cli-cheat-sheet/ — accessed 2026-08-18
- Sourcegraph Cody vs Qodo (2026) — https://www.augmentcode.com/tools/sourcegraph-cody-vs-qodo — accessed 2026-08-18
- The 6 Best Open-Source Devin Alternatives in 2026 — https://www.openhands.dev/blog/devin-ai-alternatives — accessed 2026-08-18
- The 9 Best Coding Agents in 2026, Ranked — https://www.openhands.dev/blog/best-coding-agents — accessed 2026-08-18
- The Atlassian Rovo MCP Server now supports Bitbucket Cloud — https://www.atlassian.com/blog/bitbucket/the-atlassian-rovo-mcp-server-now-supports-bitbucket-cloud — accessed 2026-08-18
- The Complete Guide to Cursor Pricing in 2026 — https://flexprice.io/blog/cursor-pricing-guide — accessed 2026-08-18
- Why GitLab Premium? — https://about.gitlab.com/pricing/premium/ — accessed 2026-08-18 (primary-source snippet via WebSearch; direct WebFetch of about.gitlab.com blocked by this environment's egress proxy)

Not directly WebFetched but found via WebSearch on primary/official domains (github.blog, docs.github.com, resources.github.com) in the v1.1 pass, tagged `[UNVERIFIED-FACT]` in-text pending a direct fetch:
- Evolving GitHub Advanced Security: Greater flexibility, easier to access — https://resources.github.com/evolving-github-advanced-security/
- GitHub Advanced Security license billing — https://docs.github.com/en/billing/concepts/product-billing/github-advanced-security

Attempted and blocked by this environment's egress proxy (could not be directly WebFetched in this pass): `about.gitlab.com`, `docs.github.com`.

---

## Next Phases

- **Phase 6 — Product Primitive Discovery**: pending.
- **Phase 7 — Requirements Elicitation**: pending.
- **Phase 8 — Requirements Specification (full 55-section document)**: pending.
- **Phase 9 — MVP Definition**: pending.
- **Phase 10 — Architecture Design**: pending.
- **Phase 11 — Red-Team Review**: pending.
- **Phase 12 — Stakeholder Validation**: pending.
- **Phase 13 — Final Baseline**: pending.
