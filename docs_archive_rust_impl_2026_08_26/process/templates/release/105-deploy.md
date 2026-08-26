# 生产部署记录 / Production Deploy Record

> **[TEMPLATE]** 本文件为空白模板，使用时按章节填入项目实际内容。
> 标记 `[TEMPLATE]` 的章节为必填；标记 `[OPTIONAL]` 的章节按项目需要决定是否填写。
> 标记 `[REFERENCE]` 的章节为参考链接，指向现有设计文档，不在本文件展开。

**关联信息 / Metadata:**

| 项 | 值 |
|---|---|
| 流程任务编号 | 105 |
| 阶段 | 发布 — 生产部署 |
| 主要交付物 | 部署结果 |
| 责任人 | SRE |
| 关联设计文档 | [`./104-prod-env.md`](./104-prod-env.md) · [`../../design/detailed-design/13-admin-api-and-ops-ui.md`](../../../design/detailed-design/13-admin-api-and-ops-ui.md) §13.5 |
| 模板版本 | v1.0 (2026-08-20) |
| 依据 | IPA 共通框架 2013 |

---


## 部署元数据 / Deploy Metadata

| 字段 | 值 |
|---|---|
| 部署时间 | YYYY-MM-DD HH:MM |
| 版本 | vX.Y.Z |
| commit | — |
| 镜像 SHA | — |

## 部署步骤 / Deploy Steps

| 步骤 | 完成时间 | 状态 |
|---|---|---|
| Helm install / kubectl apply | — | ✅ |
| Migration 跑完 | — | ✅ |
| Health check | — | ✅ |
| Smoke test | — | ✅ |

---

**导航 / Navigation:**
[← 流程总览](../../workflow.md) · [← 流程 README](../../README.md)
