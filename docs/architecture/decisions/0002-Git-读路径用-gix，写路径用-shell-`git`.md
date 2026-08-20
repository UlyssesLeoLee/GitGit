# ADR-0002: Git 读路径用 gix，写路径用 shell `git`

> **Status**: Accepted
> **Date**: 2026-08-19
> **Deciders**: SA + TL + EM
> **Consulted**: SEC + PO
> **Informed**: 全体

## Context and Problem Statement

Git 服务需要：(a) 高性能读 (clone / fetch / log / diff) (b) 安全写 (push + hooks)。候选：gix (纯 Rust) / libgit2 (C 绑定) / shell `git` 子进程。

## Decision Drivers

- 读路径性能
- 写路径必须可靠 (绝对不能丢 commit)
- 维护成本 (不重复造 Git 协议轮子)

## Considered Options

1. **gix 全栈 (读 + 写)**
2. **shell `git` 全栈**
3. **gix (读) + shell `git` (写) — **chosen****

## Decision Outcome

**Chosen option**: "Option C", because Option C。读路径高频 → 性能优先；写路径低频但关键 → 复用成熟 `git` 设施。

### Consequences

**Good:**
- [+] 读 P99 ≤ 50ms
- [+] 写路径零数据丢失风险
- [+] gix 与 shell `git` 一致性测试覆盖 100%

**Bad:**
- [-] 需要维护两套代码路径
- [-] 调试跨语言边界更复杂

### Confirmation

tech-selection.md §7 / 详细设计 §06 (Git Server) / 实施前 QA QA-011

## Pros and Cons of the Options

### gix 全栈 (读 + 写)

[+] 纯 Rust；性能强；与 Rust 生态契合
[-] gix 写路径成熟度低于读路径；push / receive-pack 协议支持有限

### shell `git` 全栈

[+] 零兼容风险；与系统 Git 完全一致
[-] 子进程开销；fork / exec 不如直接调用；权限边界模糊

### gix (读) + shell `git` (写) — **chosen**

[+] (see chosen)
[-] (see chosen)


## References

[`../tech-selection.md` §7](../tech-selection.md#7-git-库gixgitoxide-git-protocol-library)

## Revision History

| Date | Author | Change |
|---|---|---|
| YYYY-MM-DD | — | Initial draft |
| YYYY-MM-DD | — | Status → Accepted |
