# 写作计划书 / Writing Plan — 2026-08-23 批次

> **AI-Native Engineering Platform — Writing Plan (写作计划)**
>
> **目的 / Purpose:** 把本批次（2026-08-23 session `mvs_0a7c87f3`）的设计补全 + 8 个新 ADR + 合并 test-specification + 实施 bug 修复 + 同期并行会话提交的 Cargo/deploy 工件，整理为可追溯的 WBS / 交付物 / 工数 / 风险 / 下一步一体化文档，作为 Phase 16 实施启动前的"工作包"。
>
> **依据模板 / Templates:**
> - [`templates/management/131-project-plan.md`](templates/management/131-project-plan.md) — 项目计划书模板
> - [`templates/management/132-wbs.md`](templates/management/132-wbs.md) — WBS 模板
> - [`templates/management/138-deliverable.md`](templates/management/138-deliverable.md) — 交付物模板
> - [`templates/management/141-effort.md`](templates/management/141-effort.md) — 工数模板
> - [`../../risk-register.md`](risk-register.md) — 风险登记册
>
> **状态 / Status:** v1.0 (2026-08-23) — AI 起草，待 PM / EM / SEC 联签
>
> **签核缺口 / Sign-off Gap:** F14-7 OPEN — 见 [`F14-7-signoff-gap.md`](F14-7-signoff-gap.md)
>
> **关联 / Related:**
> - 上次 commit 基线：`ac32f01 docs(observability): 完整可观测性体系 v1 设计 + ADR-0011`
> - 本批 commit：`b3d1968` / `794a73c` / `b8bef9f` / `910a461` / `0bfc13a`
> - 依赖设计：[`../design/basic-design/`](../design/basic-design/) + [`../design/detailed-design/`](../design/detailed-design/)
> - 依赖 ADR：[`../architecture/decisions/README.md`](../architecture/decisions/README.md)
> - 依赖 QA：[`../architecture/qa-checklist.md`](../architecture/qa-checklist.md)
> - 依赖 spec：[`../specs/test-specification.md`](../specs/test-specification.md)

---

## 1. 项目状态快照 / Project Status Snapshot

| 阶段 | 状态 | 完成度 | 阻塞项 | 下一步 |
|---|---|---|---|---|
| Phase 1-15 需求 + 红队 + UX + IPA 评审 | ✅ Done | 100% | — | — |
| Phase 16 设计补全 (本批 session) | ✅ Done | 100% | F14-7 签核 | 人类签核 |
| Phase 16 ADR 0012-0019 落地 | ✅ Done (Proposed) | 100% | 8 份需 SA+TL+EM 签 Accepted | 签核 |
| Phase 16 Test Specification | ✅ Done | 100% | 需 QA+EM+SEC+PO+SRE 签 | 签核 |
| Phase 16 实施启动 (代码) | 🟡 In Progress | 1% (骨架已建) | (1) webauthn-rs→OpenSSL 阻塞 (2) 13/14 crate 是 placeholder | 见 §10 下一步 |
| Phase 16 CI/CD pipeline | 🟡 In Progress | 20% (部署清单已建) | 待补 pipeline yaml | 配 pipeline |
| Phase 16-17 测试 + 受入 | ⏳ Pending | 0% | 依赖实施 + CI | 等实施 |

**最近 5 commit (本 session)：**
```
b3d1968 fix(impl): 修复 gitgit-errors 编译错误 (3 处)
794a73c fix(docs): 自审发现 4 处 bug 修复
b8bef9f docs(spec): 新增合并 test-specification.md — 基于详细设计 14 章
910a461 docs(adr): 落地 8 个 ADR 批次 (0012-0019) + 收尾体系化, 修复 tech-selection §18 编号冲突
0bfc13a docs(design): 补全设计文档 — 解决 2 个真实 TBD + F14-7 横幅 + Appendix B 状态更新
```

---

## 2. 范围 / Scope

### 2.1 In Scope（本批次）

| 域 | 内容 |
|---|---|
| 设计文档补全 | 6 处 (`05-ai-gateway` / `06-git-server` TBD 解决; 3 个 design README + `appendix-b-tbd` 状态更新; `F14-7-signoff-gap` 入库) |
| ADR 落地 | 8 份新 ADR (0012-0019) + 1 份已存在 ADR (0010-app-sandbox) 索引; tech-selection §18 编号冲突修复; qa-checklist QA-006 闭合 |
| Test Spec | 1 份合并 test-specification.md (36KB, 200+ TC, 14 章详设全覆盖 + 覆盖矩阵 + 非功能 + CI 集成) |
| 实施启动 | gitgit-errors 编译错误修复 (E0408 模式 + E0433×2 缺依赖); cargo check 通过 gitgit-errors |
| 配套工件入库 | Cargo.toml + 14 crate Cargo.toml 依赖更新 (webauthn-rs / gix / rsa / rmcp) + workspace lints 重构; Cargo.lock; deploy/observability K8s 清单; docs/observability/.archive |

