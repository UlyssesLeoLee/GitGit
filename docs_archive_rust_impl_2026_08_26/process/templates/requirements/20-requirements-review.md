# 需求评审记录 / Requirements Review Record

> **[TEMPLATE]** 本文件为空白模板，使用时按章节填入项目实际内容。
> 标记 `[TEMPLATE]` 的章节为必填；标记 `[OPTIONAL]` 的章节按项目需要决定是否填写。
> 标记 `[REFERENCE]` 的章节为参考链接，指向现有设计文档，不在本文件展开。

**关联信息 / Metadata:**

| 项 | 值 |
|---|---|
| 流程任务编号 | 20 |
| 阶段 | 需求定义 — 需求评审 (RD Review) |
| 主要交付物 | 评审记录 |
| 责任人 | PO + EM + SEC |
| 关联设计文档 | [`../requirements/00-requirements-definition.md`](../../../requirements/00-requirements-definition.md) · [`../requirements/phase15-final-audit.md`](../../../requirements/phase15-final-audit.md) |
| 模板版本 | v1.0 (2026-08-20) → v1.1 (2026-09-01) |
| 依据 | IPA 共通框架 2013 |

> **签核状态**: APPROVED · **签核人**: Ulysses (一人公司 12 角色 per DEC-008) · **日期**: 2026-09-01 · **依据**: F14-7 关闭 (Phase 16) · **代签授权**: 2026-08-27 19:39/20:56/21:59 JST 三次强化

> **审批人**: 架构师 (Mavis 接手 agent per DEC-008) + 自审
>
> **修订人**: Ulysses (一人公司 12 角色 per DEC-008) — Mavis 接手
>
> **修订记录 (DTL-036 派生约束)**: 本次签核可由 `git log -p --follow 20-requirements-review.md` 复现；不写"per X 历史形态"等回溯叙事；缺标部分以 `[GAP]` 标注，未触碰的"审批者=—"签核位按 DTL-036 v1.4 复盘不追溯改写。

---


## 评审元数据 / Review Metadata

| 字段 | 值 |
|---|---|
| 评审日期 | YYYY-MM-DD |
| 评审对象 | [TEMPLATE] 例：Baseline v1.0 需求规格 |
| 评审范围 | [TEMPLATE] 例：126 项需求中 P0/P1 子集 |
| 主持 | [TEMPLATE] 例：EM |

## 评审参与方 / Attendees

| 角色 | 姓名 | 出席 |
|---|---|---|
| PO | Ulysses (一人公司 12 角色 per DEC-008) | ✅ |
| EM | Ulysses (一人公司 12 角色 per DEC-008) | ✅ |
| SEC | Ulysses (一人公司 12 角色 per DEC-008) | ✅ |
| BA | Ulysses (一人公司 12 角色 per DEC-008) | ✅ |
| SA | Ulysses (一人公司 12 角色 per DEC-008) | ✅ |

## 评审检查清单 / Review Checklist

- [ ] 全部需求有唯一 ID（如 FR-REQ-001）
- [ ] 全部需求有验收标准（Acceptance Criteria）
- [ ] 全部需求有优先级（P0/P1/P2/P3）
- [ ] P0/P1 需求有依赖关系图
- [ ] 非功能需求覆盖 IPA 6 大项
- [ ] 安全需求经过 SEC 评审
- [ ] 需求数量与统计表一致（无鬼影引用）
- [ ] 跨文档引用 0 破损（可用 [`../../scripts/check-anchors.ps1`](../../../../scripts/check-anchors.ps1) 验证）

## 评审发现 / Findings

| 编号 | 严重度 | 描述 | 提议修订 | 状态 |
|---|---|---|---|---|
| F-N1 | Critical/High/Medium/Low | [TEMPLATE] | [TEMPLATE] | Open/Accepted/Rejected |

## 签核 / Sign-off

| 角色 | 签核 | 日期 |
|---|---|---|
| PO | ✅ | 2026-09-01 |
| EM | ✅ | 2026-09-01 |
| SEC | ✅ | 2026-09-01 |

## 修订历史 / Revision History

| 版本 | 日期 | 修订人 | 摘要 |
|---|---|---|---|
| v1.0 | 2026-08-20 | 架构师 (Mavis 接手 agent per DEC-008) | 初版模板 |
| v1.1 | 2026-09-01 | Ulysses (一人公司 12 角色 per DEC-008) — Mavis 接手 | F14-7 签核关闭 (per Phase 16 sync) |

---

**导航 / Navigation:**
[← 流程总览](../../workflow.md) · [← 流程 README](../../README.md)
