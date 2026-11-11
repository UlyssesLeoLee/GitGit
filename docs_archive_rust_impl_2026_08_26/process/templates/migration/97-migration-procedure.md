# 迁移步骤书 / Migration Procedure

> **[TEMPLATE]** 本文件为空白模板，使用时按章节填入项目实际内容。
> 标记 `[TEMPLATE]` 的章节为必填；标记 `[OPTIONAL]` 的章节按项目需要决定是否填写。
> 标记 `[REFERENCE]` 的章节为参考链接，指向现有设计文档，不在本文件展开。

**关联信息 / Metadata:**

| 项 | 值 |
|---|---|
| 流程任务编号 | 97 |
| 阶段 | 迁移 — 迁移步骤制定 |
| 主要交付物 | 迁移步骤书 |
| 责任人 | DBA + SRE |
| 关联设计文档 | [`./96-migration-plan.md`](./96-migration-plan.md) · [`../../design/basic-design/09-migration-design.md`](../../../design/basic-design/09-migration-design.md) |
| 模板版本 | v1.0 (2026-08-20) |
| 依据 | IPA 共通框架 2013 |

---


## 前置检查 / Pre-flight

- [ ] 源端 schema 版本与目标端一致
- [ ] 目标端容量 ≥ 源端 × 1.5
- [ ] 网络带宽与延迟满足 SLO
- [ ] 回滚脚本已就绪

## 步骤清单 / Procedure

| 步骤 | 命令 / 工具 | 超时 | 回滚命令 |
|---|---|---|---|
| 1. 停止源端写入 | 应用只读模式 | — | 恢复读写 |
| 2. pg_dump | pg_dump -Fc | X min | — |
| 3. git bundle | git bundle create | X min | — |
| 4. 加密导出包 | tar + gpg | X min | — |
| 5. 传输 | rsync / sftp | X min | — |
| 6. 完整性校验 | sha256sum -c | 5 min | — |
| 7. 目标端 restore | pg_restore | X min | — |
| 8. git clone | git clone bundle | X min | — |
| 9. 数据校验 | 行数 + checksum 对比 | X min | — |
| 10. 切换流量 | DNS / LB | 5 min | 切回源端 |

## 校验标准 / Verification

| 指标 | 目标 | 方法 |
|---|---|---|
| 数据完整性 | 100% | 行数 + checksum |
| 应用启动 | 通过 | health check |
| 关键 API | P99 ≤ 基线 × 1.2 | k6 |

---

**导航 / Navigation:**
[← 流程总览](../../workflow.md) · [← 流程 README](../../README.md)
