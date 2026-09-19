# Git Push 流程（Graph-Aware Hook）

> 详细设计：[`../06-git-server.md`](../06-git-server.md) §6.5 (Graph-Aware 钩子)
> 关联 REQ：GIT-REQ-005/006 (钩子 + fail-closed) / GRF-REQ-002 (类型注册)
> 关联流程：任务 67 (IT) / 69 (ITa) / 75 (回归)

## 时序图

```mermaid
sequenceDiagram
    actor Dev as Developer
    participant Git as git client
    participant Hook as pre-receive hook
    participant TypeReg as Type Registry
    participant Policy as Policy Engine
    participant DB as PostgreSQL
    participant Storage as Git Storage (bare repo)

    Dev->>Git: git push origin main
    Git->>Hook: 触发 pre-receive
    activate Hook
    Hook->>TypeReg: 校验 push 中的 commit message
是否引用合法 Issue Node
    TypeReg-->>Hook: 校验结果
    alt 类型注册失败
        Hook-->>Git: 拒绝 + 错误消息
        Git-->>Dev: ! [remote rejected] policy violation
    else 类型注册通过
        Hook->>Policy: evaluate(push, repo)
        Policy->>DB: SELECT policies WHERE enabled
        Policy-->>Hook: allowed=true
        alt 策略拒绝
            Hook-->>Git: 拒绝
        else 策略通过
            Hook->>Storage: update refs (bare repo)
            Hook->>DB: 事务: INSERT nodes(commit), edges, events
            Hook->>DB: NOTIFY event_inserted
            Hook-->>Git: 接受
        end
    end
    deactivate Hook
    Git-->>Dev: push 完成
```

## Fail-Closed 行为

任何一步失败 → 拒绝 push，事务回滚，零数据写入。

`GIT-REQ-006` 强约束。
