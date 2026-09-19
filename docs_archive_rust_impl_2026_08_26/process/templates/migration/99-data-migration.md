# 数据迁移结果 / Data Migration Result

> **[TEMPLATE]** 本文件为空白模板，使用时按章节填入项目实际内容。
> 标记 `[TEMPLATE]` 的章节为必填；标记 `[OPTIONAL]` 的章节按项目需要决定是否填写。
> 标记 `[REFERENCE]` 的章节为参考链接，指向现有设计文档，不在本文件展开。

**关联信息 / Metadata:**

| 项 | 值 |
|---|---|
| 流程任务编号 | 99 |
| 阶段 | 迁移 — 数据迁移 |
| 主要交付物 | 数据迁移结果 |
| 责任人 | DBA |
| 关联设计文档 | [`./97-migration-procedure.md`](./97-migration-procedure.md) · [`./98-migration-rehearsal.md`](./98-migration-rehearsal.md) |
| 模板版本 | v1.0 (2026-08-20) |
| 依据 | IPA 共通框架 2013 |

---


## 迁移元数据 / Metadata

| 字段 | 值 |
|---|---|
| 开始时间 | YYYY-MM-DD HH:MM |
| 结束时间 | YYYY-MM-DD HH:MM |
| 数据量 | — |
| 操作人 | DBA |

## 校验结果 / Verification

| 表 / 对象 | 源端行数 | 目标端行数 | 差异 | 状态 |
|---|---|---|---|---|
| nodes | — | — | 0 | ✅ |
| edges | — | — | 0 | ✅ |
| events | — | — | 0 | ✅ |
| secrets (加密) | — | — | 0 | ✅ |

## 数据抽样校验 / Sample Verification


[TEMPLATE] 例：抽取 1000 条 node，hash 比对 100% 匹配。


---

**导航 / Navigation:**
[← 流程总览](../../workflow.md) · [← 流程 README](../../README.md)
