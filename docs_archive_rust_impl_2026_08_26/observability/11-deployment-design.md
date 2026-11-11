# 部署设计 / Deployment Design

> **关联 OBS-REQ**: OBS-REQ-017（部署形态） / OBS-REQ-018（GitOps） / OBS-REQ-019（HA）
> **关联设计**: [`../architecture/decisions/0006-k3s-deployment.md`](../architecture/decisions/0006-k3s-deployment.md) / [`10-security-design.md`](10-security-design.md)
> **目标**: 监控基础设施在 K3s 集群内 + GitOps + Helm + 高可用 + 容量可控
> **强约束**: 监控组件部署在独立 `observability` namespace；**禁止**与业务 Pod 共享 node

## 1. 部署形态

### 1.1 拓扑

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         Internet / User                                  │
└────────────────────────────────────────┬────────────────────────────────┘
                                         │
                                         ▼
                          ┌──────────────────────────────┐
                          │  Cloud LB / Cilium Ingress   │
                          │  + cert-manager (TLS)        │
                          │  (只暴露 Grafana)            │
                          └──────────────┬───────────────┘
                                         │
                                         ▼
            ┌────────────────────────────────────────────────────────┐
            │  observability namespace (独立 K3s namespace)          │
            │                                                        │
            │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐    │
            │  │ Grafana     │  │ Alertmanager│  │ Pyroscope   │    │
            │  │ (2 副本)    │  │ (3 副本)    │  │ (1 副本)    │    │
            │  │ :3000       │  │ :9093       │  │ :4040       │    │
            │  └─────────────┘  └─────────────┘  └─────────────┘    │
            │                                                        │
            │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐    │
            │  │ Prometheus  │  │ Loki        │  │ Tempo       │    │
            │  │ (2 副本)    │  │ (3 副本)    │  │ (2 副本)    │    │
            │  │ :9090       │  │ :3100       │  │ :4317/:9411 │    │
            │  └─────────────┘  └─────────────┘  └─────────────┘    │
            │                                                        │
            │  ┌─────────────────────────────────────────────────┐  │
            │  │ OTel Collector (DaemonSet + Gateway Deployment) │  │
            │  │ 每节点 1 副本 (DaemonSet) + 2 副本 Gateway       │  │
            │  └─────────────────────────────────────────────────┘  │
            │                                                        │
            │  ┌─────────────────────────────────────────────────┐  │
            │  │ pg_exporter / node-exporter / kube-state-metrics│  │
            │  │ kube-prometheus-stack 默认 exporters             │  │
            │  └─────────────────────────────────────────────────┘  │
            └────────────────────────────────────────────────────────┘
                                          ▲
                                          │ OTLP / Prom scrape
            ┌─────────────────────────────┴──────────────────────────┐
            │  gitgit namespace (业务)                              │
            │  gitgit-server / gitgit-admin / gitgit-cli            │
            │  + 各 business crate                                 │
            └──────────────────────────────────────────────────────┘
```

### 1.2 资源占用预估

| 组件 | 副本 | 单副本资源 | 总资源 | 存储 |
|---|---|---|---|---|
| Grafana | 2 | 100m / 256Mi | 200m / 512Mi | 1Gi (PG) |
| Prometheus | 2 | 500m / 2Gi | 1 / 4Gi | 200Gi (PVC) |
| Alertmanager | 3 | 50m / 64Mi | 150m / 192Mi | 5Gi |
| Loki | 3 | 500m / 1Gi | 1.5 / 3Gi | 500Gi (S3) |
| Tempo | 2 | 500m / 1Gi | 1 / 2Gi | 200Gi (S3) |
| OTel Collector (DaemonSet) | N | 200m / 512Mi | 1.6 / 4Gi (8 节点) | 10Gi |
| OTel Collector (Gateway) | 2 | 500m / 1Gi | 1 / 2Gi | 10Gi |
| Pyroscope | 1 | 200m / 512Mi | 200m / 512Mi | 50Gi (S3) |
| pg_exporter | 1 | 50m / 64Mi | 50m / 64Mi | — |
| kube-state-metrics | 1 | 100m / 128Mi | 100m / 128Mi | — |
| node-exporter (DaemonSet) | N | 50m / 32Mi | 400m / 256Mi (8 节点) | — |

**总 CPU**: 约 5 核 / **总内存**: 约 16Gi / **总存储**: 约 1TB

> **节点规划**: 监控需独立 2 个 agent 节点（不在 control plane）+ 1 个对象存储后端。

## 2. 命名空间与隔离

### 2.1 独立 Namespace

```yaml
apiVersion: v1
kind: Namespace
metadata:
  name: observability
  labels:
    name: observability
    purpose: monitoring
    pod-security.kubernetes.io/enforce: restricted
    pod-security.kubernetes.io/audit: restricted
    pod-security.kubernetes.io/warn: restricted
    # 禁止业务 Pod 调度到此
    workload-class: observability
