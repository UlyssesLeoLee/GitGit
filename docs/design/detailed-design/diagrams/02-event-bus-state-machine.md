# 中心事件总线状态机

> 详细设计：[`../07-app-coordination.md`](../07-app-coordination.md) §7.4 (中心事件总线)
> 关联 REQ：APP-REQ-001/004 (中心事件) / AISEC-REQ-013 (App 沙箱)
> 关联流程：任务 67 (IT) / 83 (ST)

## 事件生命周期

```mermaid
stateDiagram-v2
    [*] --> Persisted: 业务事务
写入 event_stream
    Persisted --> OutboxQueued: 同事务
写入 coord_outbox
    OutboxQueued --> Dispatching: Event Relay
LISTEN/NOTIFY 唤醒
    Dispatching --> Delivered: 订阅者 ACK
    Dispatching --> RetryQueued: 订阅者失败
retry_count++
    RetryQueued --> Dispatching: 等待 next_retry_at
    Dispatching --> DLQ: retry_count > MAX (3)
    RetryQueued --> DLQ: 24h 未成功
    Delivered --> [*]
    DLQ --> ManualReplay: SRE 人工介入
    ManualReplay --> OutboxQueued
```

## 时序图

```mermaid
sequenceDiagram
    actor User
    participant API as Platform API
    participant DB as PostgreSQL
    participant Relay as Event Relay (后台 worker)
    participant App1 as App A
    participant App2 as App B

    User->>API: 触发业务操作 (如 commit push)
    activate API
    API->>DB: BEGIN
    API->>DB: INSERT nodes / edges
    API->>DB: INSERT events
    API->>DB: INSERT coord_outbox
    API->>DB: COMMIT
    deactivate API

    DB->>Relay: NOTIFY event_inserted
    activate Relay
    Relay->>DB: SELECT pending outbox
    Relay->>DB: 查 event_subscriptions (目标 App)
    alt 目标 App A 在线
        Relay->>App1: Webhook / 内部 proc call
        App1-->>Relay: 200 OK
        Relay->>DB: UPDATE outbox SET published_at = now()
    else 目标 App B 失败
        Relay->>App2: 内部 proc call
        App2-->>Relay: 5xx
        Relay->>DB: UPDATE outbox SET retry_count = retry_count + 1
    end
    deactivate Relay
```

## DLQ 触发条件

- `retry_count > 3` (MAX_RETRIES)
- 距首次发布 > 24h
- App 沙箱权限拒绝

## 状态字段

`coord_outbox.state` (隐式，由 published_at + retry_count 推算)：

| 状态 | 条件 |
|---|---|
| Pending | `published_at IS NULL AND retry_count = 0` |
| Dispatching | `published_at IS NULL AND retry_count > 0` |
| Delivered | `published_at IS NOT NULL` |
| DLQ | 已迁入 `event_dlq` 表 |
