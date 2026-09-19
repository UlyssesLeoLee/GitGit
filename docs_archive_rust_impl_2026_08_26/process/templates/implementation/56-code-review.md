# 代码评审检查单 / Code Review Checklist

> **[TEMPLATE]** 本文件为空白模板，使用时按章节填入项目实际内容。
> 标记 `[TEMPLATE]` 的章节为必填；标记 `[OPTIONAL]` 的章节按项目需要决定是否填写。
> 标记 `[REFERENCE]` 的章节为参考链接，指向现有设计文档，不在本文件展开。

**关联信息 / Metadata:**

| 项 | 值 |
|---|---|
| 流程任务编号 | 56 |
| 阶段 | 实现 — 代码评审 (CR) |
| 主要交付物 | CR 记录 |
| 责任人 | TL + 实施工程师 |
| 关联设计文档 | [`../../architecture/tech-selection.md`](../../../architecture/tech-selection.md) §15.1 (REQ-TS-001 禁 unsafe) |
| 模板版本 | v1.0 (2026-08-20) |
| 依据 | IPA 共通框架 2013 |

---


## PR 元数据 / PR Metadata

| 字段 | 值 |
|---|---|
| PR 编号 | #NNN |
| 作者 | — |
| 评审者 | — |
| 变更 LOC | +X / -Y |

## 评审检查清单 / Checklist


### 正确性 / Correctness

- [ ] 实现符合详细设计规格
- [ ] 边界条件（空集 / 极大 / 并发）有处理
- [ ] 错误路径有测试覆盖

### 安全 / Security

- [ ] 无 SQL 拼接（用参数化查询）
- [ ] 无 unsafe 块
- [ ] 敏感字段已 redact（日志 / span）
- [ ] 权限校验放在最前

### 性能 / Performance

- [ ] 无 N+1 查询
- [ ] 热路径有基准测试
- [ ] 大对象未走 Clone（用引用 / Arc）

### 可维护性 / Maintainability

- [ ] 公开 API 有 doc 注释
- [ ] 函数长度 ≤ 100 行
- [ ] 嵌套深度 ≤ 4 层
- [ ] 无重复代码（DRY）

### 测试 / Testing

- [ ] 单元测试覆盖新代码
- [ ] 失败用例先写（红 → 绿 → 重构）
- [ ] 无 flaky test

## 评审记录 / Comments


[TEMPLATE] 用 PR 评论逐项记录，发现按 `nit / suggestion / request changes / blocking` 分级。


## 签核 / Approval

| 评审者 | 签核 | 日期 |
|---|---|---|
| Reviewer 1 | ☐ | — |
| Reviewer 2 (TL) | ☐ | — |

---

**导航 / Navigation:**
[← 流程总览](../../workflow.md) · [← 流程 README](../../README.md)
