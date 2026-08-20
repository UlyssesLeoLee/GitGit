# ADR-0004: App 一级化与中心事件总线（不引入独立 Service Mesh）

> **Status**: Accepted
> **Date**: 2026-08-19
> **Deciders**: SA + TL + EM
> **Consulted**: SEC + PO
> **Informed**: 全体

## Context and Problem Statement

扩展性需求：第三方可开发 App 插件。需要：(a) App 一级对象 (b) 跨 App 事件 (c) 沙箱隔离。考虑引入 Istio / Linkerd vs 自建。

## Decision Drivers

- MVP 单进程多 App
- V1+ 可拆 Pod (K8s)
- App 沙箱 DB role 隔离 (AISEC-REQ)

## Considered Options

1. **引入 Istio / Linkerd Service Mesh**
2. **中心事件总线 (PG LISTEN/NOTIFY + Outbox) + App Manifest 抽象**

## Decision Outcome

**Chosen option**: "Option B", because Option B。MVP 单进程多 App (共享 PG，schema 隔离)；V1+ 拆 Pod 时 App 间走 mTLS + NetworkPolicy，**不** 提前上 Service Mesh。

### Consequences

**Good:**
- [+] MVP 部署简单
- [+] 沙箱安全 (DB role 隔离)
- [+] V1+ 平滑过渡

**Bad:**
- [-] App loader 自研
- [-] App 升级需自管（无 K8s Deployment 兜底）

### Confirmation

基本设计 §13 (App 集群) / 详细设计 §12 (App Registry) / 实施前 QA QA-003 (App 沙箱)

## Pros and Cons of the Options

### 引入 Istio / Linkerd Service Mesh

[+] 成熟；mTLS 自动化；流量管理
[-] Local-First 太重；MVP 不必要；K8s 强耦合

### 中心事件总线 (PG LISTEN/NOTIFY + Outbox) + App Manifest 抽象

[+] 无新组件；事务一致；V1+ 可拆 Pod 时 App 间走 mTLS
[-] 需自研 App loader / 沙箱


### Option C

[+] ...
[-] ...

## References

[`../../design/basic-design/13-app-cluster-and-plugins.md`](../../design/basic-design/13-app-cluster-and-plugins.md) · [`../../design/detailed-design/12-app-registry-and-plugin-loader.md`](../../design/detailed-design/12-app-registry-and-plugin-loader.md)

## Revision History

| Date | Author | Change |
|---|---|---|
| YYYY-MM-DD | — | Initial draft |
| YYYY-MM-DD | — | Status → Accepted |