```

### 2.2 节点隔离

```yaml
# 业务 Pod 通过 nodeSelector / taint 强制调度到非监控节点
apiVersion: apps/v1
kind: Deployment
metadata:
  namespace: gitgit
  name: gitgit-server
spec:
  template:
    spec:
      nodeSelector:
        workload-class: business
      tolerations: []      # 不容忍监控 taint
      affinity:
        podAntiAffinity:
          requiredDuringSchedulingIgnoredDuringExecution:
            - labelSelector:
                matchLabels:
                  name: gitgit-server
              topologyKey: kubernetes.io/hostname
```

```yaml
# 监控节点 taint
apiVersion: v1
kind: Node
metadata:
  name: k3s-agent-obs-1
spec:
  taints:
    - key: workload-class
      value: observability
      effect: NoSchedule
```

### 2.3 Resource Quota

```yaml
apiVersion: v1
kind: ResourceQuota
metadata:
  name: observability-quota
  namespace: observability
spec:
  hard:
    requests.cpu: "8"
    requests.memory: 24Gi
    limits.cpu: "20"
    limits.memory: 64Gi
    persistentvolumeclaims: "20"
    requests.storage: 2Ti
    services: "30"
    secrets: "50"
    configmaps: "50"
```

## 3. 部署清单

### 3.1 Helm Chart 依赖

| Chart | 版本 | 用途 | 路径 |
|---|---|---|---|
| `kube-prometheus-stack` | 65.x | Prometheus + Alertmanager + Grafana + exporters | `deploy/observability/kube-prometheus-stack` |
| `loki` | 6.x | 日志聚合 | `deploy/observability/loki` |
| `tempo` | 1.x | 追踪后端 | `deploy/observability/tempo` |
| `opentelemetry-collector` | 0.110.x | 遥测汇聚 | `deploy/observability/otel-collector` |
| `pyroscope` | 1.x | 持续 profiling | `deploy/observability/pyroscope`（可选） |
| `cert-manager` | 1.16.x | TLS 证书 | 已存在 |
| `external-secrets` | 0.10.x | Secret 同步 | 已存在 |

### 3.2 部署模式

| 组件 | Helm 安装方式 | 副本策略 |
|---|---|---|
| Prometheus | `kube-prometheus-stack` 内嵌 | 2 副本 + Thanos sidecar（未来） |
| Alertmanager | `kube-prometheus-stack` 内嵌 | 3 副本（gossip 集群） |
| Grafana | `kube-prometheus-stack` 内嵌 | 2 副本（无状态） |
| Loki | `grafana/loki` 独立 chart | 3 副本（distributor + ingester + querier 分层） |
| Tempo | `grafana/tempo` 独立 chart | 2 副本 |
| OTel Collector (DaemonSet) | `opentelemetry-collector` | 每节点 1 副本 |
| OTel Collector (Gateway) | `opentelemetry-collector` | 2 副本（Deployment） |
| pg_exporter | 自定义 | 1 副本（Deployment） |
| kube-state-metrics | `kube-prometheus-stack` 内嵌 | 1 副本 |

### 3.3 副本数与 HA

| 组件 | 副本 | 失败容忍 | 数据丢失风险 |
|---|---|---|---|
| Prometheus | 2 | 1 | 切换时丢 15s 数据 |
| Alertmanager | 3 | 1 | 不丢（gossip 同步） |
| Grafana | 2 | 1 | 无状态，DB 在外部 PG |
| Loki | 3 (distributor + ingester) | 1 | ingester 切换时丢 < 1m 缓冲 |
| Tempo | 2 | 1 | 接收时丢 < 30s |
| OTel Collector DaemonSet | 1/节点 | 节点 down | 切到 Gateway |

> **RTO**: 监控组件 RTO ≤ 5min；**RPO**: ≤ 1m（除 Prometheus 切换）

## 4. 存储

### 4.1 存储矩阵

| 组件 | 类型 | 大小 | 介质 | 备份 |
|---|---|---|---|---|
| Prometheus TSDB | PVC | 200Gi × 2 | SSD (IOPS > 3000) | 每日 snapshot |
| Grafana DB | 外部 PG (gitgit DB) | 1Gi | SSD | 随主库 |
| Loki chunks | S3 (MinIO 自托管) | 500Gi | HDD | S3 lifecycle 90d |
| Loki index | BoltDB + S3 | 50Gi | SSD | — |
| Tempo blocks | S3 | 200Gi | HDD | S3 lifecycle 30d |
| Alertmanager | PVC | 5Gi × 3 | SSD | — |
| OTel Collector | emptyDir | 10Gi | 本地 | 无（无状态） |

### 4.2 StorageClass

```yaml
apiVersion: storage.k8s.io/v1
kind: StorageClass
metadata:
  name: monitoring-ssd