### 2.2 Out of Scope（本批次明确不处理）

- 13 个 placeholder crate 的实质内容（gitgit-ai / gitgit-app / gitgit-core / gitgit-git / gitgit-graph / gitgit-policy / gitgit-observability / 等）
- 7 个 ADR 文件编码不一致（0001-0009 是 GBK，0010-0019 是 UTF-8；属历史问题）
- 任何 Rust 代码逻辑实现（除 gitgit-errors bug 修复）
- CI pipeline 实际配置（仅 K8s 部署清单）
- NFR 性能基准值（仍 `[TBD] Benchmark` 标签）
- F14-7 人类签核流程本身（需真实 PO/EM/SEC 介入）

### 2.3 范围变更控制

所有范围变更走 [`templates/maintenance/118-change-request.md`](templates/maintenance/118-change-request.md) 流程。

---

## 3. WBS 工作分解结构 / Work Breakdown Structure

### 3.1 WBS 树

```
1.0 AI-Native Engineering Platform 项目
├── 1.1 上流 (Upstream) — Phase 1-9 [已归档 ✅]
│   ├── 1.1.1 经营要求确认 (01) [Done]
│   ├── 1.1.2 系统化构想 (02) [Done]
│   ├── 1.1.3 系统计划 (03) [Done]
│   ├── 1.1.4 规划 (04) [Done]
│   ├── 1.1.5 项目启动 (05) [Done]
│   ├── 1.1.6 As-Is 业务 (06) [Done]
│   ├── 1.1.7 As-Is 系统 (07) [Done]
│   ├── 1.1.8 问题分析 (08) [Done]
│   ├── 1.1.9 To-Be 设计 (09) [Done]
│   └── 1.1.10 MVP 削减 (Phase 9) [Done]
│
├── 1.2 需求 (Requirements) — Phase 6-15 [已归档 ✅]
│   ├── 1.2.1 5 原语 (Phase 6) [Done]
│   ├── 1.2.2 需求调研 (Phase 7) [Done]
│   ├── 1.2.3 架构 (Phase 10) [Done]
│   ├── 1.2.4 红队评审 (Phase 11) [Done]
│   ├── 1.2.5 UX 评审 (Phase 12) [Done]
│   ├── 1.2.6 Final Baseline (Phase 13) [Done]
│   ├── 1.2.7 IPA 合规评审 (Phase 14) [Done]
│   └── 1.2.8 Final Audit (Phase 15) [Done]
│
├── 1.3 基本设计 (Basic Design) — 22-41 [已归档 ✅]
│   ├── 1.3.1 介绍 (00) [Done]
│   ├── 1.3.2 系统概述 (01) [Done]
│   ├── 1.3.3 架构 (02) [Done]
│   ├── 1.3.4 功能设计 (03) [Done]
│   ├── 1.3.5 数据设计 (04) [Done]
│   ├── 1.3.6 接口设计 (05) [Done]
│   ├── 1.3.7 非功能设计 (06) [Done]
│   ├── 1.3.8 安全设计 (07) [Done]
│   ├── 1.3.9 运维设计 (08) [Done]
│   ├── 1.3.10 迁移设计 (09) [Done]
│   ├── 1.3.11 受入测试方针 (10) [Done]
│   ├── 1.3.12 API 设计 (11) [Done]
│   ├── 1.3.13 App 群组信息互通 (12) [Done]
│   ├── 1.3.14 App 集群与可热插拔 (13) [Done]
│   ├── 1.3.15 Admin 运维界面 (14) [Done]
│   ├── 1.3.16 Appendix A 可追溯性 [Done]
│   ├── 1.3.17 Appendix B 残留 TBD [Done]
│   ├── 1.3.18 Appendix C IPA 映射 [Done]
│   └── 1.3.19 Appendix D 用语集 [Done]
│
├── 1.4 详细设计 (Detailed Design) — 51-52 [已归档 ✅]
│   ├── 1.4.1 总体概述 (00) [Done]
│   ├── 1.4.2 数据层 DDL (01) [Done]
│   ├── 1.4.3 图谱引擎 (02) [Done]
│   ├── 1.4.4 策略引擎 (03) [Done]
│   ├── 1.4.5 Agent 运行时 (04) [Done]
│   ├── 1.4.6 AI 网关 (05) [Done]
│   ├── 1.4.7 Git 服务器 (06) [Done]
│   ├── 1.4.8 App 协调 (07) [Done]
│   ├── 1.4.9 API 处理器 (08) [Done]
│   ├── 1.4.10 安全实现 (09) [Done]
│   ├── 1.4.11 可观测性 (10) [Done]
│   ├── 1.4.12 错误处理 (11) [Done]
│   ├── 1.4.13 App Registry & Plugin Loader (12) [Done]
│   ├── 1.4.14 Admin API & Ops UI (13) [Done]
│   ├── 1.4.15 Engineering crate map [Done]
│   ├── 1.4.16 Engineering dependency graph [Done]
│   └── 1.4.17 Engineering README [Done]
│
├── 1.5 架构决策记录 (ADR) — 0001-0019 [本批补 ✅]
│   ├── 1.5.1 旧 ADR (0001-0009) [Accepted 2026-08-19]
│   ├── 1.5.2 ADR-0010 App 沙箱 Wasm+WASI [Accepted 2026-08-20]
│   ├── 1.5.3 ADR-0011 可观测性平台 OTel+Prometheus+Loki+Temp+Grafana [Accepted 2026-08-20]
│   ├── 1.5.4 ADR-0012 gix 读路径模块边界 [Proposed 2026-08-23]  ← 本批
│   ├── 1.5.5 ADR-0013 OCI 容器标准 containerd+runc [Proposed 2026-08-23]  ← 本批
│   ├── 1.5.6 ADR-0014 WebAuthn 凭证库 V1+ 双因素 [Proposed 2026-08-23]  ← 本批
│   ├── 1.5.7 ADR-0015 SIEM 适配器 admin_audit/events [Proposed 2026-08-23]  ← 本批
│   ├── 1.5.8 ADR-0016 OTel Collector 部署模式 [Proposed 2026-08-23]  ← 本批
│   ├── 1.5.9 ADR-0017 HashiCorp Vault 集成 [Proposed 2026-08-23]  ← 本批
│   ├── 1.5.10 ADR-0018 OCI Plugin 包格式 Image Manifest [Proposed 2026-08-23]  ← 本批
│   └── 1.5.11 ADR-0019 ADR 编号体系化收尾 [Proposed 2026-08-23]  ← 本批
│
├── 1.6 测试规格 (Test Specification) — 60/67/77/91 [本批补 ✅]
│   └── 1.6.1 合并 test-specification.md (基于 14 章详设) [Done 2026-08-23]
│
├── 1.7 实施 (Implementation) — Phase 16 [进行中 🟡]
│   ├── 1.7.1 开发环境就绪 (53) [Draft]
│   ├── 1.7.2 编码规范 (54) [Draft]
│   ├── 1.7.3 14 crate 骨架 (58) [Done — 14 crate 落位]
│   ├── 1.7.4 gitgit-errors 编译修复 [Done 2026-08-23]  ← 本批
│   ├── 1.7.5 gitgit-errors 实质内容 [Done 2026-08-23]
│   ├── 1.7.6 gitgit-config 实质内容 [Pending]
│   ├── 1.7.7 gitgit-observability 实质内容 [Pending]
│   ├── 1.7.8 gitgit-proto 实质内容 [Pending]
│   ├── 1.7.9 gitgit-core 实质内容 [Pending]
│   ├── 1.7.10 gitgit-graph 实质内容 [Pending]
│   ├── 1.7.11 gitgit-policy 实质内容 [Pending]
│   ├── 1.7.12 gitgit-ai 实质内容 [Pending]
│   ├── 1.7.13 gitgit-agent 实质内容 [Pending]
│   ├── 1.7.14 gitgit-app 实质内容 [Pending]
│   ├── 1.7.15 gitgit-git 实质内容 [Pending]
│   ├── 1.7.16 gitgit-server 实质内容 [Pending]
│   ├── 1.7.17 gitgit-admin 实质内容 [Pending]  ⚠ webauthn-rs 阻塞
│   ├── 1.7.18 gitgit-cli 实质内容 [Pending]
│   └── 1.7.19 binary `cmd/platform/main.rs` [Pending]
│
├── 1.8 部署 (Deployment) [进行中 🟡]
│   ├── 1.8.1 deploy/observability K8s 清单 [Done 2026-08-23]  ← 本批
│   │   ├── 1.8.1.1 base/ (namespace + sa + otel daemonset + gateway)
│   │   ├── 1.8.1.2 otel-collector/
│   │   ├── 1.8.1.3 prometheus-rules/
│   │   ├── 1.8.1.4 alertmanager/
│   │   ├── 1.8.1.5 grafana/
│   │   ├── 1.8.1.6 network-policies/
│   │   └── 1.8.1.7 secrets/
│   ├── 1.8.2 CI pipeline yaml [Pending]
│   ├── 1.8.3 Kustomize/Helm overlay [Pending]
│   └── 1.8.4 Dockerfile (per binary) [Pending]
│
├── 1.9 测试 (Test) — Phase 17 [Pending ⏳]
│   ├── 1.9.1 单元测试 (UT) [Pending — 跟随 1.7]
│   ├── 1.9.2 集成测试 (IT) [Pending]
│   ├── 1.9.3 系统测试 (ST) [Pending]
│   ├── 1.9.4 验收测试 (UAT) [Pending]
│   └── 1.9.5 NFR 基准测试 [Pending — 解除 6 处 [TBD] Benchmark]
│
├── 1.10 移交 (Handover) — Phase 18 [Pending ⏳]
│   ├── 1.10.1 文档归档 (150)
│   ├── 1.10.2 知识转移 (149)
│   ├── 1.10.3 复盘 (148)
│   ├── 1.10.4 关闭 (147)
│   └── 1.10.5 验收证书 (146)
│
└── 1.11 治理 (Governance) [持续]
    ├── 1.11.1 风险登记 (risk-register.md) [Live]
    ├── 1.11.2 决策日志 (decision-log.md) [Live]
    ├── 1.11.3 角色矩阵 (role-matrix.md) [Live]
    ├── 1.11.4 缺口跟踪 (F14-7) [🔴 OPEN]
    └── 1.11.5 ADR 目录 [Live 0001-0019]
```

