# 分阶段实施方案 / Phased Implementation Plan

> **关联 OBS-REQ**: OBS-REQ-023（分阶段实施） / OBS-REQ-024（风险控制）
> **关联设计**: [`11-deployment-design.md`](11-deployment-design.md) / [`14-code-impact.md`](14-code-impact.md)
> **目标**: 8 阶段分步实施，每阶段可独立验证 / 回滚；**禁止**一次性改造全量
> **节奏**: 每阶段 1-2 周；总周期 12-16 周

## 1. 总览

| Phase | 名称 | 周期 | 风险 | 验证 | 回滚 |
|---|---|---|---|---|---|
| **Phase 0** | 现状调查 + ADR | 1 周 | 低 | 文档完成 | — |
| **Phase 1** | 基础设施监控 | 2 周 | 低 | Node / K8s dashboard | 关闭 scrape |
| **Phase 2** | 应用 Metrics | 2 周 | 中 | RED dashboard | 移除 OTel SDK |
| **Phase 3** | 日志集中化 | 1.5 周 | 中 | 日志查询可用 | 回退 stdout |
| **Phase 4** | Distributed Trace | 2 周 | 高 | 端到端 trace 完整 | 关闭 OTLP 出口 |
| **Phase 5** | Dashboard 9 大盘 | 1.5 周 | 低 | 全部 panel 通过 5 问 | — |
| **Phase 6** | Alert 多窗口 | 1.5 周 | 中 | 告警触发演练 | 关闭 alert rule |
| **Phase 7** | SLO 体系 | 1 周 | 低 | 错误预算可视化 | — |
| **Phase 8** | 自动化运维 | 2 周 | 中 | 自动响应流程 | 关闭自动动作 |

**总周期**: 12.5-16 周

## 2. Phase 0: 现状调查 + ADR（已完成）

> **状态**: ✅ **本任务输出**。本阶段交付物已生成。

### 2.1 交付物

- [x] `00-current-state-analysis.md` 现状分析
- [x] `01-architecture.md` 4 层架构
- [x] `02-metrics.md` 命名规约 + 指标体系
- [x] `03-logs.md` 日志规约
- [x] `04-tracing.md` Trace 设计
- [x] ADR-0011 可观测性体系
- [x] 需求追溯矩阵

### 2.2 验证

```bash
# 文档交叉引用
scripts/check-anchors.ps1
# 期望: 0 broken links
# 期望: 0 日文字符（除 workflow.md §4 例外）
```

### 2.3 完成判据

- [x] 现状盘点完整（服务 / DB / 中间件）
- [x] 架构分层清晰
- [x] 技术选型有理由
- [x] ADR 通过评审
- [x] 需求 ID 完整

## 3. Phase 1: 基础设施监控（Week 1-2）

### 3.1 目标

仅监控基础设施层（Node / K3s / DB 物理资源），不触业务代码。

### 3.2 交付物

- [ ] `kube-prometheus-stack` 部署（Helm）
- [ ] node-exporter（DaemonSet）
- [ ] kube-state-metrics
- [ ] pg_exporter 部署
- [ ] 10-Infrastructure Dashboard
- [ ] 20-Kubernetes Dashboard
- [ ] Node 基础告警 5 条
- [ ] K3s 基础告警 5 条

### 3.3 实施步骤

```bash
# Day 1-2: 准备 namespace + 节点
kubectl apply -f deploy/observability/base/namespace.yaml
kubectl apply -f deploy/observability/base/resource-quota.yaml
kubectl label node k3s-agent-obs-1 workload-class=observability
kubectl taint node k3s-agent-obs-1 workload-class=observability:NoSchedule

# Day 3-5: 部署 kube-prometheus-stack
helm repo add prometheus-community https://prometheus-community.github.io/helm-charts
helm install prometheus prometheus-community/kube-prometheus-stack \
  -n observability \
  -f deploy/observability/kube-prometheus-stack/values.yaml

# Day 6-7: 部署 pg_exporter
kubectl apply -f deploy/observability/exporters/pg-exporter.yaml

# Day 8-9: 配置 dashboard
kubectl apply -f deploy/observability/dashboards/10-infra.json
kubectl apply -f deploy/observability/dashboards/20-k8s.json

# Day 10: 验证 + 调整告警
promtool check rules deploy/observability/rules/infrastructure.yaml
```

