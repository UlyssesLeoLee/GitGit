# 决策日志 / Decision Log

> **[TEMPLATE]** 本文件为空白模板，使用时按章节填入项目实际内容。
> 标记 `[TEMPLATE]` 的章节为必填；标记 `[OPTIONAL]` 的章节按项目需要决定是否填写。
> 标记 `[REFERENCE]` 的章节为参考链接，指向现有设计文档，不在本文件展开。

**关联信息 / Metadata:**

| 项 | 值 |
|---|---|
| 流程任务编号 | 131-144 |
| 阶段 | 管理 |
| 主要交付物 | 决策日志 |
| 责任人 | EM |
| 关联设计文档 | [`../../../../architecture/decisions/`](../../../../architecture/decisions/) (技术 ADR) · [`../architecture/qa-checklist.md`](../architecture/qa-checklist.md) |
| 模板版本 | v1.0 (2026-08-20) |
| 依据 | IPA 共通框架 2013 |

---


本日志记录**非技术**的流程 / 范围 / 资源类决策。**技术架构决策**走 [`../../../../architecture/decisions/`](../../../../architecture/decisions/) 的 ADR 流程。


## 决策条目模板 / Entry Template

| 字段 | 说明 |
|---|---|
| 编号 | DEC-NNN（按时间顺序递增） |
| 日期 | YYYY-MM-DD |
| 提出方 | 谁提出的决策（个人 / 角色） |
| 决策方 | 谁最终拍板（须有签核权） |
| 上下文 | 为什么需要这个决策 |
| 选项 | A / B / C / ... |
| 决策 | 选了哪个 + 为什么 |
| 影响 | 对范围 / 进度 / 成本 / 风险的影响 |
| 状态 | Proposed / Accepted / Deprecated / Superseded |

## 示例条目（不构成实际决策记录）


> DEC-001 · 2026-08-20 · 提出方：EM · 决策方：EM + PO


> **上下文**: MVP 范围 37 项 vs 43 项（Phase 13）vs 45 项（Phase 14）


> **选项**: A. 严格 MVP 37 / B. 加上 Phase 13 补的 4 项 / C. 再加 Phase 14 补的 2 项


> **决策**: C（45 项）— 因为 Phase 14 的 SEC-REQ-008/009/010 是 Phase 11 红队的 Critical / High 级发现，删了等于丢掉红队结论


> **影响**: 范围 +2 项，工期 +1 周；安全风险降低


> **状态**: Accepted


## 本项目实际决策列表（待填）


[TEMPLATE] 实际使用时请按 DEC-001 起编号，逐条记录。


---

**导航 / Navigation:**
[← 流程总览](workflow.md) · [← 流程 README](README.md)
