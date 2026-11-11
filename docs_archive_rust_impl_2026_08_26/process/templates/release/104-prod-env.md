# 生产环境构建记录 / Production Environment Build

> **[TEMPLATE]** 本文件为空白模板，使用时按章节填入项目实际内容。
> 标记 `[TEMPLATE]` 的章节为必填；标记 `[OPTIONAL]` 的章节按项目需要决定是否填写。
> 标记 `[REFERENCE]` 的章节为参考链接，指向现有设计文档，不在本文件展开。

**关联信息 / Metadata:**

| 项 | 值 |
|---|---|
| 流程任务编号 | 104 |
| 阶段 | 发布 — 生产环境构建 |
| 主要交付物 | 生产环境就绪 |
| 责任人 | SRE + DBA |
| 关联设计文档 | [`../../design/basic-design/08-operations-design.md`](../../../design/basic-design/08-operations-design.md) §8.6 (V1+ K8s) · [`../../design/detailed-design/13-admin-api-and-ops-ui.md`](../../../design/detailed-design/13-admin-api-and-ops-ui.md) §13.5 |
| 模板版本 | v1.0 (2026-08-20) |
| 依据 | IPA 共通框架 2013 |

---


## 环境清单 / Environment Inventory

| 资源 | 规格 | 数量 | 状态 |
|---|---|---|---|
| K8s cluster | vX.Y | 1 | ✅ |
| PostgreSQL | primary + 2 replica | 3 | ✅ |
| Object storage (Git LFS) | S3-compatible | 1 | ✅ |
| Load balancer | L7 | 1 | ✅ |

## 安全基线 / Security Baseline

- [ ] mTLS 启用
- [ ] NetworkPolicy 启用
- [ ] Pod Security Standard = restricted
- [ ] Secrets 全部经 Vault / KMS
- [ ] 审计日志开启 (admin_audit)

## 备份配置 / Backup

- [ ] PITR 配置 (WAL 流式 + 每日 base backup)
- [ ] RPO ≤ 15min, RTO ≤ 1h

---

**导航 / Navigation:**
[← 流程总览](../../workflow.md) · [← 流程 README](../../README.md)
