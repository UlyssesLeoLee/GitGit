# 测试计划 / Test Plan (UT / IT / ST / UAT 通用模板)

> **[TEMPLATE]** 本文件为空白模板，使用时按章节填入项目实际内容。
> 标记 `[TEMPLATE]` 的章节为必填；标记 `[OPTIONAL]` 的章节按项目需要决定是否填写。
> 标记 `[REFERENCE]` 的章节为参考链接，指向现有设计文档，不在本文件展开。

**关联信息 / Metadata:**

| 项 | 值 |
|---|---|
| 流程任务编号 | 59 / 66 / 76 / 90 |
| 阶段 | 单元测试 / 集成测试 / 系统测试 / 验收测试 |
| 主要交付物 | 测试计划 |
| 责任人 | QA + EM |
| 关联设计文档 | [`../../design/basic-design/10-acceptance-test-policy.md`](../../../design/basic-design/10-acceptance-test-policy.md) §10.1 (测试级别) |
| 模板版本 | v1.0 (2026-08-20) |
| 依据 | IPA 共通框架 2013 |

---


> **[TEMPLATE]** 本模板通用于 UT / IT / ST / UAT 四个测试阶段；填写时请在顶部标注本计划适用的测试级别与对应的需求 / 设计章节。


## 测试级别 / Test Level

| 级别 | 适用范围 | 责任方 | 对应任务 |
|---|---|---|---|
| UT | 单元 / 函数 / 类 | 实施工程师 | 59-65 |
| IT | 模块 / 服务 / DB | QA + 实施 | 66-75 |
| ST | 全系统 + 性能 + 安全 | QA + SEC | 76-89 |
| UAT | 业务场景 + 验收 | PO + QA | 90-95 |

## 测试范围 / Scope


### In Scope

- [ ] [TEMPLATE] 例：所有 P0 需求 100% 覆盖
- [ ] [TEMPLATE] 例：所有 P1 需求 80% 覆盖

### Out of Scope

- [ ] [TEMPLATE] 例：性能压测（走 §80 PT）
- [ ] [TEMPLATE] 例：第三方 SaaS 兼容性

## 测试策略 / Strategy

| 类型 | 方法 | 工具 |
|---|---|---|
| 功能 | 等价类 + 边界值 | — |
| 性能 | k6 / wrk | — |
| 安全 | OWASP Top 10 + 红队清单 | — |
| 兼容 | 矩阵测试 | — |

## 环境 / Environment

| 环境 | 配置 | 数据 |
|---|---|---|
| dev | 本地 | 合成数据 |
| staging | 类生产 | 脱敏快照 |
| prod-like | K8s staging cluster | 脱敏快照 |

## 进入 / 退出条件 / Entry & Exit


### 进入条件

- [ ] 详细设计已 Baseline（任务 52）
- [ ] 测试环境就绪
- [ ] 测试规格书已签核

### 退出条件

- [ ] 全部用例通过
- [ ] 缺陷密度 ≤ 1/KLOC
- [ ] 无 Critical / High 缺陷遗留

## 缺陷管理 / Defect Management

| 严重度 | 响应时间 | 修复时间 | 验证 |
|---|---|---|---|
| Critical | 立即 | 24h | Retest |
| High | 8h | 3d | Retest |
| Medium | 24h | 1w | Retest |
| Low | 1w | 下个迭代 | Retest |

## 风险 / Risks

- [ ] [TEMPLATE] 例：测试数据不足

## 签核 / Sign-off

| 角色 | 签核 | 日期 |
|---|---|---|
| QA | ☐ | — |
| EM | ☐ | — |
| PO (UAT only) | ☐ | — |

---

**导航 / Navigation:**
[← 流程总览](../../workflow.md) · [← 流程 README](../../README.md)