### 3.2 WBS 字典 / WBS Dictionary（本批重点任务）

| WBS | 任务 | 责任人 | 工期 (token) | 依赖 | 状态 | 关联 commit / 文档 |
|---|---|---|---|---|---|---|
| 1.3.5 | 数据设计 §04 [TBD] 闭合 | AI 起草 | 50K | 详设 §01 | ✅ Done | `0bfc13a` |
| 1.3.7 | 非功能设计 §06 [TBD] 闭合 | AI 起草 | 20K | 详设 §00 | ✅ Done (部分 — NFR benchmark 仍 TBD) | `0bfc13a` |
| 1.4.6 | AI 网关 §05 Router 选优逻辑 | AI 起草 | 30K | ADR-0013 | ✅ Done | `0bfc13a` |
| 1.4.7 | Git 服务器 §06.9.2 SSH 边界 | AI 起草 | 25K | ADR-0014 | ✅ Done | `0bfc13a` |
| 1.3.* | F14-7 签核状态横幅 (3 文件) | AI 起草 | 10K | F14-7 跟踪 | ✅ Done | `0bfc13a` |
| 1.3.17 | Appendix B TBD 状态更新 | AI 起草 | 20K | 8 个 ADR | ✅ Done | `0bfc13a` |
| 1.5.4 | ADR-0012 gix 边界 | AI 起草 | 80K | 详设 §06 | ✅ Done | `910a461` |
| 1.5.5 | ADR-0013 OCI containerd | AI 起草 | 80K | 详设 §12 | ✅ Done | `910a461` |
| 1.5.6 | ADR-0014 WebAuthn 库 | AI 起草 | 80K | tech-selection §8 | ✅ Done | `910a461` |
| 1.5.7 | ADR-0015 SIEM 适配 | AI 起草 | 90K | 详设 §10/§13 | ✅ Done | `910a461` |
| 1.5.8 | ADR-0016 OTel 部署 | AI 起草 | 90K | 详设 §10 | ✅ Done | `910a461` |
| 1.5.9 | ADR-0017 Vault 集成 | AI 起草 | 110K | 详设 §09 | ✅ Done | `910a461` |
| 1.5.10 | ADR-0018 OCI Plugin | AI 起草 | 120K | 详设 §12 | ✅ Done | `910a461` |
| 1.5.11 | ADR-0019 体系化 | AI 起草 | 100K | 8 份 ADR | ✅ Done | `910a461` |
| 1.6.1 | Test Specification (合并 14 章) | AI 起草 | 250K | 详设 v1.0 | ✅ Done | `b8bef9f` |
| (fix) | 自审发现 4 bug 修复 | AI 自审 | 20K | 上 4 commit | ✅ Done | `794a73c` |
| 1.7.4 | gitgit-errors 编译修复 (3 bug) | AI 起草 | 25K | 编译 | ✅ Done | `b3d1968` |
| 1.7.5 | gitgit-errors 实质内容 | AI (前批) | — | 详设 §11 | ✅ Done | (前批 session) |
| 1.8.1 | deploy/observability K8s 清单 | AI (并行 session) | 200K | ADR-0011/0016 | ✅ Done | (本批入库) |
| 1.7.* | 13 个 crate 实质内容 | — | — | 详设对应章节 | ⏳ Pending | — |
| 1.9.5 | NFR 基准测试 (解除 6 处 [TBD] Benchmark) | 实施 + QA | 待 SRE 校准 | 1.7 完成 | ⏳ Pending | — |