### 3.4 风险与缓解

| 风险 | 概率 | 缓解 |
|---|---|---|
| kube-prometheus-stack 默认配置过重 | 高 | 自定义 values.yaml，资源限制 |
| pg_exporter 连接 PG 失败 | 中 | 用 readonly 账号，配置连接测试 |
| 监控节点资源不足 | 中 | 独立 2 节点 |
| 抓取频率过高影响业务 | 低 | 默认 15s 足够 |

### 3.5 验证

- [ ] Grafana 可访问（OIDC 登录）
- [ ] 10-Infrastructure 所有 panel 有数据
- [ ] 20-Kubernetes 所有 panel 有数据
- [ ] Node 告警测试（手动停止 node-exporter 模拟）

### 3.6 回滚

```bash
helm uninstall prometheus -n observability
kubectl delete namespace observability
```

## 4. Phase 2: 应用 Metrics（Week 3-4）

### 4.1 目标

业务 crate 接入 OTel SDK，上报 RED 指标。**不开启 trace / log 改造**。

### 4.2 交付物

- [ ] `gitgit-observability` crate（OTel SDK 封装）
- [ ] 各业务 crate Cargo.toml 引入依赖
- [ ] main.rs 初始化 OTel meter provider
- [ ] HTTP middleware（tower-otel）
- [ ] sqlx instrument（自动）
- [ ] 30-Application Dashboard
- [ ] 业务 RED 告警 10 条

### 4.3 实施步骤

```bash
# Day 11-13: 实现 gitgit-observability crate
# 文件: crates/gitgit-observability/src/lib.rs
# 导出: init_meter, init_tracer, init_logger, shutdown

# Day 14-16: 各 crate 集成
# 编辑每个 crate:
#   1. Cargo.toml: + opentelemetry + opentelemetry-otlp
#   2. main.rs: 调用 init_meter() / shutdown()
#   3. Axum: 挂载 tower_http::trace::TraceLayer

# Day 17-18: 部署 OTel Collector
kubectl apply -f deploy/observability/otel-collector/

# Day 19-20: 验证 + dashboard
```

### 4.4 关键代码

```rust
// crates/gitgit-observability/src/lib.rs
use opentelemetry::metrics::MeterProvider;
use opentelemetry::trace::TracerProvider;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{
    metrics::SdkMeterProvider,
    runtime,
    trace::TracerProvider as SdkTracerProvider,
    Resource,
};

pub fn init_meter() -> SdkMeterProvider {
    let exporter = opentelemetry_otlp::new_exporter()
        .tonic()
        .with_endpoint("http://otel-collector.observability.svc:4317");

    let provider = opentelemetry_otlp::new_pipeline()
        .metrics(opentelemetry_sdk::runtime::Tokio)
        .with_exporter(exporter)
        .with_resource(Resource::new(vec![
            opentelemetry::KeyValue::new("service.name", env!("CARGO_PKG_NAME")),
            opentelemetry::KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
        ]))
        .build()
        .expect("failed to build meter provider");

    opentelemetry::global::set_meter_provider(provider.clone());
    provider
}

pub fn init_tracer() -> SdkTracerProvider {
    // 类似 meter
}

pub fn init_logger() -> impl tracing_subscriber::Layer<tracing_subscriber::Registry> {
    // JSON formatter + trace_id injection
}

pub async fn shutdown(provider: SdkMeterProvider) {
    let _ = provider.shutdown();
}
```

### 4.5 风险与缓解

| 风险 | 概率 | 缓解 |
|---|---|---|
| 业务代码改动引入 bug | 中 | 单元测试 + 灰度（金丝雀） |
| OTel SDK 性能开销超预期 | 低 | Phase 1 压测验证 + 降级开关 |
| 引入 GPL/AGPL 依赖 | 低 | 强制 cargo-deny 扫描 |
| Cardinality 爆炸 | 中 | OTel Collector limit processor |

