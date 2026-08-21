# 可观测性安全设计 / Observability Security Design

> **关联 OBS-REQ**: OBS-REQ-014（监控安全） / OBS-REQ-015（数据保护） / OBS-REQ-016（网络隔离）
> **关联设计**: [`../design/detailed-design/09-security-impl.md`](../design/detailed-design/09-security-impl.md) / [`06-middleware-observability.md`](06-middleware-observability.md) §2.5 NetworkPolicy
> **关联 ADR**: ADR-0007（零信任） / ADR-0008（不可篡改审计）
> **核心原则**: **监控数据 = 业务数据 = 必须同等保护**；**监控系统不裸露公网**

## 1. 威胁模型

### 1.1 资产清单

| 资产 | 敏感度 | 存储位置 | 影响 |
|---|---|---|---|
| **指标数据** | 中 | Prometheus TSDB | 暴露内部组件、容量 |
| **日志数据** | **高** | Loki | 包含用户行为、错误详情、可能含敏感数据 |
| **追踪数据** | **高** | Tempo | 包含 SQL 参数、HTTP body、内部通信 |
| **告警数据** | 中 | Alertmanager | 暴露内部弱点 |
| **Dashboard** | 中 | Grafana DB | 暴露业务架构、容量 |
| **审计哈希链** | 极高 | PG `admin_audit` | 合规关键 |

### 1.2 威胁列表

| 威胁 | 风险 | 防御 |
|---|---|---|
| **T1**: 监控系统公网暴露 | 严重 | NetworkPolicy + Ingress + IP 白名单 |
| **T2**: Grafana 弱密码 / 默认凭据 | 严重 | OIDC 集成 + 强密码 + MFA |
| **T3**: 日志泄露用户数据 / 密钥 | 严重 | 字段过滤 + 静态扫描 + 告警 |
| **T4**: Trace 含 SQL 凭证 | 高 | 字段过滤 + 脱敏 |
| **T5**: 监控组件 RCE | 严重 | 镜像签名 + 非 root + read-only fs |
| **T6**: 监控后端数据篡改 | 高 | admin_audit 哈希链 + 备份隔离 |
| **T7**: 监控组件横向移动 | 中 | 独立 namespace + NetworkPolicy 隔离 |
| **T8**: Cardinality 攻击（恶意 label 爆炸） | 中 | 强 label 校验 + 限流 + 告警 |
| **T9**: 凭证轮换 / 撤销未传播到监控 | 中 | Vault 集成 + 短 TTL |
| **T10**: 监控数据用于训练 / 调试外传 | 中 | 部署隔离 + 访问审计 |

## 2. Grafana 安全

### 2.1 认证（Authentication）

#### 强制: 关闭匿名

```ini
# grafana.ini
[auth]
disable_login_form = false
disable_signout_menu = false

[auth.anonymous]
enabled = false    # 强制禁止

[users]
allow_sign_up = false
auto_assign_org_role = Viewer
```

#### 首选: OIDC 集成

```ini
[auth.generic_oauth]
enabled = true
name = GitGit
client_id = grafana
client_secret = ${GRAFANA_OAUTH_CLIENT_SECRET}
auth_url = https://auth.example.com/oauth/authorize
token_url = https://auth.example.com/oauth/token
api_url = https://auth.example.com/oauth/userinfo
role_attribute_path = role
role_attribute_strict = true
allow_assign_grafana_admin = false
scopes = openid profile email groups
```

> **OIDC 提供方**: 复用项目自身的 `gitgit-admin` 认证域（SEC-REQ-011），不引入新 IdP。

### 2.2 授权（Authorization）

#### 角色矩阵

| 角色 | 查看权限 | 编辑权限 | Admin |
|---|---|---|---|
| `Viewer` | 所有大盘 | ❌ | ❌ |
| `Editor` | 所有大盘 | 自己创建 | ❌ |
| `Admin` | 所有大盘 | 所有 | dashboard / datasources / users |
| `Grafana Admin` | 全部 | 全部 | 系统级 |

#### Folder 权限

| Folder | 角色 | Audience |
|---|---|---|
| `GitGit / 00-System Overview` | Viewer | All |
| `GitGit / 10-Infrastructure` | Editor | sre |
| `GitGit / 20-Kubernetes` | Editor | sre |
| `GitGit / 30-Application` | Editor | backend-dev |
| `GitGit / 40-Database` | Editor | sre-dba |
| `GitGit / 50-Middleware` | Editor | sre-mw |
| `GitGit / 60-Network` | Editor | sre |
| `GitGit / 70-Performance` | Editor | sre |
| `GitGit / 80-Security` | Viewer | sec-oncall |
| `GitGit / 90-SLO` | Viewer | sre / pm / em |
| `GitGit / _internal` | Admin | sre-admin |

