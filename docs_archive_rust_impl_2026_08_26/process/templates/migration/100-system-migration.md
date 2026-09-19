# 系统迁移结果 / System Migration Result

> **[TEMPLATE]** 本文件为空白模板，使用时按章节填入项目实际内容。
> 标记 `[TEMPLATE]` 的章节为必填；标记 `[OPTIONAL]` 的章节按项目需要决定是否填写。
> 标记 `[REFERENCE]` 的章节为参考链接，指向现有设计文档，不在本文件展开。

**关联信息 / Metadata:**

| 项 | 值 |
|---|---|
| 流程任务编号 | 100 |
| 阶段 | 迁移 — 系统迁移 |
| 主要交付物 | 系统迁移结果 |
| 责任人 | SRE |
| 关联设计文档 | [`./97-migration-procedure.md`](./97-migration-procedure.md) · [`./99-data-migration.md`](./99-data-migration.md) |
| 模板版本 | v1.0 (2026-08-20) |
| 依据 | IPA 共通框架 2013 |

---


## 切换步骤 / Cutover Steps

| 步骤 | 完成时间 | 状态 |
|---|---|---|
| 应用只读 | — | ✅ |
| 流量切到目标 | — | ✅ |
| 健康检查 | — | ✅ |
| 应用恢复读写 | — | ✅ |

## 切换期间指标 / Cutover Metrics

| 指标 | 值 |
|---|---|
| 停机时长 | X min (目标 ≤ 60 min) |
| 流量切换 | DNS TTL = 300s |
| 错误率 | ≤ 0.1% |

---

**导航 / Navigation:**
[← 流程总览](../../workflow.md) · [← 流程 README](../../README.md)