### 4.6 验证

- [ ] 所有业务 crate 启动后能上报 metric
- [ ] 30-Application RPS / Error / Latency 有数据
- [ ] 压测 1k QPS 时 OTel Collector CPU < 50%
- [ ] 业务 P99 延迟增量 < 5ms

### 4.7 回滚

```rust
// 在 main.rs 中通过环境变量控制
if std::env::var("OTEL_ENABLED").unwrap_or_default() == "true" {
    let meter = init_meter();
    // ...
}
```

```bash
# 全量关闭
kubectl set env deploy/gitgit-server OTEL_ENABLED=false
```

## 5. Phase 3: 日志集中化（Week 5-6.5）

### 5.1 目标

业务日志从 stdout 转为 Loki 集中存储。trace_id 注入。

### 5.2 交付物

- [ ] Loki 部署
- [ ] OTel Collector logging pipeline
- [ ] JSON 格式化层
- [ ] 日志脱敏 transform
- [ ] 日志查询 dashboard
- [ ] ERROR 日志告警 3 条

### 5.3 实施步骤

```bash
# Day 21-23: 部署 Loki
helm install loki grafana/loki -n observability -f deploy/observability/loki/values.yaml

# Day 24-26: 业务 crate 改造
# - tracing-subscriber JSON layer
# - 日志字段标准化（trace_id / span_id）
# - 静态扫描 + 字段过滤

# Day 27-28: 验证
```

### 5.4 关键代码

```rust
// crates/gitgit-observability/src/logger.rs
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use tracing_subscriber::fmt::format::FmtSpan;

pub fn init_logger() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    let fmt_layer = tracing_subscriber::fmt::layer()
        .json()
        .with_current_span(true)
        .with_span_list(false)
        .with_target(true)
        .with_file(false)
        .with_line_number(false)
        .with_thread_ids(false);

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt_layer)
        .init();
}

// 使用
use tracing::{info, instrument};

#[instrument]
async fn handle(req: Request) -> Response {
    info!(
        method = %req.method(),
        path = %req.path(),
        "handling request"
    );
    // 输出: {"timestamp":"...","level":"INFO","message":"handling request",
    //        "span":{"name":"handle"},"target":"gitgit_server::handler",...}
}
```

### 5.5 风险与缓解

| 风险 | 概率 | 缓解 |
|---|---|---|
| 日志包含敏感数据 | 高 | 静态扫描 + transform |
| 日志量爆炸 | 中 | 分级保留 + INFO 限流 |
| 业务线程阻塞 | 低 | 异步 + 缓冲 |

### 5.6 验证

- [ ] Loki 中有业务日志
- [ ] 日志含 trace_id
- [ ] 敏感字段已脱敏
- [ ] ERROR 日志可触发告警

### 5.7 回滚

```rust
// 切换到 stdout
if std::env::var("LOKI_ENABLED").unwrap_or_default() == "true" {
    init_loki_logger();
} else {
    init_stdout_logger();
}
```

## 6. Phase 4: Distributed Trace（Week 7-8）

### 6.1 目标

端到端 trace，含 HTTP / gRPC / sqlx / 中心事件 / 外部 API。

### 6.2 交付物

- [ ] Tempo 部署
- [ ] OTel Collector trace pipeline
- [ ] tail_sampling 配置
- [ ] 业务 span 标准化
- [ ] trace_id 跨进程传播
- [ ] 30-Application Trace 面板

### 6.3 实施步骤

```bash
# Day 29-31: 部署 Tempo
helm install tempo grafana/tempo -n observability -f deploy/observability/tempo/values.yaml

# Day 32-34: 业务 crate 集成
# - tracing_opentelemetry layer
# - gRPC interceptor
# - sqlx 已有（自动）
# - 中心事件：手动 instrument 跨进程 span

# Day 35-36: tail_sampling 调优
# Day 37-38: 验证
```

### 6.4 关键代码