### 2.3 Dashboard / Data Source 权限

```ini
[dashboards]
min_refresh_interval = 10s   # 防止过频刷新

[unified_alerting]
enabled = true
# alert 规则也用 RBAC 控制编辑
execute_alerts = true
```

### 2.4 审计 Grafana 操作

```ini
[log]
level = info
mode = console

# 关键操作: dashboard 创建 / 修改 / 删除 / datasources 编辑
# → 发送至 Loki → 关联 trace_id
```

## 3. Prometheus 安全

### 3.1 访问控制

| 端点 | 鉴权 | 网络 |
|---|---|---|
| `/api/v1/query` | Bearer token (mTLS) | ClusterIP only |
| `/api/v1/write` | mTLS (Prom 2.43+ native) | ClusterIP only |
| `/metrics` | ClusterIP only | 无外网 |
| `/api/v1/admin/*` | mTLS + admin role | ClusterIP only |

### 3.2 mTLS 通信

所有 OTLP / Prometheus remote_write / scrape 走 mTLS:

```yaml
# prometheus.yml
global:
  scrape_interval: 15s
  scrape_timeout: 10s

scrape_configs:
  - job_name: 'gitgit-server'
    scheme: https
    tls_config:
      ca_file: /etc/prometheus/ca.crt
      cert_file: /etc/prometheus/client.crt
      key_file: /etc/prometheus/client.key
      server_name: gitgit-server.observability.svc
    static_configs:
      - targets: ['gitgit-server.observability.svc:3000']
```

### 3.3 数据隔离

| Tenant | 保留 | 查询隔离 |
|---|---|---|
| 默认 | 30d | 全量 |
| 合规 | 1y | 限制为 sec-oncall |
| 测试 | 7d | 所有人可读 |

> **多租户隔离**: 项目初期单租户，但 Prometheus 已按 `tenant` label 分桶存储（TSDB head 切分）。

## 4. Loki 安全

### 4.1 租户隔离

```yaml
# loki config
auth_enabled: true

# 租户 ID 通过 header 传递
multitenancy:
  enabled: true
  tenant_id:
    header: X-Scope-OrgID

# 业务服务写日志时强制设 header
```

### 4.2 字段脱敏（Otel Collector transform processor）

```yaml
processors:
  transform:
    trace_statements:
      - context: log
        statements:
          # 1. 静态已知敏感字段
          - replace_all_patterns(attributes, "value", "\\b\\w+@\\w+\\.\\w+\\b", "[REDACTED_EMAIL]")
          - replace_all_patterns(body, "value", "password[\"']?\\s*[:=]\\s*[\"']?\\w+[\"']?", "password=[REDACTED]")
          - replace_all_patterns(body, "value", "Bearer\\s+[A-Za-z0-9._-]+", "Bearer [REDACTED]")
          # 2. 字段名过滤
          - replace_all_patterns(attributes, "key", "password|secret|api_key|token|cookie", "[REDACTED_KEY]")
```

### 4.3 LogQL 权限

```yaml
# Grafana data source 配置
datasources:
  - name: Loki
    jsonData:
      httpHeaderName1: X-Scope-OrgID
    secureJsonData:
      httpHeaderValue1: tenant-gitgit
```

> **多租户模式**: 每个业务租户仅能查询自己的日志。

### 4.4 日志保留与删除

```yaml
# compactor
compactor:
  working_directory: /data/compactor
  retention_enabled: true
  retention_delete_delay: 2h
  delete_request_store: filesystem
  delete_batch_size: 100

# 表征 retention：loki 不会主动删除，会生成删除请求
# 合规删除：手动执行 + 审计
```

## 5. Tempo 安全

### 5.1 Trace 采样与脱敏

```yaml
# OTel Collector tail_sampling processor
processors:
  tail_sampling:
    decision_wait: 10s
    num_traces: 100000
    expected_new_traces_per_sec: 1000
    policies:
      - name: errors
        type: status_code
        status_code: { status_codes: [ERROR] }
      - name: slow
        type: latency
        latency: { threshold_ms: 2000 }
      - name: auth-admin
        type: string_attribute
        string_attribute: { key: http.route, values: ["/admin/*", "/auth/*"] }
      - name: ai
        type: string_attribute
        string_attribute: { key: ai.caller, values: [".*"] }
      - name: probabilistic
        type: probabilistic
        probabilistic: { sampling_percentage: 5 }
```

