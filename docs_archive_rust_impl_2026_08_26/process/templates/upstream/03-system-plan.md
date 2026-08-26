# 系统化计划书 / System Plan

> **[TEMPLATE]** 本文件为空白模板，使用时按章节填入项目实际内容。
> 标记 `[TEMPLATE]` 的章节为必填；标记 `[OPTIONAL]` 的章节按项目需要决定是否填写。
> 标记 `[REFERENCE]` 的章节为参考链接，指向现有设计文档，不在本文件展开。

**关联信息 / Metadata:**

| 项 | 值 |
|---|---|
| 流程任务编号 | 03 |
| 阶段 | 超上流 — 系统化计划 |
| 主要交付物 | 系统化计划书 |
| 责任人 | PM + 架构师 |
| 关联设计文档 | [`../requirements/phase9-mvp-reduction.md`](../../../requirements/phase9-mvp-reduction.md) (MVP 精简) · [`../requirements/phase10-architecture.md`](../../../requirements/phase10-architecture.md) (架构) |
| 模板版本 | v1.0 (2026-08-20) |
| 依据 | IPA 共通框架 2013 |

---


## 项目阶段总览 / Phase Overview


按 [`../workflow.md` 150 阶段](../../workflow.md) 切分。

| 阶段 | 时间 | 关键交付 |
|---|---|---|
| 超上流 (01-09) | Month 0-1 | 本目录全部文档 |
| 需求 (10-21) | Month 1-2 | [`../requirements/00-requirements-definition.md`](../../../requirements/00-requirements-definition.md) |
| 基本设计 (22-41) | Month 2-3 | 18 个基本设计文件 |
| 详细设计 (42-52) | Month 3-4 | 14 个详细设计文件 |
| 实现 (53-58) | Month 4-7 | MVP 可运行 |
| 测试 (59-89) | Month 7-9 | ST 通过 |
| UAT (90-95) | Month 9-10 | UAT 通过 |
| 迁移 / 发布 (96-108) | Month 10-11 | Go-Live |
| 运维 / 维护 (109-126) | Month 11+ | 持续 |

## 资源计划 / Resource Plan

| 角色 | 人数 | 投入月份 |
|---|---|---|
| 架构师 | 1 | Month 1-7 |
| 后端工程师 | 3 | Month 3-9 |
| 前端工程师 | 1 | Month 5-9 |
| 测试工程师 | 1 | Month 7-10 |
| 运维 / DBA | 1 | Month 8-11 |

## 主要里程碑 / Key Milestones

| 里程碑 | 日期 | 退出条件 |
|---|---|---|
| M1 立项 | Month 1 | 任务 09 完成 |
| M2 需求冻结 | Month 2 | 任务 21 Baseline |
| M3 设计冻结 | Month 4 | 任务 52 DD Review |
| M4 MVP Ready | Month 7 | 任务 65 UT 完成 |
| M5 系统测试通过 | Month 9 | 任务 89 ST 完成 |
| M6 UAT 通过 | Month 10 | 任务 95 验收证书 |
| M7 Go-Live | Month 11 | 任务 107 服务开始 |

## 预算 / Budget

| 类别 | 金额 | 备注 |
|---|---|---|
| 人力 | X 万元 | 见资源计划 |
| 基础设施 | Y 万元 | 云服务 / 自建硬件 |
| 软件许可 | Z 万元 | 第三方库 / 工具 |

---

**导航 / Navigation:**
[← 流程总览](../../workflow.md) · [← 流程 README](../../README.md)