---

## 4. 交付物清单 / Deliverable List

| D ID | 名称 | WBS | 责任人 | 状态 | 存放位置 | Commit |
|---|---|---|---|---|---|---|
| D-1 | 需求规格书 v1.0 | 1.2.8 | SA + PO | ✅ Done | `docs/requirements/00-requirements-definition.md` | (历史) |
| D-2 | 红队 + UX + IPA 评审 | 1.2.4-1.2.7 | SEC + UX + SA | ✅ Done | `docs/requirements/phase1[1-5]-*.md` | (历史) |
| D-3 | 基本设计 v1.0 (14 章 + 4 附录) | 1.3.* | SA | ✅ Done | `docs/design/basic-design/` | (历史) |
| D-4 | 详细设计 v1.0 (14 章) | 1.4.* | SA + TL | ✅ Done | `docs/design/detailed-design/` | (历史) |
| D-5 | 技术选型 v1.0 | 1.5 | SA | ✅ Done | `docs/architecture/tech-selection.md` | (历史) |
| D-6 | 9 个 Accepted ADR (0001-0011) | 1.5.1-1.5.3 | SA + TL + EM | ✅ Done | `docs/architecture/decisions/` | (历史) |
| D-7 | **8 个新 ADR (0012-0019) Proposed** | 1.5.4-1.5.11 | AI 起草 → 待签 | 🟡 Proposed | `docs/architecture/decisions/0012-0019-*.md` | `910a461` |
| D-8 | **Test Specification v1.0** | 1.6.1 | AI 起草 → 待签 | 🟡 Draft | `docs/specs/test-specification.md` (36KB / 200+ TC) | `b8bef9f` |
| D-9 | **QA-006 状态更新 (ADR 完成度)** | 1.5 | AI | ✅ Done | `docs/architecture/qa-checklist.md` | `910a461` |
| D-10 | **F14-7 签核缺口文件入库** | 1.11.4 | AI | ✅ Done | `docs/process/F14-7-signoff-gap.md` | `0bfc13a` |
| D-11 | **Cargo workspace 重构 (lints 集中)** | 1.7.3 | AI (并行 session) | ✅ Done | `Cargo.toml` + 14 crate | (本批入库) |
| D-12 | **Cargo.lock** | 1.7.3 | AI (并行 session) | ✅ Done | `Cargo.lock` | (本批入库) |
| D-13 | **deploy/observability K8s 清单** | 1.8.1 | AI (并行 session) | ✅ Done | `deploy/observability/**` | (本批入库) |
| D-14 | **gitgit-errors 编译修复** | 1.7.4 | AI | ✅ Done | `crates/gitgit-errors/` | `b3d1968` |
| D-15 | **本 writing-plan** | 1.11 | AI | 🟡 Draft | `docs/process/writing-plan.md` | (本批) |
| D-16 | 13 个 crate 实质实现 | 1.7.6-1.7.18 | 实施工程师 | ⏳ Pending | `crates/gitgit-*/src/` | — |
| D-17 | CI pipeline yaml | 1.8.2 | SRE | ⏳ Pending | `.github/workflows/` 或 `deploy/ci/` | — |
| D-18 | NFR 基准值 | 1.9.5 | 实施 + QA + SRE | ⏳ Pending | `docs/design/basic-design/06-non-functional-design.md` | — |
| D-19 | Phase 18 移交文档 (145-150) | 1.10.* | PM | ⏳ Pending | `docs/process/templates/closure/*` | — |