```rust
// crates/gitgit-observability/src/tracer.rs
use opentelemetry::trace::TracerProvider as _;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::trace::TracerProvider;
use opentelemetry_sdk::Resource;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::EnvFilter;

pub fn init_tracer() -> TracerProvider {
    let exporter = opentelemetry_otlp::new_exporter()
        .tonic()
        .with_endpoint("http://otel-collector.observability.svc:4317");

    let provider = opentelemetry_otlp::new_pipeline()
        .tracing(opentelemetry_sdk::runtime::Tokio)
        .with_exporter(exporter)
        .with_trace_config(
            opentelemetry_sdk::trace::Config::default()
                .with_resource(Resource::new(vec![
                    opentelemetry::KeyValue::new("service.name", env!("CARGO_PKG_NAME")),
                ]))
                .with_sampler(opentelemetry_sdk::trace::Sampler::ParentBased(
                    Box::new(opentelemetry_sdk::trace::Sampler::TraceIdRatioBased(0.05))
                ))
        )
        .install_batch(opentelemetry_sdk::runtime::Tokio)
        .expect("failed to install tracer");

    opentelemetry::global::set_tracer_provider(provider.clone());
    provider
}

pub fn attach_tracing() {
    use opentelemetry::trace::TracerProvider;
    let tracer = opentelemetry::global::tracer_provider()
        .tracer(env!("CARGO_PKG_NAME"));

    let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);
    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env())
        .with(otel_layer)
        .init();
}
```

### 6.5 风险与缓解

| 风险 | 概率 | 缓解 |
|---|---|---|
| Trace 包含 SQL 参数（敏感） | 中 | transform processor 过滤 |
| Trace 采样率过高致后端压力 | 中 | tail_sampling 5% |
| 跨进程 trace_id 丢失 | 中 | 显式注入 W3C Trace Context |

### 6.6 验证

- [ ] 端到端 trace 完整（HTTP → gRPC → sqlx）
- [ ] trace_id 跨服务传播
- [ ] tail_sampling 决策正确
- [ ] 错误请求 100% 采样

### 6.7 回滚

```bash
# 关闭 OTLP 出口
kubectl set env deploy/gitgit-server OTEL_TRACES_EXPORTER=none
```

## 7. Phase 5: Dashboard 9 大盘（Week 9-10.5）

### 7.1 目标

完整 9 大盘上线，DRILL-DOWN 链接全通。

### 7.2 交付物

- [ ] 9 大盘 JSON（实际生成）
- [ ] Provisioning 配置
- [ ] Variable 配置
- [ ] DataLink 配置（trace_id / log / runbook）
- [ ] Folder 权限

### 7.3 实施步骤

```bash
# Day 39-42: 生成 dashboard JSON
# 基于 07-dashboard-design.md 表格 → JSON
python3 scripts/gen-dashboards.py  # 项目脚本（v1 由 agent 生成）

# Day 43-45: 部署到 Grafana
kubectl create configmap -n observability grafana-dashboards \
  --from-file=deploy/observability/dashboards/ \
  --dry-run=client -o yaml | kubectl apply -f -

# Day 46-47: 验证 panel 渲染
# Day 48-49: 调整 + 权限
```

### 7.4 验证

- [ ] 9 大盘全部加载
- [ ] 每个 panel 含 5 问之一
- [ ] Variable 切换正常
- [ ] DataLink 跳转正常
- [ ] Folder 权限生效

## 8. Phase 6: Alert 多窗口（Week 11-12.5）

### 8.1 目标

完整告警规则上线，**多窗口 + 持续时间 + 抑制**。

### 8.2 交付物

- [ ] 6 域告警规则
- [ ] Alertmanager 路由 + 抑制
- [ ] PagerDuty / Slack 集成
- [ ] 告警演练

### 8.3 实施步骤

```bash
# Day 50-52: 编写告警规则
# 基于 08-alert-design.md 5 节

# Day 53-55: Alertmanager 配置
# - 路由
# - 抑制规则
# - 通道集成

# Day 56-58: 演练
# - 手动停止 node-exporter
# - 手动注入慢查询
# - 验证 PagerDuty 触发
```

### 8.4 验证

- [ ] critical 告警 5min 内响 PagerDuty
- [ ] 抑制规则生效
- [ ] runbook 链接可访问
- [ ] 静默可手动应用