provisioner: local-path.csi.k3s.io
volumeBindingMode: WaitForFirstConsumer
parameters:
  # 优先使用 SSD
  fsType: ext4
```

### 4.3 S3 替代（MinIO 自托管）

> **强约束**: 监控数据**禁止**用公网 S3 / 公有云 OSS（合规要求数据本地化）。
> 部署内部 MinIO，绑定在 `storage` namespace。

```yaml
# minio 端点
endpoint: http://minio.storage.svc:9000
region: cn-north-1
bucket:
  loki-chunks: loki
  tempo-blocks: tempo
  pyroscope-blocks: pyroscope
accessKey: ${MINIO_ACCESS_KEY}
secretKey: ${MINIO_SECRET_KEY}
```

## 5. GitOps 部署

### 5.1 仓库结构

```
gitgit-iac/                          # 独立 IaC 仓库
├── clusters/
│   ├── prod/
│   │   ├── kustomization.yaml
│   │   ├── namespace.yaml
│   │   └── observability-patch.yaml
│   └── staging/
├── observability/
│   ├── base/
│   │   ├── kustomization.yaml
│   │   ├── namespace.yaml
│   │   ├── resource-quota.yaml
│   │   ├── network-policies/
│   │   │   ├── default-deny.yaml
│   │   │   ├── otel-collector-allow.yaml
│   │   │   └── grafana-restrict.yaml
│   │   ├── kube-prometheus-stack/
│   │   │   ├── kustomization.yaml
│   │   │   ├── prometheus.yaml
│   │   │   ├── alertmanager.yaml
│   │   │   ├── grafana.yaml
│   │   │   └── rules/
│   │   │       ├── infrastructure.yaml
│   │   │       ├── database.yaml
│   │   │       ├── application.yaml
│   │   │       ├── middleware.yaml
│   │   │       ├── security.yaml
│   │   │       └── slo.yaml
│   │   ├── loki/
│   │   │   ├── kustomization.yaml
│   │   │   ├── values.yaml
│   │   │   └── config.yaml
│   │   ├── tempo/
│   │   │   ├── kustomization.yaml
│   │   │   └── values.yaml
│   │   ├── otel-collector/
│   │   │   ├── kustomization.yaml
│   │   │   ├── daemonset.yaml
│   │   │   ├── gateway.yaml
│   │   │   └── config/
│   │   │       ├── receivers.yaml
│   │   │       ├── processors.yaml
│   │   │       ├── exporters.yaml
│   │   │       ├── service.yaml
│   │   │       └── pipeline.yaml
│   │   ├── dashboards/
│   │   │   ├── provider.yaml
│   │   │   └── dashboards/   # 实际 JSON
│   │   └── exporters/
│   │       ├── pg-exporter.yaml
│   │       └── node-exporter.yaml
│   └── overlays/
│       ├── prod/
│       │   ├── kustomization.yaml
│       │   ├── replica-counts.yaml
│       │   ├── replicas-overrides.yaml
│       │   └── secrets.yaml
│       └── staging/
└── README.md
```

### 5.2 ArgoCD / Flux 选择

> **强约束**: 项目使用 **ArgoCD**（与 K3s 友好集成，已是 K3s 默认 GitOps）。

```yaml
# argocd app
apiVersion: argoproj.io/v1alpha1
kind: Application
metadata:
  name: observability
  namespace: argocd
spec:
  project: monitoring
  source:
    repoURL: https://github.com/gitgit/iac
    targetRevision: main
    path: observability/overlays/prod
  destination:
    server: https://kubernetes.default.svc
    namespace: observability
  syncPolicy:
    automated:
      prune: true
      selfHeal: true
    syncOptions:
      - CreateNamespace=false
      - PrunePropagationPolicy=foreground
      - ServerSideApply=true
    retry:
      limit: 3
      backoff:
        duration: 10s
        factor: 2
        maxDuration: 5m
