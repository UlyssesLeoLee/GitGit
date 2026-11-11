# ADR-0008: Plugin / App Manifest Schema (app.yaml) 与版本兼容策略

> **Status**: Accepted
> **Date**: 2026-08-19
> **Deciders**: SA + TL + EM
> **Consulted**: SEC + PO
> **Informed**: 全体

## Context and Problem Statement

App 一级化后，需要 App Manifest 定义权限 / schema / API。Schema 演化策略？

## Decision Drivers

- App 沙箱权限显式声明
- App 升级回滚
- Manifest 版本管理

## Considered Options

1. **Schema-less (任意 JSON)**
2. **JSON Schema + SemVer**
3. **K8s-style CRD (CustomResourceDefinition)**

## Decision Outcome

**Chosen option**: "Option B", because Option B。MVP 用 JSON Schema 1.0 + SemVer。App Manifest 在 [`../../design/basic-design/13-app-cluster-and-plugins.md`](../../design/basic-design/13-app-cluster-and-plugins.md) §13.2 定义。

### Consequences

**Good:**
- [+] Schema 校验在安装时
- [+] SemVer + deprecation policy
- [+] 向后兼容有依据

**Bad:**
- [-] Schema 仓库需维护
- [-] App 作者需学习

### Confirmation

基本设计 §13.2 / 详细设计 §12.1 / 实施前 QA QA-003

## Pros and Cons of the Options

### Schema-less (任意 JSON)

[+] 灵活
[-] 无法校验；无版本；升级不安全

### JSON Schema + SemVer

[+] 校验；版本；可演化
[-] 需维护 schema 仓库

### K8s-style CRD (CustomResourceDefinition)

[+] (see chosen)
[-] (see chosen)


## References

[`../../design/basic-design/13-app-cluster-and-plugins.md#132-app-manifest-appyaml-schema`](../../design/basic-design/13-app-cluster-and-plugins.md#132-app-manifest-appyaml-schema)

## Revision History

| Date | Author | Change |
|---|---|---|
| YYYY-MM-DD | — | Initial draft |
| YYYY-MM-DD | — | Status → Accepted |