## 9. Phase 7: SLO 体系（Week 13）

### 9.1 目标

10 个核心 SLO 上线，错误预算可视化。

### 9.2 交付物

- [ ] SLO 定义（sloth / pyrra）
- [ ] Recording rules
- [ ] 90-SLO Dashboard
- [ ] SLO 告警（双窗口 Burn Rate）

### 9.3 实施步骤

```bash
# Day 59-61: 编写 SLO + recording rules
# 基于 09-slo-design.md

# Day 62-63: 部署 + 验证
# Day 64-65: 评审 + 调整
```

## 10. Phase 8: 自动化运维（Week 14-16）

### 10.1 目标

减少 on-call 工作量；自动修复简单问题。

### 10.2 交付物

- [ ] 自动扩容（KEDA / HPA 增强）
- [ ] 自动清理（孤儿 / 旧备份）
- [ ] 自动响应剧本（轻量级 webhook）
- [ ] 容量预测（时序预测）
- [ ] 运维 runbook 自动化

### 10.3 实施步骤

```bash
# Day 66-70: KEDA 部署（事件驱动扩缩容）
# Day 71-75: 自动清理 CronJob
# Day 76-80: 自动响应 webhook（轻量）
# Day 81-85: 容量预测
# Day 86-90: 整体回归测试
```

### 10.4 风险与缓解

| 风险 | 概率 | 缓解 |
|---|---|---|
| 自动扩容振荡 | 中 | HPA 行为调优 + stabilization window |
| 自动清理误删 | 中 | 灰名单（90d 未访问才删） |
| 自动响应误操作 | 中 | 任何自动动作必须先 dry-run |

## 11. 整体时间表

```
Week  1  2  3  4  5  6  7  8  9 10 11 12 13 14 15 16
      ├──┴──┤
      P0     P1
         ├──┴──┤
         P2
            ├──┴──┤
            P3
               ├──┴──┤
               P4
                  ├──┴──┤
                  P5
                     ├──┴──┤
                     P6
                        ├──┤
                        P7
                           ├──┴──┴──┤
                           P8
```

> **关键路径**: P0 → P1 → P2 → P4 → P5 → P6 → P7
> **非关键**: P3（可与 P4 并行） / P8（最后）

## 12. 总体验收

### 12.1 验收清单（每 Phase 通用）

- [ ] 文档 / 配置全部入库
- [ ] 部署自动化（GitOps 同步成功）
- [ ] 监控自监控（meta-observability）无告警
- [ ] 回滚脚本测试通过
- [ ] on-call 培训完成

### 12.2 整体 Go-Live 标准

- [ ] 9 大盘全部活跃
- [ ] 10 个 SLO 全部上线
- [ ] critical 告警 5min 内必有响应
- [ ] MTTR < 4h
- [ ] 告警疲劳 < 10 告警 / shift
- [ ] meta-observability 健康

## 13. 关联文档

- 上游: [`14-code-impact.md`](14-code-impact.md) 改造清单
- 上游: [`11-deployment-design.md`](11-deployment-design.md) 部署模式
- 下游: [`15-self-review-v2.md`](15-self-review-v2.md) 复盘

## 14. 需求 ID 索引

| 需求 ID | 标题 | 优先级 |
|---|---|---|
| OBS-REQ-023 | 分阶段实施 | P0 |
| OBS-REQ-024 | 风险控制 | P0 |
| IMPL-REQ-001 | Phase 1 基础设施 | P0 |
| IMPL-REQ-002 | Phase 2 应用 Metrics | P0 |
| IMPL-REQ-003 | Phase 3 日志 | P0 |
| IMPL-REQ-004 | Phase 4 Trace | P0 |
| IMPL-REQ-005 | Phase 5 Dashboard | P0 |
| IMPL-REQ-006 | Phase 6 Alert | P0 |
| IMPL-REQ-007 | Phase 7 SLO | P0 |
| IMPL-REQ-008 | Phase 8 自动化 | P1 |
| IMPL-REQ-009 | 每 Phase 回滚 | P0 |
| IMPL-REQ-010 | 整体 Go-Live | P0 |