```

### 5.3 同步策略

| 资源 | 自动同步 | Self-heal | 备注 |
|---|---|---|---|
| namespace | ✅ | ✅ | — |
| resource-quota | ✅ | ✅ | — |
| network-policies | ✅ | ✅ | — |
| kube-prometheus-stack | ✅ | ✅ | 重要变更需 PR review |
| loki / tempo | ✅ | ✅ | — |
| otel-collector | ✅ | ✅ | — |
| dashboards | ✅ | ✅ | JSON 由 9 大盘生成 |
| alert rules | ✅ | ✅ | 重要变更需 PR review |
| secrets | ❌ | ❌ | 由 External Secrets Operator 同步 |

## 6. NetworkPolicy 部署

> 详见 [`10-security-design.md`](10-security-design.md) §6。本节只列部署清单。

```bash
observability/
└── base/
    └── network-policies/
        ├── 00-default-deny.yaml
        ├── 01-otel-collector-egress.yaml
        ├── 02-otel-collector-ingress.yaml
        ├── 03-prometheus-ingress.yaml
        ├── 04-grafana-ingress.yaml
        ├── 05-loki-ingress.yaml
        ├── 06-tempo-ingress.yaml
        └── 99-dns-allow-all.yaml
```

## 7. RBAC

### 7.1 ServiceAccount

```yaml
---
apiVersion: v1
kind: ServiceAccount
metadata:
  name: prometheus
  namespace: observability
---
apiVersion: v1
kind: ServiceAccount
metadata:
  name: otel-collector
  namespace: observability
---
apiVersion: v1
kind: ServiceAccount
metadata:
  name: alertmanager
  namespace: observability
```

### 7.2 最小权限

```yaml
# prometheus SA
apiVersion: rbac.authorization.k8s.io/v1
kind: ClusterRole
metadata:
  name: prometheus-read
rules:
  - apiGroups: [""]
    resources: ["nodes", "nodes/proxy", "services", "endpoints", "pods"]
    verbs: ["get", "list", "watch"]
  - apiGroups: ["extensions", "apps", "networking.k8s.io"]
    resources: ["deployments", "statefulsets", "daemonsets", "ingresses", "networkpolicies"]
    verbs: ["get", "list", "watch"]
```

## 8. 滚动更新策略

### 8.1 监控组件特殊性

| 组件 | 更新窗口 | 中断容忍 |
|---|---|---|
| Prometheus | 低峰期 | 30s 中断可接受 |
| Alertmanager | 任何 | 滚动 1 副本 |
| Grafana | 任何 | 滚动无中断（无状态） |
| Loki | 低峰期 | 5min 中断可接受（业务可短暂不写日志） |
| Tempo | 低峰期 | 5min 中断可接受 |
| OTel Collector (DaemonSet) | 任何 | 滚动无中断 |
| OTel Collector (Gateway) | 任何 | 滚动无中断 |
| pg_exporter | 任何 | 滚动无中断 |

### 8.2 强制策略

```yaml
spec:
  strategy:
    type: RollingUpdate
    rollingUpdate:
      maxUnavailable: 0
      maxSurge: 1
  template:
    spec:
      terminationGracePeriodSeconds: 60
      containers:
        - name: prometheus
          lifecycle:
            preStop:
              exec:
                command: ["/bin/sh", "-c", "wget --post-data='{}' http://localhost:9090/-/ready"]
          readinessProbe:
            httpGet:
              path: /-/ready
              port: 9090
            initialDelaySeconds: 30
            periodSeconds: 10
          livenessProbe:
            httpGet:
              path: /-/healthy
              port: 9090
            initialDelaySeconds: 60
            periodSeconds: 30
            failureThreshold: 3
```

## 9. 备份与恢复

### 9.1 备份策略

| 组件 | 备份 | 频率 | 保留 |
|---|---|---|---|
| Prometheus TSDB | snapshot → S3 | 每日 03:00 | 30d |
| Grafana DB | pg_dump → S3 | 每日 03:00 | 30d |
| Alertmanager | snapshot → S3 | 每日 03:00 | 7d |
| Loki chunks | S3 lifecycle | 持续 | 90d |
| Tempo blocks | S3 lifecycle | 持续 | 30d |
| 告警规则 / dashboard | Git | 持续 | 永久 |

### 9.2 Velero（K3s 备份）

```bash
# 备份 observability namespace（仅元数据，TSDB 用 snapshot）
velero backup create obs-meta-$(date +%F) \
  --include-namespaces observability \
  --include-resources configmap,secret,deployment,statefulset,daemonset,service \
  --ttl 720h