### 5.2 Span 字段过滤

```yaml
processors:
  transform:
    trace_statements:
      - context: span
        statements:
          # HTTP body 默认就不在 span attributes 内（Otel SDK 默认不记录）
          # 但如果自定义埋点，需要强制过滤
          - replace_all_patterns(attributes, "key", "http.request.body|db.statement.params", "[REDACTED]")
          - replace_all_patterns(attributes, "value", "password|secret|api_key|token", "[REDACTED_VALUE]")
```

### 5.3 凭证字段全过滤清单

> **强约束**: 以下字段**绝不允许**出现在任何 trace / log / metric attribute 中。

| 字段名模式 | 匹配规则 |
|---|---|
| `password` / `passwd` / `pwd` | 精确 + 大小写不敏感 |
| `secret` | 字段名前缀 |
| `api_key` / `apikey` | 精确 |
| `token` | 精确（`token_type` 等元数据除外） |
| `cookie` / `set-cookie` | 精确 |
| `authorization` (值) | header |
| `bearer` (值) | 完整值 |
| `private_key` / `private-key` | 精确 |
| `ssh_key` | 精确 |
| `client_secret` | 精确 |
| `*.password` / `*.secret` | 嵌套字段 |
| 邮箱 | 正则 `\w+@\w+\.\w+` |
| 中国身份证 | 正则 `\d{17}[\dX]` |
| 信用卡 | 正则 `\d{4}[-\s]?\d{4}[-\s]?\d{4}[-\s]?\d{4}` |

## 6. Kubernetes NetworkPolicy（监控网络隔离）

### 6.1 命名空间

```yaml
# observability namespace
apiVersion: v1
kind: Namespace
metadata:
  name: observability
  labels:
    name: observability
    pod-security.kubernetes.io/enforce: restricted
    pod-security.kubernetes.io/audit: restricted
```

### 6.2 拒绝所有入站 + 出站（基线）

```yaml
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: observability-default-deny
  namespace: observability
spec:
  podSelector: {}
  policyTypes:
    - Ingress
    - Egress
```

### 6.3 显式允许（白名单）

```yaml
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: otel-collector-allow
  namespace: observability
spec:
  podSelector:
    matchLabels:
      app: otel-collector
  policyTypes:
    - Ingress
    - Egress
  ingress:
    # 接收业务 Pod OTLP
    - from:
        - namespaceSelector:
            matchLabels:
              name: gitgit
      ports:
        - port: 4317    # OTLP gRPC
        - port: 4318    # OTLP HTTP
        - port: 8888    # self-metrics
    # 接收 prometheus scrape
    - from:
        - podSelector:
            matchLabels:
              app: prometheus
      ports:
        - port: 8888
  egress:
    # 写到 prometheus / loki / tempo
    - to:
        - podSelector:
            matchLabels:
              app: prometheus
      ports:
        - port: 9090
    - to:
        - podSelector:
            matchLabels:
              app: loki
      ports:
        - port: 3100
    - to:
        - podSelector:
            matchLabels:
              app: tempo
      ports:
        - port: 4317
        - port: 9411
        - port: 14250
        - port: 14268
    # DNS
    - to:
        - namespaceSelector: {}
      ports:
        - port: 53
          protocol: UDP
        - port: 53
          protocol: TCP
    # Alertmanager
    - to:
        - podSelector:
            matchLabels:
              app: alertmanager
      ports:
        - port: 9093
```

### 6.4 监控组件之间隔离

```yaml
# 禁止 grafana 直接访问 prometheus 内部 admin API
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: grafana-restrict
  namespace: observability
spec:
  podSelector:
    matchLabels:
      app: grafana
  ingress:
    - from:
        - namespaceSelector:
            matchLabels:
              name: ingress-nginx
      ports:
        - port: 3000
```

### 6.5 Cilium L7 NetworkPolicy（精细化）

```yaml
apiVersion: cilium.io/v2
kind: CiliumNetworkPolicy
metadata:
  name: observability-egress-restrict
  namespace: observability
spec:
  endpointSelector:
    matchLabels:
      app: otel-collector
  egressDeny:
    # 禁止访问业务数据库
    - toFQDNs:
        - matchName: "pg-primary.gitgit.svc"
        - matchName: "pg-replica-1.gitgit.svc"
  egress:
    - toFQDNs:
        - matchName: "prometheus.observability.svc"
        - matchName: "loki.observability.svc"
        - matchName: "tempo.observability.svc"
```

