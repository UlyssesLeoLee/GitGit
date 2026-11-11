# 风险登记 / Risk Register

> **[TEMPLATE]** 本文件为空白模板，使用时按章节填入项目实际内容。
> 标记 `[TEMPLATE]` 的章节为必填；标记 `[OPTIONAL]` 的章节按项目需要决定是否填写。
> 标记 `[REFERENCE]` 的章节为参考链接，指向现有设计文档，不在本文件展开。

**关联信息 / Metadata:**

| 项 | 值 |
|---|---|
| 流程任务编号 | 135 |
| 阶段 | 管理 |
| 主要交付物 | 风险登记表 |
| 责任人 | EM |
| 关联设计文档 | [`../architecture/qa-checklist.md`](../architecture/qa-checklist.md) (26 项实施前 QA) |
| 模板版本 | v1.0 (2026-08-20) |
| 依据 | IPA 共通框架 2013 |

---


本表跟踪**项目执行期**的运行时风险（工期 / 资源 / 依赖 / 干系人 / 外部）。**架构 / 技术风险**走 [`../architecture/qa-checklist.md`](../architecture/qa-checklist.md)。


## 风险条目模板 / Entry Template

| 字段 | 说明 |
|---|---|
| 编号 | RISK-NNN |
| 日期 | YYYY-MM-DD（识别日期） |
| 类别 | 工期 / 资源 / 技术 / 依赖 / 干系人 / 外部 |
| 描述 | 风险是什么 |
| 影响 | 如果发生，影响什么（范围 / 进度 / 成本 / 质量） |
| 概率 | Low / Medium / High |
| 影响等级 | Low / Medium / High / Critical |
| 风险分数 | 概率 × 影响（1-9） |
| 缓解措施 | 预防 + 应急 |
| 责任人 | 谁负责监控 |
| 状态 | Open / Mitigated / Closed / Accepted |

## 本项目当前风险列表（待填）


[TEMPLATE] 实际使用时请按 RISK-001 起编号，逐条记录。


## 已知启动风险（来自本项目历史）

| 编号 | 类别 | 描述 | 影响 | 缓解 |
|---|---|---|---|---|
| RISK-001 | 干系人 | F14-7 — 全流程无人类签核（Phase 14 finding） | 质量保证链断裂 | Phase 16 收尾时显式声明缺口；正式项目必须有 PO 签核 |
| RISK-002 | 技术 | gix 与 Rust 生态兼容风险（QA-011） | 读路径实现延期 | 锁 gix 0.x 版本；CI 中加 compat test |
| RISK-003 | 依赖 | sqlx 编译期 SQL 校验依赖测试 DB（QA-010） | CI 启动慢 | 用 sqlx-cli prepare 预生成 .sqlx 目录 |
| RISK-004 | 外部 | AI Provider API 稳定性（QA-018） | AI Gateway 故障 | 多 provider fallback + 本地缓存 |

---

**导航 / Navigation:**
[← 流程总览](workflow.md) · [← 流程 README](README.md)
