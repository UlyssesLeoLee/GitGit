# 发布判定书 / Go/No-Go Decision

> **[TEMPLATE]** 本文件为空白模板，使用时按章节填入项目实际内容。
> 标记 `[TEMPLATE]` 的章节为必填；标记 `[OPTIONAL]` 的章节按项目需要决定是否填写。
> 标记 `[REFERENCE]` 的章节为参考链接，指向现有设计文档，不在本文件展开。

**关联信息 / Metadata:**

| 项 | 值 |
|---|---|
| 流程任务编号 | 103 |
| 阶段 | 发布 — 发布判定 (Go/No-Go) |
| 主要交付物 | 发布判定书 |
| 责任人 | PO + EM + SRE |
| 关联设计文档 | [`./102-release-plan.md`](./102-release-plan.md) · [`../../design/basic-design/10-acceptance-test-policy.md`](../../../design/basic-design/10-acceptance-test-policy.md) §10.4 |
| 模板版本 | v1.0 (2026-08-20) |
| 依据 | IPA 共通框架 2013 |

---


## Go/No-Go 检查清单 / Gate Checklist

- [ ] UAT 验收证书已签发 (任务 95)
- [ ] 性能 benchmark 满足 Provisional 等级 (QA-009)
- [ ] 安全测试通过 (QA-002/003/005/012/021)
- [ ] 72h soak test 通过 (QA-019)
- [ ] Release Notes 已审
- [ ] Hypercare 团队就位 (任务 108)
- [ ] 回滚方案演练通过 (任务 98)
- [ ] 无 P0 缺陷遗留

## 决策 / Decision

| 选项 | 选择 | 理由 |
|---|---|---|
| Go | ☐ | — |
| No-Go | ☐ | — |
| 延期至 | — | — |

## 签核 / Sign-off

| 角色 | 签核 | 日期 |
|---|---|---|
| PO | ☐ | — |
| EM | ☐ | — |
| SRE | ☐ | — |

---

**导航 / Navigation:**
[← 流程总览](../../workflow.md) · [← 流程 README](../../README.md)
