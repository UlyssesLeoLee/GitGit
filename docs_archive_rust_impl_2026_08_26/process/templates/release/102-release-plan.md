# 发布计划书 / Release Plan

> **[TEMPLATE]** 本文件为空白模板，使用时按章节填入项目实际内容。
> 标记 `[TEMPLATE]` 的章节为必填；标记 `[OPTIONAL]` 的章节按项目需要决定是否填写。
> 标记 `[REFERENCE]` 的章节为参考链接，指向现有设计文档，不在本文件展开。

**关联信息 / Metadata:**

| 项 | 值 |
|---|---|
| 流程任务编号 | 102 |
| 阶段 | 发布 — 发布计划 |
| 主要交付物 | 发布计划书 |
| 责任人 | PM + SRE |
| 关联设计文档 | [`../../design/basic-design/10-acceptance-test-policy.md`](../../../design/basic-design/10-acceptance-test-policy.md) §10.4 (发布判定) |
| 模板版本 | v1.0 (2026-08-20) |
| 依据 | IPA 共通框架 2013 |

---


## 发布信息 / Release Info

| 字段 | 值 |
|---|---|
| 版本号 | vX.Y.Z (semver) |
| 发布类型 | Major / Minor / Patch |
| 计划发布时间 | YYYY-MM-DD HH:MM |
| 发布窗口 | P0 维护时段 (周末 02:00-06:00) |

## 变更范围 / Change Scope

| 类型 | 数量 |
|---|---|
| 新功能 | — |
| 缺陷修复 | — |
| 性能改进 | — |
| 破坏性变更 | — |

## 前置条件 / Prerequisites

- [ ] UAT 通过 (任务 95)
- [ ] Release Notes 已发
- [ ] 回滚脚本就绪
- [ ] Hypercare 团队就位 (任务 108)

## 风险评估 / Risk Assessment

| 风险 | 概率 | 影响 | 缓解 |
|---|---|---|---|
| [TEMPLATE] | Low/Med/High | — | — |

---

**导航 / Navigation:**
[← 流程总览](../../workflow.md) · [← 流程 README](../../README.md)
