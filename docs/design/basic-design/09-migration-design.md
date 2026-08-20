# 09. 迁移设计 / Migration Design（Local → Cloud）

> **[PROPOSAL] IPA 共通框架 2013 章节定位：** P3.A2.T7 研讨迁移方式


## 9.1 迁移场景 / Migration Scenarios

**[PROPOSAL]** 设计覆盖以下 3 类场景：

| 场景 | 概述 | 频率 |
|---|---|---|
| **L→C 1：Local 向 Cloud 的单向迁移** | 团队成长，从 Local 运营迁移到 Cloud 托管 | 每年级别 |
| **L→L 2：Local 环境间迁移** | 硬件更新、数据中心搬迁 | 每几年一次 |
| **C→L 3：Cloud 向 Local 的逆向迁移** | 合规要求变化 | 罕见 |

## 9.2 迁移步骤 / Migration Steps（以 L→C 1 为例）

1. **事前评估**
   - 仓库数、Node / Edge / Event 数、Agent 执行历史
   - 所需停机时间估算
2. **Cloud 环境构建**
   - 托管 PostgreSQL、Kubernetes 集群、S3 兼容存储
3. **数据导出**
   - `platform export --include=graph,git,secrets,audit --out=backup-{ts}.tar`
   - 结构 diff 校验（DATA-REQ-001）
4. **数据导入**
   - 在 Cloud 侧 `platform import backup-{ts}.tar`
   - 一致性校验
5. **DNS 切换**
   - TTL 预先缩短、灰度验证、本切换
6. **旧环境退役**
   - 最后一次备份后，30 天过去再删除

## 9.3 架构一致性 / Architecture Consistency（CLOUD-REQ-001）

**[PROPOSAL]** 遵守 Phase 10 §5 的 Local → Cloud 设计：

| 项目 | Local | Cloud | 一致性 |
|---|---|---|---|
| Platform 二进制 / 镜像 | 相同 | 相同 | ✓ |
| PostgreSQL schema | 相同 | 相同 | ✓ |
| Node / Edge / Event / Policy 数据模型 | 相同 | 相同 | ✓ |
| Agent Workspace / CI Runner 模式 | 相同（本地 Docker）| 相同（Kubernetes）| ✓ |
| Git 协议行为 | 相同 | 相同 | ✓ |
| 部署包装 | Docker Compose / systemd | Helm / Terraform | 存在差异（仅打包层）|

**不出现差异的部分：** 应用逻辑、数据模型、图遍历 API、Policy 评估。
**出现差异的部分：** 打包、运维层级、水平扩展配置。
这正是"Local-first, Cloud-ready"原则的达成条件。

## 9.4 数据迁移工具 / Data Migration Tools

| 工具 | 用途 | 提供时期 |
|---|---|---|
| `platform export` | 整体导出 | MVP |
| `platform import` | 整体导入 | MVP |
| `platform verify-export` | 结构 diff 校验 | MVP |
| 增量同步工具 | 最小化 L→C 停机 | V1 |

## 9.5 兼容性矩阵 / Compatibility Matrix

| 迁移方向 | 支持对象 | 不支持 |
|---|---|---|
| MVP Local → MVP Cloud | ✓ | — |
| MVP Local → V1 Cloud | ✓ | — |
| V1 Local → V1 Cloud | ✓ | — |
| 过去版本 → 新版本 | 滚动支持（V1 制定）| EOL 后 |

---

**导航 / Navigation:**
[← 08. 运维设计](08-operations-design.md) · [README](README.md) · [10. 验收测试方针 →](10-acceptance-test-policy.md)