## 7. Secret 管理

### 7.1 监控组件的 Secret

| Secret | 来源 | 轮换 |
|---|---|---|
| Grafana OIDC client secret | Vault | 90d |
| Grafana admin password | Vault | 180d |
| Prometheus mTLS cert | cert-manager | 90d |
| Loki S3 credentials | Vault | 90d |
| Tempo S3 credentials | Vault | 90d |
| Alertmanager PD token | Vault | 90d |
| Alertmanager Slack webhook | Vault | 365d |
| PagerDuty token | Vault | 90d |

> **统一来源**: 所有 secret 通过 External Secrets Operator 从 Vault 同步到 K8s Secret。

### 7.2 短 TTL Token

```yaml
# 使用 Service Account Token Volume Projection（K8s 1.21+）
apiVersion: v1
kind: Pod
spec:
  serviceAccountName: otel-collector
  volumes:
    - name: sa-token
      projected:
        sources:
          - serviceAccountToken:
              path: token
              audience: monitoring
              expirationSeconds: 3600   # 1h
```

## 8. 镜像与运行时安全

### 8.1 镜像来源

| 组件 | 镜像 | 验证 |
|---|---|---|
| prometheus | `prom/prometheus:v2.55.x` | cosign 签名 + SBOM |
| grafana | `grafana/grafana:11.3.x` | cosign 签名 + SBOM |
| loki | `grafana/loki:3.3.x` | cosign 签名 + SBOM |
| tempo | `grafana/tempo:2.6.x` | cosign 签名 + SBOM |
| otel-collector | `otel/opentelemetry-collector-contrib:0.110.x` | cosign 签名 + SBOM |
| alertmanager | `prom/alertmanager:v0.27.x` | cosign 签名 + SBOM |

> **强约束**: 业务 Pod **禁止**拉取 `latest` 标签；只允许 SHA256 digest 或固定版本。

### 8.2 运行时加固

```yaml
# Pod Security Standards: restricted
securityContext:
  runAsNonRoot: true
  runAsUser: 65532          # nobody
  runAsGroup: 65532
  readOnlyRootFilesystem: true
  allowPrivilegeEscalation: false
  capabilities:
    drop:
      - ALL
  seccompProfile:
    type: RuntimeDefault
```

### 8.3 资源限制

| 组件 | CPU Request | Memory Request | CPU Limit | Memory Limit |
|---|---|---|---|---|
| prometheus | 500m | 2Gi | 2 | 8Gi |
| grafana | 100m | 256Mi | 500m | 512Mi |
| loki | 500m | 1Gi | 2 | 4Gi |
| tempo | 500m | 1Gi | 2 | 4Gi |
| otel-collector | 200m | 512Mi | 1 | 2Gi |
| alertmanager | 50m | 64Mi | 200m | 256Mi |

## 9. Cardinality 攻击防御

### 9.1 业务 label 强校验

```yaml
# OTel Collector filter processor
processors:
  filter:
    metrics:
      metric:
        # 业务 label 白名单
        - 'http_requests_total'
        - 'http_request_duration_seconds'
        - 'gitgit_*'
        exclude:
          match_type: regexp
          metric_names:
            - 'go_.*'         # 禁止业务报 Go runtime metric
            - 'process_.*'
  transform:
    metric_statements:
      - context: metric
        statements:
          # 业务 label 白名单（黑名单默认值会 panic）
          - truncate_all(attributes, 50)  # 截断超长值
          - limit(attributes, ..., ['http.method', 'http.status_code', 'service', 'tenant'])
```

### 9.2 Prometheus 强校验

```yaml
# prometheus.yml
global:
  external_labels:
    cluster: prod
    env: production

# 抓取时丢弃超 cardinality label
metric_relabel_configs:
  - source_labels: [__name__]
    regex: 'go_gc_.*'
    action: drop
  - source_labels: [user]
    regex: '.*@.*'
    action: labeldrop
```

### 9.3 Cardinality 告警

