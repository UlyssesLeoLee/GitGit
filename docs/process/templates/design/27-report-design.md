# 帐票设计书 / Report Design

> **[TEMPLATE]** 本文件为空白模板，使用时按章节填入项目实际内容。
> 标记 `[TEMPLATE]` 的章节为必填；标记 `[OPTIONAL]` 的章节按项目需要决定是否填写。
> 标记 `[REFERENCE]` 的章节为参考链接，指向现有设计文档，不在本文件展开。

**关联信息 / Metadata:**

| 项 | 值 |
|---|---|
| 流程任务编号 | 27 |
| 阶段 | 基本设计 — 帐票设计 |
| 主要交付物 | 帐票设计书 |
| 责任人 | BA + 实施工程师 |
| 关联设计文档 | [`../../design/basic-design/04-data-design.md`](../../../design/basic-design/04-data-design.md) (数据设计) · [`../../design/basic-design/05-interface-design.md`](../../../design/basic-design/05-interface-design.md) (接口设计) |
| 模板版本 | v1.0 (2026-08-20) |
| 依据 | IPA 共通框架 2013 |

---


> **[TEMPLATE]** 本平台 MVP 不输出传统纸质/Excel 帐票；本模板为预留结构，便于 V1+ Cloud 模式下的报表扩展。


## 帐票清单 / Report Inventory

| 编号 | 名称 | 类型 | 数据源 | 频率 | 受众 |
|---|---|---|---|---|---|
| RPT-1 | [TEMPLATE] | PDF/CSV/HTML | [TEMPLATE] | Daily/Weekly/Monthly | [TEMPLATE] |

## 每个帐票的设计


### 帐票 RPT-N

| 项 | 值 |
|---|---|
| 编号 | RPT-N |
| 名称 | [TEMPLATE] |
| 目的 | [TEMPLATE] |
| 数据源 | [TEMPLATE] 例：Node / Edge / Event 表 SQL 视图 |
| 列定义 | [TEMPLATE] 见下方 |
| 过滤条件 | [TEMPLATE] 例：按时间段 + 仓库 |
| 排序 | [TEMPLATE] |
| 输出格式 | [TEMPLATE] |
| 分页 | [TEMPLATE] |
| 权限 | [TEMPLATE] 例：仅 SEC 角色可见 |

## 实现方式 / Implementation

- [ ] 服务端渲染：模板引擎（如 Handlebars）
- [ ] 异步生成：长任务走队列 + 邮件通知
- [ ] 权限：见 [`../../design/basic-design/07-security-design.md`](../../../design/basic-design/07-security-design.md) §7.1

---

**导航 / Navigation:**
[← 流程总览](../../workflow.md) · [← 流程 README](../../README.md)
