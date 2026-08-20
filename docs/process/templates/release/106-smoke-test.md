# 启动确认报告 / Smoke Test Report

> **[TEMPLATE]** 本文件为空白模板，使用时按章节填入项目实际内容。
> 标记 `[TEMPLATE]` 的章节为必填；标记 `[OPTIONAL]` 的章节按项目需要决定是否填写。
> 标记 `[REFERENCE]` 的章节为参考链接，指向现有设计文档，不在本文件展开。

**关联信息 / Metadata:**

| 项 | 值 |
|---|---|
| 流程任务编号 | 106 |
| 阶段 | 发布 — 启动确认 (Smoke Test) |
| 主要交付物 | 启动确认报告 |
| 责任人 | QA + SRE |
| 关联设计文档 | [`./105-deploy.md`](./105-deploy.md) |
| 模板版本 | v1.0 (2026-08-20) |
| 依据 | IPA 共通框架 2013 |

---


## Smoke Test 用例

| ID | 用例 | 预期 | 实际 | 状态 |
|---|---|---|---|---|
| SMK-1 | 健康检查 /healthz | 200 | — | Pass/Fail |
| SMK-2 | 就绪检查 /readyz | 200 | — | — |
| SMK-3 | 登录 | 成功 | — | — |
| SMK-4 | 创建仓库 | 成功 | — | — |
| SMK-5 | 推送 commit | 成功 | — | — |

## 结论 / Conclusion


[TEMPLATE] All Pass / Some Failed (rollback triggered)


---

**导航 / Navigation:**
[← 流程总览](../../workflow.md) · [← 流程 README](../../README.md)