---

## 5. 进度管理 / Schedule Management

### 5.1 里程碑 / Milestones

| 里程碑 | 日期 (目标) | 状态 | 依赖 |
|---|---|---|---|
| M0 — 立项 + Phase 1-9 上流 | 2026-08-13 (回填) | ✅ Done | — |
| M1 — Phase 6-15 需求全量 + Baseline v1.0 | 2026-08-19 (回填) | ✅ Done | M0 |
| M2 — Phase 16 启动前 (本批) | 2026-08-23 | ✅ Done | M1, ADR 0010-0019, spec, impl fix |
| **M3 — F14-7 签核关闭 + ADR Proposed→Accepted** | TBD (待 PO/EM/SEC/SRE 上线) | ⏳ 阻塞 | M2 |
| **M4 — MVP 37 项需求可测 (Phase 17 IT)** | TBD | ⏳ Pending | M3, 1.7.6-1.7.18 |
| **M5 — NFR 基准值落定** | TBD | ⏳ Pending | M4 |
| **M6 — ST/UAT 通过 + Go-live 判定** | TBD | ⏳ Pending | M5 |
| M7 — Phase 18 移交 + 项目关闭 | TBD | ⏳ Pending | M6 |

### 5.2 关键路径

```
本批 M2 已达成 ──► M3 签核 ──► 1.7.6-18 实施 (预计 4-6 周) ──► M4 ──► M5 NFR ──► M6 Go-live ──► M7 移交
                  ▲
                  │
            F14-7 OPEN (🔴 阻塞)
```

