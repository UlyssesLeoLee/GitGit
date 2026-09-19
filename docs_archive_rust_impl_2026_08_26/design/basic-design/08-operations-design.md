# 08. 运维设计 / Operations & Maintenance Design

> **[PROPOSAL] IPA 共通框架 2013 章节定位：** P3.A2.T5 研讨运行性


## 8.1 部署 / Deployment

### 8.1.1 Local 单机（MVP 标准）

| 项目 | 内容 |
|---|---|
| 形态 | Docker Compose（推荐）或 systemd + 直接二进制 |
| 必要软件 | Docker Engine 20.10+ 或 Podman 4+、PostgreSQL 14+（经 Docker）|
| 安装步骤 | 通过 `curl -fsSL <url> \| bash` 官方安装程序 |
| 首次启动 | 创建 admin 用户、Organization 设置、Repository push 指南 |
| 升级 | `platform upgrade` 命令，rolling 停机时间 < 30s 目标 |
| 卸载 | 提供全数据删除步骤（含确认提示）|

### 8.1.2 Cloud（V1）

| 项目 | 内容 |
|---|---|
| 形态 | Kubernetes / K3s（Phase 10 §2）|
| Helm chart 提供 | V1 正式提供 |
| Terraform module 提供 | V1 提供主要云厂商版本（AWS / GCP / Azure）|
| Horizontal Pod Autoscaler | 基于 PostgreSQL 连接数与 CPU |
| 托管 PostgreSQL | AWS RDS / GCP Cloud SQL / Azure Database for PostgreSQL |

### 8.1.3 支持的操作系统矩阵（NFR-REQ-003, V1 正式化）

| OS | 架构 | MVP 支持 | V1 支持 |
|---|---|---|---|
| Linux（Ubuntu 22.04+, Debian 12+, RHEL 9+）| x86_64, aarch64 | ✓ | ✓ |
| macOS 13+ | aarch64, x86_64 | ✓ | ✓ |
| Windows 11 / Server 2022 | x86_64 | ✓ | ✓ |

Phase 14 F14-5（Low）的 NFR-REQ-003 在 V1 确定正式矩阵。

### 8.1.4 冷启动初始化工具（Day-1 Bootstrap Tool）

针对新团队接入时图谱为空的“Day-1 价值空白”痛点，运维工具集提供极速导入 CLI：

```bash
# 一键扫描并建立过去半年的 Git 历史、分支、Tag 与 Symbol 依赖图谱节点
gitgit bootstrap --repo ./existing-project --since 6.months.ago
```
- **耗时指标：** 10 万行级别代码仓库导入与图谱初始化在 3~5 分钟内完成。
- **自动构建：** 自动提取 Git 提交历史、Author 映射为 Human 节点、生成初始模块依赖 Edge，使用户从第一天起即可体验变更影响分析。

## 8.2 备份 / 恢复 / Backup & Recovery（BKP-REQ）

**[PROPOSAL]** 对应 BKP-REQ-001 / BKP-REQ-002 的一致性要求，PostgreSQL PITR + 文件系统级 Git 备份在同时刻获取：

```
  ┌─── backup_window (e.g., daily 02:00 UTC) ───┐
  │  1. PostgreSQL: pg_backup_start()             │
  │  2. Git objects: rsync snapshot (consistent)  │
  │  3. PostgreSQL: pg_backup_stop()              │
  │  4. WAL archive snapshot                     │
  │  5. Verify: structural diff with previous     │
  └───────────────────────────────────────────────┘
```

| 项目 | MVP（Local）| V1（Cloud）|
|---|---|---|
| RPO | 24h | 1h（WAL archive 间隔）|
| RTO | 24h | 1h（目标）|
| 备份目标 | 同一机器外接 / 另一主机 | S3 / 托管 snapshot |
| 加密 | 备份同样信封加密 | KMS 托管 |
| 恢复演练 | 文档化（上线前必经 PITR 实测验证）| 自动化灾难恢复演练 |
| 稳定性浸泡 | 上线前执行 72h 压力浸泡（Soak Test）| 持续压测演练 |

具体 RTO / RPO 数值为 **NFR-REQ-001**（Phase 14 F14-2 已作为"companion tier statement"折入 F14-1 Availability 等级，修正 BKP-REQ-002）下的 [`TBD`] Benchmark Required。

## 8.3 监控 / Monitoring（OBS-REQ, V1）

**[PROPOSAL]** OBS-REQ-001/002/003 在 MVP 阶段不作为需求确定，但本书**仅提前设计**：

| 项目 | 设计 |
|---|---|
| 指标 | OpenTelemetry Metrics → Prometheus 兼容 exporter |
| 追踪 | OpenTelemetry Traces |
| 日志 | 结构化 JSON，stdout / stderr → 采集端（Fluent Bit 等）|
| 告警 | Prometheus Alertmanager 格式规则（模板提供）|
| 仪表盘 | Grafana dashboard JSON 模板提供 |
| 健康检查 | `/healthz`（liveness），`/readyz`（readiness）|

## 8.4 日志 / Logging

| 类别 | 输出位置 | 保留 | 格式 |
|---|---|---|---|
| 应用日志 | stdout / stderr | 30 天（MVP）| JSON（结构化）|
| 审计日志 | `events` 表 + 导出文件 | 永久（仅逻辑删除）| JSON（结构化）|
| 访问日志 | 应用日志内 | 30 天（MVP）| JSON |
| Git 服务器日志 | Git 服务器日志 | 30 天（MVP）| 标准 Git 格式 |
| Agent 执行日志 | AgentRun 节点 + 文件 | 永久（AgentRun 节点）| JSON |

