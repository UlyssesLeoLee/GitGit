# ADR-0006: 采用 OCI 容器化（Docker / containerd）+ K8s 用于 V1+ Cloud

> **Status**: Accepted
> **Date**: 2026-08-19
> **Deciders**: SA + TL + EM
> **Consulted**: SEC + PO
> **Informed**: 全体

## Context and Problem Statement

部署形态：MVP 本地单 binary；V1+ Cloud 多实例。MVP 阶段是否引入 K8s。

## Decision Drivers

- MVP 本地部署简单
- V1+ Cloud 多 Pod
- 运维工具一致性

## Considered Options

1. **MVP 直接上 K8s (kind / k3d 本地)**
2. **MVP = 单 binary + systemd；V1+ = K8s Helm chart**

## Decision Outcome

**Chosen option**: "Option B", because Option B。MVP 用单 binary + systemd；V1+ 出 Helm chart + ArgoCD。Docker 镜像 MVP 也提供（备用），但**不** 强制 K8s。

### Consequences

**Good:**
- [+] MVP 部署 : 1 binary
- [+] V1+ 部署 : helm install
- [+] App Manifest 设计从 MVP 起按「可拆 Pod」写

**Bad:**
- [-] 需要维护两套部署文档
- [-] 本地 K8s 体验待 V1

### Confirmation

基本设计 §08.6 (V1+ K8s) / 详细设计 §13.5 / 实施前 QA QA-016

## Pros and Cons of the Options

### MVP 直接上 K8s (kind / k3d 本地)

[+] 一套部署形态
[-] Local-First 复杂化；MVP 阶段过度工程

### MVP = 单 binary + systemd；V1+ = K8s Helm chart

[+] MVP 极简；V1+ 平滑过渡
[-] 两套部署工具


### Option C

[+] ...
[-] ...

## References

[`../../design/basic-design/08-operations-design.md#86-v1-k8s-部署形态`](../../design/basic-design/08-operations-design.md#86-v1-k8s-部署形态-v1-k8s-deployment-topology)

## Revision History

| Date | Author | Change |
|---|---|---|
| YYYY-MM-DD | — | Initial draft |
| YYYY-MM-DD | — | Status → Accepted |