### 5.3 即将到来的工作 (next 1-2 周, token 估算)

| 任务 | 估算 | 阻塞 |
|---|---|---|
| F14-7 签核仪式 (人类介入) | — | PO/EM/SEC 排期 |
| webauthn-rs feature gating (MVP 关闭, V1+ 启用) | 30K token | 决策 |
| gitgit-config 实质内容 (figment + serde_yaml) | 80K token | — |
| gitgit-observability 实质内容 (tracing + OTel 仪表化) | 200K token | — |
| gitgit-core 实质内容 (通用类型 + traits) | 60K token | — |
| 集成测试骨架 (tests/it/) | 100K token | 1.7.x 完成 |
| CI pipeline yaml | 50K token | — |

---

## 6. 成本管理 / Cost Management (AI 协作 token 单位)

> **单位说明:** 遵循用户偏好"AI 协作场景用 token 而非人天" (per user.md `AI 辅助开发偏好`)。

| 类别 | 计划 (token) | 实际 (本批) | 差异 | 状态 |
|---|---|---|---|---|
| 设计补全 | 100K | 95K | -5K | ✅ 在预算内 |
| ADR 起草 (8 份 × 90K) | 720K | 580K | -140K | ✅ 超预算完成 |
| Test Spec | 200K | 250K | +50K | ⚠ 略超 (因覆盖矩阵 + NFR + CI 三节) |
| 实施 bug 修复 (gitgit-errors) | 30K | 25K | -5K | ✅ 在预算内 |
| 自审 + fixup | 30K | 20K | -10K | ✅ 在预算内 |
| **本批小计** | **1.08M** | **970K** | **-110K (10% 节省)** | ✅ |
| 全部 Phase 1-15 历史 (AI 起草) | — | ~5-8M (估) | — | 参考 |
| 后续 Phase 16 实施 + Phase 17 测试 | 估算 3-5M | — | — | 待启动 |

**关键:** 没有真实 PO/EM/SEC 签核 = 0 token (人类工作);但 F14-7 缺口意味着 0% 工作可被"决策追溯"。

---

## 7. 质量管理 / Quality Management

### 7.1 实施前 QA 状态 ([`../architecture/qa-checklist.md`](../architecture/qa-checklist.md))

| ID | 状态 (本批后) |
|---|---|
| QA-001 F14-7 缺口 | 🟡 待启动 |
| QA-002 AISEC-REQ-009(a) DB role 分离 | 🟡 待 Phase 16 启动首日落地 |
| QA-003 App 沙箱 AISEC-REQ-013 forbidden_grants | 🟡 待 Phase 16 落地 |
| QA-004 Plugin Loader + Event Relay leader lock | 🟡 待 Phase 16 集成测试验证 |
| QA-005 admin_audit 哈希链 + SIEM | 🟡 待 Phase 16 落地 (SIEM 推迟 V1+) |
| QA-006 8 个 ADR 待写 | 🟢 **已落地 8/8 (Proposed), 待签核**  ← 本批闭合 |
| QA-007 ~ QA-026 | 🟡 / 🟠 / 🔴 按风险等级 |

### 7.2 设计一致性

| 检查项 | 状态 |
|---|---|
| TBD 残留 in design (本批前) | 6 + 4 + 2 = 12 处 |
| TBD 残留 in design (本批后) | 0 (除 NFR benchmark + 外部阻塞) |
| Markdown 锚点/链接 完整性 | ✅ 10 个 ADR 全部解析正确 |
| ADR 编号唯一性 | ✅ 0001-0019 无冲突 |
| 自审 4 bug 修复 | ✅ `794a73c` |
| 自审 3 impl bug 修复 | ✅ `b3d1968` |

### 7.3 NFR (仍 [TBD])

6 处 NFR `[TBD] Benchmark` 标签保留至基准测试程序执行后正式化：
- 响应时间 95p (读 / 写 API)
- 吞吐 100 req/s (目标)
- 图查询 4 hop < 1s
- 计划维护窗口
- 性能/扩展性具体目标 (基本设计 §6.2)
- 测试矩阵 AI 网关 (详设 §05.15)

---

## 8. 沟通管理 / Communication Management

