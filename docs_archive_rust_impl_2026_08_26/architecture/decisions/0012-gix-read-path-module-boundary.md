# ADR-0012: gix 读路径与 shell `git` 写路径的模块边界

> **Status**: Proposed
> **Date**: 2026-08-23
> **Deciders**: SA + TL + EM
> **Consulted**: SEC + SRE
> **Informed**: 全部

## Context and Problem Statement

[技术选型文档 §7](../tech-selection.md#7-git-生态-gix--shell-git-强约束) 选定 gix (gitoxide 0.66+) 用于 Git 读路径、shell `git` 进程用于写路径,但没有明确"哪些 API 走 gix、哪些走 shell"。

实施阶段如果各模块独立决定,会出现:
1. 同一类操作在不同模块用不同实现,行为不一致(如:一个 `git log` 走 gix,另一个走 shell)
2. 写路径误用纯 Rust 库替代,违反 [phase10-architecture §6 第 2 项](../../requirements/phase10-architecture.md) 的强约束
3. 性能与安全边界模糊,难做 code review

## Decision Drivers

- [DRIVER-1] 读路径性能与可控性(避免 spawn 进程的开销)
- [DRIVER-2] 写路径必须由成熟 Git 实现处理 pack / negotiation / hooks
- [DRIVER-3] 模块间一致,避免"哪个 API 走哪条路径"的反复讨论
- [DRIVER-4] 测试与回归可观察(每个 Git 操作都有清晰的"走哪条路径"trace)

## Considered Options

1. **Option A — 按操作类型分**(本次推荐): 读操作 (clone/fetch/log/diff/rev-parse/blame/ls-tree/cat-file -p) 走 gix;写操作 (push/commit/pack-objects/receive-pack/update-ref) 走 shell `git`
2. **Option B — 按数据大小分**: 小于某阈值走 gix,否则 shell
3. **Option C — 全部走 shell `git`**: 简单但失去性能优势,与技术选型 §7 矛盾

## Decision Outcome

**Chosen option**: "Option A — 按操作类型分", because 与技术选型 §7 决策一致;模块边界清晰;code review 有明确判定标准。

### 具体模块边界

| Git 操作 | 实现 | 理由 |
|---|---|---|
| `clone` (拉取) | gix | 只读;性能关键 |
| `fetch` (拉取更新) | gix | 只读;频率高 |
| `log` / `rev-walk` / `ls-tree` / `cat-file -p` / `blame` / `diff` | gix | 只读 API |
| `rev-parse` (引用解析) | gix | 只读 |
| `push` (推送) | shell `git` | 写;需要 pack/negotiation 成熟实现 |
| `commit` (创建提交) | shell `git` | 写;hook 触发依赖 |
| `pack-objects` (打包) | shell `git` | 写;高风险 |
| `receive-pack` (服务端接收) | shell `git` | 写;需要 pre/post-receive hook |
| `update-ref` (更新引用) | shell `git` | 写;锁语义复杂 |
| `merge` / `rebase` (合并) | shell `git` | 写;算法与冲突处理复杂 |

### Consequences

**Good:**
- [+] 实施时"哪个 API 走哪里"一目了然,review 成本低
- [+] 读路径快 (gix 是 Rust 原生,无进程 fork)
- [+] 写路径零风险 (用项目自带的 git 二进制,bug 由上游 Git 团队负责)

**Bad:**
- [-] 两套 API 需要学习,模块 owner 需明确(列入 [specs/test-specification.md §06](../../specs/test-specification.md#6-git-服务器) 测试矩阵)
- [-] gix 与 Git 输出格式若存在边缘差异(如:date format、ref format),需要在 CI 验证一致性

### Confirmation

[CONFIRM] 在 CI 集成测试中:对同一仓库同时用 gix 和 shell `git` 执行 `log` / `ls-tree` / `cat-file`,对比输出必须 byte-equal(忽略时间戳精度)。详设 §06 测试矩阵 TC-GIT-001~010 覆盖此验证。

## Pros and Cons of the Options

### Option A — 按操作类型分
[+] 边界清晰;code review 简单;与上游 Git 行为一致性最高
[-] 需要维护 gix 升级与 Git 升级的同步

### Option B — 按数据大小分
[+] 看似灵活
[-] 阈值难以确定;边界 case 行为不一致;code review 难

### Option C — 全部走 shell `git`
[+] 实现简单
[-] 失去 gix 性能优势;违反技术选型 §7

## References

- [技术选型 §7 Git 生态](../tech-selection.md#7-git-生态-gix--shell-git-强约束)
- [phase10-architecture §6 第 2 项](../../requirements/phase10-architecture.md) — 强约束"禁止为 Rust 原则重复实现成熟高风险 Git 基础设施"
- [详细设计 §06 Git 服务器](../../design/detailed-design/06-git-server.md)
- [specs/test-specification.md §06 Git 服务器测试](../../specs/test-specification.md#6-git-服务器)

## Revision History

| Date | Author | Change |
|---|---|---|
| 2026-08-23 | Mavis (AI 起草) | Initial draft (Proposed) |
| YYYY-MM-DD | — | Status → Accepted (待 SA + TL + EM 签核) |
