# 测试规格书 / Test Specification

> **[TEMPLATE]** 本文件为空白模板，使用时按章节填入项目实际内容。
> 标记 `[TEMPLATE]` 的章节为必填；标记 `[OPTIONAL]` 的章节按项目需要决定是否填写。
> 标记 `[REFERENCE]` 的章节为参考链接，指向现有设计文档，不在本文件展开。

**关联信息 / Metadata:**

| 项 | 值 |
|---|---|
| 流程任务编号 | 60 / 67 / 77 / 91 |
| 阶段 | 测试 — 测试规格书 |
| 主要交付物 | 测试规格书 |
| 责任人 | QA |
| 关联设计文档 | [`./test-plan.md`](test-plan.md) · [`../../design/basic-design/10-acceptance-test-policy.md`](../../../design/basic-design/10-acceptance-test-policy.md) |
| 模板版本 | v1.0 (2026-08-20) |
| 依据 | IPA 共通框架 2013 |

---


## 测试用例 / Test Cases

| ID | 需求 | 标题 | 前置条件 | 步骤 | 预期 | 实际 | 状态 | 严重度 |
|---|---|---|---|---|---|---|---|---|
| TC-001 | REQ-NNN | [TEMPLATE] | [TEMPLATE] | [TEMPLATE] | [TEMPLATE] | — | Pass/Fail/Blocked | — |

## 测试矩阵 / Coverage Matrix

| 需求 ID | TC 数量 | 状态 |
|---|---|---|
| REQ-1 | — | — |

## 非功能测试 / Non-Functional Test


### 性能

| 指标 | 目标 | 测试方法 |
|---|---|---|
| API 响应 P99 | [TEMPLATE] – TBD Benchmark | k6 |

### 安全

- [ ] OWASP Top 10 覆盖
- [ ] SQL 注入 / XSS / CSRF 拒绝
- [ ] 权限提升拒绝

### 兼容

| OS | 架构 | 支持 |
|---|---|---|
| Linux | x86_64 / aarch64 | P0 |
| macOS | aarch64 | P0 |
| Windows | x86_64 + WSL2 | P1 |

---

**导航 / Navigation:**
[← 流程总览](../../workflow.md) · [← 流程 README](../../README.md)