## 8.5 支持与 FAQ / Support

- 官方文档站点（V1）
- 社区论坛（V1，GitHub Discussions 集成）
- 支持合同（V2，面向 Enterprise）
- 安全联系：`security@example.com`（实际值在运维时确定）

## 8.6 V1+ K8s 部署形态 / V1+ K8s Deployment Topology

**[PROPOSAL]** MVP 阶段为单进程多 App（共享 PG，schema 隔离）。V1+ Cloud 部署引入 K8s，把"主进程"+"Admin UI"+"Plugin Loader"+"App Instance"拆为独立 Deployment。

**部署层次（V1+ Cloud）**：

```
Namespace: platform
├─ platform-postgres (StatefulSet, 1 主 + 2 备)
├─ platform-api-server (Deployment, 2 副本)        # 终端 API
├─ platform-admin (Deployment, 2 副本)              # Admin API + UI
├─ platform-plugin-loader (Deployment, 2 副本)     # 监听 App 节点，启动 instance
├─ platform-event-relay (Deployment, 1 副本)       # 中心事件分发（leader 锁）
├─ platform-app-<app_id> (Deployment, 1+ 副本)     # 每个 App 一个 Deployment
└─ platform-otel-collector (DaemonSet)              # 遥测
```

**与 MVP Local-First 的兼容性**：

| 维度 | MVP Local-First | V1+ K8s Cloud |
|---|---|---|
| 进程数 | 1 个主进程 + 1 个 Admin UI + 1 个 Plugin Loader | 每组件 2 副本 + 每 App 1+ Pod |
| PostgreSQL | 单实例 / 嵌入式 | StatefulSet 1 主 2 备 |
| 网络 | localhost | K8s Service + NetworkPolicy |
| 鉴权 | OIDC 可选（默认本地）| OIDC + mTLS + 双因素 |
| 备份 | 手动 + 定时 cron | Velero + PITR |
| 监控 | 本地 OTel collector | K8s Prometheus + OTel collector |
| 升级 | `git pull` + 重启 | rolling / blue-green / canary（§13.7）|

**为什么 MVP 不上 K8s**：

- §0.5 设计原则 1: **Local-First**（OPS-REQ-001）要求"单机、完全离线运行"
- K8s 引入额外运维复杂度（etcd、网络、存储）
- MVP 用户主要是个人开发者 + 小团队，K8s 收益 < 复杂度成本
- 但 App Manifest 抽象**从 MVP 开始就按"可拆 Pod"设计**，避免后期重写

**MVP → V1+ 迁移路径**：

1. V1 发布时 `platform` 推出 `k8s-install` 命令，生成 Helm Chart
2. 用户在 K8s 集群运行 `helm install platform platform/...`
3. 平台数据通过 §9 迁移设计从 Local PG 导入 Cloud PG
4. App 自动按 manifest 在 K8s 中创建 Deployment
5. Plugin Loader 切换为 K8s Operator 模式（仍监听 nodes 表，但创建/删除 K8s Deployment 而非本地进程）

**App 在 K8s 中的生命周期**（V1+）：

| 状态 | K8s 资源 | 心跳来源 |
|---|---|---|
| `installed` | Deployment 创建中 | Pod 启动 → instance 节点创建 |
| `healthy` | Deployment Ready ≥ 1 | Pod Ready 探针 → 5s 心跳 |
| `degraded` | Deployment Ready < minInstances | 部分 Pod NotReady |
| `upgrading` | 旧 + 新 Deployment 共存 | 新 Pod 启动 + 旧 Pod NotReady |
| `disabled` | Deployment 副本数 = 0 | 无心跳 |
| `rolled_back` | Deployment 回滚到旧镜像 | 旧镜像 Pod Ready |

K8s 探针配置：

```yaml
livenessProbe:
  httpGet: { path: /api/apps/<app_id>/health, port: http }
  initialDelaySeconds: 10
  periodSeconds: 30
readinessProbe:
  httpGet: { path: /api/apps/<app_id>/ready, port: http }
  initialDelaySeconds: 5
  periodSeconds: 10
```

**关键不变量**：

- [PROPOSAL-REQ-OPS-K8S-001] K8s 部署 manifest 与平台 App manifest 双向同步（manifest 变更 → K8s Deployment 同步更新；K8s 状态变化 → manifest 状态同步）
- [PROPOSAL-REQ-OPS-K8S-002] App 升级通过 K8s rolling update 策略触发，与 §13.7 升级策略联动
- [PROPOSAL-REQ-OPS-K8S-003] Pod 资源限额（CPU/memory）由 manifest.spec.resource_limits 派生
- [PROPOSAL-REQ-OPS-K8S-004] Pod 调度禁止与终端 API 部署同节点（podAntiAffinity）
- [PROPOSAL-REQ-OPS-K8S-005] Pod 网络策略默认 deny，仅允许必要的入站（API Server, Admin, Plugin Loader）

详细 K8s 部署清单、Operator 设计、Helm Chart 见 [详细设计 §13.5 V1+ K8s 部署清单](../detailed-design/13-admin-api-and-ops-ui.md#135-v1-k8s-部署清单) + 详细设计 §12.2 Plugin Loader 进程。

---

**导航 / Navigation:**
[← 07. 安全设计](07-security-design.md) · [README](README.md) · [09. 迁移设计 →](09-migration-design.md)
