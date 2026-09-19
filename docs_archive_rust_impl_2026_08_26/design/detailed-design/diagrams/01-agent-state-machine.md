# Agent 状态机

> 详细设计：[`../04-agent-runtime.md`](../04-agent-runtime.md) §4.4 (状态机)
> 关联 REQ：AGT-REQ-001 (Agent 状态机)
> 关联流程：任务 62 (UT) / 83 (ST)

## 状态转移图

```mermaid
stateDiagram-v2
    [*] --> Pending: 创建 Run
    Pending --> Running: 资源就绪
    Pending --> Failed: 启动失败 (e.g. 资源不足)

    Running --> AwaitingApproval: 需要人类批准 (AGT-REQ-002)
    Running --> Completed: 正常完成
    Running --> Failed: 运行时错误
    Running --> Compensating: 部分失败需回滚

    AwaitingApproval --> Approved: 人类批准
    AwaitingApproval --> Rejected: 人类拒绝
    AwaitingApproval --> Failed: 批准超时 (默认 24h)

    Approved --> Running: 继续执行
    Approved --> Completed: 无后续步骤
    Rejected --> [*]: 终止

    Compensating --> Completed: 补偿成功
    Compensating --> Failed: 补偿失败 (需人工)

    Completed --> [*]
    Failed --> [*]
```

## 状态字段

`agent_runs.state` 字段值集合（与状态机对齐）：

| 状态 | 描述 | 终止？ |
|---|---|---|
| `pending` | 等待资源分配 | 否 |
| `running` | 实际执行中 | 否 |
| `awaiting_approval` | 等待人类批准 (AGT-REQ-002) | 否 |
| `approved` | 人类已批准 | 否 |
| `rejected` | 人类已拒绝 | 是 |
| `compensating` | 执行补偿 | 否 |
| `completed` | 正常完成 | 是 |
| `failed` | 失败 | 是 |

## 转移守卫条件

| 转移 | 守卫 |
|---|---|
| Pending → Running | 资源配额检查通过 |
| Running → AwaitingApproval | 检测到 AGT-REQ-002 标记的 action |
| AwaitingApproval → Approved | 人类批准 (signed JWT) |
| AwaitingApproval → Failed | `now() > started_at + human_approval_timeout_sec` |
| Running → Compensating | 事务失败 + 存在补偿步骤 |