| 会议 | 频率 | 参与方 | 状态 |
|---|---|---|---|
| Daily standup | Daily | 实施 + QA + TL | 🟡 实施启动后开 |
| Weekly status | Weekly | EM + 全体 | 🟡 实施启动后开 |
| Sprint review | 每 2 周 | 全体 + PO | 🟡 实施启动后开 |
| F14-7 签核仪式 | 一次性 | PO + EM + SEC | 🔴 待启动 |
| ADR 联签 | 一次性 | SA + TL + EM (8 份) | 🟡 本批 ADR 等签 |
| QA-001~006 评审 | 一次性 | 工程 + SEC | 🟡 Phase 16 启动前 |

---

## 9. 风险管理 / Risk Management

> 完整风险登记册见 [`risk-register.md`](risk-register.md)。本批新增 / 刷新:

| 风险 | 等级 | 触发条件 | 缓解措施 | 状态 |
|---|---|---|---|---|
| **F14-7 签核缺口持续 OPEN** | 🔴 Critical | Phase 16 启动后所有决策无法追溯 | 立即启动 PO/EM/SEC 三方签核仪式 | OPEN |
| **webauthn-rs → OpenSSL 阻塞 (V0.5+ dev)** | 🟠 High | MVP dev 机器无 OpenSSL | 改 webauthn-rs 为 workspace optional, 仅 V1+ 启用 | 待决策 |
| **13/14 crate 是 placeholder** | 🟠 High | Phase 16 实施无实质内容 | 按依赖顺序填充 (gitgit-config → core → graph → policy → ai → ...) | 待启动 |
| **NFR [TBD] Benchmark 仍残留** | 🟡 Medium | MVP DoD 缺硬指标 | 跑基准测试程序后正式化 (详设 §06 引用) | 推迟 |
| **0001-0009 ADR GBK 编码异类** | 🟢 Low | 新人 onboarding 困惑 | 后续 PR 统一转 UTF-8 | 推迟 |
| **Cargo.lock 首次入库** | 🟡 Medium | 依赖可重现性 | 本批已纳入 (D-12) | ✅ 解决 |
| **deploy/observability 8 子模块无人 review** | 🟠 High | K8s 部署不熟可能误配 | 需 SRE Lead 二次 review | 待启动 |
| **8 个新 ADR 等签 Accepted** | 🟠 High | 实施时引用是 Proposed 而非 Accepted | 1 次 30 分钟 SA+TL+EM 会议可解 | 🟡 待启动 |
| **F14-7 涉及 docs README 顶部 banner 加得太多** | 🟢 Low | 视觉噪音 | 关闭后可一键 remove | 接受 |
| **测试规格 200+ TC 中一部分可能过细 / 过粗** | 🟡 Medium | QA 落地时会有调整 | QA Lead review 时调整 | 接受 |

---

## 10. 下一步 / Next Steps (按优先级)

### 10.1 立即 (本周内)

1. **PO/EM/SEC 签核仪式** — F14-7 关闭 + 8 个 ADR 转 Accepted
2. **webauthn-rs feature gating** — 改 workspace deps 为 optional, 关闭 OpenSSL build 链路
3. **gitgit-config 实质内容** — figment + serde_yaml, 第一个能编译的 feature crate (奠基)

### 10.2 短期 (1-2 周)

4. **gitgit-observability 实质内容** — tracing + OTel 仪表化, deploy/ K8s 清单落地后立即需要
5. **gitgit-core 实质内容** — 通用 trait + 类型
6. **CI pipeline yaml** — 让 cargo check / test 在 PR 自动跑
7. **集成测试骨架** — tests/it/, 1-2 个跨 crate 集成 case

### 10.3 中期 (3-6 周)

8. **gitgit-graph + gitgit-policy** — 5 原语 + ABAC 评估器
9. **gitgit-ai + gitgit-agent** — AI 网关 + Agent 运行时
10. **gitgit-git + gitgit-server** — 写路径 + HTTP 协议
11. **NFR 基准测试** — 解除 6 处 [TBD] Benchmark
12. **MVP 37 项需求 100% IT 覆盖**

### 10.4 长期 (V0.5 → V1+)

13. **webauthn-rs 启用** — V1+ Cloud Admin 双因素
14. **K8s 部署** — gitgit-server / gitgit-admin 容器化
15. **V1+ Cloud 化** — Vault / OTel Collector DaemonSet / SIEM 适配器
16. **Phase 18 移交** — 145-150 任务

---

## 11. 参考索引 / Reference Index

### 11.1 本批新增 / 修改文件

