# ADR-0010: App 沙箱采用 Wasm + WASI

> **Status**: Accepted
> **Date**: 2026-08-20
> **Deciders**: SA + TL + EM
> **Consulted**: SEC + PO
> **Informed**: 全体

## Context and Problem Statement

详细设计 §12-app-registry-and-plugin-loader.md 提出了 3 个 App 沙箱候选，但未拍板。
Phase 16 收尾时由 EM 决策：

1. **Wasm + WASI** — 跨平台、强隔离、I/O 需 hostcall
2. **子进程 + rlimit + seccomp** — Linux 强隔离、Windows 退化
3. **K8s Pod** — 最强隔离、但 MVP 单进程违反"无新组件"原则

## Decision Drivers

- MVP 单进程部署（不引入 K8s 编排）
- Windows / Linux 跨平台一致
- App 沙箱 DB role 隔离（AISEC-REQ-013 强约束）
- 中心事件总线延迟 ≤ 5s（P50）

## Considered Options

### Wasm + WASI
- 跨平台一致
- 沙箱级隔离（线性内存、明确 hostcall 边界）
- 启动快（< 100ms）

### Native 子进程
- I/O 性能更好
- 但 Windows 隔离弱

### K8s Pod
- 隔离最强
- 但 MVP 阶段不必要

## Decision Outcome

**Chosen option**: "Wasm + WASI"

理由：
1. 跨平台一致（Windows / Linux 行为相同）
2. 沙箱隔离 = 线性内存边界 + 显式 hostcall（满足 AISEC-REQ-013 强约束）
3. MVP 单进程（不破坏 Local-First 部署）
4. 启动快，适合 hot-plug（任务 108）

### Consequences

**Good:**
- 跨平台一致
- 沙箱强隔离
- 启动快（< 100ms cold start）

**Bad:**
- I/O 性能损失（10-30%）
- 需 hostcall 抽象层（filesystem / network / DB 调用全部走 host）
- Wasm 生态仍在演化（部分 Rust crate 不支持 wasm32 target）

### Confirmation

- 验证：crates/gitgit-app 必须含 wasm32-unknown-unknown 编译 target
- 性能：hostcall P99 < 5ms
- 安全：DB 调用必须经 PL/pgSQL SECURITY DEFINER 走 DB role 隔离

## Pros and Cons of the Options

### Wasm + WASI
[+] 跨平台 / 强隔离 / 启动快
[-] I/O 性能损失 / 需 hostcall 抽象 / Wasm 生态限制

### Native 子进程
[+] I/O 性能更好
[-] Windows 隔离弱 / 部署复杂

### K8s Pod
[+] 隔离最强
[-] MVP 不必要 / 部署复杂

## References

- [../../design/basic-design/13-app-cluster-and-plugins.md](../../design/basic-design/13-app-cluster-and-plugins.md) §13.1-13.7
- [../../design/detailed-design/12-app-registry-and-plugin-loader.md](../../design/detailed-design/12-app-registry-and-plugin-loader.md) §12.1-12.4
- [../../architecture/tech-selection.md §7](../../architecture/tech-selection.md) (Git 库选型)
- ADR-0004 (App 一级化与中心事件总线)

## Revision History

| Date | Author | Change |
|---|---|---|
| 2026-08-20 | EM | Initial proposal, status → Accepted |
