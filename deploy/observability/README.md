# observability/

> GitGit 平台可观测性部署资产
> 关联: [docs/observability/](../../docs/observability/) 14 篇设计文档

## 目录结构

```
observability/
├── README.md                              # 本文件
├── base/
│   ├── namespace.yaml                     # observability namespace + ResourceQuota
│   ├── serviceaccount.yaml                # SA + 最小权限
│   ├── otel-collector-daemonset.yaml      # OTel Collector (每节点 1 副本)
│   └── otel-collector-gateway.yaml        # OTel Collector Gateway (2 副本)
├── otel-collector/
│   ├── config.yaml                        # DaemonSet 配置 (接收 + 轻处理)
│   └── config-gateway.yaml                # Gateway 配置 (写后端 + tail_sampling)
├── prometheus-rules/
│   ├── prometheus.yaml                    # Prometheus 抓取配置
│   ├── infrastructure.yaml                # 节点 / K3s / OTel 告警
│   ├── database.yaml                      # PG / 审计 / 凭证告警
│   ├── application.yaml                   # 业务 RED / 中间件告警
│   └── slo.yaml                           # SLO recording + 告警
├── alertmanager/
│   └── alertmanager.yaml                  # 路由 + 抑制 + 通道
├── grafana/
│   ├── grafana.ini                        # 主配置
│   ├── provisioning-datasources.yaml      # DS provisioning
│   └── provisioning-dashboards.yaml       # Dashboard provider
├── network-policies/
│   └── default-deny.yaml                  # default-deny + 显式白名单
└── secrets/                               # (git 留空, 由 External Secrets Operator 同步)
```

## 快速开始

```bash
# 1. 创建 namespace + ResourceQuota
kubectl apply -f base/namespace.yaml

# 2. 创建 ServiceAccount + RBAC
kubectl apply -f base/serviceaccount.yaml

# 3. 部署 OTel Collector DaemonSet + Gateway
kubectl apply -f base/otel-collector-daemonset.yaml
kubectl apply -f base/otel-collector-gateway.yaml

# 4. 部署 kube-prometheus-stack (Helm)
helm install prometheus prometheus-community/kube-prometheus-stack \
  -n observability \
  -f prometheus-rules/prometheus.yaml

# 5. 应用告警规则
kubectl create configmap -n observability prometheus-rules \
  --from-file=prometheus-rules/ \
  --dry-run=client -o yaml | kubectl apply -f -

# 6. 部署 Grafana provisioning
kubectl create configmap -n observability grafana-dashboards \
  --from-file=grafana/ \
  --dry-run=client -o yaml | kubectl apply -f -

# 7. 部署 NetworkPolicy
kubectl apply -f network-policies/

# 8. 验证
./scripts/check-observability.sh
```

## 验证清单

```bash
# 0 broken anchor
powershell -File scripts/check-anchors.ps1

# 0 日文
python -c "import re,glob; jp = re.compile('[\u3040-\u30ff\u4e00-\u9fff]'); print(any(jp.search(open(f,encoding='utf-8').read()) for f in glob.glob('deploy/observability/**/*.yaml', recursive=True)))"

# Prometheus rules 语法
promtool check rules deploy/observability/prometheus-rules/*.yaml

# Prometheus 配置
promtool check config deploy/observability/prometheus-rules/prometheus.yaml

# OTel Collector 配置
otelcol-contrib validate --config=deploy/observability/otel-collector/config.yaml
otelcol-contrib validate --config=deploy/observability/otel-collector/config-gateway.yaml

# Alertmanager 配置
amtool check-config deploy/observability/alertmanager/alertmanager.yaml
```

## 关联文档

- 设计: [`../../docs/observability/`](../../docs/observability/)
  - 00-current-state-analysis.md
  - 01-architecture.md
  - 02-metrics.md / 03-logs.md / 04-tracing.md
  - 05-database-observability.md
  - 06-middleware-observability.md
  - 07-dashboard-design.md
  - 08-alert-design.md
  - 09-slo-design.md
  - 10-security-design.md
  - 11-deployment-design.md
  - 12-performance-retention.md
  - 13-implementation-phases.md
  - 14-code-impact.md
  - 15-self-review-v2.md
  - requirements-traceability.md
- ADR: [`../../docs/architecture/decisions/0011-observability-platform.md`](../../docs/architecture/decisions/0011-observability-platform.md)
- 业务: [`../../crates/gitgit-observability/`](../../crates/gitgit-observability/) (待 Phase 2 实施)