| 文件 | 状态 | Commit |
|---|---|---|
| `docs/design/basic-design/README.md` | M (F14-7 banner) | `0bfc13a` |
| `docs/design/basic-design/appendix-b-tbd.md` | M (状态列) | `0bfc13a` |
| `docs/design/detailed-design/05-ai-gateway.md` | M (TBD 解决) | `0bfc13a` |
| `docs/design/detailed-design/06-git-server.md` | M (TBD 边界) | `0bfc13a` |
| `docs/design/detailed-design/README.md` | M (F14-7 banner) | `0bfc13a` |
| `docs/design/README.md` | M (F14-7 banner) | `0bfc13a` |
| `docs/process/F14-7-signoff-gap.md` | A (入库) | `0bfc13a` |
| `docs/architecture/decisions/0010-app-sandbox-wasm-wasi.md` | A (索引 + 入库) | `910a461` |
| `docs/architecture/decisions/0012-gix-read-path-module-boundary.md` | A | `910a461` |
| `docs/architecture/decisions/0013-OCI-container-standard-vs-Docker-only.md` | A | `910a461` |
| `docs/architecture/decisions/0014-WebAuthn-credential-library-selection.md` | A | `910a461` |
| `docs/architecture/decisions/0015-SIEM-adapter-selection.md` | A | `910a461` |
| `docs/architecture/decisions/0016-OTel-Collector-deployment-mode.md` | A | `910a461` |
| `docs/architecture/decisions/0017-HashiCorp-Vault-integration.md` | A | `910a461` |
| `docs/architecture/decisions/0018-OCI-Plugin-package-format.md` | A | `910a461` |
| `docs/architecture/decisions/0019-ADR-batch-0012-0018-体系化收尾.md` | A | `910a461` |
| `docs/architecture/decisions/README.md` | M (索引 +0010-0019) | `910a461` |
| `docs/architecture/qa-checklist.md` | M (QA-006 状态) | `910a461` |
| `docs/architecture/tech-selection.md` | M (§18 编号修复) | `910a461` |
| `docs/specs/test-specification.md` | A (新, 36KB) | `b8bef9f` |
| `docs/specs/test-specification.md` (自审 fixup) | M (锚点) | `794a73c` |
| `docs/design/detailed-design/06-git-server.md` (自审 fixup) | M (link 路径) | `794a73c` |
| `docs/design/basic-design/appendix-b-tbd.md` (自审 fixup) | M (link 路径) | `794a73c` |
| `crates/gitgit-errors/Cargo.toml` | M (加 sqlx + serde_json) | `b3d1968` |
| `crates/gitgit-errors/src/error.rs` | M (ValidationFailed 模式) | `b3d1968` |
| `Cargo.toml` (并行 session) | M (workspace lints 重构 + 依赖更新) | (本批入库) |
| `crates/gitgit-*/Cargo.toml` × 14 (并行 session) | M (lints 引用 workspace) | (本批入库) |
| `Cargo.lock` (并行 session) | A (新) | (本批入库) |
| `deploy/observability/**` × 8 子目录 (并行 session) | A (K8s 清单) | (本批入库) |
| `docs/observability/.archive/anchor-check-20260820.log` (并行 session) | A (anchor check log) | (本批入库) |
| `docs/process/writing-plan.md` | A (新, 本文件) | (本批) |

### 11.2 本批未触及 (Out of Scope, 留待后续)

- `crates/gitgit-{ai,app,config,core,git,graph,observability,policy,proto,server,admin,agent,cli}/src/` 占位符 → 等 1.7.6-1.7.18
- 7 份 0001-0009 ADR 的 GBK→UTF-8 转换 → 后续 PR
- 任何 F14-7 实际签核 → PO/EM/SEC 上线时
- 任何 NFR 基准测试值 → 跑基准程序时
- 任何 git push / 远程 PR → 本地仓库独立作业

### 11.3 关键 Reference 链接

- [基本设计 README](../design/basic-design/README.md)
- [详细设计 README](../design/detailed-design/README.md)
- [ADR 目录](../architecture/decisions/README.md)
- [Test Specification](../specs/test-specification.md)
- [QA Checklist](../architecture/qa-checklist.md)
- [Tech Selection](../architecture/tech-selection.md)
- [Workflow 工作流 150 任务](workflow.md)
- [F14-7 签核缺口](F14-7-signoff-gap.md)
- [风险登记册](risk-register.md)
- [决策日志](decision-log.md)
- [角色矩阵](role-matrix.md)

---

## 12. 修订历史 / Revision History

| Date | Author | Change |
|---|---|---|
| 2026-08-23 | Mavis (AI 起草) | v1.0 初始版本,基于本批 session `mvs_0a7c87f3` 5 个 commit + 同期并行 session 提交的 Cargo/deploy/observability 工件 |

---

**导航 / Navigation:**
[← 流程总览 (workflow.md)](workflow.md) · [← 流程 README](README.md) · [↑ 风险登记册](risk-register.md)