```yaml
- alert: PrometheusHighCardinality
  expr: |
    count by (__name__) ({__name__=~".+"}) > 10000
  for: 10m
  labels:
    severity: warning
    domain: observability
  annotations:
    summary: "Metric {{ $labels.__name__ }} cardinality > 10k"
    runbook: |
      1. 检查业务代码: 是否新加 label 无界
      2. 检查 OTel Collector: 是否漏配 limit
      3. 重启 Prometheus 强制重载
```

## 10. 备份与不可篡改

### 10.1 监控数据备份

| 数据 | 备份 | 保留 | 加密 |
|---|---|---|---|
| Prometheus TSDB | 每日 snapshot | 30d | AES-256 |
| Grafana DB (Postgres) | 每日 pg_dump | 30d | AES-256 |
| Loki chunks | S3 lifecycle 90d | 90d | SSE-S3 |
| Tempo blocks | S3 lifecycle 30d | 30d | SSE-S3 |

### 10.2 不可篡改 admin_audit（ADR-0008）

> 监控组件**本身不存储** admin_audit。`admin_audit` 在 PG 内，由 wal2json 同步。

```promql
# 关键告警：哈希链断裂 → critical + compliance
- alert: AdminAuditChainBroken
  expr: gitgit_audit_hash_chain_broken == 1
```

## 11. 访问审计

### 11.1 Grafana 访问日志

```ini
[log]
level = info
filters = "grafana.data_source.*,grafana.dashboard.*,grafana.user.*,alerting.*"
```

### 11.2 Prometheus 访问日志

```yaml
# prometheus cli flag
--web.enable-lifecycle   # 启用 reload
--web.enable-remote-write-receiver  # 启用 remote write receiver（需 mTLS）
--web.console.libraries=/usr/share/prometheus/console_libraries
```

> **注**: Prometheus 自身访问日志有限，依赖 Kubernetes audit log 补充。

### 11.3 Kubernetes Audit Policy

```yaml
# audit-policy.yaml
apiRules:
  - level: RequestResponse
    resources:
      - group: ""
        resources: ["secrets", "configmaps"]
    namespaces: ["observability"]
  - level: Metadata
    resources:
      - group: "apps"
        resources: ["deployments", "statefulsets"]
    namespaces: ["observability"]
omitStages:
  - RequestReceived
```

## 12. 应急响应

### 12.1 监控系统被入侵

| 步骤 | 操作 |
|---|---|
| T+0 | 隔离 observability namespace：`kubectl cordon ...` |
| T+5min | 暂停所有监控组件 `kubectl scale deploy --replicas=0 ...` |
| T+10min | 切换到备用监控系统（不部署在同一集群） |
| T+1h | 启动 IR 流程 + 全集群扫描 |
| T+24h | 恢复后做渗透测试 + 加固 |

### 12.2 数据泄露

| 步骤 | 操作 |
|---|---|
| T+0 | 立即禁用所有监控数据源 read |
| T+5min | 评估泄露范围（哪些 query 命中） |
| T+30min | 启动 IR 流程 + 通知合规 |
| T+1d | 修复 + 强化脱敏规则 + 复盘 |

## 13. 关联文档

- 上游: [`../design/detailed-design/09-security-impl.md`](../design/detailed-design/09-security-impl.md)（应用层安全）
- 上游: [`01-architecture.md`](01-architecture.md) §2 架构分层
- 下游: [`11-deployment-design.md`](11-deployment-design.md) §6 NetworkPolicy 部署
- 下游: [`15-self-review-v2.md`](15-self-review-v2.md) §4 安全复盘

## 14. 需求 ID 索引

| 需求 ID | 标题 | 优先级 |
|---|---|---|
| OBS-REQ-014 | 监控安全 | P0 |
| OBS-REQ-015 | 数据保护 | P0 |
| OBS-REQ-016 | 网络隔离 | P0 |
| OBS-SEC-001 | Grafana RBAC | P0 |
| OBS-SEC-002 | Prometheus mTLS | P0 |
| OBS-SEC-003 | Loki 字段脱敏 | P0 |
| OBS-SEC-004 | Tempo 字段过滤 | P0 |
| OBS-SEC-005 | K8s NetworkPolicy | P0 |
| OBS-SEC-006 | Secret 管理 | P0 |
| OBS-SEC-007 | 镜像签名 | P0 |
| OBS-SEC-008 | 运行时加固 | P0 |
| OBS-SEC-009 | Cardinality 防御 | P0 |
| OBS-SEC-010 | 不可篡改审计 | P0 |
| OBS-SEC-011 | 访问审计 | P0 |
| OBS-SEC-012 | 应急响应 | P0 |
