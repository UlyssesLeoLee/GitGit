# 10. 验收测试方针 / Acceptance Test Policy

> **[PROPOSAL] IPA 共通框架 2013 章节定位：** P9 软件受入过程 全部活动


## 10.1 测试级别 / Test Levels

| 级别 | 目的 | 负责人 | 工具 |
|---|---|---|---|
| 单元测试 | 函数 / 类级别的正确性 | 实现工程师 | 语言标准（Rust / Go test、pytest）|
| 集成测试 | 子系统间接口 | 实现工程师 | 语言标准 + Docker Compose 集成 |
| 系统测试 | 10 步参考循环的端到端 | QA / 工程师 | Playwright（UI）、curl（API）、自定义 E2E |
| 验收测试 | 需求定义书验收标准（§51）的符合度 | Product Owner | 清单 + 自动回归 |
| 安全测试 | OWASP Top 10、CWE Top 25、AI 特有威胁 | 安全负责人 | ZAP、CodeQL、自定义提示词注入套件 |
| 性能测试 | NFR 需求（[§6 非功能设计](06-non-functional-design.md)）的符合度 | 工程师 | k6、Locust、wrk、pgbench |
| 灾难恢复测试 | 备份 / 恢复 | 运维负责人 | 自动化脚本 |

## 10.2 MVP 验收条件（重申 Phase 9 §6 的 Definition of Done）

`[PROPOSAL]` MVP 的 Definition of Done 须满足以下：

1. 10 步参考循环（Repository → Issue → AI Context → Agent Branch → Code Change → CI → AI Review → Human Approval → Merge → Engineering Graph Update）能用一个 E2E 测试走完
2. 图查询（Issue → 下游 4 hop 所有 Node）实现并满足 Phase 9 DoD 第 9 步
3. Chaos test：AI Gateway 在运行中被 kill 后，Git 操作仍能继续（GIT-REQ-010）
4. 安全测试：AISEC-REQ 6 条攻击场景被阻止
5. Policy 测试：没有人类审批时，Agent 的 `merge` 类操作被拒绝
6. 导出 / 导入：round-trip 结构 diff 为空（无差异）

## 10.3 验收测试清单 / Acceptance Checklist

针对每项 MVP 需求（37 项）验证以下：

- [ ] 实现与需求 ID（如 GIT-REQ-001）相关联
- [ ] 自动测试覆盖需求的验收标准
- [ ] 测试结果以需求 ID 索引化
- [ ] 相关文档（运维 Runbook、UI 文本）已更新
- [ ] 安全评审已完成
- [ ] 性能测试满足 NFR 暂定等级（[§6 非功能设计](06-non-functional-design.md)）

## 10.4 发布判定 / Release Decision

| 发布判定项 | 标准 |
|---|---|
| 功能覆盖率 | MVP 37 项需求 100% 实现 + 测试通过 |
| 重大 Bug | Critical / High Bug 0 件 |
| 安全 | AISEC-REQ 6 条攻击测试全通过 + AISEC-REQ-009(a) DB 角色级强制验证 |
| 性能 | NFR 暂定等级达成（[TBD] Benchmark）|
| 文档 | 安装 / 运维 / 安全指南完成 |
| 批准 | Product Owner 正式签核（Phase 14 F14-7 仍未解决）|

## 10.5 回归测试策略 / Regression Test Strategy

- 全部 E2E 测试套件随每次 PR 自动执行
- 每夜运行全 E2E + 性能测试
- 发布前运行安全测试 + 灾难恢复测试
- 测试结果与需求 ID 关联后保留 30 天以上

---

**导航 / Navigation:**
[← 09. 迁移设计](09-migration-design.md) · [README](README.md) · [11. API 设计 →](11-api-design.md)
