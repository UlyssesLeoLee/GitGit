# 补丁应用报告 / Patch Apply Report

> **[TEMPLATE]** 本文件为空白模板，使用时按章节填入项目实际内容。
> 标记 `[TEMPLATE]` 的章节为必填；标记 `[OPTIONAL]` 的章节按项目需要决定是否填写。
> 标记 `[REFERENCE]` 的章节为参考链接，指向现有设计文档，不在本文件展开。

**关联信息 / Metadata:**

| 项 | 值 |
|---|---|
| 流程任务编号 | 122 |
| 阶段 | 维护 — 补丁应用 (Patch) |
| 主要交付物 | 补丁应用报告 |
| 责任人 | 实施 + SRE |
| 关联设计文档 | [`./123-vulnerability.md`](./123-vulnerability.md) · [`../../design/basic-design/07-security-design.md`](../../../design/basic-design/07-security-design.md) §7.4 |
| 模板版本 | v1.0 (2026-08-20) |
| 依据 | IPA 共通框架 2013 |

---


## 补丁元数据 / Patch Metadata

| 字段 | 值 |
|---|---|
| PATCH ID | PATCH-NNN |
| 来源 | GitHub Advisory / RustSec / NVD |
| CVE | — |
| 严重度 | Critical/High/Medium/Low |

## 影响范围 / Affected

| 依赖 | 影响版本 | 修复版本 |
|---|---|---|
| [TEMPLATE] | < X.Y.Z | ≥ X.Y.Z |

## 应用步骤 / Apply Steps

- [ ] [TEMPLATE] 升级 Cargo.toml
- [ ] [TEMPLATE] cargo update
- [ ] [TEMPLATE] 重新构建 + 跑回归
- [ ] [TEMPLATE] 部署到 staging
- [ ] [TEMPLATE] 部署到生产

## 回滚 / Rollback


[TEMPLATE] cargo update --precise + 重建。


---

**导航 / Navigation:**
[← 流程总览](../../workflow.md) · [← 流程 README](../../README.md)
