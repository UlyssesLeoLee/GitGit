# 迁移确认书 / Migration Verification

> **[TEMPLATE]** 本文件为空白模板，使用时按章节填入项目实际内容。
> 标记 `[TEMPLATE]` 的章节为必填；标记 `[OPTIONAL]` 的章节按项目需要决定是否填写。
> 标记 `[REFERENCE]` 的章节为参考链接，指向现有设计文档，不在本文件展开。

**关联信息 / Metadata:**

| 项 | 值 |
|---|---|
| 流程任务编号 | 101 |
| 阶段 | 迁移 — 迁移结果确认 |
| 主要交付物 | 迁移确认书 |
| 责任人 | PM + DBA + SRE + PO |
| 关联设计文档 | [`./99-data-migration.md`](./99-data-migration.md) · [`./100-system-migration.md`](./100-system-migration.md) |
| 模板版本 | v1.0 (2026-08-20) |
| 依据 | IPA 共通框架 2013 |

---


## 迁移总评 / Migration Summary

| 项 | 结果 |
|---|---|
| 数据完整性 | ✅ 100% |
| RTO | ✅ ≤ 1h |
| RPO | ✅ ≤ 15min |
| 应用功能 | ✅ Smoke Test 通过 |
| 性能 | ✅ P99 ≤ 基线 × 1.2 |

## 遗留事项 / Outstanding Items


[TEMPLATE] 例：源端保留 7 天观察期。


## 签核 / Sign-off

| 角色 | 签核 | 日期 |
|---|---|---|
| PM | ☐ | — |
| DBA | ☐ | — |
| SRE | ☐ | — |
| PO | ☐ | — |

---

**导航 / Navigation:**
[← 流程总览](../../workflow.md) · [← 流程 README](../../README.md)