```

## 10. 验证部署

### 10.1 健康检查脚本

```bash
#!/bin/bash
# scripts/check-observability.sh
set -e

NS=observability

echo "=== Pod 状态 ==="
kubectl -n $NS get pods

echo "=== 关键服务 ==="
for svc in prometheus grafana alertmanager loki tempo otel-collector; do
  ep=$(kubectl -n $NS get svc $svc -o jsonpath='{.spec.clusterIP}')
  if [ -n "$ep" ]; then
    echo "$svc: $ep"
  else
    echo "❌ $svc: 无 ClusterIP"
    exit 1
  fi
done

echo "=== OTLP 端点测试 ==="
nc -zv otel-collector.observability.svc 4317
nc -zv otel-collector.observability.svc 4318

echo "=== Prometheus 端点 ==="
curl -sf http://prometheus.observability.svc:9090/-/ready

echo "=== Loki 端点 ==="
curl -sf http://loki.observability.svc:3100/ready

echo "=== Tempo 端点 ==="
curl -sf http://tempo.observability.svc:3200/ready

echo "=== Grafana 端点 ==="
curl -sf http://grafana.observability.svc:3000/api/health
```

### 10.2 烟测（业务侧）

```rust
// 业务 crate 启动时验证 OTLP 端点可达
#[tokio::main]
async fn main() {
    // 1. 启动时上报一条自检 span
    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(opentelemetry_otlp::new_exporter()
            .tonic()
            .with_endpoint("http://otel-collector.observability.svc:4317"))
        .install_batch(opentelemetry_sdk::runtime::Tokio);

    // 2. 业务 span
    use tracing::instrument;
    #[instrument]
    async fn start() {
        info!("startup ok");
    }
    start().await;

    // 3. 启动 HTTP server
    // ...
}
```

## 11. 升级与迁移

### 11.1 升级路径

| 阶段 | 操作 | 风险 |
|---|---|---|
| 升级前 | 阅读 release notes / breaking changes | — |
| 备份 | 完整 snapshot + 规则导出 | — |
| staging | 先在 staging 验证 1 周 | 低 |
| prod 灰度 | 滚动 1 副本观察 1h | 中 |
| 全面升级 | 全部滚动 | 中 |
| 升级后 | 验证 9 大盘 + alert 触发测试 | — |

### 11.2 大版本升级

- **Prometheus 2.x → 3.x**: 数据格式变更，需双写 + 切读
- **Grafana 11 → 12**: 评估 panel 兼容性
- **Loki 3.x → 4.x**: 评估 query 兼容性
- **Tempo 2.x → 3.x**: 评估 block 格式

## 12. 关联文档

- 上游: [`10-security-design.md`](10-security-design.md) §6 NetworkPolicy
- 上游: [`../architecture/decisions/0006-k3s-deployment.md`](../architecture/decisions/0006-k3s-deployment.md)
- 下游: [`13-implementation-phases.md`](13-implementation-phases.md) §1-§8
- 下游: [`15-self-review-v2.md`](15-self-review-v2.md) §5 部署复盘

## 13. 需求 ID 索引

| 需求 ID | 标题 | 优先级 |
|---|---|---|
| OBS-REQ-017 | 部署形态 | P0 |
| OBS-REQ-018 | GitOps | P0 |
| OBS-REQ-019 | HA | P0 |
| DEPLOY-REQ-001 | 独立 namespace | P0 |
| DEPLOY-REQ-002 | 节点隔离 | P0 |
| DEPLOY-REQ-003 | Resource Quota | P0 |
| DEPLOY-REQ-004 | Helm Charts | P0 |
| DEPLOY-REQ-005 | 副本与 HA | P0 |
| DEPLOY-REQ-006 | 存储矩阵 | P0 |
| DEPLOY-REQ-007 | 内部 MinIO | P0 |
| DEPLOY-REQ-008 | ArgoCD | P0 |
| DEPLOY-REQ-009 | 滚动更新 | P0 |
| DEPLOY-REQ-010 | 备份 | P0 |
| DEPLOY-REQ-011 | 烟测 | P0 |
| DEPLOY-REQ-012 | 升级路径 | P1 |
